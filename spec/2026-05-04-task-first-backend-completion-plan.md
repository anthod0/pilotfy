# Task-first Backend Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the backend capabilities needed for a task-first WebUI: reliable task lifecycle, task events, workspace confirmation, idempotent creation/failure recovery, and task-level controls.

**Architecture:** Keep `tasks` as the user-facing lifecycle projection and `turns/events` as the execution-layer projection. Reuse existing session/turn services where possible, but move shared task dispatch and task state synchronization into focused application-layer helpers so `POST /tasks`, confirmation, and event ingestion all use the same semantics.

**Tech Stack:** Rust, Axum, SQLx, SQLite migrations, serde_json, existing llmparty domain event projection.

---

## Context

The data migration from `spec/2026-05-02-global-task-workspace-data-design.md` is mostly in place:

- `migrations/0003_global_workspaces_tasks.sql` creates `workspaces`, `tasks`, and `task_events`.
- `src/application/tasks.rs` supports global task creation and workspace/session/turn association.
- `src/transport/http/mod.rs` exposes basic task and workspace endpoints.
- `tests/global_workspace_tasks.rs` covers basic create/list/get flows.

However, before the WebUI can safely become task-first, the backend still has five blocking gaps:

1. Task state does not follow turn lifecycle after dispatch.
2. Task events are written but not queryable.
3. `needs_confirmation` tasks cannot be confirmed and dispatched.
4. `POST /tasks` lacks idempotency and robust failure-state recovery.
5. Users cannot control active work through task-level endpoints.

## Execution Strategy

Use **three separate agent sessions** to execute this plan in order. Do not try to complete the whole backend pass in one long session.

### Agent Session 1: Task event visibility

Covers:

- Phase 1: Introduce task lifecycle primitives
- Phase 2: Expose task events API

Goal: make task lifecycle history queryable before changing more state behavior.

Exit criteria:

```bash
cargo test --test global_workspace_tasks -- --nocapture
cargo test --all
```

Recommended commit after this session:

```bash
git commit -m "feat: expose task events"
```

### Agent Session 2: Task lifecycle and confirmation dispatch

Covers:

- Phase 3: Synchronize task state from turn lifecycle
- Phase 4: Add workspace confirmation and redispatch for ambiguous tasks

Goal: make tasks accurately follow execution and allow `needs_confirmation` tasks to be resolved.

Exit criteria:

```bash
cargo test --test global_workspace_tasks -- --nocapture
cargo test --all
```

Recommended commits in this session:

```bash
git commit -m "feat: sync task state from turn lifecycle"
git commit -m "feat: confirm workspace for pending tasks"
```

### Agent Session 3: Idempotency, failure recovery, and task controls

Covers:

- Phase 5: Make task creation idempotent and failure-aware
- Phase 6: Add task-level control endpoints

Goal: make task mutations safe for WebUI retries and allow users to operate through task-level controls.

Exit criteria:

```bash
cargo test --test global_workspace_tasks -- --nocapture
cargo test --all
cd apps/web && pnpm build
```

Recommended commits in this session:

```bash
git commit -m "feat: make task creation idempotent"
git commit -m "feat: add task-level controls"
```

---

## Implementation Phases

### Phase 1: Introduce task lifecycle primitives

**Purpose:** Add shared backend primitives before adding more HTTP surface area. This prevents duplicating dispatch/state logic across `create_task`, confirmation, event ingestion, and controls.

**Files:**

- Modify: `src/application/views.rs`
- Modify: `src/application/mapping.rs`
- Modify: `src/application/queries.rs`
- Modify: `src/application/tasks.rs`
- Test: `tests/global_workspace_tasks.rs`

**New/changed concepts:**

- Add `TaskEventView` for API responses.
- Add query methods:
  - `list_task_events(task_id: &str) -> Result<Vec<TaskEventView>>`
  - optionally `find_task_by_turn(turn_id: &str) -> Result<Option<TaskView>>`
- Extract internal task helper methods inside `TaskCommandService` or a focused helper module:
  - `record_task_event(task_id, event_type, payload)`
  - `mark_task_failed(task_id, reason, payload)`
  - `dispatch_task(task_id, workspace_id/canonical_path, client_type)`

**Tasks:**

- [ ] Add `TaskEventView` to `src/application/views.rs` with fields:
  - `event_id: String`
  - `task_id: String`
  - `event_type: String`
  - `payload: Value`
  - `created_at: String`
- [ ] Add `row_to_task_event_view` to `src/application/mapping.rs`.
- [ ] Add `ExternalQueryService::list_task_events` to `src/application/queries.rs`.
- [ ] Add focused tests that create a task and verify task events can be loaded from the application query layer or HTTP layer once Phase 2 exposes it.

**Verification:**

```bash
cargo test --test global_workspace_tasks -- --nocapture
```

Expected: existing tests still pass.

---

### Phase 2: Expose task events API

**Purpose:** Allow WebUI to show task routing and lifecycle history.

**Files:**

