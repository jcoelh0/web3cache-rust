FROM alpine:latest AS deployment

WORKDIR /deployment

ARG ENV_NAME
COPY deployment/$ENV_NAME .

FROM rust as base
RUN rustup default stable
WORKDIR /app



COPY dummy.rs .
COPY Cargo.toml .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml


FROM base AS dev


COPY src/ src/
RUN cargo build --release


FROM debian:bookworm-slim AS app
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl-dev curl && rm -rf /var/lib/apt/lists/*
RUN curl -O https://s3.us-west-2.amazonaws.com/amazon-eks/1.25.6/2023-01-30/bin/linux/amd64/kubectl
RUN chmod +x ./kubectl
RUN mkdir -p $HOME/bin && cp ./kubectl $HOME/bin/kubectl && export PATH=$PATH:$HOME/bin
RUN echo 'export PATH=$PATH:$HOME/bin' >> ~/.bashrc
COPY --from=dev /app/target/release/web3cache .
COPY deployments/ deployments/


CMD ["./web3cache"]