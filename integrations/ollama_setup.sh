#!/bin/bash
# Ollama Installation and Setup Script
# Для локального запуска LLM моделей

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  УСТАНОВКА OLLAMA - Локальный LLM сервер                      ║"
echo "╚═══════════════════════════════════════════════════════════════╝"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    SUDO="sudo"
else
    SUDO=""
fi

# Install Ollama
echo ""
echo "1. Установка Ollama..."
if command -v ollama &> /dev/null; then
    echo "   ✓ Ollama уже установлена"
    ollama --version
else
    echo "   Скачиваем и устанавливаем..."
    curl -fsSL https://ollama.com/install.sh | sh
    echo "   ✓ Ollama установлена"
fi

# Start Ollama service
echo ""
echo "2. Запуск сервиса Ollama..."
$SUDO systemctl enable ollama 2>/dev/null || true
$SUDO systemctl start ollama 2>/dev/null || ollama serve &
sleep 3
echo "   ✓ Ollama сервер запущен на http://localhost:11434"

# Pull recommended models for voice AI
echo ""
echo "3. Загрузка рекомендуемых моделей..."
echo "   (Это может занять несколько минут)"

# Small fast model for testing
echo ""
echo "   Загружаем qwen2.5:3b (быстрая для тестов)..."
ollama pull qwen2.5:3b || echo "   ⚠ Не удалось загрузить qwen2.5:3b"

# Good balance model for production
echo ""
echo "   Загружаем deepseek-coder:6.7b (для кодинга)..."
ollama pull deepseek-coder:6.7b || echo "   ⚠ Не удалось загрузить deepseek-coder"

# Russian language model
echo ""
echo "   Загружаем llama3.1:8b (универсальная)..."
ollama pull llama3.1:8b || echo "   ⚠ Не удалось загрузить llama3.1:8b"

# List installed models
echo ""
echo "4. Установленные модели:"
ollama list

# Test
echo ""
echo "5. Тест модели..."
echo "   Отправляем запрос к qwen2.5:3b..."
curl -s http://localhost:11434/api/generate -d '{
  "model": "qwen2.5:3b",
  "prompt": "Скажи одним словом: работаю",
  "stream": false
}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('response', 'Ошибка'))" 2>/dev/null || echo "Тест не удался"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "✓ Установка завершена!"
echo ""
echo "Доступные модели для голосового продавца:"
echo "  - qwen2.5:3b      (1.5GB, быстрая)"
echo "  - llama3.1:8b     (4.5GB, качественная)"
echo "  - deepseek-coder  (4GB, для кода)"
echo ""
echo "Использование:"
echo "  ollama run qwen2.5:3b"
echo "  ollama run llama3.1:8b"
echo ""
echo "API endpoint: http://localhost:11434"
echo "═══════════════════════════════════════════════════════════════"
