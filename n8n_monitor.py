#!/usr/bin/env python3
"""
N8N Service Monitor with Auto-Restart
Мониторинг N8N с автоматическим перезапуском при недоступности
"""

import asyncio
import aiohttp
import logging
from datetime import datetime
from typing import Optional
import sys
import os
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

# Configuration
N8N_URL = os.getenv("N8N_URL")
N8N_API_KEY = os.getenv("N8N_API_KEY")
CHECK_INTERVAL = int(os.getenv("CHECK_INTERVAL"))
RESTART_COMMAND = os.getenv("N8N_RESTART_COMMAND")
TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN")
TELEGRAM_CHAT_ID = os.getenv("TELEGRAM_CHAT_ID")
MAX_RETRIES = int(os.getenv("MAX_RETRIES"))
TIMEOUT = int(os.getenv("TIMEOUT"))
API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class N8NMonitor:
    """Monitor N8N service and restart if needed."""

    def __init__(self):
        self.consecutive_failures = 0
        self.last_restart = None
        self.telegram_client = None

        if TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID:
            from telethon import TelegramClient
            self.telegram_client = TelegramClient('n8n_monitor', API_ID, API_HASH)

    async def send_telegram_alert(self, message: str):
        """Send alert to Telegram."""
        if not self.telegram_client:
            return

        try:
            await self.telegram_client.connect()
            if not await self.telegram_client.is_user_authorized():
                logger.warning("Telegram client not authorized, skipping alert")
                return

            await self.telegram_client.send_message(
                int(TELEGRAM_CHAT_ID),
                f"🚨 N8N Monitor Alert\n\n{message}\n\nTime: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}"
            )
            logger.info(f"Telegram alert sent: {message}")
        except Exception as e:
            logger.error(f"Failed to send Telegram alert: {e}")
        finally:
            await self.telegram_client.disconnect()

    async def check_n8n_health(self) -> bool:
        """Check if N8N is responding."""
        try:
            async with aiohttp.ClientSession() as session:
                headers = {}
                if N8N_API_KEY:
                    headers['X-N8N-API-KEY'] = N8N_API_KEY

                async with session.get(
                    f"{N8N_URL}/healthz",
                    headers=headers,
                    timeout=aiohttp.ClientTimeout(total=TIMEOUT),
                    ssl=False  # Для self-signed сертификатов
                ) as response:
                    if response.status == 200:
                        logger.info("✅ N8N is healthy")
                        return True
                    else:
                        logger.warning(f"❌ N8N returned status {response.status}")
                        return False
        except asyncio.TimeoutError:
            logger.error(f"❌ N8N health check timeout after {TIMEOUT}s")
            return False
        except aiohttp.ClientError as e:
            logger.error(f"❌ N8N connection error: {e}")
            return False
        except Exception as e:
            logger.error(f"❌ Unexpected error checking N8N: {e}")
            return False

    async def restart_n8n(self):
        """Restart N8N service."""
        if self.last_restart:
            time_since_restart = (datetime.now() - self.last_restart).seconds
            if time_since_restart < 300:  # 5 минут минимум между рестартами
                logger.warning(f"Skipping restart, last restart was {time_since_restart}s ago")
                return False

        logger.info("🔄 Attempting to restart N8N...")
        await self.send_telegram_alert(f"Restarting N8N after {self.consecutive_failures} failed checks")

        try:
            # Выполняем команду перезапуска
            process = await asyncio.create_subprocess_shell(
                RESTART_COMMAND,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await process.communicate()

            if process.returncode == 0:
                logger.info("✅ N8N restart command executed successfully")
                self.last_restart = datetime.now()
                await self.send_telegram_alert("✅ N8N restarted successfully")

                # Ждём 10 секунд перед проверкой
                await asyncio.sleep(10)

                # Проверяем, что сервис поднялся
                is_healthy = await self.check_n8n_health()
                if is_healthy:
                    self.consecutive_failures = 0
                    return True
                else:
                    logger.error("❌ N8N still unhealthy after restart")
                    await self.send_telegram_alert("⚠️ N8N restarted but still unhealthy")
                    return False
            else:
                error_msg = stderr.decode() if stderr else "Unknown error"
                logger.error(f"❌ Failed to restart N8N: {error_msg}")
                await self.send_telegram_alert(f"❌ Failed to restart N8N: {error_msg}")
                return False

        except Exception as e:
            logger.error(f"❌ Exception during restart: {e}")
            await self.send_telegram_alert(f"❌ Exception during restart: {e}")
            return False

    async def monitor_loop(self):
        """Main monitoring loop."""
        logger.info(f"🚀 Starting N8N monitor for {N8N_URL}")
        logger.info(f"Check interval: {CHECK_INTERVAL}s")
        logger.info(f"Restart command: {RESTART_COMMAND}")

        # Отправляем стартовое уведомление
        await self.send_telegram_alert("🚀 N8N Monitor started")

        while True:
            try:
                is_healthy = await self.check_n8n_health()

                if is_healthy:
                    if self.consecutive_failures > 0:
                        logger.info(f"✅ N8N recovered after {self.consecutive_failures} failures")
                        await self.send_telegram_alert(f"✅ N8N recovered after {self.consecutive_failures} failures")
                    self.consecutive_failures = 0
                else:
                    self.consecutive_failures += 1
                    logger.warning(f"⚠️ Consecutive failures: {self.consecutive_failures}/{MAX_RETRIES}")

                    if self.consecutive_failures >= MAX_RETRIES:
                        logger.error(f"❌ N8N failed {MAX_RETRIES} health checks, initiating restart")
                        await self.restart_n8n()

                await asyncio.sleep(CHECK_INTERVAL)

            except KeyboardInterrupt:
                logger.info("👋 Monitor stopped by user")
                await self.send_telegram_alert("👋 N8N Monitor stopped")
                break
            except Exception as e:
                logger.error(f"❌ Unexpected error in monitor loop: {e}")
                await asyncio.sleep(CHECK_INTERVAL)


async def main():
    """Entry point."""
    monitor = N8NMonitor()
    await monitor.monitor_loop()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n👋 Monitor stopped")
        sys.exit(0)
