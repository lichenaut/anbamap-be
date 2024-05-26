FROM rust:latest
WORKDIR /usr/src/anbamap-be
COPY . .
RUN cargo build --release
ENTRYPOINT ["./target/release/anbamap-be"]