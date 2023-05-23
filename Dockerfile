# Start from the Rust 1.69.0 image
FROM rust:1.69.0-buster

# Install the protocol buffer compiler (protoc)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/app
