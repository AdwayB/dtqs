FROM rust:1.66 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin dtqs_cli

FROM debian:buster-slim
COPY --from=builder /app/target/release/dtqs_cli /usr/local/bin/dtqs_cli
CMD ["dtqs_cli"]
