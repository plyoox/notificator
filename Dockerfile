FROM rust:1.67 as builder

WORKDIR /usr/builder

COPY src ./src
COPY Cargo.toml .

RUN cargo install --path .

FROM rust:1.67-slim

WORKDIR /usr/app

COPY --from=builder /usr/builder/target/release/twitch-service ./service

CMD ["./service"]
