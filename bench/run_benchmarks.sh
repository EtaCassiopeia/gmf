#!/usr/bin/env bash
set -uo pipefail

# gRPC Server Benchmark Runner
# Uses ghz (https://ghz.sh) to benchmark gRPC servers under identical conditions.

PROTO_FILE="/opt/proto/helloworld/helloworld.proto"
IMPORT_PATH="/opt/proto"
CALL="helloworld.Greeter.SayHello"
DATA='{"name": "world"}'

# Benchmark parameters
CONCURRENCY="${CONCURRENCY:-200}"
TOTAL="${TOTAL:-100000}"
WARMUP_TOTAL=5000
WARMUP_CONCURRENCY=50

RESULTS_DIR="/tmp/bench-results"
mkdir -p "$RESULTS_DIR"

# Each server gets a unique port to avoid TIME_WAIT conflicts
servers=(
    "helloworld-gmf-server:GMF (monoio):50051"
    "helloworld-gmf-tokio-server:GMF (tokio):50052"
    "helloworld-tonic-server:Tonic (tokio):50053"
)

run_bench() {
    local binary="$1"
    local label="$2"
    local port="$3"
    local host="127.0.0.1:${port}"
    local output_file="$RESULTS_DIR/${binary}.json"

    echo "============================================"
    echo "Benchmarking: $label"
    echo "  Binary: $binary (port $port)"
    echo "  Concurrency: $CONCURRENCY, Total: $TOTAL"
    echo "============================================"

    # Start server with unique port in its own process group
    GRPC_PORT="$port" setsid "$binary" &
    local pid=$!
    sleep 2

    # Verify server is running
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "FAILED: $binary did not start"
        return 1
    fi

    # Warmup
    echo "  Warmup ($WARMUP_TOTAL requests)..."
    ghz --insecure \
        --proto "$PROTO_FILE" \
        --import-paths "$IMPORT_PATH" \
        --call "$CALL" \
        -d "$DATA" \
        -c "$WARMUP_CONCURRENCY" \
        -n "$WARMUP_TOTAL" \
        "$host" > /dev/null 2>&1 || true

    # Actual benchmark
    echo "  Running benchmark..."
    ghz --insecure \
        --proto "$PROTO_FILE" \
        --import-paths "$IMPORT_PATH" \
        --call "$CALL" \
        -d "$DATA" \
        -c "$CONCURRENCY" \
        -n "$TOTAL" \
        --format json \
        "$host" > "$output_file" 2>&1

    # Print latency distribution from JSON
    python3 -c "
import json, sys
try:
    d = json.load(open('$output_file'))
    print(f\"  RPS: {d['rps']:.0f}\")
    print(f\"  Avg: {d['average']/1e6:.2f} ms\")
    for p in d.get('latencyDistribution', []):
        print(f\"  p{int(p['percentage'])}:  {p['latency']/1e6:.2f} ms\")
    codes = d.get('statusCodeDistribution', {})
    for code, count in codes.items():
        print(f\"  [{code}] {count} responses\")
except Exception as e:
    print(f'  Error parsing results: {e}', file=sys.stderr)
" 2>&1

    # Stop server and all child threads/processes
    kill -9 -"$pid" 2>/dev/null || kill -9 "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    sleep 2

    echo ""
}

echo "gRPC Server Benchmark Suite"
echo "==========================="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Concurrency: $CONCURRENCY"
echo "Total requests: $TOTAL"
echo "CPU cores: $(nproc)"
echo ""

for entry in "${servers[@]}"; do
    IFS=':' read -r binary label port <<< "$entry"

    if command -v "$binary" &>/dev/null; then
        run_bench "$binary" "$label" "$port" || true
    else
        echo "SKIP: $binary not found"
        echo ""
    fi
done

echo "==========================="
echo "Results saved to $RESULTS_DIR/"
echo ""

# Summary table
echo "| Framework | Avg Latency | p99 Latency | Throughput (rps) |"
echo "|-----------|-------------|-------------|------------------|"
for entry in "${servers[@]}"; do
    IFS=':' read -r binary label port <<< "$entry"
    json="$RESULTS_DIR/${binary}.json"
    if [ -f "$json" ]; then
        avg=$(python3 -c "import json; d=json.load(open('$json')); print(f\"{d['average']/1e6:.2f} ms\")" 2>/dev/null || echo "N/A")
        p99=$(python3 -c "import json; d=json.load(open('$json')); lats=d.get('latencyDistribution',[]); p99=[x for x in lats if x['percentage']==99]; print(f\"{p99[0]['latency']/1e6:.2f} ms\" if p99 else 'N/A')" 2>/dev/null || echo "N/A")
        rps=$(python3 -c "import json; d=json.load(open('$json')); print(f\"{d['rps']:.0f}\")" 2>/dev/null || echo "N/A")
        echo "| $label | $avg | $p99 | $rps |"
    fi
done
