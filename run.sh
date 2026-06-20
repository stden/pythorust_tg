#!/bin/bash
# Run Python scripts using uv
# Usage: ./run.sh script.py [args...]

if [ -z "$1" ]; then
    echo "Usage: ./run.sh <script.py> [args...]"
    echo "Example: ./run.sh n8n_monitor.py"
    exit 1
fi

uv run python "$@"
