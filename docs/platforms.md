# Platform Notes

## Runtime / Platform Matrix

| | Linux | macOS | Windows |
|---|:---:|:---:|:---:|
| **monoio** (default) | io_uring + CPU pinning | kqueue | not supported |
| **glommio** | io_uring + CPU pinning | not supported | not supported |
| **tokio** | epoll + CPU pinning | kqueue | IOCP |

## Linux

Linux is the primary target. All three runtimes are supported with full performance features:

- **io_uring**: monoio and glommio use io_uring for kernel-level async IO (requires kernel 5.1+).
- **CPU pinning**: Each worker thread is pinned to a physical CPU core for cache locality.
- **`SO_REUSEPORT`**: The kernel distributes incoming connections across per-core listeners, enabling true shared-nothing load balancing.

## macOS

macOS is supported for development with monoio and tokio:

- Uses **kqueue** instead of io_uring.
- **No CPU pinning** — macOS does not expose `sched_setaffinity`. Threads are not bound to specific cores.
- **No `SO_REUSEPORT` load balancing** — macOS supports `SO_REUSEPORT` but does not distribute connections across sockets the way Linux does. Multiple listeners on the same port will work, but the kernel may not balance evenly.
- Performance will be lower than Linux. Use macOS for development; benchmark on Linux.

## Docker

For developing on macOS while targeting Linux features (glommio, io_uring, CPU pinning):

```bash
./build_docker_image.sh
./cargo-docker.sh check --features glommio-runtime --no-default-features
```

See [development.md](development.md) for Docker setup details.

## Docker IPv6

To enable IPv6 in Docker containers:

1. Edit `/etc/docker/daemon.json`:
   ```json
   {
     "ipv6": true,
     "fixed-cidr-v6": "2001:db8:1::/64"
   }
   ```

2. Restart Docker:
   ```bash
   systemctl restart docker
   ```

The `fixed-cidr-v6` subnet is used for the `docker0` bridge and container IPv6 addresses. Adjust the subnet to match your network setup.

## Compile-Time Guards

GMF enforces platform constraints at compile time:

```rust
// At least one runtime feature must be enabled
#[cfg(not(any(feature = "monoio-runtime", feature = "glommio-runtime", feature = "tokio-runtime")))]
compile_error!("Enable at least one runtime feature");

// glommio is Linux-only
#[cfg(all(feature = "glommio-runtime", not(target_os = "linux")))]
compile_error!("glommio-runtime requires Linux");
```
