# GMF - Thread-Per-Core gRPC Server Framework

![Rust](https://github.com/EtaCassiopeia/gmf/actions/workflows/rust.yml/badge.svg)

A high-performance, runtime-agnostic gRPC server framework for Rust using thread-per-core architecture.

GMF pins one event loop per physical CPU core with no work-stealing, no lock contention, and no cross-thread synchronization. In benchmarks, this achieves ~84% higher throughput than standard tonic.

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

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | Thread-per-core design, IO model, runtime traits |
| [Benchmarking](docs/benchmarking.md) | How to benchmark with `ghz` and criterion |
| [Development](docs/development.md) | Building, testing, Docker, Nix setup |
| [Platform Notes](docs/platforms.md) | Linux vs macOS differences, Docker IPv6 |

## Performance

| Metric | tonic (baseline) | GMF |
|--------|-----------------|-----|
| Requests/sec | 6,512 | 12,010 |
| Avg latency | 28.00 ms | 16.54 ms |
| Min latency | 7.71 ms | 1.58 ms |

*200 concurrent connections, 2000 total requests via `ghz`.*

## License

MIT
