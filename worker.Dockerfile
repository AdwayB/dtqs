FROM rust:1.66 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin dtqs_worker

FROM debian:buster-slim
COPY --from=builder /app/target/release/dtqs_worker /usr/local/bin/dtqs_worker
CMD ["dtqs_worker"]
