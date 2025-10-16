FROM alpine:latest AS deployment

WORKDIR /deployment

ARG ENV_NAME
COPY deployment/$ENV_NAME .

FROM rust as base
RUN rustup default stable
WORKDIR /app
COPY dummy.rs web3cache/dummy.rs
COPY web3cache/Cargo.toml web3cache/Cargo.toml
COPY my-workspace-hack/ my-workspace-hack/
COPY Cargo.toml Cargo.toml 
RUN sed -i 's#src/main.rs#dummy.rs#' web3cache/Cargo.toml
RUN cargo build --package web3cache --release
RUN sed -i 's#dummy.rs#src/main.rs#' web3cache/Cargo.toml

FROM base AS dev

COPY web3cache/ web3cache/
RUN cargo build --package web3cache --release


FROM debian:bookworm-slim AS app


COPY --from=dev /app/target/release/web3cache .

EXPOSE 3000
CMD ["./web3cache"]