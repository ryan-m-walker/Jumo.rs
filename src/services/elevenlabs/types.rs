use futures::stream::{SplitSink, SplitStream};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

#[derive(Debug, Serialize, Deserialize)]
pub struct ElevenLabsTranscription {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElevenLabsSendTextMessage {
    pub text: String,
    pub model_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketInitMessage {
    pub text: String,
    pub voice_settings: VoiceSettings,
    pub xi_api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketTextChunk {
    pub text: String,
    pub flush: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketAudioOutput {
    pub audio: String, // Base64 encoded
}

pub type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
