use cpal::{
    Stream,
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

enum RecordingEvent {
    Samples(Vec<f32>),
    Stop,
}

pub struct AudioRecorder {
    event_sender: mpsc::Sender<AppEvent>,
    is_recording: Arc<AtomicBool>,
    channels: u16,
    sample_rate: u32,
    samples_tx: Sender<Result<RecordingEvent, String>>,
    samples_rx: Receiver<Result<RecordingEvent, String>>,
    /// keep stream alive to avoid closing device
    stream: Option<Stream>,
}

impl AudioRecorder {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        let (samples_tx, samples_rx) = bounded(50);

        Self {
            event_sender,
            is_recording: Arc::new(AtomicBool::new(false)),
            channels: 2,
            sample_rate: 44100,
            samples_tx,
            samples_rx,
            stream: None,
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let event_sender = self.event_sender.clone();

        let host = cpal::default_host();

        let Some(device) = host.default_input_device() else {
            return Err(anyhow::anyhow!("No default input audio device found"));
        };

        let device_name = device.name().unwrap_or(String::from("<unknown>"));

        let _ = event_sender
            .send(AppEvent::AudioSetInputDevice(device_name))
            .await;

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        self.channels = channels;
        self.sample_rate = sample_rate;

        // Create ring buffer for 2 seconds of audio at sample rate
        let buffer_size = (sample_rate * 2) as usize; // 2 seconds
        let detection_buffer: Arc<Mutex<HeapRb<f32>>> =
            Arc::new(Mutex::new(HeapRb::new(buffer_size)));

        let _volume_threshold = 0.03f32; // Adjust this threshold as needed
        let window_size = 1024; // Analysis window size

        // Debounce mechanism
        let last_event_time = Arc::new(Mutex::new(Instant::now()));
        let cooldown_duration = Duration::from_millis(5);

        let samples_tx = self.samples_tx.clone();
        let err_tx = self.samples_tx.clone();

        let input_stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let detection_buffer_clone = detection_buffer.clone();
                let last_event_time_clone = last_event_time.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        let _ = samples_tx.send(Ok(RecordingEvent::Samples(data.to_vec())));

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
                        let _ = err_tx.send(Err(err.to_string()));
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

        input_stream.play().unwrap();
        self.stream = Some(input_stream);

        Ok(())
    }

    pub fn start_recording(&mut self) {
        if self.is_recording.load(Ordering::Relaxed) {
            return;
        }

        self.is_recording.store(true, Ordering::Relaxed);

        let event_sender = self.event_sender.clone();
        let samples_rx = self.samples_rx.clone();

        let sample_rate = self.sample_rate;
        let channels = self.channels;

        tokio::spawn(async move {
            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::AudioRecordingError(message.to_string()))
                    .await;
            };

            let _ = event_sender.send(AppEvent::AudioRecordingStarted).await;

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
                Err(e) => {
                    let _ = event_sender
                        .send(AppEvent::AudioRecordingFailed(e.to_string()))
                        .await;
                    return;
                }
            };

            for event in samples_rx {
                match event {
                    Ok(RecordingEvent::Samples(data)) => {
                        for sample in data {
                            if let Err(e) = writer.write_sample(sample) {
                                send_error(&format!("Failed to write sample: {e}")).await;
                            }
                        }
                    }
                    Ok(RecordingEvent::Stop) => {
                        break;
                    }
                    Err(err) => {
                        send_error(&format!("Failed to write sample: {err}")).await;
                    }
                }
            }

            if let Err(e) = writer.finalize() {
                let _ = event_sender
                    .send(AppEvent::AudioRecordingFailed(e.to_string()))
                    .await;
                return;
            }

            let _ = event_sender
                .send(AppEvent::AudioRecordingCompleted(buf))
                .await;
        });
    }

    pub fn stop_recording(&mut self) {
        if !self.is_recording.load(Ordering::Relaxed) {
            return;
        }

        self.is_recording.store(false, Ordering::Release);
        let _ = self.samples_tx.send(Ok(RecordingEvent::Stop));
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
}
