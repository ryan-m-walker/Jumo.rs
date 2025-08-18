# Robo RS

Experiments with AI while making a Rust based, Raspberry PI robot companion

## TODO

### A/V

- [x] Audio recording
- [x] Audio transcription
- [x] Audio playback
- [x] Image capture
- [ ] Audio input detection (no need for record button)
- [ ] Voice keywords (cancel, stop, exit, update, etc)

### Tools

- [x] Basic tool calling
- [ ] update(production = false) - git pull && cargo build && ./bin
- [x] set_view(view) - set the active TUI view in app
- [ ] shutdown() - quit app
- [ ] pass() - do nothing
- [ ] output_text(text) - print text to TUI output view
- [ ] read_file(path) - read file from own source code
- [ ] view_logs(from, to) - view logs from from to to
- [ ] query_memory(query) - run SQL queries on episodic memory DB

- [x] MCP

### Memory

- [x] Sqlite
- [ ] Maybe remote postgres DB?
- [ ] Summarizer
- [ ] Vector DB (Qdrant)
  - (Rust client) https://github.com/qdrant/rust-client
- [ ] Knowledge Base
- [ ] Knowledge Graph
  - (Rust Neo4j driver) https://github.com/neo4j-labs/neo4rs

### Prompting

- [x] System prompt
- [ ] Core values
- [ ] Shared notes file
- [ ] Prompt composer

### Capabilities

- [ ] Thinking (https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)
- [x] Emotes/Color

## Embedding

- [x] Setup Raspberry Pi environment

### TUI

- [x] Audio playback bars/visualization (input)
- [ ] Audio playback bars/visualization (output)
- [ ] Chat view
- [ ] Scrolling logs
  - https://crates.io/crates/tui-scrollview

### Diagnostics

- [ ] System info (CPU, RAM, etc)
  - https://crates.io/crates/sysinfo
- [ ] AV info (input, output, etc)
- [ ] Data storage stats (DB size, etc)
