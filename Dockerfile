FROM rust:1.67-slim as builder

WORKDIR /usr/builder

COPY src ./src
COPY Cargo.toml Cargo.lock ./

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /usr/app

COPY --from=builder /usr/builder/target/release/notificator ./app

CMD ["./app"]
