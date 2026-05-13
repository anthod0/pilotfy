# llmparty Dashboard v2

New Svelte + Vite + shadcn-svelte dashboard. This app is separate from the legacy dashboard in `apps/web`.

## Development

```bash
pnpm --dir apps/dashboard install
LLMPARTY_EXTERNAL_API_TOKEN=dev-token cargo run
pnpm --dir apps/dashboard dev
```

The Vite dev server proxies `/external/*` to `http://127.0.0.1:8080`.

## Build and serve through llmparty

```bash
pnpm --dir apps/dashboard build
LLMPARTY_DASHBOARD_SOURCE=apps/dashboard/dist LLMPARTY_EXTERNAL_API_TOKEN=dev-token cargo run
```

Open <http://127.0.0.1:8080/dashboard>.

Equivalent TOML config:

```toml
[dashboard]
source = "apps/dashboard/dist"
```

Use `source = "apps/web/dist"` to switch back to the legacy dashboard.

## Implementation plan

See `../../specs/plans/2026-05-13-dashboard-v2.md`.
