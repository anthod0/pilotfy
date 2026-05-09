# Session Inbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add session inbox APIs and dispatcher so busy sessions can queue or interrupt pending user messages before they become turns.

**Architecture:** Add a focused inbox application service backed by `inbox_messages`, reuse `TurnCommandService::create_and_dispatch_turn` for turn execution, and trigger synchronous drain after inbox submission and terminal turn ingestion. Keep legacy `POST /turns` unchanged. Add minimal dashboard API/store/composer support.

**Tech Stack:** Rust 2024, Axum, SQLx/SQLite migrations, Tokio tests; Svelte/TypeScript dashboard with pnpm.

---

### Task 1: Persistence and backend API surface

**Files:**
- Create: `migrations/0004_session_inbox.sql`
- Modify: `src/ids.rs`
- Modify: `src/application/views.rs`
- Create: `src/application/inbox.rs`
- Modify: `src/application/mod.rs`
- Modify: `src/transport/http/mod.rs`
- Modify: `src/transport/http/external.rs`
- Test: `tests/session_inbox_api.rs`

- [ ] Write failing tests for idle `after_idle`, busy `after_idle`, list/get/cancel, and idempotent replay.
- [ ] Run `cargo test --test session_inbox_api` and verify failures are missing route/table/service.
- [ ] Add migration, inbox view/request/outcome types, message id generation, routes, and service methods.
- [ ] Run `cargo test --test session_inbox_api` and verify green.

### Task 2: Dispatcher and turn refactor

**Files:**
- Modify: `src/application/turns.rs`
- Modify: `src/application/inbox.rs`
- Modify: `src/application/events.rs`
- Test: `tests/session_inbox_api.rs`

- [ ] Add failing tests for terminal-event drain, priority ordering, and cancel preventing dispatch.
- [ ] Run targeted tests and verify red.
- [ ] Extract `TurnCommandService::create_and_dispatch_turn(session_id, input, metadata)` from submit-turn.
- [ ] Implement `InboxCommandService::drain_inbox` with dispatch preconditions and conditional `pending -> dispatching` update.
- [ ] Trigger drain after terminal/interrupted turn ingest.
- [ ] Run targeted tests and verify green.

### Task 3: Interrupt policy

**Files:**
- Modify: `src/application/inbox.rs`
- Test: `tests/session_inbox_api.rs`

- [ ] Add failing tests for superseding pending interrupts and unsupported interrupt marking message failed.
- [ ] Run targeted tests and verify red.
- [ ] Implement `interrupt_now` transaction/supersede behavior and call existing runtime interrupt flow for active turns.
- [ ] Run targeted tests and verify green.

### Task 4: Dashboard minimal inbox UX

**Files:**
- Modify: `apps/web/src/api/client.ts`
- Create: `apps/web/src/stores/inbox.ts`
- Modify: `apps/web/src/components/turns/TurnComposer.svelte`
- Modify: `apps/web/src/services/refreshCoordinator.ts` if refresh wiring is needed.

- [ ] Add failing typecheck expectations by using new inbox API/store from composer.
- [ ] Implement `submitInboxMessage`, inbox types, store actions, and composer buttons for queue vs interrupt when busy.
- [ ] Run `pnpm --dir apps/web typecheck` and verify green.

### Task 5: Full verification

- [ ] Run `cargo fmt --check` (fix with `cargo fmt` if needed).
- [ ] Run `cargo test`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `pnpm --dir apps/web typecheck`.
- [ ] Run `pnpm --dir apps/web build`.
