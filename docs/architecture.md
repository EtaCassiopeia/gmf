# Architecture

## Thread-Per-Core Design

GMF uses a **shared-nothing, thread-per-core** architecture. Each physical CPU core runs its own independent event loop with:

- Its own TCP listener (via `SO_REUSEPORT` on Linux)
- Its own connection pool and semaphore
- No shared mutable state between cores
- CPU pinning for cache locality

This eliminates the overhead of work-stealing schedulers, cross-thread synchronization, and lock contention that affect traditional multi-threaded servers.

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│   Core 0    │  │   Core 1    │  │   Core 2    │
│             │  │             │  │             │
│ TcpListener │  │ TcpListener │  │ TcpListener │
│ EventLoop   │  │ EventLoop   │  │ EventLoop   │
│ Semaphore   │  │ Semaphore   │  │ Semaphore   │
│ HTTP/2 Conn │  │ HTTP/2 Conn │  │ HTTP/2 Conn │
└─────────────┘  └─────────────┘  └─────────────┘
      │                │                │
      └────────────────┼────────────────┘
                       │
              SO_REUSEPORT (kernel)
                       │
                 ┌─────┴─────┐
                 │ :50051    │
                 └───────────┘
```

## IO Model

The performance advantage comes from the **thread-per-core scheduling model**, not from zero-copy IO at the HTTP layer.

- **monoio**: Uses io_uring (Linux) or kqueue (macOS) for completion-based IO at the kernel level. However, hyper requires poll-based IO, so `monoio-compat` introduces a copy at the HTTP layer.
- **glommio**: Similar to monoio — io_uring-backed, with a `HyperIo` bridge from `futures_lite` traits to `hyper::rt` traits.
- **tokio**: Standard poll-based IO via `TokioIo` from `hyper-util`.

All three runtimes share the same accept loop and HTTP/2 serving logic via the `Runtime` trait abstraction.

## Runtime Trait System

The core abstraction is a set of traits in `gmf::server::runtime`:

```rust
pub trait Runtime: Sized + 'static {
    type TcpListener: RuntimeTcpListener;
    type Executor: RuntimeExecutor + Clone + Default;
    type Semaphore: RuntimeSemaphore;

    fn run_multi_core<F, Fut>(cores: usize, f: F) -> Result<(), GmfError>
    where
        F: Fn(usize) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = Result<(), GmfError>> + 'static;
}

pub trait RuntimeTcpListener: Sized { ... }
pub trait RuntimeTcpStream: Sized + 'static { ... }
pub trait RuntimeExecutor: Clone + Default + 'static { ... }
pub trait RuntimeSemaphore: Sized { ... }
```

Each runtime (monoio, glommio, tokio) implements these traits. The `GmfServer<R: Runtime>` is generic over the runtime, and the accept loop is shared.

### Service Adaptation

Tonic produces `tower_service::Service` implementations, but hyper 1.x has its own `hyper::service::Service` trait (takes `&self`, no `poll_ready`). GMF bridges this with `TowerToHyperService<S>`, which clones the inner service on each call — the standard pattern for tonic services that are `Arc`-wrapped internally.

## Module Structure

```
gmf/src/
├── lib.rs                    # Feature gates, compile-time validation
└── server/
    ├── mod.rs                # Module exports, type aliases (MonoioServer, etc.)
    ├── config.rs             # ServerConfig
    ├── error.rs              # GmfError (thiserror)
    ├── runtime.rs            # Core abstraction traits
    ├── gmf_server.rs         # GmfServer<R>, builder, accept loop, TowerToHyperService
    ├── monoio_runtime.rs     # MonoioRuntime (default)
    ├── glommio_runtime.rs    # GlommioRuntime (Linux only)
    ├── tokio_runtime.rs      # TokioRuntime (fallback)
    └── hyper_io.rs           # HyperIo<T> bridge (glommio only)
```

## Future Compatibility

- **gRPC-Rust / Protobuf Arenas**: The official gRPC-Rust crate (with zero-copy IO and protobuf arenas) is in development. When it ships, arena-based protobuf can be integrated as a codec option through tonic's pluggable codec system. The `Runtime` trait design is forward-compatible.
- **Native HTTP/2**: True end-to-end zero-copy would require a native monoio HTTP/2 implementation (`monoio-http` exists but is immature). The current architecture is ready to swap in alternative HTTP/2 stacks when they mature.
