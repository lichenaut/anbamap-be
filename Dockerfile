FROM rust:latest AS builder
WORKDIR /scraper
COPY . /scraper
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev pip python3 python3-venv
COPY --from=builder /scraper/target/release/anbamap-scraper /scraper/
ENTRYPOINT ["/scraper/anbamap-scraper"]