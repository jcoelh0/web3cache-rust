FROM rust

WORKDIR /app

RUN mkdir static
COPY src/ src/
COPY Cargo.toml .

RUN cargo build

EXPOSE 3100

CMD ["cargo","run"]