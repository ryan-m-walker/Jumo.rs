use std::time::Duration;

use cpal::{
    BufferSize, SampleRate, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_util::sync::CancellationToken;

use crate::events::AppEvent;

pub struct AudioPlayer {
    event_sender: mpsc::Sender<AppEvent>,
    output_stream: Option<cpal::Stream>,
    is_running: bool,
    cancel_token: Option<CancellationToken>,
}

impl AudioPlayer {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {
            event_sender,
            output_stream: None,
            is_running: false,
            cancel_token: None,
        }
    }

    pub async fn play(
        &mut self,
        audio_bytes: &Bytes,
        duration_seconds: f64,
    ) -> Result<(), anyhow::Error> {
        self.event_sender
            .send(AppEvent::AudioPlaybackStarted)
            .await?;

        let audio_bytes = audio_bytes.clone();
        let event_sender = self.event_sender.clone();

        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();

        let config = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(44100),
            buffer_size: BufferSize::Default,
        };

        let samples: Vec<f32> = audio_bytes
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / i16::MAX as f32
            })
            .collect();

        let mut sample_index = 0;
        let samples_clone = samples.clone();

        let output_stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for sample in data {
                        *sample = samples_clone.get(sample_index).copied().unwrap_or(0.0);
                        sample_index += 1;
                    }
                },
                move |err| {
                    eprintln!("an error occurred on stream: {}", err);
                },
                None,
            )
            .unwrap();

        self.output_stream = Some(output_stream);
        let stream = self.output_stream.as_mut().unwrap();
        stream.play().unwrap();

        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();

        tokio::spawn(async move {
            let duration_seconds = duration_seconds + 0.5;
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs_f64(duration_seconds + 0.5)) => {
                    event_sender
                        .send(AppEvent::AudioPlaybackCompleted)
                        .await
                        .unwrap();
                }
                _ = cancel_clone.cancelled() => {
                    // playback was cancelled
                }
            }
        });

        self.cancel_token = Some(cancel_token);

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(stream) = &self.output_stream {
            stream.pause().unwrap();
            self.output_stream = None;
        }

        if let Some(token) = &self.cancel_token {
            token.cancel();
            self.cancel_token = None;
        }
    }
}
