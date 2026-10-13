#!/bin/bash
# Ollama Installation and Setup Script
# For running local LLM models

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  OLLAMA INSTALLATION - Local LLM server                       ║"
echo "╚═══════════════════════════════════════════════════════════════╝"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
else
    SUDO=""
fi

# Install Ollama
echo ""
echo "1. Installing Ollama..."
if command -v ollama &> /dev/null; then
    echo "   ✓ Ollama is already installed"
    ollama --version
else
    echo "   Downloading and installing..."
    curl -fsSL https://ollama.com/install.sh | sh
    echo "   ✓ Ollama installed"
fi

# Start Ollama service
echo ""
echo "2. Starting Ollama service..."
$SUDO systemctl enable ollama 2>/dev/null || true
$SUDO systemctl start ollama 2>/dev/null || ollama serve &
sleep 3
echo "   ✓ Ollama server is running at http://localhost:11434"

# Pull recommended models for voice AI
echo ""
echo "3. Downloading recommended models..."
echo "   (This may take a few minutes)"

# Small fast model for testing
echo ""
echo "   Pulling qwen2.5:3b (fast for tests)..."
ollama pull qwen2.5:3b || echo "   ⚠ Failed to pull qwen2.5:3b"

# Good balance model for production
echo ""
echo "   Pulling deepseek-coder:6.7b (for coding)..."
ollama pull deepseek-coder:6.7b || echo "   ⚠ Failed to pull deepseek-coder"

# Russian language model
echo ""
echo "   Pulling llama3.1:8b (general-purpose)..."
ollama pull llama3.1:8b || echo "   ⚠ Failed to pull llama3.1:8b"

# List installed models
echo ""
echo "4. Installed models:"
ollama list

# Test
echo ""
echo "5. Model test..."
echo "   Sending a request to qwen2.5:3b..."
curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:3b",
  "prompt": "Say in one word: working",
  "stream": false
}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('response', 'Error'))" 2>/dev/null || echo "Test failed"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✓ Installation complete!"
echo ""
echo "Available models for the voice sales agent:"
echo "  - qwen2.5:3b      (1.5GB, fast)"
echo "  - llama3.1:8b     (4.5GB, high quality)"
echo "  - deepseek-coder  (4GB, for coding)"
echo ""
echo "Usage:"
echo "  ollama run qwen2.5:3b"
echo "  ollama run llama3.1:8b"
echo ""
echo "API endpoint: http://localhost:11434"
echo "═══════════════════════════════════════════════════════════════"
