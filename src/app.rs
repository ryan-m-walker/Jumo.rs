use std::{io::Stdout, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use ratatui::{Terminal, prelude::CrosstermBackend};

use crate::{
    audio::player::AudioPlayer,
    events::EventBus,
    renderer::Renderer,
    state::{Speaker, TranscriptLine, TranscriptMessage},
};
use crate::{audio::recorder::AudioRecorder, events::AppEvent};
use crate::{
    services::{anthropic::AnthropicService, elevenlabs::ElevenLabsService},
    state::AppState,
};

pub struct App {
    event_bus: EventBus,
    anthropic: AnthropicService,
    elevenlabs: ElevenLabsService,
    audio_recorder: AudioRecorder,
    audio_player: AudioPlayer,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    renderer: Renderer,
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
            renderer: Renderer::new(),
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
                Some(event) = self.event_bus.recv() => self.handle_app_event(&event).await?,
                Some(Ok(event)) = events.next() => self.handle_terminal_event(&event).await?,
            }
        }

        ratatui::restore();
        Ok(())
    }

    async fn handle_app_event(&mut self, event: &AppEvent) -> Result<(), anyhow::Error> {
        match event {
            AppEvent::AudioRecordingStarted => {
                self.state.is_audio_recording_running = true;
                self.state.is_audio_transcription_running = true;
            }
            AppEvent::AudioRecordingCompleted(temp_path) => {
                self.state.is_audio_recording_running = false;
                self.elevenlabs.transcribe(temp_path).await?;
            }
            AppEvent::AudioRecordingFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_recording_running = false;
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
            AppEvent::TranscriptionFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_audio_transcription_running = false;
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
            AppEvent::LLMRequestFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_llm_message_running = false;
            }
            AppEvent::TTSStarted => {
                self.state.is_tts_running = true;
            }
            AppEvent::TTSCompleted(result) => {
                self.audio_player
                    .play(&result.audio_bytes, result.duration_seconds)?;
            }
            AppEvent::TTSFailed(error) => {
                self.state.error = Some(error.to_string());
                self.state.is_tts_running = false;
            }
            _ => {
                // TODO: handle other events
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
        self.terminal
            .draw(|frame| self.renderer.render(frame, &self.state))?;
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
