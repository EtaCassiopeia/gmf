# GMF - Thread-Per-Core gRPC Server Framework

![Rust](https://github.com/EtaCassiopeia/gmf/actions/workflows/rust.yml/badge.svg)

A high-performance, runtime-agnostic gRPC server framework for Rust using thread-per-core architecture.

GMF pins one event loop per physical CPU core with no work-stealing, no shared task queues, and no lock contention on the request path.

## Runtimes

| Runtime | Feature | Platforms | Backend |
|---------|---------|-----------|---------|
| **monoio** (default) | `monoio-runtime` | Linux, macOS | io_uring / kqueue |
| **glommio** | `glommio-runtime` | Linux only | io_uring |
| **tokio** | `tokio-runtime` | All | epoll / kqueue |

## Quick Start

```toml
[dependencies]
gmf = "2.0.0"
```

```rust
use gmf::server::MonoioServer;

MonoioServer::builder()
    .addr("0.0.0.0:50051".parse()?)
    .max_connections(10240)
    .num_cores(4)
    .build()
    .serve(GreeterServer::new(MyGreeter))?;
```

To use a different runtime:

```toml
[dependencies]
gmf = { version = "2.0.0", default-features = false, features = ["tokio-runtime"] }
```

## Graceful Shutdown

```rust
MonoioServer::builder()
    .addr(addr)
    .build()
    .serve_with_shutdown(service, signal)?;
```

Where `signal` is any `Future<Output = ()> + Send + 'static` (e.g. a ctrl-c handler).

## How It Works

![Thread-Per-Core Architecture](docs/diagrams/thread-per-core.svg)

Each core runs an independent event loop with its own TCP listener and connection limiter. On Linux, the kernel distributes connections across cores via `SO_REUSEPORT` and each core gets its own io_uring instance (monoio/glommio) or epoll fd (tokio). No userspace load balancing, no shared task queues.

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | Thread-per-core design, io_uring, CPU pinning, request lifecycle |
| [Benchmarking](docs/benchmarking.md) | How to benchmark with `ghz` and criterion |
| [Development](docs/development.md) | Building, testing, Docker setup |
| [Platform Notes](docs/platforms.md) | Linux vs macOS differences, Docker IPv6 |

## Performance

GMF's architecture is designed to outperform work-stealing runtimes on dedicated multi-core
Linux servers by eliminating shared task queues and lock contention, leveraging io_uring
syscall batching (monoio/glommio runtimes), and maintaining CPU cache locality through
core pinning. The advantage grows with core count and hardware isolation.

See [Benchmarking](docs/benchmarking.md) for how to run your own benchmarks and
[Architecture](docs/architecture.md) for a deep dive into why thread-per-core scales better.

## License

Apache-2.0
