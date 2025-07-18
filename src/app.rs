use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{
    Terminal,
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};

use crate::{
    audio::player::AudioPlayer,
    events::EventBus,
    state::{Speaker, TranscriptLine, TranscriptMessage},
};
use crate::{audio::recorder::AudioRecorder, events::AppEvent};
use crate::{
    services::{anthropic::AnthropicService, elevenlabs::ElevenLabsService},
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
struct ElevenLabsSendTextMessage {
    text: String,
    model_id: String,
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
    stream: bool,
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

pub struct App {
    event_bus: EventBus,
    anthropic: AnthropicService,
    elevenlabs: ElevenLabsService,
    audio_recorder: AudioRecorder,
    audio_player: AudioPlayer,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: AppState,
}

const FRAMES_PER_SECOND: f32 = 60.0;

impl App {
    pub fn new() -> Self {
        let event_bus = EventBus::new();
        let anthropic = AnthropicService::new(event_bus.sender());
        let elevenlabs = ElevenLabsService::new(event_bus.sender());
        let audio_recorder = AudioRecorder::new(event_bus.sender());
        let audio_player = AudioPlayer::new(event_bus.sender());

        Self {
            event_bus,
            anthropic,
            elevenlabs,
            audio_recorder,
            audio_player,
            terminal: ratatui::init(),
            state: AppState::default(),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        self.state.is_app_running = true;

        let period = Duration::from_secs_f32(1.0 / FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while self.state.is_app_running {
            tokio::select! {
                _ = interval.tick() => self.render()?,
                Some(Ok(event)) = events.next() => self.handle_terminal_event(&event).await?,
                Some(event) = self.event_bus.recv() => self.handle_app_event(&event).await?,
            }
        }

        ratatui::restore();
        Ok(())
    }

    async fn handle_app_event(&mut self, event: &AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::AudioRecordingStarted => {
                self.state.is_audio_transcription_running = true;
            }
            AppEvent::AudioRecordingCompleted(temp_path) => {
                self.elevenlabs.transcribe(temp_path).await?;
            }
            AppEvent::TranscriptionStarted => {
                self.state.is_audio_transcription_running = true;
            }
            AppEvent::TranscriptionCompleted(text) => {
                self.state.is_audio_transcription_running = false;
                self.anthropic
                    .send_message(text, &self.state.transcript)
                    .await?;

                let message = TranscriptMessage {
                    speaker: Speaker::User,
                    text: text.to_string(),
                };

                self.state
                    .transcript
                    .push(TranscriptLine::TranscriptMessage(message));
            }
            AppEvent::LLMMessageStarted => {
                self.state.is_llm_message_running = true;
            }
            AppEvent::LLMMessageCompleted(text) => {
                self.state.is_llm_message_running = false;
                self.elevenlabs.synthesize(text).await?;

                let message = TranscriptMessage {
                    speaker: Speaker::Assistant,
                    text: text.to_string(),
                };

                self.state
                    .transcript
                    .push(TranscriptLine::TranscriptMessage(message));
            }
            AppEvent::TTSStarted => {
                self.state.is_tts_running = true;
            }
            AppEvent::TTSCompleted(result) => {
                self.audio_player
                    .play(&result.audio_bytes, result.duration_seconds)?;
            }
            _ => {
                panic!("Unhandled app event: {:?}", event);
            }
        }

        Ok(())
    }

    async fn handle_terminal_event(&mut self, event: &Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => self.exit(),
                    KeyCode::Char(' ') => self.toggle_recording().await?,
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), anyhow::Error> {
        self.terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Fill(1)].as_ref())
                .split(frame.area());

            let title = Line::from(" Transcript ".bold());
            let block = Block::bordered()
                .title(title.centered())
                .border_style(Style::new().yellow())
                .border_type(BorderType::Rounded);

            let header_text = if self.audio_recorder.is_recording() {
                "Recording audio..."
            } else if self.state.is_audio_transcription_running {
                "Transcribing audio..."
            } else if self.state.is_llm_message_running {
                "Sending message to LLM..."
            } else {
                "Press space to start recording audio..."
            };

            let header = Paragraph::new(header_text).style(Style::new().dim()).block(
                Block::bordered()
                    .border_style(Style::new().yellow())
                    .border_type(BorderType::Rounded),
            );

            let mut lines = vec![];

            for line in self.state.transcript.iter() {
                match line {
                    TranscriptLine::TranscriptMessage(line) => {
                        match line.speaker {
                            Speaker::User => {
                                lines.push(Line::from("[User]:").style(Style::new().yellow()));
                            }
                            Speaker::Assistant => {
                                lines.push(Line::from("[Claude]:").style(Style::new().red()));
                            }
                        }

                        lines.push(Line::from(line.text.clone()));
                    }
                    TranscriptLine::TranscriptError(line) => {
                        lines.push(Line::from(line.text.clone()));
                    }
                }
            }

            let transcript = Paragraph::new(lines)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true })
                .block(block);

            frame.render_widget(header, chunks[0]);
            frame.render_widget(transcript, chunks[1]);
        })?;

        Ok(())
    }

    fn exit(&mut self) {
        self.state.is_app_running = false;
    }

    async fn toggle_recording(&mut self) -> Result<(), anyhow::Error> {
        if self.audio_recorder.is_recording() {
            self.audio_recorder.stop().await?;
        } else {
            self.audio_recorder.start().await?;
        }

        Ok(())
    }
}
