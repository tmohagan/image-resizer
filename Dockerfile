FROM rust:1.68 as builder
WORKDIR /usr/src/image-resizer
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/image-resizer /usr/local/bin/image-resizer
CMD ["image-resizer"]