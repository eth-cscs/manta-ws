FROM rust:1.90.0-slim-bookworm AS builder
# Install cmake for building the `librdkafka` crate statically
RUN apt-get update && apt-get install -y --no-install-recommends \
pkg-config \
libssl-dev \
g++ \
cmake \
make
WORKDIR /usr/src/manta-ws
COPY ./src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release --jobs $(nproc)

FROM debian:12.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev
COPY --from=builder /usr/src/manta-ws/target/release/manta-ws /usr/local/bin/manta-ws
CMD ["/usr/local/bin/manta-ws"]
