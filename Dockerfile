FROM rust:latest as builder
WORKDIR /usr/src/image-resizer
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/image-resizer /usr/local/bin/image-resizer

# Expose the port the app runs on
EXPOSE 8080

# Set the PORT environment variable
ENV PORT=8080

CMD ["image-resizer"]