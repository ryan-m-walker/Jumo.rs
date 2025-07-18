use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};

use crate::events::{AppEvent, TTSResult};

const VOICE_ID: &str = "Fahco4VZzobUeiPqni1S";
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
        }
    }

    pub async fn transcribe(&self, audio_path: &TempPath) -> Result<(), anyhow::Error> {
        self.event_sender
            .send(AppEvent::TranscriptionStarted)
            .await?;

        let mut file = File::open(&audio_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let file_bytes = reqwest::multipart::Part::bytes(buffer)
            .file_name("recording.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .text("model_id", "scribe_v1")
            .part("file", file_bytes);

        let eleven_labs_resp = self
            .client
            .post("https://api.elevenlabs.io/v1/speech-to-text")
            .header("xi-api-key", &self.api_key)
            .multipart(form)
            .send()
            .await?
            .json::<ElevenLabsTranscription>()
            .await?;

        let text = eleven_labs_resp.text;

        self.event_sender
            .send(AppEvent::TranscriptionCompleted(text))
            .await?;
        Ok(())
    }

    pub async fn synthesize(&self, text: &str) -> Result<(), anyhow::Error> {
        self.event_sender.send(AppEvent::TTSStarted).await?;

        let body = ElevenLabsSendTextMessage {
            text: text.to_string(),
            model_id: String::from(TTS_MODEL_ID),
        };

        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{VOICE_ID}?output_format={OUTPUT_FORMAT}"
        );

        let speech_resp = self
            .client
            .post(url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if let Some(content_type) = speech_resp.headers().get("content-type") {
            // TODO: error handling
        }

        let audio_bytes = speech_resp.bytes().await?;

        let bytes_per_sample = 2; // 16-bit = 2 bytes
        let channels = 1;
        let sample_rate = 44100;
        let duration_seconds =
            audio_bytes.len() as f64 / (bytes_per_sample * channels * sample_rate) as f64;

        self.event_sender
            .send(AppEvent::TTSCompleted(TTSResult {
                audio_bytes,
                duration_seconds,
            }))
            .await?;

        Ok(())
    }
}
