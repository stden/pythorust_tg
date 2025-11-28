"""
Refactored autoanswer bot following SOLID principles.

This is an example of how to use the new base classes.
Original autoanswer.py is kept for backward compatibility.
"""

import os
from pathlib import Path

from telethon import events
from dotenv import load_dotenv

from telegram_bot_base import ConfigurableTelegramBot
from ai_service import OpenAIService, ConversationManager

load_dotenv()


class AutoAnswerBot(ConfigurableTelegramBot):
    """
    AI-powered auto-answer bot (SOLID: SRP, OCP, DIP).
    
    Responsibilities:
    - Message handling
    - Routing to AI service
    - Response delivery
    
    Uses dependency injection for AI service (DIP).
    """

    def __init__(
        self,
        ai_service: OpenAIService,
        system_instructions: str,
        **kwargs
    ):
        """
        Initialize auto-answer bot.
        
        Args:
            ai_service: AI service for generating responses (DIP)
            system_instructions: Instructions for the AI
            **kwargs: Passed to ConfigurableTelegramBot
        """
        super().__init__(**kwargs)
        self.ai_service = ai_service
        self.system_instructions = system_instructions
        self.conversation = ConversationManager(system_instructions)

    async def setup_handlers(self):
        """Setup message event handler."""
        @self.client.on(events.NewMessage)
        async def handle_message(event):
            await self._handle_message(event)

    async def on_start(self):
        """Called when bot starts."""
        print("Auto-answer bot запущен. Ожидаю сообщения...")

    async def _handle_message(self, event):
        """
        Handle incoming message (SOLID: SRP).
        
        Responsibilities:
        - Filtering (outgoing, empty)
        - AI interaction
        - Response sending
        """
        # Filter out own messages
        if event.out:
            return

        # Get message text
        user_message = event.message.message.strip()
        if not user_message:
            return

        try:
            # Build message list for AI
            messages = [
                {"role": "system", "content": self.system_instructions},
                {"role": "user", "content": user_message}
            ]

            # Get AI response
            bot_reply = await self.ai_service.chat_completion(messages)

            # Send response
            await event.respond(bot_reply)

        except Exception as e:
            print(f"Ошибка при генерации ответа: {e}")


def create_autoanswer_bot() -> AutoAnswerBot:
    """
    Factory function to create configured bot (SOLID: SRP).
    
    Returns:
        Configured AutoAnswerBot instance
    """
    # Load config
    config_path = Path(__file__).resolve().parent / "config.yml"
    
    # Initialize AI service
    openai_model = os.getenv("OPENAI_MODEL", "gpt-4o-mini")
    ai_service = OpenAIService(default_model=openai_model)
    
    # System instructions
    system_instructions = (
        "Ты - полезный ассистент, который отвечает на вопросы в Telegram-чате. "
        "Старайся давать подробные, ясные и понятные ответы. "
        "Отвечай нейтральным тоном, при необходимости давай примеры кода и избегай ненормативной лексики. "
        "Если пользователь задаёт технический вопрос, постарайся дать максимально понятный и точный ответ. "
        "Если пользователь не указал иное, отвечай на русском языке."
    )
    
    # Create and return bot
    return AutoAnswerBot(
        ai_service=ai_service,
        system_instructions=system_instructions,
        config_path=str(config_path),
        use_session_lock=True,
    )


async def main():
    """Main entry point."""
    bot = create_autoanswer_bot()
    await bot.run()


if __name__ == '__main__':
    import asyncio
    asyncio.run(main())
