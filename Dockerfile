FROM rust:1.86.0-slim-bookworm AS builder
# Install cmake for building the `librdkafka` crate statically
RUN apt-get update && apt-get install -y cmake
WORKDIR /usr/src/manta-ws
COPY . .
RUN cargo install --jobs $(nproc) --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/manta-ws /usr/local/bin/manta-ws
CMD ["/usr/local/bin/manta-ws"]
