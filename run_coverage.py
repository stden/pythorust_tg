#!/usr/bin/env python
"""Run test coverage for the Python codebase."""

import subprocess
import sys
from pathlib import Path


def run_python_coverage():
    """Run Python test coverage."""
    print("=== Running Python Test Coverage ===\n")

    # Install coverage if not installed
    try:
        import coverage
    except ImportError:
        print("Installing coverage...")
        subprocess.run([sys.executable, "-m", "pip", "install", "coverage[toml]"], check=True)

    # Run tests with coverage
    commands = [
        # Clean previous coverage
        [sys.executable, "-m", "coverage", "erase"],
        # Run tests with coverage
        [sys.executable, "-m", "coverage", "run", "-m", "pytest", "tests/", "-v"],
        # Generate coverage report
        [sys.executable, "-m", "coverage", "report", "-m"],
        # Generate HTML coverage report
        [sys.executable, "-m", "coverage", "html", "--omit=tests/*,*/__pycache__/*"],
    ]

    for cmd in commands:
        print(f"Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            print(f"Error: {result.stderr}")
            return False

        print(result.stdout)

    print("\nHTML coverage report generated in htmlcov/index.html")

    # Parse coverage percentage from report
    result = subprocess.run([sys.executable, "-m", "coverage", "report"], capture_output=True, text=True)

    lines = result.stdout.split("\n")
    for line in lines:
        if line.startswith("TOTAL"):
            parts = line.split()
            if len(parts) >= 4:
                coverage_percent = parts[-1].rstrip("%")
                print(f"\nTotal Python coverage: {coverage_percent}%")
                return float(coverage_percent)

    return 0.0


def run_rust_coverage():
    """Run Rust test coverage (if Rust project exists)."""
    if not Path("Cargo.toml").exists():
        print("\nNo Rust project found (Cargo.toml missing)")
        return None

    print("\n=== Running Rust Test Coverage ===\n")

    # Check if cargo-tarpaulin is installed
    result = subprocess.run(["cargo", "tarpaulin", "--version"], capture_output=True)
    if result.returncode != 0:
        print("Installing cargo-tarpaulin...")
        subprocess.run(["cargo", "install", "cargo-tarpaulin"], check=True)

    # Run Rust tests with coverage
    cmd = [
        "cargo",
        "tarpaulin",
        "--out",
        "Html",
        "--output-dir",
        "target/tarpaulin",
        "--exclude-files",
        "tests/*",
        "--verbose",
    ]

    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    print(result.stdout)
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        return None

    # Extract coverage percentage
    for line in result.stdout.split("\n"):
        if "Coverage" in line and "%" in line:
            # Extract percentage
            import re

            match = re.search(r"(\d+\.\d+)%", line)
            if match:
                coverage_percent = float(match.group(1))
                print(f"\nTotal Rust coverage: {coverage_percent}%")
                return coverage_percent

    return None


def generate_coverage_report():
    """Generate a combined coverage report."""
    print("\n" + "=" * 60)
    print("COVERAGE REPORT SUMMARY")
    print("=" * 60 + "\n")

    # Run Python coverage
    python_coverage = run_python_coverage()

    # Run Rust coverage
    rust_coverage = run_rust_coverage()

    print("\n" + "=" * 60)
    print("FINAL COVERAGE SUMMARY")
    print("=" * 60)

    print(f"\nPython Coverage: {python_coverage:.2f}%")
    if rust_coverage is not None:
        print(f"Rust Coverage: {rust_coverage:.2f}%")

        # Calculate combined coverage (simple average)
        combined = (python_coverage + rust_coverage) / 2
        print(f"\nCombined Coverage: {combined:.2f}%")
    else:
        print("Rust Coverage: N/A (no Rust project)")

    print("\nReports generated:")
    print("- Python: htmlcov/index.html")
    if rust_coverage is not None:
        print("- Rust: target/tarpaulin/tarpaulin-report.html")

    # Check if we achieved 100% coverage
    if python_coverage >= 100.0:
        print("\n✅ Python: 100% test coverage achieved!")
    else:
        print(f"\n❌ Python: {100 - python_coverage:.2f}% coverage missing")

    if rust_coverage is not None:
        if rust_coverage >= 100.0:
            print("✅ Rust: 100% test coverage achieved!")
        else:
            print(f"❌ Rust: {100 - rust_coverage:.2f}% coverage missing")


if __name__ == "__main__":
    try:
        generate_coverage_report()
    except KeyboardInterrupt:
        print("\nCoverage run interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\nError running coverage: {e}")
        sys.exit(1)