- Modify: `src/transport/http/mod.rs`
- Modify: `src/transport/http/external.rs`
- Test: `tests/global_workspace_tasks.rs`

**Endpoint:**

```text
GET /external/v1/tasks/{task_id}/events
```

**Response shape:**

```json
{
  "data": {
    "events": [
      {
        "event_id": "evt_...",
        "task_id": "task_...",
        "event_type": "task.created",
        "payload": {},
        "created_at": "..."
      }
    ]
  },
  "error": null
}
```

**Tasks:**

- [ ] Write failing HTTP test: create a task, call `GET /external/v1/tasks/{task_id}/events`, expect at least `task.created`.
- [ ] Add route in `src/transport/http/mod.rs`.
- [ ] Add handler in `src/transport/http/external.rs`:
  - authenticate
  - ensure task exists, return 404 if missing
  - return `ExternalQueryService::list_task_events`
- [ ] Run targeted test.

**Verification:**

```bash
cargo test --test global_workspace_tasks task_events -- --nocapture
```

Expected: PASS.

---

### Phase 3: Synchronize task state from turn lifecycle

**Purpose:** Keep global tasks accurate after their backing turn starts, completes, fails, is interrupted, or is cancelled.

**Files:**

- Modify: `src/application/events.rs`
- Modify: `src/application/tasks.rs` or create a focused internal helper in `src/application/tasks.rs`
- Test: `tests/global_workspace_tasks.rs`

**Mapping:**

| Domain event | Task update | Task event |
| --- | --- | --- |
| `turn.started` | `state = running` | `task.running` |
| `turn.completed` | `state = completed` | `task.completed` |
| `turn.failed` | `state = failed` | `task.failed` |
| `turn.interrupted` | `state = cancelled` or `failed`? | `task.cancelled` or `task.failed` |
| `turn.cancelled` | `state = cancelled` | `task.cancelled` |

Decision: use `task.cancelled` for `turn.interrupted` only if the interruption was user-requested through a task/session control. Otherwise use `task.failed` if the interruption represents runtime failure. For first implementation, map `turn.interrupted` to `cancelled` because existing control APIs treat interrupt as user action.

**Important constraints:**

- Only update a task when `tasks.turn_id = event.turn_id`.
- Do not regress terminal task states.
- Do not update unrelated tasks in the same session.
- Keep event ingestion idempotent: duplicate domain events must not create duplicate task events.

**Suggested implementation:**

- After `EventIngestService::ingest_event` commits event/projection updates, call a task synchronization helper for non-duplicate turn lifecycle events.
- Because `existing_event_state_version` returns early for duplicate events, only perform task sync on newly accepted events.
- Helper should:
  - find task by `turn_id`
  - check current task state
  - update state if transition is allowed
  - insert one `task_events` record

**Tasks:**

- [ ] Write failing test: create task with workspace, ingest `turn.started`, assert task becomes `running` and task event `task.running` exists.
- [ ] Write failing test: ingest `turn.completed`, assert task becomes `completed` and task event `task.completed` exists.
- [ ] Write failing test: duplicate `turn.completed` event does not create duplicate task lifecycle events.
- [ ] Implement minimal task lifecycle sync.
- [ ] Run targeted tests.

**Verification:**

```bash
cargo test --test global_workspace_tasks task_state_follows_turn_lifecycle -- --nocapture
```

Expected: PASS.

---

### Phase 4: Add workspace confirmation and redispatch for ambiguous tasks

**Purpose:** Let WebUI resolve `needs_confirmation` tasks by selecting a workspace, then let backend route, create/select session, and submit the turn.

**Files:**

- Modify: `src/application/tasks.rs`
- Modify: `src/transport/http/mod.rs`
- Modify: `src/transport/http/external.rs`
- Test: `tests/global_workspace_tasks.rs`

**Endpoint:**

```text
POST /external/v1/tasks/{task_id}/confirm-workspace
```

**Request:**

```json
{
  "workspace": "/path/to/project",
  "client_type": "generic"
}
```

`client_type` can default to `generic` if omitted, matching `CreateTaskRequest`.

**Response:**

```json
{
  "data": {
    "task": { "task_id": "task_...", "state": "queued", "workspace_id": "wks_...", "session_id": "sess_...", "turn_id": "turn_..." }
  },
  "error": null
}
```

**State rules:**

- Allowed only when:
  - `state = needs_confirmation`, or
  - `routing_state IN ('ambiguous', 'failed')` and no `turn_id` exists.
- Reject with conflict when task already has a `turn_id` or is terminal.
- Confirmation should:
  - upsert workspace
  - set `routing_state = confirmed` or `matched`
  - record `task.workspace_confirmed`
  - select/create session
  - submit turn
  - set `session_id`, `turn_id`, and task state

**Tasks:**

