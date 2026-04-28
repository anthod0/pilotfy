# M1.5 pi Adapter Event Outbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining M1.5 pi adapter bridge work by ingesting confirmed non-RPC adapter facts from a workspace JSONL event outbox.

**Architecture:** The pi runtime exports a session-scoped `LLMPARTY_ADAPTER_EVENT_LOG` path inside `.llmparty/adapter-events.jsonl`. A new application observation service reads this file, validates each JSONL record against the current session/runtime binding, converts only confirmed generic domain facts into persisted domain events, and records malformed adapter records as explicit non-terminal adapter error events. No pi TUI state is inferred and no completed/output event is forged.

**Tech Stack:** Rust 2024, SQLx/SQLite, serde_json, tmux runtime, existing domain event store/projections, integration tests.

---

### Task 1: Runtime exports adapter event outbox path

**Files:**
- Modify: `src/runtime/mod.rs`
- Test: `tests/pi_adapter_m15.rs`

- [ ] Write a failing test that creates a pi session and asserts runtime binding metadata contains `adapter_event_log` ending with `.llmparty/adapter-events.jsonl`.
- [ ] Run: `cargo test --test pi_adapter_m15 pi_runtime_binding_exposes_adapter_event_log -- --test-threads=1`; expect FAIL because metadata/env path is absent.
- [ ] Add the adapter event log path to runtime metadata and export `LLMPARTY_ADAPTER_EVENT_LOG` in the runtime script.
- [ ] Re-run the focused test; expect PASS.

### Task 2: Ingest confirmed output/completed records from event outbox

**Files:**
- Modify: `src/application/mod.rs`
- Test: `tests/pi_adapter_m15.rs`

- [ ] Write a failing test that submits a pi turn, appends JSONL records for `turn.output` and `turn.completed` to `adapter-events.jsonl`, calls a new `PiAdapterEventOutboxService::observe_session`, and asserts the turn becomes `completed` with output summary and events include output/completed from `agent_adapter`.
- [ ] Run the focused test; expect FAIL because service does not exist.
- [ ] Implement minimal service: load runtime metadata, read JSONL file, validate session_id/turn_id/type/source, ingest `turn.output`, `turn.completed`, and `turn.failed` as `EventSource::AgentAdapter` with generic payload only.
- [ ] Re-run focused test; expect PASS.

### Task 3: Malformed adapter records produce explicit error semantics without terminal forgery

**Files:**
- Modify: `src/application/mod.rs`
- Test: `tests/pi_adapter_m15.rs`

- [ ] Write a failing test that appends malformed/unsupported adapter JSONL, observes the session, and asserts a `turn.failed` is not forged while a `session.error` with adapter error payload is recorded.
- [ ] Run focused test; expect FAIL until error handling exists.
- [ ] Extend observation to emit `session.error` from `agent_adapter` for invalid adapter records and continue processing other valid records where safe.
- [ ] Re-run focused test; expect PASS.

### Task 4: Documentation and verification

**Files:**
- Modify: `README.md`
- Modify: `MILESTONE.md`

- [ ] Update README M1.5 section with adapter event outbox format and local validation behavior.
- [ ] Mark M1.5 remaining items complete only if verification passes.
- [ ] Run: `cargo fmt --check && cargo test --test pi_adapter_m15 -- --test-threads=1 && cargo test --test internal_event_api && cargo test --test generic_adapter_contract`.
