# Composio SDK Benchmarks

This directory contains performance benchmarks for the Composio Rust SDK.

## Overview

The benchmarks measure:
- **Session creation time** - How long it takes to create a Tool Router session
- **Tool execution time** - Latency for executing tools and meta tools
- **Retry logic overhead** - Impact of retry mechanism on performance
- **JSON operations** - Serialization/deserialization performance

## Running Benchmarks

### Quick Start

```bash
# Run all benchmarks
cargo bench --package composio-sdk

# Run with detailed output
cargo bench --package composio-sdk -- --verbose
```

### Run Specific Benchmarks

```bash
# Session creation benchmarks only
cargo bench --package composio-sdk -- session_creation

# Tool execution benchmarks only
cargo bench --package composio-sdk -- tool_execution

# Retry logic benchmarks only
cargo bench --package composio-sdk -- retry_logic

# JSON operations benchmarks only
cargo bench --package composio-sdk -- json_operations
```

### Save Baseline for Comparison

```bash
# Save current performance as baseline
cargo bench --package composio-sdk -- --save-baseline main

# Compare against baseline
cargo bench --package composio-sdk -- --baseline main
```

## Benchmark Files

- **`sdk_benchmarks.rs`** - Main Rust SDK benchmarks using Criterion
- **`python_comparison.py`** - Python SDK comparison benchmarks (optional)
- **`README.md`** - This file

## Understanding Results

Criterion outputs several metrics for each benchmark:

- **Mean** - Average execution time
- **Median** - Middle value (50th percentile)
- **StdDev** - Standard deviation (consistency measure)
- **Min/Max** - Fastest and slowest execution times

### Example Output

```
session_creation_minimal
                        time:   [1.234 ms 1.245 ms 1.256 ms]
                        change: [-2.3% -1.1% +0.5%] (p = 0.23 > 0.05)
                        No change in performance detected.
```

This shows:
- Mean time: 1.245 ms
- 95% confidence interval: [1.234 ms, 1.256 ms]
- Change from baseline: -1.1% (slight improvement)
- Statistical significance: p = 0.23 (not significant)

## Python SDK Comparison (Optional)

To compare with Python SDK performance:

```bash
# Install Python dependencies
pip install composio-core

# Run Python benchmarks
python composio-sdk/benches/python_comparison.py
```

Note: The Python comparison uses mock implementations. For real comparison, you would need to set up actual Composio Python SDK with network mocking.

## Performance Goals

The SDK aims to maintain:
- **Memory footprint**: ≤2 MB additional to application
- **SDK overhead**: <1ms per request (excluding network I/O)
- **Retry overhead**: Negligible on successful requests
- **Concurrent throughput**: Efficient handling via tokio async runtime

## Interpreting Results

### What to Look For

1. **Consistency**: Low standard deviation indicates stable performance
2. **Regressions**: Compare against baseline to detect performance degradation
3. **Bottlenecks**: Identify which operations are slowest
4. **Overhead**: SDK overhead should be minimal compared to network I/O

### Expected Performance

- **Session creation**: 1-5 ms (mostly network I/O)
- **Tool execution**: 2-10 ms (mostly network I/O)
- **JSON serialization**: <0.1 ms
- **Retry logic check**: <0.01 ms

Network I/O typically dominates (50-500ms), so SDK overhead should be <1% of total request time.

## Advanced Usage

### Flamegraph Generation

Generate a flamegraph to visualize performance hotspots:

```bash
cargo install flamegraph
cargo flamegraph --bench sdk_benchmarks --package composio-sdk
```

### Memory Profiling

Profile memory usage with valgrind:

```bash
valgrind --tool=massif --massif-out-file=massif.out \
    cargo bench --package composio-sdk -- --profile-time=10
ms_print massif.out
```

### Binary Size Analysis

Analyze what contributes to binary size:

```bash
cargo bloat --release --package composio-sdk
cargo bloat --release --package composio-sdk --crates
```

## CI/CD Integration

Benchmarks run automatically in CI on:
- Pull requests (compare against main branch)
- Main branch commits (update baseline)
- Release tags (document performance)

Performance regressions >10% will fail CI checks.

## Troubleshooting

### Benchmarks Take Too Long

Reduce the number of iterations:

```bash
cargo bench --package composio-sdk -- --sample-size 10
```

### Inconsistent Results

Ensure stable environment:
- Close other applications
- Disable CPU frequency scaling
- Run multiple times and average

### Mock Server Issues

If benchmarks fail with connection errors:
- Check that wiremock is properly installed
- Verify no port conflicts
- Review mock server setup in `sdk_benchmarks.rs`

## Contributing

When adding new benchmarks:

1. Add benchmark function to `sdk_benchmarks.rs`
2. Add to appropriate criterion group
3. Update this README with description
4. Update `PERFORMANCE_REPORT.md` with results
5. Ensure benchmarks are deterministic and reproducible

## Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Composio SDK Documentation](../README.md)
- [Performance Report](../PERFORMANCE_REPORT.md)
