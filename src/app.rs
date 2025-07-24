use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};
use uuid::Uuid;

use crate::{
    audio::player::AudioPlayer,
    database::{
        Database,
        models::{
            log::{Log, LogLevel},
            message::{Message, MessageContent, MessageType},
        },
    },
    events::EventBus,
    state::View,
    text_processor::TextProcessor,
    widgets::app_layout::AppLayout,
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
        self.audio_player.start().await?;

        let messages = self.db.get_messages()?;
        self.state.messages = messages;

        self.state.log(Log::new("App started", LogLevel::Info));

        let period = Duration::from_secs_f32(1.0 / FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while self.state.is_app_running {
            tokio::select! {
                _ = interval.tick() => self.render()?,
                Some(event) = self.event_bus.recv() => self.handle_app_event(event).await?,
                Some(Ok(event)) = events.next() => self.handle_terminal_event(&event).await?,
            }
        }

        Ok(())
    }

    async fn handle_app_event(&mut self, event: AppEvent) -> Result<(), anyhow::Error> {
        match event {
            // audio events
            AppEvent::AudioRecordingStarted => {
                self.state
                    .log(Log::new("Audio recording started", LogLevel::Info));
                self.state.is_audio_recording_running = true;
            }
            AppEvent::AudioRecordingCompleted(temp_path) => {
                self.state
                    .log(Log::new("Audio recording completed", LogLevel::Info));
                self.state.is_audio_recording_running = false;

                if let Err(error) = self.elevenlabs.transcribe(&temp_path).await {
                    self.state.log(Log::new(
                        &format!("Audio recording failed: {error}"),
                        LogLevel::Error,
                    ));
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::AudioRecordingFailed(error) => {
                self.state.log(Log::new(
                    &format!("Audio recording failed: {error}"),
                    LogLevel::Error,
                ));
                self.state.error = Some(error.to_string());
                self.state.is_audio_recording_running = false;
            }
            AppEvent::AudioPlaybackError(error) => {
                self.state.log(Log::new(
                    &format!("Audio playback error: {error}"),
                    LogLevel::Error,
                ));
                self.state.error = Some(error.to_string());
            }
            AppEvent::AudioSetInputDevice(device_name) => {
                self.state.audio_input_device = device_name;
            }
            AppEvent::AudioSetOutputDevice(device_name) => {
                self.state.audio_output_device = device_name;
            }

            // transcription events
            AppEvent::TranscriptionStarted => {
                self.state
                    .log(Log::new("Transcription started", LogLevel::Info));
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

                let result = self
                    .anthropic
                    .send_message(&text, &self.state.messages)
                    .await;

                self.db.insert_message(&message)?;
                self.state.messages.push(message);

                self.state
                    .log(Log::new("Transcription complete", LogLevel::Info));

                if let Err(error) = result {
                    self.state.error = Some(error.to_string());
                    self.state.log(Log::new(
                        &format!("Transcription failed: {error}"),
                        LogLevel::Error,
                    ));
                }
            }
            AppEvent::TranscriptionFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_transcription_running = false;
                self.state.log(Log::new(
                    &format!("Transcription failed: {error}"),
                    LogLevel::Error,
                ));
            }

            // llm events
            AppEvent::LLMMessageStarted(payload) => {
                self.state.is_llm_message_running = true;

                self.state
                    .log(Log::new("LLM message started", LogLevel::Info));

                let message = Message {
                    id: payload.message_id,
                    message_type: MessageType::Assistant,
                    content: MessageContent::Assistant {
                        text: "".to_string(),
                    },
                    created_at: None,
                };

                self.state.messages.push(message);
                self.elevenlabs.start_stream().await?;
            }
            AppEvent::LLMMessageDelta(payload) => {
                self.state.on_llm_text_delta(&payload);
                self.text_processor.process_delta(&payload.text).await?;
            }
            AppEvent::LLMMessageCompleted(payload) => {
                self.state.is_llm_message_running = false;

                self.state
                    .log(Log::new("LLM message completed", LogLevel::Info));

                self.text_processor.flush().await?;

                if let Some(message) = self.state.get_message(&payload.message_id) {
                    self.db.insert_message(message)?;
                }
            }
            AppEvent::LLMRequestFailed(error) => {
                self.state.log(Log::new(
                    &format!("LLM request failed: {error}"),
                    LogLevel::Error,
                ));
                self.state.error = Some(error.to_string());
                self.state.is_llm_message_running = false;
            }

            AppEvent::TextProcessorTextChunk(payload) => {
                self.state
                    .log(Log::new("Text processor text chunk", LogLevel::Info));
                self.elevenlabs.send_text(&payload.text).await?;
            }
            AppEvent::TextProcessorFlushed => {
                self.state
                    .log(Log::new("Text processor flushed", LogLevel::Info));
                self.elevenlabs.end_stream().await?;
            }

            // tts events
            AppEvent::TTSChunk(audio_bytes) => {
                if let Err(error) = self.audio_player.push_audio_chunk(&audio_bytes) {
                    self.state.log(Log::new(
                        &format!("TTS chunk failed: {error}"),
                        LogLevel::Error,
                    ));
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::TTSError(error) => {
                self.state
                    .log(Log::new(&format!("TTS error: {error}"), LogLevel::Error));
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }
            AppEvent::TTSFailed(error) => {
                self.state
                    .log(Log::new(&format!("TTS failed: {error}"), LogLevel::Error));
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }

            // log events
            AppEvent::Log(payload) => {
                let log = Log {
                    id: Uuid::new_v4().to_string(),
                    text: payload.message,
                    level: payload.level,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                self.state.log(log);
            }

            AppEvent::SetView(view) => {
                self.state.view = view.to_owned();
            }
        }

        Ok(())
    }

    async fn handle_terminal_event(&mut self, event: &Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    // general
                    KeyCode::Esc => self.cancel(),
                    KeyCode::Char('q') => self.quit(),

                    // audio
                    KeyCode::Char(' ') => self.toggle_recording().await?,

                    // navigation
                    KeyCode::Char('1') => self.state.view = View::Home,
                    KeyCode::Char('2') => self.state.view = View::Logs,
                    KeyCode::Tab => self.tab_view_forward(),
                    KeyCode::BackTab => self.tab_view_backward(),

                    KeyCode::Right => self.next_message(),
                    KeyCode::Left => self.previous_message(),
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn tab_view_forward(&mut self) {
        match self.state.view {
            View::Home => self.state.view = View::Logs,
            View::Logs => self.state.view = View::Home,
        }
    }

    fn next_message(&mut self) {
        if self.state.home_view.message_index >= self.state.messages.len() - 1 {
            return;
        }

        self.state.home_view.message_index += 1;
    }

    fn previous_message(&mut self) {
        if self.state.home_view.message_index == 0 {
            return;
        }

        self.state.home_view.message_index -= 1;
    }

    fn tab_view_backward(&mut self) {
        match self.state.view {
            View::Home => self.state.view = View::Logs,
            View::Logs => self.state.view = View::Home,
        }
    }

    fn render(&mut self) -> Result<(), anyhow::Error> {
        self.terminal
            .draw(|frame| frame.render_widget(AppLayout::new(&self.state), frame.area()))?;
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
