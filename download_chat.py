"""
Download chat by ID with pause between operations.
Usage: python download_chat.py <chat_id> [limit]
"""
import os
import sys
import asyncio
import time
import re

from telethon.tl.types import PeerChannel, ReactionEmoji
from telegram_session import get_client, known_senders, SessionLock

# Get client with SQLite session
client = get_client()
known = known_senders.copy()


def sanitize_filename(name: str) -> str:
    """Convert chat title to safe filename."""
    # Remove emojis and special characters
    name = re.sub(r'[^\w\s\-]', '', name)
    name = re.sub(r'\s+', '_', name.strip())
    return name[:50] if name else 'unknown_chat'


async def download_chat(chat_id: int, limit: int = 200):
    """Download chat messages by ID."""
    print(f"Connecting to chat {chat_id}...")

    try:
        entity = await client.get_entity(PeerChannel(chat_id))
        chat_title = getattr(entity, 'title', f'chat_{chat_id}')
        filename = sanitize_filename(chat_title)
        print(f"Chat: {chat_title} -> {filename}.md")
    except Exception as e:
        print(f"Error getting entity: {e}")
        # Try as regular ID
        try:
            entity = await client.get_entity(chat_id)
            chat_title = getattr(entity, 'title', f'chat_{chat_id}')
            filename = sanitize_filename(chat_title)
            print(f"Chat: {chat_title} -> {filename}.md")
        except Exception as e2:
            print(f"Failed to get chat: {e2}")
            return None

    print(f"Downloading {limit} messages...")
    messages = await client.get_messages(entity, limit=limit)
    print(f"Got {len(messages)} messages")

    # Save to markdown
    output_path = f"chats/{filename}.md"
    os.makedirs("chats", exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as md:
        md.write(f"# {chat_title}\n")
        md.write(f"Chat ID: {chat_id}\n")
        md.write(f"Messages: {len(messages)}\n\n---\n\n")

        for m in messages[::-1]:  # Oldest first
            # Get sender name
            if m.sender_id in known:
                sender_name = known[m.sender_id]
            else:
                sender = await m.get_sender()
                if sender:
                    try:
                        sender_name = f"{sender.first_name or ''} {sender.last_name or ''}".strip()
                    except:
                        sender_name = f"@{getattr(sender, 'username', 'unknown')}"
                else:
                    sender_name = "Unknown"
                known[m.sender_id] = sender_name

            timestamp = m.date.strftime('%d.%m.%Y %H:%M')

            # Reactions
            emojis = ''
            if hasattr(m, 'reactions') and m.reactions:
                emojis_list = []
                for r in m.reactions.results:
                    if isinstance(r.reaction, ReactionEmoji):
                        emojis_list.append(r.reaction.emoticon)
                emojis = ''.join(emojis_list)

            # Write message
            text = m.text or ''
            if m.media:
                text += ' [Media]' if text else '[Media]'

            if text.strip():
                md.write(f"**{sender_name}** ({timestamp}):\n{text} {emojis}\n\n")

    print(f"Saved: {output_path}")
    return output_path


async def main():
    if len(sys.argv) < 2:
        print("Usage: python download_chat.py <chat_id> [limit]")
        print("Example: python download_chat.py 1234567890 200")
        return

    chat_id = int(sys.argv[1])
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 200

    await download_chat(chat_id, limit)


if __name__ == "__main__":
    with SessionLock():
        with client:
            client.loop.run_until_complete(main())
