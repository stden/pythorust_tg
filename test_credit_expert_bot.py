#!/usr/bin/env python3
"""
Test script for Credit Expert Bot components
Tests database connection, Telegram session, and AI integration
"""

import asyncio
import os
import sys
from pathlib import Path

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent))

from dotenv import load_dotenv

load_dotenv()


async def test_telegram_connection():
    """Test basic Telegram connection using existing session"""
    print("\n=== Testing Telegram Connection ===")

    from telethon import TelegramClient

    API_ID = int(os.getenv("TELEGRAM_API_ID"))
    API_HASH = os.getenv("TELEGRAM_API_HASH")
    SESSION_FILE = "telegram_session"

    print(f"API_ID: {API_ID}")
    print(f"Session file: {SESSION_FILE}")

    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)

    try:
        await client.connect()

        if await client.is_user_authorized():
            me = await client.get_me()
            print(f"✅ Connected as: {me.first_name} (@{me.username})")
            print(f"   User ID: {me.id}")

            # Get recent dialogs
            dialogs = await client.get_dialogs(limit=5)
            print(f"\n📱 Recent chats ({len(dialogs)}):")
            for d in dialogs[:5]:
                print(f"   - {d.name}")

            return True
        else:
            print("❌ Not authorized. Run `cargo run -- init-session` first.")
            return False

    except Exception as e:
        print(f"❌ Connection error: {e}")
        return False
    finally:
        await client.disconnect()


def test_mysql_connection():
    """Test MySQL database connection"""
    print("\n=== Testing MySQL Connection ===")

    try:
        import pymysql

        config = {
            "host": os.getenv("MYSQL_HOST", "localhost"),
            "port": int(os.getenv("MYSQL_PORT", 3306)),
            "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
            "user": os.getenv("MYSQL_USER", "pythorust_tg"),
            "password": os.getenv("MYSQL_PASSWORD"),
            "charset": "utf8mb4",
            "cursorclass": pymysql.cursors.DictCursor,
        }

        print(f"Host: {config['host']}:{config['port']}")
        print(f"Database: {config['database']}")
        print(f"User: {config['user']}")

        conn = pymysql.connect(**config)

        with conn.cursor() as cursor:
            cursor.execute("SELECT VERSION()")
            version = cursor.fetchone()
            print(f"✅ MySQL Version: {version['VERSION()']}")

            # Check if tables exist
            cursor.execute("SHOW TABLES LIKE 'bot_%'")
            tables = cursor.fetchall()
            print(f"\n📊 Bot tables found: {len(tables)}")
            for t in tables:
                print(f"   - {list(t.values())[0]}")

        conn.close()
        return True

    except ImportError:
        print("❌ pymysql not installed")
        return False
    except Exception as e:
        print(f"❌ MySQL error: {e}")
        return False


async def test_ai_client():
    """Test AI client (OpenAI/Google)"""
    print("\n=== Testing AI Client ===")

    openai_key = os.getenv("OPENAI_API_KEY")
    google_key = os.getenv("GOOGLE_API_KEY")

    if openai_key:
        print("Using OpenAI API")
        try:
            from integrations.openai_client import OpenAIClient

            client = OpenAIClient()

            messages = [
                {"role": "system", "content": "Ты тестовый бот. Ответь одним словом: Привет"},
                {"role": "user", "content": "Тест"},
            ]

            response = await client.chat_completion(messages)
            print(f"✅ AI Response: {response.choices[0].message.content[:100]}...")
            return True
        except Exception as e:
            print(f"❌ OpenAI error: {e}")
            return False
    elif google_key:
        print("Using Google Gemini API")
        print("✅ GOOGLE_API_KEY is set")
        return True
    else:
        print("⚠️ No AI API key configured (OPENAI_API_KEY or GOOGLE_API_KEY)")
        return False


async def test_send_message():
    """Test sending a message to a specific chat"""
    print("\n=== Testing Send Message ===")

    from telethon import TelegramClient

    API_ID = int(os.getenv("TELEGRAM_API_ID"))
    API_HASH = os.getenv("TELEGRAM_API_HASH")
    SESSION_FILE = "telegram_session"

    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)

    try:
        await client.connect()

        if not await client.is_user_authorized():
            print("❌ Not authorized")
            return False

        # Send test message to "Saved Messages" (self)
        await client.get_me()

        test_message = "🧪 Тестовое сообщение от Credit Expert Bot test script"
        sent = await client.send_message("me", test_message)
        print(f"✅ Sent test message to Saved Messages (id: {sent.id})")

        return True

    except Exception as e:
        print(f"❌ Error: {e}")
        return False
    finally:
        await client.disconnect()


async def main():
    print("=" * 50)
    print("Credit Expert Bot - Component Tests")
    print("=" * 50)

    results = {}

    # Test Telegram
    results["telegram"] = await test_telegram_connection()

    # Test MySQL
    results["mysql"] = test_mysql_connection()

    # Test AI
    results["ai"] = await test_ai_client()

    # Test sending message
    if results["telegram"]:
        results["send_message"] = await test_send_message()
    else:
        results["send_message"] = False

    # Summary
    print("\n" + "=" * 50)
    print("SUMMARY")
    print("=" * 50)

    for component, status in results.items():
        emoji = "✅" if status else "❌"
        print(f"{emoji} {component.upper()}: {'OK' if status else 'FAILED'}")

    all_passed = all(results.values())
    print("\n" + ("🎉 All tests passed!" if all_passed else "⚠️ Some tests failed"))

    return 0 if all_passed else 1


if __name__ == "__main__":
    exit(asyncio.run(main()))