- [ ] Write failing test: create task without workspace, confirm workspace, assert task is dispatched and linked to workspace/session/turn.
- [ ] Write failing test: confirming an already dispatched task returns conflict.
- [ ] Extract shared dispatch logic from `create_task` so confirmation and direct creation share behavior.
- [ ] Add request type `ConfirmTaskWorkspaceRequest`.
- [ ] Add application method `confirm_workspace(task_id, request, idempotency_key)`.
- [ ] Add HTTP route and handler.
- [ ] Run targeted tests.

**Verification:**

```bash
cargo test --test global_workspace_tasks confirm_workspace -- --nocapture
```

Expected: PASS.

---

### Phase 5: Make task creation idempotent and failure-aware

**Purpose:** Prevent duplicate task creation from UI retries and make partial failures visible as task failures instead of hidden inconsistencies.

**Files:**

- Modify: `src/application/tasks.rs`
- Modify: `src/transport/http/external.rs`
- Test: `tests/global_workspace_tasks.rs`

**Behavior:**

- `POST /external/v1/tasks` should honor `Idempotency-Key`, like session and turn creation already do.
- Duplicate request with same idempotency key should return the same response body and status semantics.
- If routing/session/turn dispatch fails after the task row is created, update task to a stable error state:
  - `state = failed` for dispatch/runtime errors
  - `routing_state = failed` for routing/workspace resolution errors
  - `routing_reason = error message`
  - task event `task.failed` or `task.routing_failed`

**Tasks:**

- [ ] Write failing test: two `POST /tasks` calls with same `Idempotency-Key` return same `task_id`.
- [ ] Write failing test for invalid `client_type` returns error and does not create a task.
- [ ] Write failing test for dispatch failure if a deterministic failure seam exists. If no clean seam exists, document this as follow-up and test the recoverable validation paths.
- [ ] Add idempotency lookup/store to `TaskCommandService::create_task`.
- [ ] Read `Idempotency-Key` in `external::create_task` and pass it through.
- [ ] Wrap post-insert routing/dispatch failure paths so the task row is updated before returning the error.
- [ ] Run targeted tests.

**Verification:**

```bash
cargo test --test global_workspace_tasks task_creation_idempotency -- --nocapture
```

Expected: PASS.

---

### Phase 6: Add task-level control endpoints

**Purpose:** Allow WebUI users to operate on tasks without manually mapping to session/turn IDs.

**Files:**

- Modify: `src/application/tasks.rs`
- Modify: `src/application/runtime_control.rs` only if shared lower-level control helper is needed
- Modify: `src/transport/http/mod.rs`
- Modify: `src/transport/http/external.rs`
- Test: `tests/global_workspace_tasks.rs`

**Endpoints:**

```text
POST /external/v1/tasks/{task_id}/interrupt
POST /external/v1/tasks/{task_id}/cancel
```

**Interrupt semantics:**

- Requires task has `session_id` and `turn_id`.
- Delegates to existing `RuntimeControlService::interrupt_turn(session_id, turn_id, idempotency_key)`.
- Returns task after the control event has been ingested and task sync has run.

**Cancel semantics:**

- If task has no `turn_id` and is not terminal:
  - set `state = cancelled`
  - record `task.cancelled`
- If task has active turn:
  - either delegate to interrupt, or ingest `turn.cancelled` if supported.
- If task is terminal:
  - return conflict or idempotent current terminal state. For first implementation, return conflict unless same idempotency key is replayed.

**Tasks:**

- [ ] Write failing test: task interrupt delegates to existing active turn interrupt and updates task state.
- [ ] Write failing test: cancelling a `needs_confirmation` task marks it cancelled without needing a session/turn.
- [ ] Add application methods `interrupt_task` and `cancel_task`.
- [ ] Add HTTP routes and handlers.
- [ ] Ensure idempotency keys are supported for mutating task controls.
- [ ] Run targeted tests.

**Verification:**

```bash
cargo test --test global_workspace_tasks task_level_controls -- --nocapture
```

Expected: PASS.

---

## Final Verification

Run the full backend and WebUI build checks before handing off to frontend work:

```bash
cargo test --all
cd apps/web && pnpm build
```

Expected:

- All Rust tests pass.
- WebUI still builds even before task-first UI changes.

## Recommended Commit Breakdown

Use these commits across the three agent sessions:

### Agent Session 1

1. `feat: expose task events`

### Agent Session 2

2. `feat: sync task state from turn lifecycle`
3. `feat: confirm workspace for pending tasks`

### Agent Session 3

4. `feat: make task creation idempotent`
5. `feat: add task-level controls`

## Notes and Open Decisions

- `turn.interrupted` is mapped to `task.cancelled` for the first implementation. If later runtime observation distinguishes user interrupt from runtime failure, refine the mapping.
- Full automatic workspace routing is still out of scope. The confirmation endpoint provides the WebUI path for ambiguous tasks.
- A global task event stream is useful but not part of the five blocking backend tasks. Polling `GET /tasks` and `GET /tasks/{task_id}/events` is enough for first WebUI implementation.
- Workspace management beyond listing and task confirmation is also out of scope for this backend completion pass.
