#!/usr/bin/env python3
"""
Chat Analyzer CLI

AI-powered Telegram chat analysis tool.

Usage:
    python chat_analyzer.py @channel_name
    python chat_analyzer.py https://t.me/channel_name
    python chat_analyzer.py --help

Examples:
    # Analyze with default settings (OpenAI, 1000 messages, 30 days)
    python chat_analyzer.py @vibecod3rs

    # Use Claude instead of OpenAI
    python chat_analyzer.py @vibecod3rs --provider claude

    # Analyze last 7 days only
    python chat_analyzer.py @vibecod3rs --days 7

    # Analyze 2000 messages
    python chat_analyzer.py @vibecod3rs --limit 2000

    # Save only JSON output
    python chat_analyzer.py @vibecod3rs --output-format json

    # Use custom prompt
    python chat_analyzer.py @vibecod3rs --prompt my_prompt.md

Environment Variables Required:
    TELEGRAM_API_ID         - Telegram API ID (from https://my.telegram.org/)
    TELEGRAM_API_HASH       - Telegram API Hash
    OPENAI_API_KEY          - OpenAI API key (if using --provider openai)
    ANTHROPIC_API_KEY       - Anthropic API key (if using --provider claude)
    GOOGLE_API_KEY          - Google API key (if using --provider gemini)

Output:
    Analysis results saved to ./analysis_results/ directory
    - JSON file with structured data
    - Markdown file with formatted report
"""

from chat_analysis.analyzer import run_cli

if __name__ == "__main__":
    exit(run_cli())
