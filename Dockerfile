# Start from nightly-bookwork image, bookworm is the Debian version #12.
# For more information, see https://github.com/instrumentisto/rust-docker-image/blob/main/README.md
FROM ghcr.io/rust-lang/rust:nightly-bookworm

# Install the protocol buffer compiler (protoc)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-udeps
RUN rustup component add clippy

# Set the working directory
WORKDIR /usr/src/app
