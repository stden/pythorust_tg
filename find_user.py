#!/usr/bin/env python3
"""Find user ID by name in chat."""
import os
import sys
from telethon.tl.types import PeerChannel
from telegram_session import get_client, SessionLock

client = get_client()

async def find_user(search_term, chat_id):
    if not chat_id:
        print("Error: Set FIND_USER_CHAT_ID env var or pass chat_id as second argument")
        return
    entity = await client.get_entity(PeerChannel(int(chat_id)))
    messages = await client.get_messages(entity, limit=500)

    senders = {}
    for m in messages:
        if m.sender_id and m.sender_id not in senders:
            sender = await m.get_sender()
            if sender:
                try:
                    name = f'{sender.first_name or ""} {sender.last_name or ""}'.strip()
                except:
                    name = str(sender.username)
                senders[m.sender_id] = name

    for sid, name in senders.items():
        if search_term.lower() in name.lower():
            print(f'{sid}: {name}')

search = sys.argv[1] if len(sys.argv) > 1 else ""
chat_id = sys.argv[2] if len(sys.argv) > 2 else os.getenv("FIND_USER_CHAT_ID", "")

if not search:
    print("Usage: python find_user.py <search_term> [chat_id]")
    print("Or set FIND_USER_CHAT_ID env var")
    sys.exit(1)

with SessionLock():
    with client:
        client.loop.run_until_complete(find_user(search, chat_id))
