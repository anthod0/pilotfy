# AGENTS.md

Workspace instructions for coding agents working in `/home/cheny/projects/llmparty`.

## Project snapshot

- `llmparty` is a Rust console/control plane for coding agents with a web dashboard and client integrations.
- Backend: Rust 2024, Axum, Tokio, SQLx/SQLite.
- Frontend/dashboard and client plugins use pnpm.
- Key paths: `src/`, `tests/`, `apps/web/`, `clients/pi/`, `clients/claude-code/`, `spec/`, `MILESTONE.md`, `README.md`.

## Architecture rules

- External state must come from the event store and projections. Do not treat tmux state, runtime logs, pi/Claude internals, or workspace files as authoritative External API state.
- Dashboard and orchestrators should use `/external/v1/*` only. The Web UI must not read SQLite, runtime directories, workspace files, or `/internal/v1/*` directly.
- `/internal/v1/events` is for runtime / adapter / agent-client confirmed facts only.
- Keep client-specific behavior inside adapter/runtime/client-plugin boundaries (`src/adapters/`, `src/runtime/`, `clients/*/`). Do not leak pi/Claude-specific fields into generic domain events or External API view models.
- pi and Claude Code turn output/completion/failure must be reported by hooks through the Internal Event API. Do not parse TUI screen contents, runtime logs, or tmux process exit as turn completion facts.
- Preserve idempotency behavior for mutating External API routes that accept `Idempotency-Key`.

## Runtime rules

- Runtimes are long-lived tmux sessions named like `llmparty_<sanitized_session_id>`.
- Default runtime/data root is `~/.local/share/llmparty`; `LLMPARTY_DATA_DIR` can override it.
- Per-session diagnostics live under `runtimes/<session_id>/` and include runtime logs, adapter event logs, current-turn context, and hook logs.

## Common commands

- Backend checks/tests:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- Dashboard:
  - `pnpm --dir apps/web typecheck`
  - `pnpm --dir apps/web build`
- Client packages:
  - `pnpm --dir clients/pi test`
  - `pnpm --dir clients/pi typecheck`
  - `pnpm --dir clients/claude-code test`
  - `pnpm --dir clients/claude-code typecheck`

## Resource index

- Claude Code plugin reference: <https://code.claude.com/docs/en/plugins-reference> — technical reference for plugin schemas, hooks, commands, and components.

Notes:

- Client plugin packages currently have `test` and `typecheck` scripts, not `build` scripts.
- See `.env.example`, `README.md`, and package READMEs for detailed runtime configuration and local manual validation steps.
- If enabling the optional Rust `lbug` feature in a fresh environment, run `scripts/download-ladybug-prebuilt.sh` first.
