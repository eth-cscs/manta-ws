FROM rust:1.85.1 as builder
# Install cmake for building the `librdkafka` crate statically
RUN apt-get update && apt-get install -y cmake
WORKDIR /usr/src/cama
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/cama /usr/local/bin/cama
CMD ["cama"]
