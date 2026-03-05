# Benchmarking

## Docker (Recommended)

The Docker environment ensures fair, reproducible comparisons across all frameworks.

### Build

```bash
docker build --platform linux/amd64 -t gmf-bench .
```

### Run

```bash
docker run --platform linux/amd64 --rm gmf-bench
```

Tune parameters via environment variables:

```bash
docker run --platform linux/amd64 --rm \
    -e CONCURRENCY=500 \
    -e TOTAL=200000 \
    gmf-bench
```

| Variable | Default | Description |
|----------|---------|-------------|
| `CONCURRENCY` | 200 | Concurrent gRPC connections |
| `TOTAL` | 100000 | Total requests per benchmark |

### Frameworks Compared

| Framework | Binary | Description |
|-----------|--------|-------------|
| GMF (monoio) | `helloworld-gmf-server` | Thread-per-core, io_uring (default) |
| GMF (tokio) | `helloworld-gmf-tokio-server` | Thread-per-core, epoll |
| Tonic (tokio) | `helloworld-tonic-server` | Standard work-stealing tokio |

## Local Benchmarks with `ghz`

[ghz](https://github.com/bojand/ghz) is a high-performance gRPC benchmarking tool.

### Install

```bash
brew install ghz
```

### Run

Start the server first, then:

```bash
ghz --insecure \
    --proto examples/proto/helloworld/helloworld.proto \
    --call helloworld.Greeter.SayHello \
    -d '{"name":"Test"}' \
    -c 200 \
    -n 100000 \
    0.0.0.0:50051
```

| Flag | Description |
|------|-------------|
| `--insecure` | Disable TLS |
| `--proto` | Path to `.proto` file |
| `--call` | Fully-qualified method name |
| `-d` | Request payload (JSON) |
| `-c` | Concurrent connections |
| `-n` | Total requests |

## Criterion Microbenchmarks

```bash
# Start the server first
cargo run --release -p examples --bin helloworld-gmf-server

# In another terminal
cargo bench -p examples
```

Reports are saved to `target/criterion/report/index.html`.

## Important Notes

- **Run on native Linux** for meaningful results. Docker Desktop on macOS uses QEMU
  emulation which does not support io_uring, negating the thread-per-core advantage.
  Under QEMU, all three frameworks will show similar performance.
- **On native Linux with io_uring**, GMF (monoio) is expected to significantly outperform
  standard tonic due to: zero-copy kernel IO, no work-stealing overhead, CPU-pinned
  threads, and per-core `SO_REUSEPORT` load balancing.
- Run server and client on the same machine to minimize network variance.
- For production benchmarks, increase `-n` to at least 100,000 and warm up first.
- All servers support `GRPC_PORT` env var to change the listening port (default: 50051).
- Compare runtimes locally by running different binaries:
  ```bash
  cargo run --release -p examples --bin helloworld-gmf-server                                         # monoio (default)
  cargo run --release -p examples --bin helloworld-gmf-tokio-server --no-default-features --features gmf-tokio  # tokio
  cargo run --release -p examples --bin helloworld-tonic-server                                       # tonic baseline
  ```
