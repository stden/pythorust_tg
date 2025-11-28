"""
Download chat by ID with pause between operations.
Usage: python download_chat.py <chat_id> [limit]
"""
import sys

from telethon.tl.types import PeerChannel
from telegram_session import get_client, known_senders, SessionLock
from chat_export_utils import sanitize_filename, fetch_and_export_messages

# Get client with SQLite session
client = get_client()
known = known_senders.copy()


async def download_chat(chat_id: int, limit: int = 200):
    """Download chat messages by ID."""
    print(f"Connecting to chat {chat_id}...")

    try:
        entity = await client.get_entity(PeerChannel(chat_id))
        chat_title = getattr(entity, 'title', f'chat_{chat_id}')
        filename = sanitize_filename(chat_title, fallback=f"chat_{chat_id}")
        print(f"Chat: {chat_title} -> {filename}.md")
    except Exception as e:
        print(f"Error getting entity: {e}")
        # Try as regular ID
        try:
            entity = await client.get_entity(chat_id)
            chat_title = getattr(entity, 'title', f'chat_{chat_id}')
            filename = sanitize_filename(chat_title, fallback=f"chat_{chat_id}")
            print(f"Chat: {chat_title} -> {filename}.md")
        except Exception as e2:
            print(f"Failed to get chat: {e2}")
            return None

    print(f"Downloading {limit} messages...")
    output_path, count = await fetch_and_export_messages(
        client,
        entity,
        cache=known,
        filename=filename,
        title=chat_title,
        meta=[f"Chat ID: {chat_id}"],
        limit=limit,
    )

    print(f"Saved {count} messages: {output_path}")
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
