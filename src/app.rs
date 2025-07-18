use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{
    Terminal,
    layout::Alignment,
    prelude::CrosstermBackend,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph},
};
use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};

use crate::recorder::AudioRecorder;

#[derive(Debug, Clone)]
enum Speaker {
    User,
    Robot,
}

#[derive(Debug, Clone)]
enum AppEvent {
    AudioTranscript((Speaker, String)),
}

#[derive(Debug, Deserialize, Clone)]
struct ElevenLabsTranscription {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeInput {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponseMessage {
    #[serde(rename = "type")]
    message_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponse {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    content: Vec<ClaudeResponseMessage>,
}

#[derive(Debug)]
struct TranscriptLine {
    speaker: Speaker,
    text: String,
}

pub struct App {
    is_app_running: bool,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    audio_recorder: AudioRecorder,
    transcript: Vec<TranscriptLine>,
    app_event_sender: mpsc::Sender<AppEvent>,
    app_event_receiver: mpsc::Receiver<AppEvent>,
}

const FRAMES_PER_SECOND: f32 = 60.0;

impl App {
    pub fn new() -> Self {
        let (app_event_sender, app_event_receiver) = mpsc::channel(100);

        Self {
            terminal: ratatui::init(),
            audio_recorder: AudioRecorder::new(),
            is_app_running: false,
            transcript: Vec::new(),
            app_event_sender,
            app_event_receiver,
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        self.is_app_running = true;

        let period = Duration::from_secs_f32(1.0 / FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while self.is_app_running {
            tokio::select! {
                _ = interval.tick() => self.render()?,
                Some(Ok(event)) = events.next() => self.handle_terminal_event(&event)?,
                Some(event) = self.app_event_receiver.recv() => self.handle_app_event(&event)?,
            }
        }

        ratatui::restore();
        Ok(())
    }

    fn handle_app_event(&mut self, event: &AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::AudioTranscript((speaker, text)) => self.transcript.push(TranscriptLine {
                speaker: speaker.clone(),
                text: text.clone(),
            }),
        }

        Ok(())
    }

    fn handle_terminal_event(&mut self, event: &Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => self.exit(),
                    KeyCode::Char(' ') => self.toggle_recording()?,
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), anyhow::Error> {
        let header = if self.audio_recorder.is_recording() {
            "Recording audio..."
        } else {
            "Press space to start recording audio..."
        };

        let title = Line::from(" Robo RS ".bold());
        let block = Block::bordered()
            .title(title.centered())
            .border_style(Style::new().yellow())
            .border_type(BorderType::Rounded);

        let mut lines = vec![Line::from(header), Line::from("---")];

        for line in self.transcript.iter() {
            match line.speaker {
                Speaker::User => {
                    lines.push(Line::from("[User]:").style(Style::new().yellow()));
                }
                Speaker::Robot => {
                    lines.push(Line::from("[Claude]:").style(Style::new().red()));
                }
            }

            lines.push(Line::from(line.text.clone()));
        }

        let transcript = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(block);

        self.terminal.draw(|frame| {
            frame.render_widget(transcript, frame.area());
        })?;

        Ok(())
    }

    fn exit(&mut self) {
        self.is_app_running = false;
    }

    fn toggle_recording(&mut self) -> Result<(), anyhow::Error> {
        if self.audio_recorder.is_recording() {
            let temp_path = self.audio_recorder.stop()?;
            self.transcribe_audio(temp_path)?;
        } else {
            self.audio_recorder.record()?;
        }

        Ok(())
    }

    fn transcribe_audio(&self, temp_path: TempPath) -> Result<(), anyhow::Error> {
        let elevenlabs_api_key = std::env::var("ELEVENLABS_API_KEY");
        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY");

        let Ok(elevenlabs_api_key) = elevenlabs_api_key else {
            panic!("ELEVENLABS_API_KEY is not set");
        };

        let Ok(anthropic_api_key) = anthropic_api_key else {
            panic!("ANTHROPIC_API_KEY is not set");
        };

        let sender = self.app_event_sender.clone();

        tokio::spawn(async move {
            let mut file = File::open(&temp_path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;

            let file_bytes = reqwest::multipart::Part::bytes(buffer)
                .file_name("recording.wav")
                .mime_str("audio/wav")?;

            let form = reqwest::multipart::Form::new()
                .text("model_id", "scribe_v1")
                .part("file", file_bytes);

            let client = reqwest::Client::new();
            let eleven_labs_resp = client
                .post("https://api.elevenlabs.io/v1/speech-to-text")
                .header("xi-api-key", elevenlabs_api_key)
                .multipart(form)
                .send()
                .await?
                .json::<ElevenLabsTranscription>()
                .await?;

            dbg!(&eleven_labs_resp);

            let text = eleven_labs_resp.text;

            sender
                .send(AppEvent::AudioTranscript((Speaker::User, text.clone())))
                .await?;

            let body = ClaudeInput {
                model: String::from("claude-sonnet-4-20250514"),
                max_tokens: 1024,
                messages: vec![ClaudeMessage {
                    role: String::from("user"),
                    content: text,
                }],
            };

            let claude_resp = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", anthropic_api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?
                .json::<ClaudeResponse>()
                .await?;

            sender
                .send(AppEvent::AudioTranscript((
                    Speaker::Robot,
                    claude_resp.content[0].text.clone(),
                )))
                .await?;

            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }
}
