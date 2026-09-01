# Fleet Control unified dev commands
# Requires: just, cargo, pnpm, docker compose

set fallback
set shell := ["bash", "-uc"]

default:
    @just --list

setup:
    cd backend && cargo fetch
    cd frontend && pnpm install

setup-env:
    @if [ ! -f .env ]; then cp .env.example .env && echo "Created .env from .env.example"; else echo ".env already exists"; fi

db-up:
    docker compose up -d postgres redis

db-down:
    docker compose down

backend-dev:
    docker compose up -d postgres redis backend
    @echo "Backend at http://127.0.0.1:23801"

frontend-dev:
    cd frontend && pnpm dev -- --host 0.0.0.0 --port 5173

dev:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v tmux >/dev/null 2>&1; then
      tmux new-session -d -s fleet-control "just backend-dev"
      tmux split-window -h -t fleet-control "just frontend-dev"
      tmux attach -t fleet-control
    else
      echo "tmux not found. Run in two terminals: just backend-dev | just frontend-dev"
    fi

fmt-rust:
    cd backend && cargo fmt --all

fmt-check-rust:
    cd backend && cargo fmt --all -- --check

fmt-frontend:
    cd frontend && pnpm format:check

fmt: fmt-rust fmt-frontend

clippy:
    cd backend && cargo clippy --workspace --all-targets

typecheck:
    cd frontend && pnpm typecheck

lint:
    cd frontend && pnpm lint

test-frontend:
    cd frontend && pnpm test

test-backend:
    cd backend && cargo test --workspace -- --test-threads=1

test: test-backend test-frontend

e2e:
    cd frontend && pnpm exec playwright test

gate: fmt-check-rust clippy typecheck lint test-frontend test-backend

build-frontend:
    cd frontend && pnpm build

build-backend:
    cd backend && cargo build --release

build: build-backend build-frontend

api-codegen:
    cd backend && cargo run -p api --bin gen-openapi > ../openapi/openapi.json
    cd frontend && pnpm generate:api

api-docs:
    xdg-open http://127.0.0.1:23801/swagger-ui/ 2>/dev/null || open http://127.0.0.1:23801/swagger-ui/

git-status:
    git status --short

clean:
    cd backend && cargo clean
    cd frontend && rm -rf dist node_modules/.vitest
