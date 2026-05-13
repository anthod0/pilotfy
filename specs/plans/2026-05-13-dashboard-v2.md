# Dashboard v2 Implementation Plan

> **For agentic workers:** Each phase is intended for one fresh implementation agent. Do not modify `apps/web`; it remains the legacy dashboard. Use `pnpm`, not `npm`. Run the verification commands listed for your phase before handing off.

**Goal:** Build a new shadcn-svelte based dashboard in `apps/dashboard` that is served from the existing `/dashboard` entrypoint by configuring `[dashboard].source = "apps/dashboard/dist"`.

**Architecture:** The backend continues to serve exactly one configured dashboard source. The new frontend is a Svelte + Vite SPA with browser-history routes under `/dashboard`, shadcn-svelte UI components, and External API-only data access. DAG Tasks are the primary workflow; sessions are shown only as task execution detail/diagnostics.

**Tech Stack:** Rust/Axum static dashboard fallback, Svelte 5, Vite, Tailwind CSS v4, shadcn-svelte, `svelte-mini-router`, pnpm.

---

## Current scaffold

Already initialized:

- `apps/dashboard/` Svelte + Vite app
- Tailwind CSS v4 via `@tailwindcss/vite`
- shadcn-svelte config: `apps/dashboard/components.json`
- Initial shadcn components under `apps/dashboard/src/lib/components/ui/`
- Router dependency: `svelte-mini-router`
- Vite base: `/dashboard/`
- Vite dev proxy: `/external -> http://127.0.0.1:8080`

Useful commands:

```bash
pnpm --dir apps/dashboard check
pnpm --dir apps/dashboard build
LLMPARTY_DASHBOARD_SOURCE=apps/dashboard/dist cargo run
```

---

## Phase 1: Hosting, routing, and app shell

**Purpose:** Make `/dashboard/*` a real SPA entrypoint and replace the scaffold screen with the dashboard frame.

**Files likely touched:**

- Backend:
  - `src/transport/http/mod.rs`
  - `src/transport/http/dashboard.rs`
  - relevant Rust tests in the same module or `tests/`
- Frontend:
  - `apps/dashboard/src/App.svelte`
  - `apps/dashboard/src/main.ts`
  - `apps/dashboard/src/routes.ts`
  - `apps/dashboard/src/components/layout/AppShell.svelte`
  - `apps/dashboard/src/components/layout/AppSidebar.svelte`
  - `apps/dashboard/src/components/layout/TopBar.svelte`
  - placeholder pages under `apps/dashboard/src/pages/`

**Work summary:**

1. Add backend fallback for `/dashboard/{*path}` while preserving `/dashboard/assets/{*path}` asset serving.
2. Ensure `/dashboard`, `/dashboard/`, and `/dashboard/tasks/example/dag` all return `index.html` when the configured source is valid.
3. Implement browser-history frontend routing with base `/dashboard`.
4. Build the shadcn sidebar layout:
   - Overview
   - DAG Tasks
   - Workspaces
   - Agent Profiles
   - Settings
   - optional collapsed Diagnostics section
5. Add placeholder pages for all target routes:
   - `/overview`
   - `/tasks`
   - `/tasks/:taskId/overview`
   - `/tasks/:taskId/dag`
   - `/tasks/:taskId/work-items`
   - `/tasks/:taskId/sessions`
   - `/tasks/:taskId/artifacts`
   - `/tasks/:taskId/activity`
   - `/workspaces`
   - `/agent-profiles`
   - `/settings`

**Acceptance criteria:**

- `apps/web` remains unchanged except unrelated lockfile side effects must be avoided.
- `pnpm --dir apps/dashboard check` passes.
- `pnpm --dir apps/dashboard build` passes.
- `cargo test dashboard` or the relevant dashboard tests pass.
- With `LLMPARTY_DASHBOARD_SOURCE=apps/dashboard/dist`, refreshing a nested route such as `/dashboard/tasks/test/dag` serves the SPA.

---

## Phase 2: API client, auth, live data foundation

**Purpose:** Port the data foundation from legacy dashboard without porting its UI structure.

**Files likely touched:**

- Create/modify:
  - `apps/dashboard/src/api/client.ts`
  - `apps/dashboard/src/api/types.ts`
  - `apps/dashboard/src/api/errors.ts`
  - `apps/dashboard/src/stores/auth.ts`
  - `apps/dashboard/src/stores/connection.ts`
  - `apps/dashboard/src/stores/tasks.ts`
  - `apps/dashboard/src/stores/workspaces.ts`
  - `apps/dashboard/src/stores/agentProfiles.ts`
  - `apps/dashboard/src/services/eventStream.ts`
  - `apps/dashboard/src/pages/SettingsPage.svelte`
  - `apps/dashboard/src/pages/OverviewPage.svelte`

