FROM alpine:latest AS deployment

WORKDIR /deployment

ARG ENV_NAME
COPY deployment/$ENV_NAME .

FROM --platform=linux/amd64 rust as base
RUN rustup default stable
WORKDIR /app
ENV CARGO_MANIFEST_DIR=/app/web3cache
COPY dummy.rs web3cache/dummy.rs
COPY web3cache/Cargo.toml web3cache/Cargo.toml
COPY my-workspace-hack/ my-workspace-hack/
COPY Cargo.toml Cargo.toml 
RUN sed -i 's#src/main.rs#dummy.rs#' web3cache/Cargo.toml
RUN cargo build --package web3cache --release
RUN sed -i 's#dummy.rs#src/main.rs#' web3cache/Cargo.toml

FROM base AS dev

COPY web3cache/src/ web3cache/src/
COPY web3cache/files/ web3cache/files/
COPY web3cache/tests/ web3cache/tests/
RUN cargo build --package web3cache --release

FROM debian:bookworm-slim AS app

COPY web3cache/files/ files/
COPY --from=dev /app/target/release/web3cache .
RUN apt-get update -y && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

CMD ["./web3cache"]