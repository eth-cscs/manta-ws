FROM rust:1.84.0 as builder
WORKDIR /usr/src/cama
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/cama /usr/local/bin/cama
CMD ["cama"]
