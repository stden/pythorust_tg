#!/usr/bin/env python3
"""Like messages from a specific user with context-appropriate reactions."""
import sys
from datetime import datetime, timezone
from telethon.tl.types import PeerChannel, ReactionEmoji
from telethon.tl.functions.messages import SendReactionRequest
from telegram_session import get_client, SessionLock

client = get_client()

import os
# User ID to like messages from - pass as argument or set via env
TARGET_USER_ID = int(sys.argv[1]) if len(sys.argv) > 1 else int(os.getenv("LIKE_TARGET_USER_ID", "0"))
TODAY_ONLY = '--today' in sys.argv
CHAT_ID = int(os.getenv("LIKE_CHAT_ID", "0"))
CHAT = PeerChannel(CHAT_ID) if CHAT_ID else None


def choose_reaction(text: str) -> str:
    """Choose appropriate reaction based on message content."""
    if not text:
        return "👍"

    text_lower = text.lower()

    # Positive/celebration
    if any(w in text_lower for w in ['поздравл', 'с днём', 'с праздник', 'ура', 'победа', 'успех', 'круто', 'супер', 'класс', 'молодец', 'отлично']):
        return "🎉"

    # Love/heart topics
    if any(w in text_lower for w in ['люблю', 'любовь', 'сердц', 'обним', 'скучаю', 'дорог', 'мил', 'нежн', 'красив', 'прекрасн']):
        return "❤️"

    # Funny/humor
    if any(w in text_lower for w in ['хаха', 'лол', 'ржу', 'смешн', 'угар', 'прикол', '😂', '🤣', 'шутк', 'анекдот']):
        return "😂"

    # Sad/support
    if any(w in text_lower for w in ['грустн', 'печаль', 'жаль', 'сочувств', 'соболезн', 'плох', 'умер', 'болезн', 'трудно']):
        return "😢"

    # Wow/surprise
    if any(w in text_lower for w in ['офиге', 'вау', 'ничего себе', 'невероят', 'удивител', 'шок', 'сенсац', '!'*3]):
        return "😮"

    # Fire/exciting
    if any(w in text_lower for w in ['огонь', 'жар', 'горяч', 'крут', 'мощ', 'бомба', 'взрыв', 'эпич']):
        return "🔥"

    # Questions - thinking
    if '?' in text and len(text) > 20:
        return "🤔"

    # Food
    if any(w in text_lower for w in ['еда', 'вкусн', 'рецепт', 'готов', 'блюд', 'ресторан', 'кафе', 'пирог', 'торт', 'салат']):
        return "😋"

    # Travel/places
    if any(w in text_lower for w in ['путешеств', 'поездк', 'отпуск', 'самолёт', 'поезд', 'страна', 'город']):
        return "✈️"

    # Events/meetups
    if any(w in text_lower for w in ['встреч', 'тусовк', 'вечерин', 'мероприят', 'собира', 'приглаш', 'придёт', 'приход']):
        return "👏"

    # Default - thumbs up
    return "👍"


async def like_user_messages():
    """Like all messages from target user in chat."""
    if not CHAT or not TARGET_USER_ID:
        print("Error: Set LIKE_CHAT_ID and LIKE_TARGET_USER_ID env vars or pass user_id as argument")
        return
    entity = await client.get_entity(CHAT)
    messages = await client.get_messages(entity, limit=1000)

    today = datetime.now(timezone.utc).date()

    liked_count = 0
    for m in messages:
        if m.sender_id != TARGET_USER_ID:
            continue

        # Filter by today if requested
        if TODAY_ONLY and m.date.date() != today:
            continue

        text = m.text or ""
        reaction = choose_reaction(text)
        timestamp = m.date.strftime('%d.%m.%Y %H:%M')
        preview = text[:60].replace('\n', ' ') if text else '[media]'

        print(f"{reaction} {timestamp}: {preview}")

        try:
            await client(SendReactionRequest(
                peer=entity,
                msg_id=m.id,
                reaction=[ReactionEmoji(emoticon=reaction)]
            ))
            liked_count += 1
        except Exception as e:
            print(f"  Error: {e}")

    print(f"\n=== Liked {liked_count} messages ===")


if __name__ == '__main__':
    with SessionLock():
        with client:
            client.loop.run_until_complete(like_user_messages())
