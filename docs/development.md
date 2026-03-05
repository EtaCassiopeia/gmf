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

## Docker (for benchmarking)

The Dockerfile builds all server binaries and runs benchmarks. See [Benchmarking](benchmarking.md) for details.

```bash
docker build --platform linux/amd64 -t gmf-bench .
docker run --platform linux/amd64 --rm gmf-bench
```
