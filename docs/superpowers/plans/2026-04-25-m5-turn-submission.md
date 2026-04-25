# M5 Turn Submission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add External API turn submission so orchestrators can submit a new turn to an idle/interrupted session and observe lifecycle through existing event-driven projections.

**Architecture:** Extend the existing application command service with submit-turn validation, idempotency, event emission, and a minimal generic AgentInputSink boundary. Expose it via `POST /external/v1/sessions/{session_id}/turns`; execution progress remains reported by Internal Event API.

**Tech Stack:** Rust, Axum, SQLx SQLite, Serde, Tokio, integration tests with temporary SQLite databases.

---

### Task 1: Add failing M5 integration tests

**Files:**
- Create: `tests/milestone5_turn_submit.rs`

- [ ] Test idle session turn submission produces a queued TurnView and created/queued events.
- [ ] Test Idempotency-Key replay returns the same turn.
- [ ] Test session state restrictions: busy/starting/exited/error rejected, interrupted accepted.
- [ ] Test active queued/running turn blocks a second turn.
- [ ] Test Internal Event API can advance submitted turn to running/completed and session busy/idle projections update.
- [ ] Run `cargo test --test milestone5_turn_submit` and verify failures are due to missing route/service.

### Task 2: Implement application submit-turn service

**Files:**
- Modify: `src/application/mod.rs`
- Modify: `src/runtime/mod.rs`

- [ ] Add `SubmitTurnRequest` and `SubmitTurnOutcome`.
- [ ] Add `TurnCommandService::submit_turn` or extend command service with a focused turn method.
- [ ] Validate session exists, state is `idle` or `interrupted`, no active turn exists, and `accept_task` capability is true.
- [ ] Generate Control Plane `turn_id` and ingest `turn.created` / `turn.queued`.
- [ ] Store/replay idempotency responses under `submit_turn:{session_id}`.
- [ ] Add a minimal generic `AgentInputSink` boundary that accepts submitted input without producing lifecycle events.
- [ ] Run focused tests and fix until green.

### Task 3: Wire External HTTP route and errors

**Files:**
- Modify: `src/transport/http/external.rs`
- Modify: `src/transport/http/mod.rs`

- [ ] Add handler for `POST /external/v1/sessions/{session_id}/turns`.
- [ ] Authenticate, read `Idempotency-Key`, call application service, return 201 for new and 200 for duplicate.
- [ ] Map state conflicts to HTTP 409 with error code `state_conflict`.
- [ ] Run `cargo test --test milestone5_turn_submit`.

### Task 4: Verification and docs

**Files:**
- Modify: `README.md`
- Modify: `MILESTONE.md`

- [ ] Add a README curl example for submitting a turn.
- [ ] Mark Milestone 5 complete with summary and verification commands.
- [ ] Run `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`.
