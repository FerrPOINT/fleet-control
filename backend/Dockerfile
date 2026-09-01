FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/shared ./shared
COPY backend/domain ./domain
COPY backend/app ./app
COPY backend/infra ./infra
COPY backend/migration ./migration
COPY backend/api ./api
COPY backend/server ./server
COPY backend/cli ./cli

RUN cargo build --release --bin server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 wget && rm -rf /var/lib/apt/lists/*
RUN groupadd -r fleet-control && useradd -r -g fleet-control -d /app -s /usr/sbin/nologin fleet-control
WORKDIR /app
COPY --from=builder /app/target/release/server /usr/local/bin/fleet-control-server
RUN mkdir -p /var/lib/fleet-control/agents && chown -R fleet-control:fleet-control /var/lib/fleet-control
USER fleet-control
EXPOSE 23801
CMD ["fleet-control-server"]
