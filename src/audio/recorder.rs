use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::bounded;
use hound::{WavSpec, WavWriter};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, RingBuffer},
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tokio::sync::mpsc;

use crate::events::AppEvent;

pub struct AudioRecorder {
    is_running: bool,
    event_sender: mpsc::Sender<AppEvent>,
    input_stream: Option<cpal::Stream>,
    temp_file: Option<NamedTempFile>,
    sample_rate: u32,
    channels: u16,
    is_playing: bool,
}

impl AudioRecorder {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {
            is_running: false,
            event_sender,
            input_stream: None,
            temp_file: None,
            sample_rate: 0,
            channels: 0,
            is_playing: false,
        }
    }

    pub async fn _start(&mut self) -> Result<(), anyhow::Error> {
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

        let event_sender = self.event_sender.clone();
        let volume_threshold = 0.03f32; // Adjust this threshold as needed
        let window_size = 1024; // Analysis window size

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
                        if let Ok(mut buf) = detection_buffer_clone.lock() {
                            // Add new samples to ring buffer
                            for &sample in data {
                                buf.push_overwrite(sample);
                            }

                            if buf.occupied_len() >= window_size {
                                let mut sum_squares = 0.0;
                                let sample_count = window_size.min(buf.occupied_len());

                                // Calculate RMS by reading from the buffer without modifying it
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

                                    // Check if cooldown period has passed
                                    if let Ok(mut last_time) = last_event_time_clone.lock() {
                                        if now.duration_since(*last_time) >= cooldown_duration {
                                            let _ = event_sender
                                                .try_send(AppEvent::AudioDetected(rms_volume));

                                            // if rms_volume > volume_threshold {
                                            //     let _ = event_sender
                                            //         .try_send(AppEvent::AudioDetected(rms_volume));
                                            // } else {
                                            //     let _ = event_sender
                                            //         .try_send(AppEvent::AudioNotDetected);
                                            // }
                                            *last_time = now;
                                        }
                                    }
                                }
                            }
                        }
                    },
                    move |err| {
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

        // Store stream to keep it alive
        self.input_stream = Some(input_stream);

        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.is_playing {
            return Ok(());
        }

        self.is_playing = true;

        self.event_sender
            .send(AppEvent::AudioRecordingStarted)
            .await?;

        let (tx, rx) = bounded(50);

        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_owned();
        self.temp_file = Some(temp_file);

        let host = cpal::default_host();

        let Some(device) = host.default_input_device() else {
            return Err(anyhow::anyhow!("No default input device found"));
        };

        let device_name = device.name().unwrap_or(String::from("<unknown>"));

        self.event_sender
            .send(AppEvent::AudioSetInputDevice(device_name))
            .await?;

        let config = device.default_input_config()?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        let input_stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    let _ = tx.send(data.to_vec());
                },
                move |err| {
                    eprintln!("an error occurred on the input audio stream: {err}");
                },
                None,
            )?,
            _ => panic!("Unsupported sample format: {:?}", config.sample_format()),
        };

        self.input_stream = Some(input_stream);

        let Some(stream) = self.input_stream.as_mut() else {
            return Err(anyhow::anyhow!("Input stream is None"));
        };

        let channels = self.channels;
        let sample_rate = self.sample_rate;

        stream.play()?;

        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
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

            let mut writer = match WavWriter::create(&temp_path, spec) {
                Ok(writer) => writer,
                Err(e) => {
                    send_error(&format!("Failed to create WAV writer: {e}")).await;
                    return;
                }
            };

            for data in rx {
                for sample in data {
                    if let Err(e) = writer.write_sample(sample) {
                        send_error(&format!("Failed to write sample: {e}")).await;
                    }
                }
            }

            if let Err(e) = writer.finalize() {
                send_error(&format!("Failed to finalize WAV file: {e}")).await;
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(stream) = &self.input_stream {
            // Wait for the recording to finish
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            stream.pause()?;
            self.is_playing = false;
        }

        self.input_stream = None;

        let Some(temp_file) = self.temp_file.take() else {
            self.event_sender
                .send(AppEvent::AudioRecordingFailed(String::from(
                    "No temp file found",
                )))
                .await?;

            return Ok(());
        };

        let path = temp_file.into_temp_path();

        self.event_sender
            .send(AppEvent::AudioRecordingCompleted(path))
            .await?;

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.input_stream.is_some()
    }
}