**Work summary:**

1. Reuse the External API contract from `apps/web/src/api/*`, but place the new copy under `apps/dashboard/src/api/*`.
2. Keep API state sourced from External API responses/SSE only; do not read SQLite, runtime dirs, or workspace files directly.
3. Implement bearer token storage and Settings UI.
4. Implement dashboard SSE stream connection state.
5. Implement minimal task/workspace/profile stores with load/error/loading states.
6. Make Overview show real summary cards from loaded tasks and connection status.

**Acceptance criteria:**

- No standalone session creation/turn composer UI is introduced.
- Mutating API calls still send `Idempotency-Key`.
- Missing token produces a clear Settings/TopBar warning.
- `pnpm --dir apps/dashboard check` and `pnpm --dir apps/dashboard build` pass.

---

## Phase 3: DAG Task workflow

**Purpose:** Build the primary product flow: create DAG Task, list tasks, inspect task tabs, and perform task-level actions.

**Files likely touched:**

- `apps/dashboard/src/pages/TasksPage.svelte`
- `apps/dashboard/src/pages/TaskDetailPage.svelte`
- `apps/dashboard/src/components/tasks/*`
- `apps/dashboard/src/components/dag/*`
- `apps/dashboard/src/stores/tasks.ts`
- `apps/dashboard/src/api/client.ts`

**Work summary:**

1. Replace legacy Normal/DAG task selector with DAG-only task creation.
2. Require/select workspace for DAG task creation.
3. Implement Tasks list with state, routing, updated time, and active task navigation.
4. Implement Task detail tabs:
   - Overview: state, open signals, current blockers, task actions.
   - DAG: work item graph/list using `TaskDagView` data. A table/tree is acceptable for v1; do not block on a visual graph library.
   - Work Items: work item runtime state and run summaries.
   - Activity: task events and DAG signals.
5. Implement task actions currently supported by External API: pause, resume, interrupt, cancel, planner input, human signal.

**Acceptance criteria:**

- Creating a DAG task calls `/external/v1/dag-tasks`, not legacy `/tasks` normal creation.
- Task detail URLs are shareable and refreshable.
- Empty/loading/error states use shadcn components.
- `pnpm --dir apps/dashboard check` and `pnpm --dir apps/dashboard build` pass.

---

## Phase 4: Configuration pages, execution detail, and polish

**Purpose:** Complete the independent configuration entrypoints and add task execution diagnostics without making sessions a primary workflow.

**Files likely touched:**

- `apps/dashboard/src/pages/WorkspacesPage.svelte`
- `apps/dashboard/src/pages/AgentProfilesPage.svelte`
- `apps/dashboard/src/pages/SettingsPage.svelte`
- `apps/dashboard/src/components/workspaces/*`
- `apps/dashboard/src/components/agent-profiles/*`
- `apps/dashboard/src/components/artifacts/*`
- `apps/dashboard/src/components/sessions/*`
- `apps/dashboard/src/stores/artifacts.ts`
- `apps/dashboard/src/stores/sessions.ts`
- `apps/dashboard/src/stores/turns.ts`

**Work summary:**

1. Build Workspaces page with root browsing and workspace registration.
2. Build Agent Profiles page with profile list/detail. Editing can be omitted unless the External API support is already sufficient and straightforward.
3. Build Settings page around token, connection state, and dashboard source instructions.
4. Implement Task `Sessions` tab as advanced execution detail only:
   - associated session metadata
   - turns/history
   - session events if needed for diagnostics
   - no standalone create session flow
5. Implement Task `Artifacts` tab with artifact discovery/list/content viewer.
6. Polish responsive behavior, navigation active states, and common empty/error/loading states.

**Acceptance criteria:**

- Workspaces and Agent Profiles have independent sidebar entries.
- Sessions are not a top-level primary workflow.
- Artifact viewing uses External API only.
- `pnpm --dir apps/dashboard check` and `pnpm --dir apps/dashboard build` pass.
- Run relevant backend tests if backend routes were touched by this phase.

---

## Final integration checklist

Run before declaring the dashboard v2 ready:

```bash
cargo fmt --check
cargo test
pnpm --dir apps/dashboard check
pnpm --dir apps/dashboard build
```

Manual smoke test:

1. Build `apps/dashboard`.
2. Start backend with `LLMPARTY_DASHBOARD_SOURCE=apps/dashboard/dist`.
3. Open `/dashboard`.
4. Set External API token.
5. Navigate Overview, Tasks, Workspaces, Agent Profiles, Settings.
6. Create a DAG task.
7. Refresh a nested task URL and verify the SPA loads.
8. Switch back to `LLMPARTY_DASHBOARD_SOURCE=apps/web/dist` and verify the legacy dashboard still serves.
