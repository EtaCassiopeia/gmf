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

## GitHub Actions (Native Linux)

The benchmark workflow runs on native Linux via GitHub Actions, triggered automatically
on push to `feat/runtime-agnostic-v2` or manually via `workflow_dispatch`.

```bash
# Trigger manually with custom parameters
gh workflow run benchmark.yml -f concurrency=500 -f total=200000
```

Results appear in the workflow's **Step Summary** and as downloadable artifacts.

## Results

### Shared CI (GitHub Actions, 2-core, 200 concurrency, 100k requests)

| Framework | RPS | Avg Latency | p50 | p99 |
|-----------|-----|-------------|-----|-----|
| GMF (monoio) | 15,769 | 7.78 ms | 7.40 ms | 24.63 ms |
| GMF (tokio) | 15,763 | 7.99 ms | 7.53 ms | 45.39 ms |
| Tonic (tokio) | 15,422 | 8.26 ms | 8.19 ms | 20.51 ms |

On shared CI hardware, all three frameworks converge in throughput. The 2-core runner
limits thread-per-core benefits, and noisy-neighbor effects dominate variance.

### Where GMF Shines

> See [Architecture: Thread-Per-Core vs Work-Stealing](architecture.md#thread-per-core-vs-work-stealing) and the [io_uring Deep Dive](architecture.md#io_uring-deep-dive) for detailed diagrams and explanations of the mechanisms below.

GMF's thread-per-core architecture is designed for **dedicated multi-core Linux servers**,
not shared CI runners. The performance advantage scales with core count and hardware
isolation:

- **Core scaling**: Each GMF core runs an independent event loop with its own
  `SO_REUSEPORT` listener. There is zero cross-thread synchronization — no work-stealing
  scheduler, no shared task queues, no mutex contention. On an 8+ core machine, this
  means near-linear throughput scaling, while tonic's work-stealing scheduler hits
  contention on the shared run queue.

- **io_uring (monoio)**: On Linux 5.6+, monoio uses io_uring for syscall batching and
  kernel-side polling. Multiple IO operations are submitted in a single syscall, and
  completions are reaped without context switches. This reduces per-request syscall
  overhead compared to epoll's `epoll_wait` + `read`/`write` cycle.

- **CPU pinning**: Each GMF thread is pinned to a specific CPU core via
  `sched_setaffinity`. This eliminates CPU migration overhead and maximizes L1/L2 cache
  hit rates. Combined with `SO_REUSEPORT`, the kernel distributes connections across
  cores without any userspace load balancing.

- **No `Send`/`Sync` overhead**: Because each core is single-threaded, GMF uses
  `Rc`/`Cell` instead of `Arc`/`Mutex` for per-connection state. This eliminates atomic
  operations on every reference count and lock acquisition.

**To see GMF's full advantage, benchmark on:**

1. Dedicated bare-metal or VM with 4+ physical cores
2. Native Linux with kernel 5.6+ (for io_uring)
3. High concurrency (500+ connections) and high request volume (1M+ requests)
4. CPU-pinned benchmarking client (e.g., `taskset` with ghz)

Under these conditions, expect GMF (monoio) to significantly outperform standard tonic in
both throughput and tail latency.

## Tips

- **Run on native Linux** for meaningful results. Docker Desktop on macOS uses QEMU
  emulation which does not support io_uring, negating the thread-per-core advantage.
  Under QEMU, all three frameworks will show similar performance.
- Run server and client on the same machine to minimize network variance.
- For production benchmarks, increase `-n` to at least 100,000 and warm up first.
- All servers support `GRPC_PORT` env var to change the listening port (default: 50051).
- Compare runtimes locally by running different binaries:
  ```bash
  cargo run --release -p examples --bin helloworld-gmf-server                                         # monoio (default)
  cargo run --release -p examples --bin helloworld-gmf-tokio-server --no-default-features --features gmf-tokio  # tokio
  cargo run --release -p examples --bin helloworld-tonic-server                                       # tonic baseline
  ```
