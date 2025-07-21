use futures::{SinkExt, StreamExt};
use tempfile::TempPath;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::{
    events::{AppEvent, TTSResult},
    services::elevenlabs::{
        types::{
            ElevenLabsSendTextMessage, ElevenLabsTranscription, VoiceSettings,
            WebSocketInitMessage, WebSocketTextChunk, WsSink, WsStream,
        },
        voices::JULES_VOICE_ID,
    },
};

mod types;
mod voices;

const OUTPUT_FORMAT: &str = "pcm_44100";
const TTS_MODEL_ID: &str = "eleven_multilingual_v2";

#[derive(Debug)]
pub struct ElevenLabsService {
    event_sender: mpsc::Sender<AppEvent>,
    api_key: String,
    is_running: bool,
    ws_sink: Option<WsSink>,
    ws_stream: Option<WsStream>,
}

impl ElevenLabsService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        let Ok(api_key) = std::env::var("ELEVENLABS_API_KEY") else {
            panic!("ELEVENLABS_API_KEY is not set");
        };

        Self {
            event_sender,
            api_key,
            is_running: false,
            ws_sink: None,
            ws_stream: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), anyhow::Error> {
        let api_key = self.api_key.clone();

        let url = format!(
            "wss://api.elevenlabs.io/v1/text-to-speech/{JULES_VOICE_ID}/stream-input?output_format={OUTPUT_FORMAT}"
        );
        let mut request = url.into_client_request()?;
        request.headers_mut().insert("xi-api-key", api_key.parse()?);

        let Ok((ws_stream, _)) = connect_async(request).await else {
            panic!("Failed to connect to elevenlabs");
        };

        let (mut ws_sink, ws_stream) = ws_stream.split();

        let init_message = WebSocketInitMessage {
            text: " ".to_string(), // Space to initialize
            voice_settings: VoiceSettings {
                stability: 0.5,
                similarity_boost: 0.8,
            },
            xi_api_key: api_key.clone(),
        };

        let init_json = serde_json::to_string(&init_message)?;
        ws_sink.send(Message::Text(init_json.into())).await?;

        self.ws_sink = Some(ws_sink);
        self.ws_stream = Some(ws_stream);

        self.start_read_loop();

        Ok(())
    }

    fn start_read_loop(&mut self) {
        if let Some(mut ws_stream) = self.ws_stream.take() {
            let event_sender = self.event_sender.clone();

            tokio::spawn(async move {
                while let Some(msg) = ws_stream.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // let json: WebSocketAudioOutput = serde_json::from_str(&text).unwrap();
                            // let audio_bytes = BASE64_STANDARD.decode(json.audio).unwrap().bytes();
                        }
                        Ok(Message::Close(_)) => {
                            break;
                        }
                        Ok(msg) => {
                            panic!("Unexpected message type: {:?}", msg);
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
            });
        }
    }

    pub async fn send_text(&mut self, text: &str) -> Result<(), anyhow::Error> {
        if let Some(ws_sink) = &mut self.ws_sink {
            let text_chunk = WebSocketTextChunk {
                text: text.to_string(),
                flush: Some(true),
            };
            let json = serde_json::to_string(&text_chunk)?;
            ws_sink.send(Message::Text(json.into())).await?;
        }
        Ok(())
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
                "https://api.elevenlabs.io/v1/text-to-speech/{JULES_VOICE_ID}?output_format={OUTPUT_FORMAT}"
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

            // TODO: check header content type to ensure successful response

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
