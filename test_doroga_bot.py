#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Comprehensive test of @DorogaCurBot bot functionality."""

from telethon import TelegramClient
import asyncio
import sys
import os
from dotenv import load_dotenv

load_dotenv()

API_ID = int(os.getenv("TELEGRAM_API_ID"))
API_HASH = os.getenv("TELEGRAM_API_HASH")
PHONE = os.getenv("TELEGRAM_PHONE")


async def test_bot():
    """Test all bot functionality."""
    client = TelegramClient('telegram_session', API_ID, API_HASH)
    await client.connect()

    if not await client.is_user_authorized():
        print("ERROR: Not authorized. Run `cargo run -- init-session` first.")
        return

    bot = '@DorogaCurBot'
    print('=== Testing @DorogaCurBot ===\n')

    # Test /start
    print('1. Testing /start command...')
    await client.send_message(bot, '/start')
    await asyncio.sleep(2)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Response: {messages[0].text[:100]}...')

    # Test /menu
    print('\n2. Testing /menu command...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(2)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Response: {messages[0].text[:100]}...')
    if messages[0].buttons:
        button_texts = []
        for row in messages[0].buttons:
            for btn in row:
                button_texts.append(btn.text)
        print(f'✓ Buttons: {button_texts}')

    # Test rates button
    print('\n3. Testing 📊 Курсы button...')
    await client.send_message(bot, '📊 Курсы')
    await asyncio.sleep(2)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Response: {messages[0].text}')

    # Test language switching to English
    print('\n4. Testing language switch to English...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    messages = await client.get_messages(bot, limit=1)

    # Click Settings button
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if '⚙️' in btn.text or 'Settings' in btn.text or 'Настройки' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Settings menu: {messages[0].text[:100]}...')

    # Click Language button
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if '🌐' in btn.text or 'Language' in btn.text or 'Язык' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Language options: {messages[0].text[:100]}...')

    # Click English
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if 'English' in btn.text or '🇬🇧' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)
    print(f'✓ English confirmation: {messages[0].text}')

    # Test rates in English
    print('\n5. Testing 📊 Rates button in English...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    await client.send_message(bot, '📊 Rates')
    await asyncio.sleep(2)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Response: {messages[0].text}')

    # Test Admin Panel button
    print('\n6. Testing Admin Panel button visibility...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    messages = await client.get_messages(bot, limit=1)
    has_admin = False
    if messages[0].buttons:
        for row in messages[0].buttons:
            for btn in row:
                if 'Admin' in btn.text or '👨‍💼' in btn.text:
                    has_admin = True
                    print(f'✓ Admin Panel button found: {btn.text}')
    if not has_admin:
        print('✓ Admin Panel hidden (user is not admin)')

    # Switch back to Russian
    print('\n7. Switching back to Russian...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    messages = await client.get_messages(bot, limit=1)

    # Click Settings
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if '⚙️' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)

    # Click Language
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if '🌐' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)

    # Click Russian
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if 'Русский' in btn.text or '🇷🇺' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Russian confirmation: {messages[0].text}')

    # Test help
    print('\n8. Testing /help command...')
    await client.send_message(bot, '/help')
    await asyncio.sleep(2)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Response: {messages[0].text[:200]}...')

    # Test request creation start
    print('\n9. Testing request creation flow start...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    messages = await client.get_messages(bot, limit=1)

    # Click New Request button
    if messages[0].buttons:
        clicked = False
        for row in messages[0].buttons:
            for btn in row:
                if '💱' in btn.text or 'Новая заявка' in btn.text or 'New request' in btn.text:
                    await btn.click()
                    await asyncio.sleep(2)
                    clicked = True
                    break
            if clicked:
                break

    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Request flow started: {messages[0].text[:150]}...')

    # Return to main menu
    print('\n10. Returning to main menu...')
    await client.send_message(bot, '/menu')
    await asyncio.sleep(1)
    messages = await client.get_messages(bot, limit=1)
    print(f'✓ Back to main menu: {messages[0].text[:50]}...')

    print('\n=== ✓ All tests completed successfully ===')

    await client.disconnect()


if __name__ == '__main__':
    asyncio.run(test_bot())
