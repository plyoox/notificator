FROM rust:1 AS chef

RUN cargo install cargo-chef
WORKDIR /usr/app

FROM chef as planner

COPY Cargo.toml Cargo.lock ./

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /usr/app

COPY --from=builder /usr/app/target/release/notificator ./app

CMD ["./app"]
