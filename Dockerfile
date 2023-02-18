FROM rust:1.67 as builder

WORKDIR /usr/builder

COPY src ./src
COPY Cargo.toml Cargo.lock ./

RUN cargo install --path . --locked

FROM scratch

WORKDIR /usr/app

COPY --from=builder /usr/builder/target/release/notificator ./notificator

CMD ["./notificator"]
