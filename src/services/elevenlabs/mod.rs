use base64::{Engine, prelude::BASE64_STANDARD};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::services::elevenlabs::types::WebSocketEndMessage;
use crate::services::elevenlabs::voices::{Voice, get_voice_id};
use crate::{
    events::AppEvent,
    services::elevenlabs::types::{
        ElevenLabsTranscription, VoiceSettings, WebSocketAudioOutput, WebSocketInitMessage,
        WebSocketTextChunk, WsSink, WsStream,
    },
};

mod types;
pub mod voices;

const OUTPUT_FORMAT: &str = "pcm_44100";

#[derive(Debug)]
pub struct ElevenLabsService {
    event_sender: mpsc::Sender<AppEvent>,
    ws_sink: Option<WsSink>,
    ws_stream: Option<WsStream>,
}

impl ElevenLabsService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {
            event_sender,
            ws_sink: None,
            ws_stream: None,
        }
    }

    pub async fn start_stream(&mut self) -> Result<(), anyhow::Error> {
        if self.ws_stream.is_some() {
            return Ok(());
        }

        let Ok(api_key) = std::env::var("ELEVENLABS_API_KEY") else {
            let message = String::from("ELEVENLABS_API_KEY is not set");
            self.event_sender.send(AppEvent::TTSFailed(message)).await?;
            return Ok(());
        };

        let voice_id = get_voice_id(Voice::Flynn);

        let url = format!(
            "wss://api.elevenlabs.io/v1/text-to-speech/{voice_id}/stream-input?output_format={OUTPUT_FORMAT}"
        );
        let mut request = url.into_client_request()?;
        request.headers_mut().insert("xi-api-key", api_key.parse()?);

        let Ok((ws_stream, _)) = connect_async(request).await else {
            let message = String::from("Failed to connect to elevenlabs");
            self.event_sender.send(AppEvent::TTSFailed(message)).await?;
            return Ok(());
        };

        let (mut ws_sink, mut ws_stream) = ws_stream.split();

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

        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::TTSFailed(message.to_string()))
                    .await;
            };

            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        let json = match serde_json::from_str::<WebSocketAudioOutput>(&text) {
                            Ok(json) => json,
                            Err(e) => {
                                send_error(&format!("Failed to deserialize JSON: {e}")).await;
                                return;
                            }
                        };

                        if let Some(audio) = json.audio {
                            let decoded = match BASE64_STANDARD.decode(audio) {
                                Ok(decoded) => decoded,
                                Err(e) => {
                                    send_error(&format!("Failed to decode audio: {e}")).await;
                                    return;
                                }
                            };

                            let audio_bytes = Bytes::from(decoded);
                            let _ = event_sender.send(AppEvent::TTSChunk(audio_bytes)).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        break;
                    }
                    Ok(msg) => {
                        send_error(&format!("Unexpected message type: {msg}")).await;
                    }
                    Err(e) => {
                        send_error(&format!("WebSocket error: {e}")).await;
                        break;
                    }
                }
            }
        });

        Ok(())
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

    pub async fn end_stream(&mut self) -> Result<(), anyhow::Error> {
        if let Some(ws_sink) = &mut self.ws_sink {
            let text_chunk = WebSocketEndMessage {
                text: "".to_string(),
            };
            let json = serde_json::to_string(&text_chunk)?;
            ws_sink.send(Message::Text(json.into())).await?;
            self.ws_sink = None;
        }
        Ok(())
    }

    pub async fn transcribe(&mut self, buffer: Vec<u8>) -> Result<(), anyhow::Error> {
        let Ok(api_key) = std::env::var("ELEVENLABS_API_KEY") else {
            let message = String::from("ELEVENLABS_API_KEY is not set");
            self.event_sender.send(AppEvent::TTSFailed(message)).await?;
            return Ok(());
        };

        self.event_sender
            .send(AppEvent::TranscriptionStarted)
            .await?;

        let client = reqwest::Client::new();

        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            let send_error = async |message: &str| {
                let _ = event_sender
                    .send(AppEvent::TranscriptionFailed(message.to_string()))
                    .await;
            };

            let file_bytes = reqwest::multipart::Part::bytes(buffer)
                .file_name("recording.wav")
                .mime_str("audio/wav");

            let Ok(file_bytes) = file_bytes else {
                send_error("Failed to create file bytes").await;
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
                send_error("Failed to send audio to ElevenLabs").await;
                return;
            };

            let json = resp.json::<ElevenLabsTranscription>().await;

            let Ok(json) = json else {
                send_error("Failed to get JSON from ElevenLabs").await;
                return;
            };

            let text = json.text;

            let _ = event_sender
                .send(AppEvent::TranscriptionCompleted(text))
                .await;
        });

        Ok(())
    }

    pub fn cancel(&mut self) {
        // TODO: cancel requests
    }
}
