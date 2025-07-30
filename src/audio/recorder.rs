use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::bounded;
use hound::{WavSpec, WavWriter};
use tempfile::NamedTempFile;
use tokio::sync::mpsc;

use crate::events::AppEvent;

pub struct AudioRecorder {
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
            event_sender,
            input_stream: None,
            temp_file: None,
            sample_rate: 0,
            channels: 0,
            is_playing: false,
        }
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
