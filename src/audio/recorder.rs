use cpal::{
    SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, bounded};
use hound::{WavSpec, WavWriter};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, RingBuffer},
};
use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

use crate::events::AppEvent;

pub struct AudioRecorder {
    event_sender: mpsc::Sender<AppEvent>,
    input_stream: Option<cpal::Stream>,

    samples_tx: Sender<Vec<f32>>,
    samples_rx: Receiver<Vec<f32>>,

    sample_rate: u32,
    channels: u16,

    is_recording: Arc<AtomicBool>,
}

impl AudioRecorder {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        let (samples_tx, samples_rx) = bounded::<Vec<f32>>(50);

        Self {
            event_sender,
            input_stream: None,
            samples_tx,
            samples_rx,
            sample_rate: 0,
            channels: 0,
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let host = cpal::default_host();

        let Some(device) = host.default_input_device() else {
            return Err(anyhow::anyhow!("No default input audio device found"));
        };

        let device_name = device.name().unwrap_or(String::from("<unknown>"));

        self.event_sender
            .send(AppEvent::AudioSetInputDevice(device_name))
            .await?;

        let config = device.default_input_config()?;

        // Create ring buffer for 2 seconds of audio at sample rate
        let sample_rate = config.sample_rate().0;
        let buffer_size = (sample_rate * 2) as usize; // 2 seconds
        let detection_buffer: Arc<Mutex<HeapRb<f32>>> =
            Arc::new(Mutex::new(HeapRb::new(buffer_size)));

        let channels = config.channels();

        self.channels = channels;
        self.sample_rate = sample_rate;

        let event_sender = self.event_sender.clone();
        let _volume_threshold = 0.03f32; // Adjust this threshold as needed
        let window_size = 1024; // Analysis window size

        let tx = self.samples_tx.clone();

        // Debounce mechanism
        let last_event_time = Arc::new(Mutex::new(Instant::now()));
        let cooldown_duration = Duration::from_millis(5);

        let input_stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let detection_buffer_clone = detection_buffer.clone();
                let last_event_time_clone = last_event_time.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        let _ = tx.send(data.to_vec());

                        if let Ok(mut buf) = detection_buffer_clone.lock() {
                            // ---------- Volume monitoring ----------
                            for &sample in data {
                                buf.push_overwrite(sample);
                            }

                            if buf.occupied_len() >= window_size {
                                let mut sum_squares = 0.0;
                                let sample_count = window_size.min(buf.occupied_len());

                                let buffer_data: Vec<f32> = buf
                                    .as_slices()
                                    .0
                                    .iter()
                                    .chain(buf.as_slices().1.iter())
                                    .rev() // Get most recent samples first
                                    .take(sample_count)
                                    .copied()
                                    .collect();

                                for sample in buffer_data {
                                    sum_squares += sample * sample;
                                }

                                if sample_count > 0 {
                                    let rms_volume = (sum_squares / sample_count as f32).sqrt();
                                    let now = Instant::now();

                                    // debounce
                                    if let Ok(mut last_time) = last_event_time_clone.lock() {
                                        if now.duration_since(*last_time) >= cooldown_duration {
                                            let _ = event_sender
                                                .try_send(AppEvent::AudioDetected(rms_volume));
                                            *last_time = now;
                                        }
                                    }
                                }
                            }
                        }
                    },
                    move |err| {
                        // TODO: error handling like player
                        eprintln!("Audio input error: {err}");
                    },
                    None,
                )?
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported sample format: {:?}",
                    config.sample_format()
                ));
            }
        };

        input_stream.play()?;
        self.input_stream = Some(input_stream);

        Ok(())
    }

    pub fn start_recording(&mut self) {
        self.is_recording.store(true, Ordering::Relaxed);

        let channels = self.channels;
        let sample_rate = self.sample_rate;

        let event_sender = self.event_sender.clone();
        let is_recording = self.is_recording.clone();
        let rx = self.samples_rx.clone();

        tokio::spawn(async move {
            let _ = event_sender.send(AppEvent::AudioRecordingStarted).await;

            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::AudioRecordingError(message.to_string()))
                    .await;
            };

            let spec = WavSpec {
                channels,
                sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            let mut buf = Vec::new();
            let cursor = Cursor::new(&mut buf);

            let mut writer = match WavWriter::new(cursor, spec) {
                Ok(writer) => writer,
                Err(err) => {
                    send_error(&format!("Failed to create wav writer: {err}")).await;
                    return;
                }
            };

            for data in rx {
                for sample in data {
                    if !is_recording.load(Ordering::Relaxed) {
                        break;
                    }

                    if let Err(e) = writer.write_sample(sample) {
                        send_error(&format!("Failed to write sample: {e}")).await;
                    }
                }
            }

            if let Err(e) = writer.finalize() {
                let _ = event_sender
                    .send(AppEvent::AudioRecordingFailed(e.to_string()))
                    .await;
                return;
            }

            let _ = event_sender.send(AppEvent::AudioRecordingEnded(buf)).await;
        });
    }

    pub fn stop_recording(&mut self) {
        self.is_recording.store(false, Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
}
