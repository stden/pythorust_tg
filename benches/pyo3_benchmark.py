#!/usr/bin/env python3
"""
Benchmark PyO3 (Rust) vs pure Python implementations.

This script compares performance of functions that have been migrated to Rust.
Run from project root: python benches/pyo3_benchmark.py
"""

import re
import time
from datetime import datetime
from statistics import mean, stdev
from typing import Callable, Optional


# Pure Python implementations (copied from original code)
def py_sanitize_filename(name: Optional[str], fallback: str = "unknown_chat", max_len: int = 50) -> str:
    """Python implementation of sanitize_filename."""
    base = re.sub(r"[^\w\s\-]", "", name or "")
    base = re.sub(r"\s+", "_", base.strip())
    return (base or fallback)[:max_len]


def py_contains_phone(text: Optional[str]) -> bool:
    """Python implementation of contains_phone."""
    if not text:
        return False
    digits = re.sub(r"\D", "", text)
    if 10 <= len(digits) <= 15:
        return True
    return bool(re.search(r"\+?\d[\d\-\s\(\)]{8,}\d", text))


def py_detect_conversion(text: str) -> Optional[str]:
    """Python implementation of detect_conversion."""
    if not text:
        return None

    phone_match = re.search(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)", text)
    if phone_match:
        return "phone_shared"

    normalized = text.lower()
    intent_keywords = [
        "беру",
        "покупаю",
        "оформляем",
        "оплачиваю",
        "готов купить",
        "давай оформим",
        "давайте оформим",
        "давай заказ",
        "берем",
        "хочу купить",
    ]
    for kw in intent_keywords:
        if kw in normalized:
            return "purchase_intent"

    delivery_keywords = ["доставка", "оплата", "адрес", "курьер", "самовывоз"]
    if any(word in normalized for word in delivery_keywords) and ("давай" in normalized or "офор" in normalized):
        return "checkout_details"

    return None


def py_is_meaningful_message(text: Optional[str]) -> bool:
    """Python implementation of is_meaningful_message."""
    if not text:
        return False
    cleaned = text.strip()
    if not cleaned:
        return False
    return not cleaned.startswith("/")


def py_parse_iso_datetime(iso_str: Optional[str]) -> Optional[datetime]:
    """Python implementation of parse_iso_datetime."""
    if not iso_str:
        return None
    try:
        if iso_str.endswith("Z"):
            iso_str = iso_str[:-1] + "+00:00"
        return datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
    except (ValueError, AttributeError):
        return None


def py_build_message_text(text: str, has_media: bool) -> str:
    """Python implementation of build_message_text."""
    if has_media:
        return f"{text} [Media]" if text else "[Media]"
    return text


# Import Rust implementations
try:
    from telegram_reader import (
        sanitize_filename as rust_sanitize_filename,
        contains_phone as rust_contains_phone,
        detect_conversion as rust_detect_conversion,
        is_meaningful_message as rust_is_meaningful_message,
        parse_iso_datetime as rust_parse_iso_datetime,
        build_message_text as rust_build_message_text,
    )

    RUST_AVAILABLE = True
except ImportError:
    RUST_AVAILABLE = False
    print("Warning: Rust module not available. Build with 'maturin develop' first.")


def benchmark(func: Callable, args: tuple, iterations: int = 100_000) -> dict:
    """Run a function multiple times and return timing statistics."""
    times = []

    # Warmup
    for _ in range(1000):
        func(*args)

    # Timed runs
    for _ in range(iterations):
        start = time.perf_counter_ns()
        func(*args)
        elapsed = time.perf_counter_ns() - start
        times.append(elapsed)

    return {
        "mean_ns": mean(times),
        "stdev_ns": stdev(times) if len(times) > 1 else 0,
        "min_ns": min(times),
        "max_ns": max(times),
        "iterations": iterations,
    }


def format_ns(ns: float) -> str:
    """Format nanoseconds to human-readable string."""
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    elif ns >= 1000:
        return f"{ns / 1000:.2f} µs"
    else:
        return f"{ns:.0f} ns"


def run_benchmark(name: str, py_func: Callable, rust_func: Callable, args: tuple, iterations: int = 100_000):
    """Run benchmark comparing Python and Rust implementations."""
    print(f"\n{'=' * 60}")
    print(f"Benchmark: {name}")
    print(f"{'=' * 60}")
    print(f"Args: {args}")
    print(f"Iterations: {iterations:,}")

    # Python benchmark
    py_stats = benchmark(py_func, args, iterations)
    print("\nPython:")
    print(f"  Mean: {format_ns(py_stats['mean_ns'])}")
    print(f"  Stdev: {format_ns(py_stats['stdev_ns'])}")

    # Rust benchmark
    if RUST_AVAILABLE:
        rust_stats = benchmark(rust_func, args, iterations)
        print("\nRust:")
        print(f"  Mean: {format_ns(rust_stats['mean_ns'])}")
        print(f"  Stdev: {format_ns(rust_stats['stdev_ns'])}")

        # Speedup calculation
        speedup = py_stats["mean_ns"] / rust_stats["mean_ns"]
        print(f"\nSpeedup: {speedup:.1f}x faster")
    else:
        print("\nRust: Not available")


