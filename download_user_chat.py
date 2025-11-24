"""
Download chat with a user by username.
Usage: python download_user_chat.py <username> [limit]
"""
import os
import sys
import re

from telethon.tl.types import ReactionEmoji
from telegram_session import get_client, known_senders, SessionLock

client = get_client()
known = known_senders.copy()


def sanitize_filename(name: str) -> str:
    """Convert to safe filename."""
    name = re.sub(r'[^\w\s\-]', '', name)
    name = re.sub(r'\s+', '_', name.strip())
    return name[:50] if name else 'unknown'


async def download_user_chat(username: str, limit: int = 500):
    """Download chat messages with a user."""
    print(f"Connecting to @{username}...")

    try:
        entity = await client.get_entity(username)
        display_name = f"{getattr(entity, 'first_name', '')} {getattr(entity, 'last_name', '')}".strip()
        if not display_name:
            display_name = username
        filename = sanitize_filename(username)
        print(f"User: {display_name} (@{username}) -> {filename}.md")
    except Exception as e:
        print(f"Error getting user: {e}")
        return None

    print(f"Downloading {limit} messages...")
    messages = await client.get_messages(entity, limit=limit)
    print(f"Got {len(messages)} messages")

    output_path = f"chats/{filename}.md"
    os.makedirs("chats", exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as md:
        md.write(f"# Chat with {display_name}\n")
        md.write(f"Username: @{username}\n")
        md.write(f"Messages: {len(messages)}\n\n---\n\n")

        for m in messages[::-1]:  # Oldest first
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

            emojis = ''
            if hasattr(m, 'reactions') and m.reactions:
                emojis_list = []
                for r in m.reactions.results:
                    if isinstance(r.reaction, ReactionEmoji):
                        emojis_list.append(r.reaction.emoticon)
                emojis = ''.join(emojis_list)

            text = m.text or ''
            if m.media:
                text += ' [Media]' if text else '[Media]'

            if text.strip():
                md.write(f"**{sender_name}** ({timestamp}):\n{text} {emojis}\n\n")

    print(f"Saved: {output_path}")
    return output_path


async def main():
    if len(sys.argv) < 2:
        print("Usage: python download_user_chat.py <username> [limit]")
        print("Example: python download_user_chat.py exampleuser 500")
        return

    username = sys.argv[1].lstrip('@')
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 500

    await download_user_chat(username, limit)


if __name__ == "__main__":
    with SessionLock():
        with client:
            client.loop.run_until_complete(main())
