#!/usr/bin/env python3
"""
Performance comparison script for Python SDK.

This script benchmarks the Python SDK to compare with Rust SDK performance.
Run this alongside the Rust benchmarks to get comparative metrics.

Requirements:
    pip install composio-core timeit

Usage:
    python benches/python_comparison.py
"""

import time
import statistics
from typing import List, Callable
import json

# Mock the Composio SDK for benchmarking purposes
# In a real scenario, you would import: from composio import Composio

class MockComposio:
    """Mock Composio client for benchmarking"""
    
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://backend.composio.dev/api/v3"
    
    def create(self, user_id: str, **kwargs):
        """Mock session creation"""
        time.sleep(0.001)  # Simulate network latency
        return MockSession(user_id, **kwargs)


class MockSession:
    """Mock session for benchmarking"""
    
    def __init__(self, user_id: str, **kwargs):
        self.user_id = user_id
        self.session_id = f"sess_{user_id}"
        self.toolkits = kwargs.get('toolkits', [])
        self.manage_connections = kwargs.get('manage_connections', True)
    
    def execute_tool(self, tool_slug: str, arguments: dict):
        """Mock tool execution"""
        time.sleep(0.002)  # Simulate network latency
        return {
            "data": {"result": "success"},
            "error": None,
            "log_id": "log_123"
        }
    
    def execute_meta_tool(self, slug: str, arguments: dict):
        """Mock meta tool execution"""
        time.sleep(0.002)  # Simulate network latency
        return {
            "data": {"tools": []},
            "error": None,
            "log_id": "log_456"
        }


def benchmark(func: Callable, iterations: int = 100) -> dict:
    """
    Benchmark a function and return statistics.
    
    Args:
        func: Function to benchmark
        iterations: Number of iterations to run
    
    Returns:
        Dictionary with timing statistics
    """
    times: List[float] = []
    
    for _ in range(iterations):
        start = time.perf_counter()
        func()
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to milliseconds
    
    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "min": min(times),
        "max": max(times),
        "iterations": iterations
    }


def bench_session_creation_minimal():
    """Benchmark minimal session creation"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(user_id="user_benchmark")
    return session


def bench_session_creation_with_toolkits():
    """Benchmark session creation with toolkits"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(
        user_id="user_benchmark",
        toolkits=["github", "gmail", "slack"]
    )
    return session


def bench_session_creation_with_config():
    """Benchmark session creation with full config"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(
        user_id="user_benchmark",
        toolkits=["github", "gmail"],
        manage_connections=True
    )
    return session


def bench_tool_execution_simple():
    """Benchmark simple tool execution"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(user_id="user_benchmark")
    result = session.execute_tool(
        "GITHUB_GET_REPOS",
        {"owner": "composio"}
    )
    return result


def bench_tool_execution_complex():
    """Benchmark tool execution with complex arguments"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(user_id="user_benchmark")
    result = session.execute_tool(
        "GITHUB_CREATE_ISSUE",
        {
            "owner": "composio",
            "repo": "composio",
            "title": "Benchmark test issue",
            "body": "This is a benchmark test with complex arguments",
            "labels": ["benchmark", "test", "performance"],
            "assignees": ["user1", "user2"]
        }
    )
    return result


def bench_meta_tool_execution():
    """Benchmark meta tool execution"""
    composio = MockComposio(api_key="test_key")
    session = composio.create(user_id="user_benchmark")
    result = session.execute_meta_tool(
        "COMPOSIO_SEARCH_TOOLS",
        {"query": "create github issue"}
    )
    return result


def bench_json_serialization():
    """Benchmark JSON serialization"""
    data = {
        "user_id": "user_benchmark",
        "toolkits": ["github", "gmail", "slack"],
        "manage_connections": True
    }
    json_str = json.dumps(data)
    return json_str


def bench_json_deserialization():
    """Benchmark JSON deserialization"""
    json_str = '{"data": {"result": "success"}, "error": null, "log_id": "log_123"}'
    data = json.loads(json_str)
    return data


def print_results(name: str, results: dict):
    """Print benchmark results in a formatted way"""
    print(f"\n{name}:")
    print(f"  Mean:   {results['mean']:.3f} ms")
    print(f"  Median: {results['median']:.3f} ms")
    print(f"  StdDev: {results['stdev']:.3f} ms")
    print(f"  Min:    {results['min']:.3f} ms")
    print(f"  Max:    {results['max']:.3f} ms")
    print(f"  Iterations: {results['iterations']}")


def main():
    """Run all benchmarks and print results"""
    print("=" * 70)
    print("Python SDK Performance Benchmarks")
    print("=" * 70)
    
    benchmarks = [
        ("Session Creation (Minimal)", bench_session_creation_minimal),
        ("Session Creation (With Toolkits)", bench_session_creation_with_toolkits),
        ("Session Creation (With Config)", bench_session_creation_with_config),
        ("Tool Execution (Simple)", bench_tool_execution_simple),
        ("Tool Execution (Complex)", bench_tool_execution_complex),
        ("Meta Tool Execution", bench_meta_tool_execution),
        ("JSON Serialization", bench_json_serialization),
        ("JSON Deserialization", bench_json_deserialization),
    ]
    
    for name, func in benchmarks:
        results = benchmark(func, iterations=100)
        print_results(name, results)
    
    print("\n" + "=" * 70)
    print("Benchmark complete!")
    print("=" * 70)
    print("\nNote: These are mock benchmarks. For real comparison, use actual")
    print("Composio Python SDK with network mocking (e.g., responses library).")
    print("\nTo compare with Rust SDK, run:")
    print("  cargo bench --package composio-sdk")


if __name__ == "__main__":
    main()
