use cpal::{
    BufferSize, SampleRate, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;

use crate::events::AppEvent;

pub struct AudioPlayer {
    event_sender: mpsc::Sender<AppEvent>,
}

impl AudioPlayer {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self { event_sender }
    }

    pub fn play(&self, audio_bytes: &Bytes, duration_seconds: f64) -> Result<(), anyhow::Error> {
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

        let stream = device.build_output_stream(
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
        )?;

        stream.play()?;
        std::thread::sleep(std::time::Duration::from_secs_f64(duration_seconds + 0.5));

        Ok::<(), anyhow::Error>(())
    }
}
