# Development

## Prerequisites

- Rust (stable, edition 2021)
- `protoc` (Protocol Buffers compiler)

### Install protoc

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
apt-get install -y protobuf-compiler

# Nix
nix-shell -p protobuf
```

## Building

```bash
# Default (monoio runtime)
cargo build

# Specific runtime
cargo build --no-default-features --features tokio-runtime

# All features (Linux only — glommio requires Linux)
cargo build --all-features
```

## Testing

```bash
cargo test
```

## Running Examples

```bash
# Start the GMF server (monoio)
cargo run -p examples --bin helloworld-gmf-server

# In another terminal, start the client
cargo run -p examples --bin helloworld-gmf-client

# Tonic baseline server (for comparison)
cargo run -p examples --bin helloworld-tonic-server
cargo run -p examples --bin helloworld-tonic-client
```

Set log level with `RUST_LOG`:

```bash
RUST_LOG=info cargo run -p examples --bin helloworld-gmf-server
RUST_LOG=debug cargo run -p examples --bin helloworld-gmf-server
```

## Verification

Run the full check pipeline before submitting changes:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Check each feature independently:

```bash
cargo clippy --features monoio-runtime -p gmf -- -D warnings
cargo clippy --features tokio-runtime --no-default-features -p gmf -- -D warnings
cargo clippy -p examples -- -D warnings
```

## Docker (for non-Linux development)

If you need to test the glommio runtime or run on a Linux-like environment:

### Build the Docker Image

```bash
chmod +x build_docker_image.sh
./build_docker_image.sh
```

### Run Commands in Docker

```bash
chmod +x cargo-docker.sh

./cargo-docker.sh check
./cargo-docker.sh test
./cargo-docker.sh run --package examples --bin helloworld-gmf-server
```

## Nix

A `shell.nix` is provided for reproducible dev environments:

```bash
nix-shell
```

This gives you `rustc`, `cargo`, and `protobuf` without installing them globally.
