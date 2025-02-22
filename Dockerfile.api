FROM rust:1.66 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin dtqs_api

FROM debian:buster-slim
COPY --from=builder /app/target/release/dtqs_api /usr/local/bin/dtqs_api
EXPOSE 8080
CMD ["dtqs_api"]
