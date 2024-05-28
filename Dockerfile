FROM rust:latest AS builder
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev pip python3 python3-venv
COPY --from=builder /target/release/anbamap-be /
ENTRYPOINT ["/anbamap-be"]