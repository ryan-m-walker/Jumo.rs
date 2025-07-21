use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};
use uuid::Uuid;

use crate::{
    audio::player::AudioPlayer,
    database::{
        Database,
        models::{Message, MessageContent, MessageType},
    },
    events::EventBus,
    text_processor::TextProcessor,
    widgets::main::MainWidget,
};
use crate::{audio::recorder::AudioRecorder, events::AppEvent};
use crate::{
    services::{anthropic::AnthropicService, elevenlabs::ElevenLabsService},
    state::AppState,
};

pub struct App {
    db: Database,
    event_bus: EventBus,
    anthropic: AnthropicService,
    elevenlabs: ElevenLabsService,
    audio_recorder: AudioRecorder,
    audio_player: AudioPlayer,
    text_processor: TextProcessor,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: AppState,
}

const FRAMES_PER_SECOND: f32 = 60.0;

impl App {
    pub fn new(terminal: Terminal<CrosstermBackend<Stdout>>) -> Self {
        let db = Database::new();
        let event_bus = EventBus::new();
        let anthropic = AnthropicService::new(event_bus.sender());
        let elevenlabs = ElevenLabsService::new(event_bus.sender());
        let audio_recorder = AudioRecorder::new(event_bus.sender());
        let audio_player = AudioPlayer::new(event_bus.sender());
        let text_processor = TextProcessor::new(event_bus.sender());

        Self {
            terminal,
            event_bus,
            db,
            anthropic,
            elevenlabs,
            audio_recorder,
            audio_player,
            text_processor,
            state: AppState::default(),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        self.state.is_app_running = true;

        self.db.init()?;
        self.elevenlabs.connect().await?;

        let messages = self.db.get_messages()?;
        self.state.messages = messages;

        let period = Duration::from_secs_f32(1.0 / FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while self.state.is_app_running {
            tokio::select! {
                _ = interval.tick() => self.render()?,
                Some(event) = self.event_bus.recv() => self.handle_app_event(&event).await?,
                Some(Ok(event)) = events.next() => self.handle_terminal_event(&event).await?,
            }
        }

        Ok(())
    }

    async fn handle_app_event(&mut self, event: &AppEvent) -> Result<(), anyhow::Error> {
        match event {
            // audio events
            AppEvent::AudioRecordingStarted => {
                self.state.is_audio_recording_running = true;
            }
            AppEvent::AudioRecordingCompleted(temp_path) => {
                self.state.is_audio_recording_running = false;

                if let Err(error) = self.elevenlabs.transcribe(temp_path).await {
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::AudioRecordingFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_recording_running = false;
            }
            AppEvent::AudioPlaybackStarted => {
                self.state.is_audio_playback_running = true;
            }
            AppEvent::AudioPlaybackCompleted => {
                self.state.is_audio_playback_running = false;
            }
            AppEvent::AudioPlaybackFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_playback_running = false;
            }

            // transcription events
            AppEvent::TranscriptionStarted => {
                self.state.is_audio_transcription_running = true;
            }
            AppEvent::TranscriptionCompleted(text) => {
                self.state.is_audio_transcription_running = false;

                let message = Message {
                    id: Uuid::new_v4().to_string(),
                    message_type: MessageType::User,
                    content: MessageContent::User {
                        text: text.to_string(),
                    },
                    created_at: None,
                };

                self.db.insert_message(&message)?;
                self.state.messages.push(message);

                let result = self
                    .anthropic
                    .send_message(text, &self.state.messages)
                    .await;

                if let Err(error) = result {
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::TranscriptionFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_transcription_running = false;
            }

            // llm events
            AppEvent::LLMMessageStarted(payload) => {
                self.state.is_llm_message_running = true;

                let message = Message {
                    id: payload.message_id.clone(),
                    message_type: MessageType::Assistant,
                    content: MessageContent::Assistant {
                        text: "".to_string(),
                    },
                    created_at: None,
                };

                self.state.messages.push(message);
            }
            AppEvent::LLMMessageDelta(payload) => {
                self.state.on_llm_text_delta(payload);
                self.text_processor.process_delta(&payload.text).await?;
            }
            AppEvent::LLMMessageCompleted(payload) => {
                self.state.is_llm_message_running = false;

                if let Some(message) = self.state.get_message(&payload.message_id) {
                    self.db.insert_message(message)?;
                }

                self.text_processor.finalize().await?;

                // if let Err(error) = self.elevenlabs.synthesize(&payload.full_text).await {
                //     self.state.error = Some(error.to_string());
                // };
            }
            AppEvent::LLMRequestFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_llm_message_running = false;
            }

            AppEvent::TextProcessorChunk(payload) => {
                // self.elevenlabs.send_text(&payload.text).await?;
            }

            // tts events
            AppEvent::TTSStarted => {
                self.state.is_tts_running = true;
            }
            AppEvent::TTSCompleted(result) => {
                self.state.is_tts_running = false;

                let result = self
                    .audio_player
                    .play(&result.audio_bytes, result.duration_seconds)
                    .await;

                if let Err(error) = result {
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::TTSFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }
        }

        Ok(())
    }

    async fn handle_terminal_event(&mut self, event: &Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Char('q') => self.quit(),
                    KeyCode::Esc => self.cancel(),
                    KeyCode::Char(' ') => self.toggle_recording().await?,
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), anyhow::Error> {
        self.terminal
            .draw(|frame| frame.render_widget(MainWidget::new(&self.state), frame.area()))?;
        Ok(())
    }

    fn quit(&mut self) {
        self.cancel();
        self.state.is_app_running = false;
    }

    fn cancel(&mut self) {
        self.audio_player.stop();
        self.state.is_audio_playback_running = false;

        self.anthropic.cancel();
        self.state.is_llm_message_running = false;

        self.elevenlabs.cancel();
        self.state.is_audio_transcription_running = false;
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
