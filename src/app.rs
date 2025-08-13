use std::{fs, io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};
use tui_input::backend::crossterm::EventHandler;
use uuid::Uuid;

use crate::{
    audio::player::AudioPlayer,
    database::{
        Database,
        models::{
            log::{Log, LogLevel},
            message::{ContentBlock, Message, Role},
        },
    },
    events::EventBus,
    services::anthropic::types::{AnthropicContentBlockDelta, AnthropicMessageStreamEvent},
    state::View,
    text_processor::TextProcessor,
    tools::tools::ToolType,
    widgets::{app_layout::AppLayout, views::chat::ChatViewMode},
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
        if let Err(err) = self.state.load_state().await {
            self.log_error(&format!("Failed to load state: {err}"))?;
        }

        self.state.is_app_running = true;

        self.db.init()?;

        self.audio_player.start().await?;
        self.audio_recorder.start().await?;

        let messages = self.db.get_messages()?;
        self.state.messages = messages;

        let message_count = self.state.get_assistant_message_count();
        self.state.home_view.message_index = if message_count > 0 {
            message_count - 1
        } else {
            0
        };

        let logs = self.db.get_logs()?;
        self.state.logs_view.logs = logs;

        self.log_info("App started")?;

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
                self.log_info("Audio recording started")?;
                self.state.is_audio_recording_running = true;
            }
            AppEvent::AudioRecordingCompleted(audio_bytes) => {
                let len = audio_bytes.len();
                self.log_info(format!("Audio recording completed ({len} bytes)").as_str())?;
                self.state.is_audio_recording_running = false;
                self.state.input_volume = 0.0;
                self.elevenlabs.transcribe(audio_bytes);
            }
            AppEvent::AudioRecordingError(error) => {
                self.log_error(&format!("Audio recording error: {error}"))?;
                self.state.error = Some(error.to_string());
                self.state.is_audio_recording_running = false;
            }
            AppEvent::AudioRecordingFailed(error) => {
                self.log_error(&format!("Audio recording failed: {error}"))?;
                self.state.error = Some(error.to_string());
                self.state.input_volume = 0.0;
                self.state.is_audio_recording_running = false;
            }
            AppEvent::AudioDetected(volume) => {
                self.state.input_volume = volume;
                self.state.audio_detected = true;
            }
            AppEvent::AudioNotDetected => {
                self.state.audio_detected = false;
            }
            AppEvent::AudioPlaybackError(error) => {
                self.log_error(&format!("Audio playback error: {error}"))?;
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
                self.log_info("Transcription started")?;
                self.state.is_audio_transcription_running = true;
            }
            AppEvent::TranscriptionCompleted(text) => {
                self.state.is_audio_transcription_running = false;

                let message = Message {
                    id: Uuid::new_v4().to_string(),
                    role: Role::User,
                    content: vec![ContentBlock::Text { text: text.clone() }],
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                };

                // TODO: don't add if llm call fails
                self.db.insert_message(&message)?;
                self.state.messages.push(message);
                self.anthropic.prompt(&self.state.messages, &self.state);
                self.log_info("Transcription complete")?;
            }
            AppEvent::TranscriptionFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_transcription_running = false;
                self.log_error(&format!("Transcription failed: {error}"))?;
            }

            // llm events
            AppEvent::LLMGenerationStarted(payload) => {
                self.state.is_llm_message_running = true;

                self.log_info("LLM message started")?;

                let message = Message {
                    id: payload.message_id,
                    role: Role::Assistant,
                    content: vec![],
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                };

                self.state.messages.push(message);
                self.elevenlabs.start_stream().await?;

                let assistant_message_count = self
                    .state
                    .messages
                    .iter()
                    .filter(|message| message.role == Role::Assistant)
                    .count();
                self.state.home_view.message_index = assistant_message_count - 1;
            }
            AppEvent::LLMStreamEvent(payload) => match payload.event {
                AnthropicMessageStreamEvent::ContentBlockStart {
                    index,
                    content_block,
                } => {
                    if let ContentBlock::ToolUse { .. } = &content_block {
                        let key = (payload.message_id.clone(), index);
                        self.state.tool_input_buffers.insert(key, String::new());
                    }

                    if let Some(message) = self.state.get_message_mut(&payload.message_id) {
                        message.content.insert(index, content_block);
                    }
                }

                AnthropicMessageStreamEvent::ContentBlockDelta { index, delta } => {
                    if let Some(message) = self.state.get_message_mut(&payload.message_id) {
                        if let Some(block) = message.content.get_mut(index) {
                            match delta {
                                AnthropicContentBlockDelta::Text { text } => {
                                    if let ContentBlock::Text { text: block_text } = block {
                                        block_text.push_str(&text);
                                    }

                                    self.text_processor.process_delta(&text).await?;
                                }
                                AnthropicContentBlockDelta::Thinking { text } => {
                                    if let ContentBlock::Thinking {
                                        content: block_content,
                                    } = block
                                    {
                                        block_content.push_str(&text);
                                    }
                                }
                                AnthropicContentBlockDelta::InputJson { partial_json } => {
                                    let key = (payload.message_id.clone(), index);
                                    let input_buffer = self.state.tool_input_buffers.get_mut(&key);

                                    if let Some(input_buffer) = input_buffer {
                                        input_buffer.push_str(&partial_json);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                AnthropicMessageStreamEvent::ContentBlockStop { index } => {
                    let key = (payload.message_id.clone(), index);

                    if let Some(buffer) = self.state.tool_input_buffers.remove(&key) {
                        if let Some(message) = self.state.get_message_mut(&payload.message_id) {
                            if let Some(ContentBlock::ToolUse { input, .. }) =
                                message.content.get_mut(index)
                            {
                                match serde_json::from_str::<serde_json::Value>(&buffer) {
                                    Ok(json_value) => {
                                        *input = json_value;
                                    }
                                    Err(_) => {
                                        *input = serde_json::Value::String(buffer);
                                    }
                                }
                            }
                        }
                    }
                }

                _ => {}
            },
            AppEvent::LLMGenerationCompleted(payload) => {
                self.state.is_llm_message_running = false;

                self.log_info("LLM message completed")?;

                self.text_processor.flush().await?;

                let mut tool_result_blocks = Vec::new();

                if let Some(message) = self.state.get_message(&payload.message_id) {
                    for block in &message.content {
                        if let ContentBlock::ToolUse { id, name, input } = block {
                            let string_input = input.to_string();

                            let result = ToolType::execute_tool(
                                name,
                                &string_input,
                                &self.state,
                                self.event_bus.sender(),
                            )
                            .await?;

                            let block = ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result,
                            };

                            tool_result_blocks.push(block);
                        }
                    }

                    self.db.insert_message(message)?;
                }

                if !tool_result_blocks.is_empty() {
                    let message = Message {
                        id: Uuid::new_v4().to_string(),
                        role: Role::User,
                        content: tool_result_blocks,
                        created_at: Some(chrono::Utc::now().to_rfc3339()),
                    };

                    self.db.insert_message(&message)?;
                    self.state.messages.push(message);

                    self.anthropic.prompt(&self.state.messages, &self.state);
                }
            }
            AppEvent::LLMGenerationFailed(error) => {
                self.log_error(&format!("LLM request failed: {error}"))?;
                self.state.error = Some(error.to_string());
                self.state.is_llm_message_running = false;
            }

            AppEvent::LLMGenerationError(error) => {
                self.log_error(&format!("LLM generation error: {error}"))?;
                self.state.error = Some(error.to_string());
            }

            AppEvent::TextProcessorTextChunk(payload) => {
                self.log_info("Text processor text chunk")?;
                self.elevenlabs.send_text(&payload.text).await?;
            }
            AppEvent::TextProcessorFlushed => {
                self.log_info("Text processor flushed")?;
                self.elevenlabs.end_stream().await?;
            }

            // tts events
            AppEvent::TTSChunk(audio_bytes) => {
                if let Err(error) = self.audio_player.push_audio_chunk(&audio_bytes) {
                    self.log_error(&format!("TTS chunk failed: {error}"))?;
                    self.state.error = Some(error.to_string());
                }
            }
            AppEvent::TTSError(error) => {
                self.log_error(&format!("TTS error: {error}"))?;
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }
            AppEvent::TTSFailed(error) => {
                self.log_error(&format!("TTS failed: {error}"))?;
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }

            // log events
            AppEvent::Log(payload) => {
                self.log(&payload.message, payload.level)?;
            }

            AppEvent::SetView(view) => {
                self.state.view = view.to_owned();
            }
            AppEvent::ClearLogs => {
                self.state.logs_view.logs.clear();
            }
            AppEvent::SetEmote(emote) => {
                self.state.emote = emote;
            }
            AppEvent::SetColor(color) => {
                self.state.color = color;

                if let Err(err) = self.state.persist_state().await {
                    self.log_error(&format!("Failed to persist state: {err}"))?;
                }
            }
        }

        Ok(())
    }

    async fn handle_terminal_event(&mut self, event: &Event) -> Result<(), anyhow::Error> {
        if self.state.view == View::Chat && self.state.chat_view.mode == ChatViewMode::Insert {
            self.state.chat_view.input.handle_event(event);
        }

        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    // general
                    KeyCode::Esc => self.cancel(),
                    KeyCode::Char('q') => self.quit(),

                    // audio
                    KeyCode::Char(' ') => self.toggle_recording(),
                    KeyCode::Char('i') => {
                        if self.state.chat_view.mode == ChatViewMode::Normal {
                            self.state.chat_view.mode = ChatViewMode::Insert;
                        }
                    }

                    // navigation
                    KeyCode::Char('1') => self.state.view = View::Home,
                    KeyCode::Char('2') => self.state.view = View::Logs,
                    KeyCode::Char('3') => self.state.view = View::Chat,
                    KeyCode::Tab => self.tab_view_forward(),
                    KeyCode::BackTab => self.tab_view_backward(),

                    // messages
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
            View::Logs => self.state.view = View::Chat,
            View::Chat => self.state.view = View::Home,
        }
    }

    fn tab_view_backward(&mut self) {
        match self.state.view {
            View::Home => self.state.view = View::Chat,
            View::Logs => self.state.view = View::Home,
            View::Chat => self.state.view = View::Logs,
        }
    }

    fn next_message(&mut self) {
        if self.state.home_view.message_index >= self.state.get_assistant_message_count() - 1 {
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
        // self.audio_recorder.stop();
        // self.state.is_audio_recording_running = false;

        self.audio_player.stop();
        self.state.is_audio_playback_running = false;

        self.anthropic.cancel();
        self.state.is_llm_message_running = false;

        self.elevenlabs.cancel();
        self.state.is_audio_transcription_running = false;
    }

    fn toggle_recording(&mut self) {
        if self.audio_recorder.is_recording() {
            self.audio_recorder.stop_recording();
        } else {
            // self.cancel();
            self.audio_recorder.start_recording();
        }
    }

    fn log(&mut self, text: &str, level: LogLevel) -> Result<(), anyhow::Error> {
        self.db.insert_log(text, level)?;
        self.state.log(Log::new(text, level));
        Ok(())
    }

    fn log_info(&mut self, text: &str) -> Result<(), anyhow::Error> {
        self.log(text, LogLevel::Info)
    }

    fn log_error(&mut self, text: &str) -> Result<(), anyhow::Error> {
        self.log(text, LogLevel::Error)
    }
}
