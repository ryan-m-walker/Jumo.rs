use std::{io::Stdout, time::Duration};

use cpal::{
    SampleRate, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use eventsource_stream::Eventsource;
use futures::SinkExt;
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
use tempfile::TempPath;
use throbber_widgets_tui::ThrobberState;
use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::claude_types::{Delta, StreamEvent};
use crate::recorder::AudioRecorder;

#[derive(Debug, Clone)]
enum Speaker {
    User,
    Robot,
}

#[derive(Debug, Clone)]
enum AppEvent {
    AudioTranscriptionStarted,
    AudioTranscriptionCompleted,
    LLMMessageStarted,
    LLMMessageCompleted,
    TranscriptMessage((Speaker, String)),
    TranscriptError(TranscriptError),
}

#[derive(Debug, Deserialize, Clone)]
struct ElevenLabsTranscription {
    text: String,
}

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

#[derive(Debug, Clone)]
struct TranscriptMessage {
    speaker: Speaker,
    text: String,
}

#[derive(Debug, Clone)]
struct TranscriptError {
    text: String,
}

#[derive(Debug, Clone)]
enum TranscriptLine {
    TranscriptMessage(TranscriptMessage),
    TranscriptError(TranscriptError),
}

pub struct App {
    is_app_running: bool,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    audio_recorder: AudioRecorder,
    transcript: Vec<TranscriptLine>,
    app_event_sender: mpsc::Sender<AppEvent>,
    app_event_receiver: mpsc::Receiver<AppEvent>,

    is_audio_transcription_running: bool,
    is_llm_message_running: bool,
    throbber_state: ThrobberState,
}

const FRAMES_PER_SECOND: f32 = 60.0;

impl App {
    pub fn new() -> Self {
        let (app_event_sender, app_event_receiver) = mpsc::channel(100);

        Self {
            terminal: ratatui::init(),
            audio_recorder: AudioRecorder::new(),
            app_event_sender,
            app_event_receiver,

            transcript: Vec::new(),
            is_app_running: false,
            is_audio_transcription_running: false,
            is_llm_message_running: false,
            throbber_state: ThrobberState::default(),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        self.is_app_running = true;

        let period = Duration::from_secs_f32(1.0 / FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while self.is_app_running {
            // self.throbber_state.calc_next();

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
            AppEvent::TranscriptMessage((speaker, text)) => {
                self.transcript
                    .push(TranscriptLine::TranscriptMessage(TranscriptMessage {
                        speaker: speaker.clone(),
                        text: text.clone(),
                    }))
            }
            AppEvent::AudioTranscriptionStarted => {
                self.is_audio_transcription_running = true;
            }
            AppEvent::AudioTranscriptionCompleted => {
                self.is_audio_transcription_running = false;
            }
            AppEvent::LLMMessageStarted => {
                self.is_llm_message_running = true;
            }
            AppEvent::LLMMessageCompleted => {
                self.is_llm_message_running = false;
            }
            AppEvent::TranscriptError(error) => {
                self.transcript
                    .push(TranscriptLine::TranscriptError(TranscriptError {
                        text: error.text.clone(),
                    }));
            }
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
            } else if self.is_audio_transcription_running {
                "Transcribing audio..."
            } else if self.is_llm_message_running {
                "Sending message to LLM..."
            } else {
                "Press space to start recording audio..."
            };

            let header = Paragraph::new(header_text).style(Style::new().dim()).block(
                Block::bordered()
                    .border_style(Style::new().yellow())
                    .border_type(BorderType::Rounded),
            );

            // let mut lines = vec![Line::from(header), Line::from("---")];
            let mut lines = vec![];

            for line in self.transcript.iter() {
                match line {
                    TranscriptLine::TranscriptMessage(line) => {
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
        self.is_app_running = false;
    }

    fn toggle_recording(&mut self) -> Result<(), anyhow::Error> {
        if self.is_audio_transcription_running || self.is_llm_message_running {
            return Ok(());
        }

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
        let transcript = self.transcript.clone();

        // let (ws_tx, mut ws_rx) = mpsc::channel(100);
        // let elevenlabs_api_key_clone = elevenlabs_api_key.clone();
        //
        // tokio::spawn(async move {
        //     let mut request =
        //         "wss://api.elevenlabs.io/v1/text-to-speech/GfVVOQdZ5Fsz9QBNYFie/stream-input"
        //             .into_client_request()
        //             .unwrap();
        //     request
        //         .headers_mut()
        //         .insert("api-key", elevenlabs_api_key_clone.parse().unwrap());
        //
        //     let (ws_stream, _) = connect_async(request).await.unwrap();
        //     let (mut write, read) = ws_stream.split();
        //
        //     while let Some(ev) = ws_rx.recv().await {
        //         let body = ElevenLabsSendTextMessage { text: ev };
        //         let json_str = serde_json::to_string(&body)?;
        //         write.send(Message::Text(json_str.into())).await?;
        //     }
        //
        //     Ok::<(), anyhow::Error>(())
        // });

        tokio::spawn(async move {
            sender.send(AppEvent::AudioTranscriptionStarted).await?;

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
                .header("xi-api-key", &elevenlabs_api_key)
                .multipart(form)
                .send()
                .await?
                .json::<ElevenLabsTranscription>()
                .await?;

            let text = eleven_labs_resp.text;

            sender
                .send(AppEvent::TranscriptMessage((Speaker::User, text.clone())))
                .await?;
            sender.send(AppEvent::AudioTranscriptionCompleted).await?;

            sender.send(AppEvent::LLMMessageStarted).await?;

            let mut messages = vec![];

            for line in transcript.iter() {
                if let TranscriptLine::TranscriptMessage(line) = line {
                    messages.push(ClaudeMessage {
                        role: match line.speaker {
                            Speaker::User => "user".to_string(),
                            Speaker::Robot => "assistant".to_string(),
                        },
                        content: line.text.clone(),
                    });
                }
            }

            messages.push(ClaudeMessage {
                role: "user".to_string(),
                content: text,
            });

            let body = ClaudeInput {
                model: String::from("claude-sonnet-4-20250514"),
                max_tokens: 1024,
                messages,
                stream: true,
            };

            let mut claude_stream = client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &anthropic_api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?
                .bytes_stream()
                .eventsource();

            let mut buffer = String::new();

            while let Some(event) = claude_stream.next().await {
                match event {
                    Ok(event) => {
                        if event.data.is_empty() {
                            continue;
                        }

                        let stream_event: Result<StreamEvent, _> =
                            serde_json::from_str(&event.data);

                        match stream_event {
                            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                                if let Delta::TextDelta { text } = delta {
                                    buffer.push_str(&text);
                                }
                            }
                            Ok(StreamEvent::MessageStop) => {
                                break;
                            }
                            Ok(StreamEvent::Error { error }) => {
                                sender
                                    .send(AppEvent::TranscriptError(TranscriptError {
                                        text: error.message,
                                    }))
                                    .await?;
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(err) => {
                        sender
                            .send(AppEvent::TranscriptError(TranscriptError {
                                text: err.to_string(),
                            }))
                            .await?;
                    }
                }
            }

            sender
                .send(AppEvent::TranscriptMessage((
                    Speaker::Robot,
                    buffer.clone(),
                )))
                .await?;
            sender.send(AppEvent::LLMMessageCompleted).await?;

            let body = ElevenLabsSendTextMessage {
                text: buffer,
                model_id: String::from("eleven_multilingual_v2"),
            };

            let voice_id = "Fahco4VZzobUeiPqni1S";
            let url = format!(
                "https://api.elevenlabs.io/v1/text-to-speech/{voice_id}?output_format=pcm_44100"
            );

            let speech_resp = client
                .post(url)
                .header("xi-api-key", &elevenlabs_api_key)
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


            let host = cpal::default_host();
            let device = host.default_output_device().expect("No output device");

            let config = StreamConfig {
                channels: 1,
                sample_rate: SampleRate(44100),
                buffer_size: cpal::BufferSize::Default,
            };

            let samples: Vec<f32> = audio_bytes
                .chunks_exact(2)
                .map(|chunk| {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    sample as f32 / i16::MAX as f32
                })
                .collect();

            let mut sample_index = 0;
            let samples_clone = samples.clone();

            let stream = device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for sample in data {
                        *sample = samples_clone.get(sample_index).copied().unwrap_or(0.0);
                        sample_index += 1;
                    }
                },
                move |err| {
                    eprintln!("an error occurred on stream: {}", err);
                },
                None,
            )?;

            stream.play()?;
            std::thread::sleep(std::time::Duration::from_secs_f64(duration_seconds + 0.5));

            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }
}
