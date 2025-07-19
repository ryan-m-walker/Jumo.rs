use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};

use crate::events::{AppEvent, TTSResult};

// kota:
const VOICE_ID: &str = "pvxGJdhknm00gMyYHtET";
// archer:
// const VOICE_ID: &str = "Fahco4VZzobUeiPqni1S";
const OUTPUT_FORMAT: &str = "pcm_44100";
const TTS_MODEL_ID: &str = "eleven_multilingual_v2";

#[derive(Debug, Serialize, Deserialize)]
struct ElevenLabsTranscription {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ElevenLabsSendTextMessage {
    text: String,
    model_id: String,
}

#[derive(Debug)]
pub struct ElevenLabsService {
    event_sender: mpsc::Sender<AppEvent>,
    client: reqwest::Client,
    api_key: String,
    is_running: bool,
}

impl ElevenLabsService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        let Ok(api_key) = std::env::var("ELEVENLABS_API_KEY") else {
            panic!("ELEVENLABS_API_KEY is not set");
        };

        Self {
            event_sender,
            api_key,
            client: reqwest::Client::new(),
            is_running: false,
        }
    }

    pub async fn transcribe(&mut self, audio_path: &TempPath) -> Result<(), anyhow::Error> {
        self.event_sender
            .send(AppEvent::TranscriptionStarted)
            .await?;

        let event_sender = self.event_sender.clone();
        let api_key = self.api_key.clone();

        let client = reqwest::Client::new();
        let mut file = File::open(audio_path).await.unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.unwrap();

        tokio::spawn(async move {
            let file_bytes = reqwest::multipart::Part::bytes(buffer)
                .file_name("recording.wav")
                .mime_str("audio/wav");

            let Ok(file_bytes) = file_bytes else {
                let message = String::from("Failed to create file bytes");
                event_sender
                    .send(AppEvent::TTSFailed(message))
                    .await
                    .unwrap();
                return;
            };

            let form = reqwest::multipart::Form::new()
                .text("model_id", "scribe_v1")
                .part("file", file_bytes);

            let resp = client
                .post("https://api.elevenlabs.io/v1/speech-to-text")
                .header("xi-api-key", api_key)
                .multipart(form)
                .send()
                .await;

            let Ok(resp) = resp else {
                let message = String::from("Failed to send audio to ElevenLabs");
                event_sender
                    .send(AppEvent::TTSFailed(message))
                    .await
                    .unwrap();
                return;
            };

            let json = resp.json::<ElevenLabsTranscription>().await;
            let Ok(json) = json else {
                let message = String::from("Failed to get JSON from ElevenLabs");
                event_sender
                    .send(AppEvent::TTSFailed(message))
                    .await
                    .unwrap();
                return;
            };

            let text = json.text;

            event_sender
                .send(AppEvent::TranscriptionCompleted(text))
                .await
                .unwrap();
        });

        Ok(())
    }

    pub async fn synthesize(&mut self, text: &str) -> Result<(), anyhow::Error> {
        self.is_running = true;
        self.event_sender.send(AppEvent::TTSStarted).await?;

        let event_sender = self.event_sender.clone();
        let api_key = self.api_key.clone();
        let text = text.to_string();

        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let body = ElevenLabsSendTextMessage {
                text,
                model_id: String::from(TTS_MODEL_ID),
            };

            let url = format!(
                "https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}?output_format={OUTPUT_FORMAT}"
            );

            let speech_resp = client
                .post(url)
                .header("xi-api-key", &api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await;

            let Ok(speech_resp) = speech_resp else {
                let message = String::from("Failed to send text to elevenlabs");
                event_sender
                    .send(AppEvent::TTSFailed(message))
                    .await
                    .unwrap();
                return;
            };

            if let Some(content_type) = speech_resp.headers().get("content-type") {
                // TODO: error handling
            }

            let Ok(audio_bytes) = speech_resp.bytes().await else {
                let message = String::from("Failed to get audio bytes from elevenlabs");
                event_sender
                    .send(AppEvent::TTSFailed(message))
                    .await
                    .unwrap();
                return;
            };

            let bytes_per_sample = 2; // 16-bit = 2 bytes
            let channels = 1;
            let sample_rate = 44100;
            let duration_seconds =
                audio_bytes.len() as f64 / (bytes_per_sample * channels * sample_rate) as f64;

            event_sender
                .send(AppEvent::TTSCompleted(TTSResult {
                    audio_bytes,
                    duration_seconds,
                }))
                .await
                .unwrap();
        });

        Ok(())
    }

    pub fn cancel(&mut self) {
        self.is_running = false;
    }
}
