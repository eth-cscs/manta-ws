FROM rust:1.86.0-slim-bookworm AS builder
# Install cmake for building the `librdkafka` crate statically
RUN apt-get update && apt-get install -y --no-install-recommends cmake
WORKDIR /usr/src/manta-ws
COPY . .
RUN cargo build --locked --release 

FROM debian:bookworm-slim
COPY --from=builder /usr/src/manta-ws/target/release/manta-ws /usr/local/bin/manta-ws
CMD ["/usr/local/bin/manta-ws"]
