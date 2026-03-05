FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Install ghz for gRPC benchmarking
RUN arch=$(dpkg --print-architecture) && \
    case "$arch" in \
        amd64) ghz_arch="x86_64" ;; \
        arm64) ghz_arch="arm64" ;; \
        *) echo "Unsupported architecture: $arch" && exit 1 ;; \
    esac && \
    curl -sSL "https://github.com/bojand/ghz/releases/download/v0.121.0/ghz-linux-${ghz_arch}.tar.gz" \
        | tar xz -C /usr/local/bin ghz

WORKDIR /usr/src/app
COPY . .

# Build all server binaries
RUN cargo build --release -p examples --bin helloworld-gmf-server
RUN cargo build --release -p examples --bin helloworld-gmf-tokio-server \
    --no-default-features --features gmf-tokio
RUN cargo build --release -p examples --bin helloworld-tonic-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y procps python3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/ghz /usr/local/bin/ghz
COPY --from=builder /usr/src/app/target/release/helloworld-gmf-server /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/helloworld-gmf-tokio-server /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/helloworld-tonic-server /usr/local/bin/
COPY examples/proto /opt/proto
COPY bench/run_benchmarks.sh /usr/local/bin/run_benchmarks.sh
RUN chmod +x /usr/local/bin/run_benchmarks.sh

ENTRYPOINT ["/usr/local/bin/run_benchmarks.sh"]
