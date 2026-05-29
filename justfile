default:
    just --list

dev:
    ./scripts/dev-dashboard.sh

backend:
    LLMPARTY_EXTERNAL_API_TOKEN=${LLMPARTY_EXTERNAL_API_TOKEN:-dev-token} cargo run

dashboard:
    pnpm --dir=apps/dashboard run dev

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test

dashboard-check:
    pnpm --dir=apps/dashboard run check

check: fmt-check clippy test dashboard-check
