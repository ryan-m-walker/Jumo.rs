use std::sync::{Arc, Mutex, mpsc};

use cpal::{
    BufferSize, SampleRate, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer},
};
use tokio_tungstenite::tungstenite::Bytes;

use crate::events::AppEvent;

type AudioBuffer = Arc<Mutex<HeapRb<f32>>>;

pub struct AudioPlayer {
    event_sender: tokio::sync::mpsc::Sender<AppEvent>,
    output_stream: Option<cpal::Stream>,
    buffer: AudioBuffer,
}

impl AudioPlayer {
    pub fn new(event_sender: tokio::sync::mpsc::Sender<AppEvent>) -> Self {
        let buffer_size = 48000 * 100;

        Self {
            event_sender,
            output_stream: None,
            buffer: Arc::new(Mutex::new(HeapRb::new(buffer_size))),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let host = cpal::default_host();

        let Some(device) = host.default_output_device() else {
            return Err(anyhow::anyhow!("No audio device found"));
        };

        let device_name = device.name().unwrap_or(String::from("<unknown>"));

        self.event_sender
            .send(AppEvent::AudioSetOutputDevice(device_name))
            .await?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(44100),
            buffer_size: BufferSize::Default,
        };

        let (err_tx, err_rx) = mpsc::channel();

        let buffer = self.buffer.clone();

        let output_stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                let Ok(mut buffer) = buffer.lock() else {
                    return;
                };

                for sample in data.iter_mut() {
                    *sample = buffer.try_pop().unwrap_or(0.0);
                }
            },
            move |err| {
                let _ = err_tx.send(err);
            },
            None,
        )?;

        let event_sender = self.event_sender.clone();
        tokio::spawn(async move {
            for err in err_rx {
                let _ = event_sender
                    .send(AppEvent::AudioPlaybackError(err.to_string()))
                    .await;
            }
        });

        self.output_stream = Some(output_stream);

        if let Some(stream) = self.output_stream.as_mut() {
            stream.play()?;
        }

        Ok(())
    }

    pub fn push_audio_chunk(&mut self, audio_bytes: &Bytes) -> Result<(), anyhow::Error> {
        let samples: Vec<f32> = audio_bytes
            .chunks_exact(2)
            .map(|chunk| {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                sample as f32 / i16::MAX as f32
            })
            .collect();

        if let Ok(mut buf) = self.buffer.lock() {
            buf.push_slice(&samples);
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }
    }
}