def main():
    print("PyO3 Benchmarks: Rust vs Python Performance Comparison")
    print("=" * 60)

    if not RUST_AVAILABLE:
        print("\nERROR: Rust module not available!")
        print("Build with: maturin develop")
        return

    # Test data
    test_filenames = [
        "Test @#$ Chat Name",
        "Very Long Chat Name That Should Be Truncated To Max Length Limit",
        "Simple_Name",
    ]

    test_phones = [
        "+7 999 123-45-67",
        "Call me at 8 (495) 123-45-67 please",
        "No phone number here",
        "Short 123",
    ]

    test_conversions = [
        "+7 999 123 45 67",
        "Хочу купить этот товар!",
        "Давай оформим доставку на адрес",
        "Просто сообщение без конверсии",
    ]

    test_messages = [
        "Hello, this is a normal message",
        "/start",
        "",
        "   ",
        "/help with something",
    ]

    test_datetimes = [
        "2024-01-15T10:30:00Z",
        "2024-12-31T23:59:59Z",
        "2024-06-15T12:00:00+03:00",
    ]

    test_texts = [
        ("Hello world", True),
        ("", True),
        ("Long message with media", True),
        ("Plain text", False),
    ]

    # Run benchmarks
    iterations = 50_000

    # 1. sanitize_filename
    for fname in test_filenames[:1]:
        run_benchmark(
            "sanitize_filename",
            lambda n: py_sanitize_filename(n, "fallback", 50),
            lambda n: rust_sanitize_filename(n, "fallback", 50),
            (fname,),
            iterations,
        )

    # 2. contains_phone
    for phone in test_phones[:2]:
        run_benchmark(
            "contains_phone",
            py_contains_phone,
            rust_contains_phone,
            (phone,),
            iterations,
        )

    # 3. detect_conversion
    for conv in test_conversions[:2]:
        run_benchmark(
            "detect_conversion",
            py_detect_conversion,
            rust_detect_conversion,
            (conv,),
            iterations,
        )

    # 4. is_meaningful_message
    for msg in test_messages[:2]:
        run_benchmark(
            "is_meaningful_message",
            py_is_meaningful_message,
            rust_is_meaningful_message,
            (msg,),
            iterations,
        )

    # 5. parse_iso_datetime (Note: Rust returns dict, Python returns datetime)
    def rust_parse_wrapper(s):
        result = rust_parse_iso_datetime(s)
        return result is not None

    def py_parse_wrapper(s):
        result = py_parse_iso_datetime(s)
        return result is not None

    for dt in test_datetimes[:1]:
        run_benchmark(
            "parse_iso_datetime",
            py_parse_wrapper,
            rust_parse_wrapper,
            (dt,),
            iterations,
        )

    # 6. build_message_text
    for text, has_media in test_texts[:2]:
        run_benchmark(
            "build_message_text",
            lambda t, m: py_build_message_text(t, m),
            lambda t, m: rust_build_message_text(t, m),
            (text, has_media),
            iterations,
        )

    # Batch benchmark - simulating real workload
    print(f"\n{'=' * 60}")
    print("Batch Benchmark: Processing 10,000 messages")
    print(f"{'=' * 60}")

    messages = [{"text": f"Message {i}", "has_media": i % 3 == 0} for i in range(10_000)]

    def process_batch_python(msgs):
        for m in msgs:
            py_build_message_text(m["text"], m["has_media"])
            py_is_meaningful_message(m["text"])
            py_contains_phone(m["text"])

    def process_batch_rust(msgs):
        for m in msgs:
            rust_build_message_text(m["text"], m["has_media"])
            rust_is_meaningful_message(m["text"])
            rust_contains_phone(m["text"])

    # Batch benchmark with fewer iterations
    batch_stats_py = benchmark(lambda: process_batch_python(messages), (), 10)
    batch_stats_rust = benchmark(lambda: process_batch_rust(messages), (), 10)

    print("\nPython (10k messages):")
    print(f"  Mean: {format_ns(batch_stats_py['mean_ns'])}")

    print("\nRust (10k messages):")
    print(f"  Mean: {format_ns(batch_stats_rust['mean_ns'])}")

    speedup = batch_stats_py["mean_ns"] / batch_stats_rust["mean_ns"]
    print(f"\nBatch Speedup: {speedup:.1f}x faster")

    print(f"\n{'=' * 60}")
    print("Benchmark Complete!")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
