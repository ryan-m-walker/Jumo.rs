# TODO

## A/V

[x] Audio recording
[x] Audio transcription
[x] Audio playback
[ ] Audio input detection (no need for record button)
[ ] Voice keywords (cancel, stop, exit, update, etc)

## Tools

[x] Basic tool calling
[ ] update(production = false) - git pull && cargo build && ./bin
[x] set_view(view) - set the active TUI view in app
[ ] shutdown() - quit app
[ ] pass() - do nothing
[ ] output_text(text) - print text to TUI output view
[ ] read_file(path) - read file from own source code
[ ] view_logs(from, to) - view logs from from to to
[ ] query_memory(query) - run SQL queries on episodic memory DB

[ ] MCP

## Memory

[ ] Summarizer
[ ] Vector DB (Qdrant)
  - (Rust client) https://github.com/qdrant/rust-client
[ ] Knowledge Base
[ ] Knowledge Graph
  - (Rust Neo4j driver) https://github.com/neo4j-labs/neo4rs

## Prompting

[ ] System prompt
[ ] Core values
[ ] Shared notes file
[ ] Prompt composer

## Capabilities

[ ] Image upload 
  - (Claude vision) https://docs.anthropic.com/en/docs/build-with-claude/vision 
  - (Webcam crate) https://crates.io/crates/nokhwa
[ ] Thinking (https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking)
[ ] Emotes/Color

## Embedding

[ ] Setup Raspberry Pi environment

## TUI

[ ] Audio playback bars/visualization
  - https://crates.io/crates/dasp_sample
[ ] Scrolling logs
  - https://crates.io/crates/tui-scrollview

## Diagnostics

[ ] System info (CPU, RAM, etc)
  - https://crates.io/crates/sysinfo
[ ] AV info (input, output, etc)
[ ] Data storage stats (DB size, etc)
