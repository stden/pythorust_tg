"""
Download chat with a user by username.
Usage: python download_user_chat.py <username> [limit]
"""
import sys

from telegram_session import get_client, known_senders, SessionLock
from chat_export_utils import sanitize_filename, fetch_and_export_messages

client = get_client()
known = known_senders.copy()


async def download_user_chat(username: str, limit: int = 500):
    """Download chat messages with a user."""
    print(f"Connecting to @{username}...")

    try:
        entity = await client.get_entity(username)
        display_name = f"{getattr(entity, 'first_name', '')} {getattr(entity, 'last_name', '')}".strip()
        if not display_name:
            display_name = username
        filename = sanitize_filename(username, fallback=username)
        print(f"User: {display_name} (@{username}) -> {filename}.md")
    except Exception as e:
        print(f"Error getting user: {e}")
        return None

    print(f"Downloading {limit} messages...")
    output_path, count = await fetch_and_export_messages(
        client,
        entity,
        cache=known,
        filename=filename,
        title=f"Chat with {display_name}",
        meta=[f"Username: @{username}"],
        limit=limit,
    )

    print(f"Saved {count} messages: {output_path}")
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
