"""Main ChatAnalyzer class."""

import asyncio
from pathlib import Path
from typing import Optional

from .config import AnalyzerConfig
from .fetcher import MessageFetcher
from .llm_analyzer import LLMAnalyzer
from .models import ChatAnalysisResult


class ChatAnalyzer:
    """Main chat analyzer orchestrating fetching and LLM analysis."""

    def __init__(self, config: Optional[AnalyzerConfig] = None):
        """Initialize ChatAnalyzer.

        Args:
            config: Optional configuration, defaults to AnalyzerConfig()
        """
        self.config = config or AnalyzerConfig()

        # Ensure output directory exists
        self.config.output_dir.mkdir(parents=True, exist_ok=True)

        # Initialize LLM analyzer
        self.llm_analyzer = LLMAnalyzer(self.config)

    async def analyze_chat(self, chat_identifier: str, prompt_template: Optional[str] = None) -> ChatAnalysisResult:
        """Analyze a Telegram chat.

        Args:
            chat_identifier: Chat username, ID, or URL
            prompt_template: Optional custom prompt template

        Returns:
            ChatAnalysisResult with complete analysis
        """
        if self.config.verbose:
            print(f"Starting analysis of chat: {chat_identifier}")
            print("Configuration:")
            print(f"  - LLM Provider: {self.config.llm_provider}")
            print(f"  - Model: {self.config.model}")
            print(f"  - Message Limit: {self.config.message_limit}")
            print(f"  - Days Back: {self.config.days_back}")
            print()

        # Fetch messages
        async with MessageFetcher(self.config) as fetcher:
            if self.config.verbose:
                print("Fetching messages...")

            messages = await fetcher.get_messages(chat_identifier)

            if not messages:
                raise ValueError(f"No messages found in chat '{chat_identifier}'")

            if self.config.verbose:
                print(f"Fetched {len(messages)} messages")
                print()

            # Format messages for LLM
            messages_text = fetcher.format_messages_for_llm(messages)

            # Get metadata
            metadata = fetcher.get_metadata(messages)

        # Analyze with LLM
        if self.config.verbose:
            print("Analyzing with LLM...")

        result = self.llm_analyzer.analyze(
            messages_text=messages_text, metadata=metadata, chat_name=chat_identifier, prompt_template=prompt_template
        )

        if self.config.verbose:
            print("Analysis complete!")
            print()

        # Save results
        await self._save_results(result, chat_identifier)

        return result

    async def _save_results(self, result: ChatAnalysisResult, chat_identifier: str):
        """Save analysis results to files.

        Args:
            result: Analysis result
            chat_identifier: Chat identifier for filename
        """
        # Sanitize filename
        safe_name = "".join(c if c.isalnum() or c in "-_" else "_" for c in chat_identifier)
        timestamp = result.analyzed_at.strftime("%Y%m%d_%H%M%S")
        base_filename = f"{safe_name}_{timestamp}"

        # Save JSON
        if self.config.output_format in ("json", "both"):
            json_path = self.config.output_dir / f"{base_filename}.json"
            result.save_json(json_path)
            if self.config.verbose:
                print(f"Saved JSON to: {json_path}")

        # Save Markdown
        if self.config.output_format in ("markdown", "both"):
            md_path = self.config.output_dir / f"{base_filename}.md"
            result.save_markdown(md_path)
            if self.config.verbose:
                print(f"Saved Markdown to: {md_path}")


def run_cli():
    """Run ChatAnalyzer from command line."""
    import argparse

    parser = argparse.ArgumentParser(description="Analyze Telegram chat using AI")

    parser.add_argument("chat", help="Chat username, ID, or URL (e.g., @channel_name)")

    parser.add_argument(
        "--provider", choices=["openai", "claude", "gemini"], default="openai", help="LLM provider (default: openai)"
    )

    parser.add_argument("--model", help="Model name (default: auto-detect based on provider)")

    parser.add_argument("--limit", type=int, default=1000, help="Maximum number of messages to analyze (default: 1000)")

    parser.add_argument("--days", type=int, default=30, help="How many days back to fetch messages (default: 30)")

    parser.add_argument(
        "--output-format", choices=["json", "markdown", "both"], default="both", help="Output format (default: both)"
    )

    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("./analysis_results"),
        help="Output directory (default: ./analysis_results)",
    )

    parser.add_argument("--prompt", type=Path, help="Path to custom prompt template")

    parser.add_argument("--quiet", action="store_true", help="Suppress verbose output")

    parser.add_argument("--include-media", action="store_true", help="Include media messages in analysis")

    parser.add_argument("--include-bots", action="store_true", help="Include bot messages in analysis")

    parser.add_argument("--min-length", type=int, default=10, help="Minimum message length to include (default: 10)")

    args = parser.parse_args()

    # Build configuration
    config = AnalyzerConfig(
        message_limit=args.limit,
        days_back=args.days,
        llm_provider=args.provider,
        model=args.model,
        output_format=args.output_format,
        output_dir=args.output_dir,
        verbose=not args.quiet,
        include_media=args.include_media,
        exclude_bots=not args.include_bots,
        min_message_length=args.min_length,
    )

    # Load custom prompt if provided
    prompt_template = None
    if args.prompt:
        if not args.prompt.exists():
            print(f"Error: Prompt file not found: {args.prompt}")
            return 1
        prompt_template = args.prompt.read_text(encoding="utf-8")

    # Run analysis
    try:
        analyzer = ChatAnalyzer(config)
        result = asyncio.run(analyzer.analyze_chat(args.chat, prompt_template))

        # Print summary
        if not args.quiet:
            print()
            print("=" * 80)
            print("ANALYSIS SUMMARY")
            print("=" * 80)
            print(f"Chat: {result.chat_name}")
            print(f"Category: {result.category}")
            print(f"Subcategories: {', '.join(result.subcategories)}")
            print(f"Sentiment: {result.sentiment}")
            print(f"Activity Level: {result.activity_level}")
            print(f"Total Messages: {result.activity_metrics.total_messages}")
            print(f"Active Users: {result.activity_metrics.active_users}")
            print(f"Messages/Day: {result.activity_metrics.messages_per_day:.1f}")
            print()
            print(f"Summary: {result.summary}")
            print()
            print(f"Topics ({len(result.topics)}):")
            for topic in result.topics[:5]:
                print(f"  - {topic.name} ({topic.mentions} mentions, {topic.sentiment})")
            print()
            print("Key Insights:")
            for insight in result.insights[:5]:
                print(f"  - {insight}")
            print("=" * 80)

        return 0

    except Exception as e:
        print(f"Error: {e}")
        if config.verbose:
            import traceback

            traceback.print_exc()
        return 1


if __name__ == "__main__":
    exit(run_cli())
