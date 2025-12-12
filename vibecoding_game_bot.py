#!/usr/bin/env python3
"""Многопользовательская мини-игра про вайбкодинг в Telegram."""

import asyncio
import logging
import os
import random
import textwrap
import uuid
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Optional

import pymysql
from dotenv import load_dotenv
from telethon import Button, events
from telethon.tl.custom import Message

from telegram_bot_base import TelegramBotBase

load_dotenv()

BOT_TOKEN = os.getenv("VIBECODING_BOT_TOKEN")
BOT_NAME = "VibeCoderzBot"
ALLOWED_USERS = [int(x) for x in os.getenv("VIBECODING_ALLOWED_USERS", "").split(",") if x.strip()]

ROUND_DURATION = int(os.getenv("VIBE_ROUND_DURATION", "90"))
VOTE_DURATION = int(os.getenv("VIBE_VOTE_DURATION", "45"))
SUBMISSION_PREVIEW = 140

MYSQL_CONFIG = {
    "host": os.getenv("MYSQL_HOST", "localhost"),
    "port": int(os.getenv("MYSQL_PORT", "3306")),
    "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
    "user": os.getenv("MYSQL_USER", "pythorust_tg"),
    "password": os.getenv("MYSQL_PASSWORD"),
    "charset": "utf8mb4",
    "cursorclass": pymysql.cursors.DictCursor,
}

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("vibecoding_game_bot")

PROMPTS: List[str] = [
    "Собери вайб-спринт: цвет, звук, движение. 3 строки, 1 эмодзи максимум.",
    "Код настроения для утреннего созвона: подсвети страх, цель и одну шутку.",
    "Архитектура пруда: три узла (ритм, поток, фокус) и как они синхронизируются.",
    "Напиши ритуал деплоя в стиле хайку: причина, действие, откат.",
    "Подготовь вайб-карту спринта: риск, яркое событие и скрытый баг.",
    "Сделай MIDI-настроение: темп, инструмент, первая нота — всё в тексте.",
    'Опиши "идеальный вечер кодера" в формате JSON из 3 ключей.',
    "Сборка команды мечты: роли трёх коев и их короткие суперсилы.",
    "Спринт без дедлайнов: как понять, что ты в потоке? Дай чеклист.",
    "Набросай эмодзи-протокол стендапа: статус, блокер, хайлайт.",
]


class MySQLLogger:
    """Запись пользователей и сообщений бота в MySQL."""

    def __init__(self, bot_name: str):
        self.bot_name = bot_name
        self.conn = None

    def connect(self):
        """Открыть соединение с MySQL."""
        self.conn = pymysql.connect(**MYSQL_CONFIG)
        logger.info("Connected to MySQL")

    def ensure_connection(self):
        """Гарантировать живое соединение."""
        if not self.conn or not getattr(self.conn, "open", False):
            self.connect()
        try:
            self.conn.ping(reconnect=True)
        except Exception:
            self.connect()

    def save_user(self, user) -> None:
        """Сохранить или обновить пользователя."""
        try:
            self.ensure_connection()
            query = """
                INSERT INTO bot_users (id, username, first_name, last_name, language_code, is_premium, is_bot)
                VALUES (%s, %s, %s, %s, %s, %s, %s)
                ON DUPLICATE KEY UPDATE
                    username = VALUES(username),
                    first_name = VALUES(first_name),
                    last_name = VALUES(last_name),
                    language_code = VALUES(language_code),
                    is_premium = VALUES(is_premium),
                    last_seen_at = CURRENT_TIMESTAMP
            """
            with self.conn.cursor() as cursor:
                cursor.execute(
                    query,
                    (
                        user.id,
                        getattr(user, "username", None),
                        getattr(user, "first_name", None),
                        getattr(user, "last_name", None),
                        getattr(user, "lang_code", None),
                        getattr(user, "premium", False),
                        getattr(user, "bot", False),
                    ),
                )
            self.conn.commit()
        except Exception:
            logger.exception("Failed to save user %s", getattr(user, "id", None))

    def save_message(
        self,
        *,
        user_id: int,
        message_id: int,
        text: str,
        direction: str,
        reply_to: Optional[int] = None,
    ) -> None:
        """Сохранить входящее/исходящее сообщение."""
        if direction not in ("incoming", "outgoing"):
            raise ValueError("direction must be 'incoming' or 'outgoing'")

        try:
            self.ensure_connection()
            query = """
                INSERT INTO bot_messages
                (telegram_message_id, user_id, bot_name, direction, message_text, reply_to_message_id)
                VALUES (%s, %s, %s, %s, %s, %s)
            """
            with self.conn.cursor() as cursor:
                cursor.execute(
                    query,
                    (message_id, user_id, self.bot_name, direction, text, reply_to),
                )
            self.conn.commit()
        except Exception:
            logger.exception("Failed to save %s message for user %s", direction, user_id)


@dataclass
class VibeRound:
    """Данные активного раунда."""

    round_id: str
    prompt: str
    submissions: Dict[int, str] = field(default_factory=dict)
    voter_choice: Dict[int, int] = field(default_factory=dict)  # voter -> target user
    vote_message_id: Optional[int] = None
    status: str = "collecting"  # collecting -> voting -> closed


@dataclass
class GameState:
    """Состояние игры для одного чата."""

    host_id: int
    host_name: str
    players: Dict[int, str] = field(default_factory=dict)
    scores: Dict[int, int] = field(default_factory=dict)
    round: Optional[VibeRound] = None
    round_task: Optional[asyncio.Task] = None
    vote_task: Optional[asyncio.Task] = None


class VibeCodingGameBot(TelegramBotBase):
    """Телеграм-бот для вайбкодинг-пати."""

    def __init__(self):
        if not BOT_TOKEN:
            raise RuntimeError("VIBECODING_BOT_TOKEN отсутствует в окружении.")

        super().__init__(
            bot_token=BOT_TOKEN,
            allowed_users=ALLOWED_USERS,
            use_session_lock=False,  # не блокируем пользовательскую сессию
        )
        self.db = MySQLLogger(BOT_NAME)
        self.games: Dict[int, GameState] = {}
        self.chat_locks: Dict[int, asyncio.Lock] = defaultdict(asyncio.Lock)

    async def setup_handlers(self):
        self.client.add_event_handler(self._start_help, events.NewMessage(pattern=r"^/(start|help)(@[\w_]+)?$"))
        self.client.add_event_handler(self._start_game, events.NewMessage(pattern=r"^/vibe_game(@[\w_]+)?$"))
        self.client.add_event_handler(self._join_game, events.NewMessage(pattern=r"^/vibe_join(@[\w_]+)?$"))
        self.client.add_event_handler(self._start_round, events.NewMessage(pattern=r"^/vibe_round(@[\w_]+)?$"))
        self.client.add_event_handler(self._stop_game, events.NewMessage(pattern=r"^/vibe_stop(@[\w_]+)?$"))
        self.client.add_event_handler(self._show_scores, events.NewMessage(pattern=r"^/vibe_score(@[\w_]+)?$"))
        self.client.add_event_handler(
            self._submit_vibe,
            events.NewMessage(pattern=r"^/vibe(@[\w_]+)?\s+(.+)$"),
        )
        self.client.add_event_handler(
            self._submit_vibe,
            events.NewMessage(pattern=r"^>vibe\s+(.+)$"),
        )
        self.client.add_event_handler(self._handle_vote, events.CallbackQuery(pattern=b"^vote\\|"))

    async def on_start(self):
        try:
            self.db.connect()
        except Exception:
            logger.exception("Не удалось подключиться к MySQL при старте бота")
        print("VibeCodingGameBot запущен.")

    # ------------------------------------------------------------------ #
    # Логирование пользователей и сообщений
    # ------------------------------------------------------------------ #
    async def _log_incoming(self, event, text_override: Optional[str] = None) -> int:
        """Сохранить данные пользователя и входящее сообщение."""
        try:
            sender = await event.get_sender()
            if sender:
                self.db.save_user(sender)
                user_id = sender.id
            else:
                user_id = event.sender_id or event.chat_id

            msg = event.message
            text = (
                text_override
                if text_override is not None
                else getattr(msg, "message", None) or getattr(event, "raw_text", "") or ""
            )
            self.db.save_message(
                user_id=user_id,
                message_id=msg.id,
                text=text,
                direction="incoming",
                reply_to=getattr(msg, "reply_to_msg_id", None),
            )
            return user_id
        except Exception:
            logger.exception("Failed to log incoming message")
            return event.sender_id or event.chat_id

    def _log_outgoing(
        self,
        *,
        user_id: int,
        message_id: int,
        text: str,
        reply_to: Optional[int] = None,
    ) -> None:
        """Записать исходящее сообщение в базу."""
        try:
            self.db.save_message(
                user_id=user_id,
                message_id=message_id,
                text=text,
                direction="outgoing",
                reply_to=reply_to,
            )
        except Exception:
            logger.exception("Failed to log outgoing message")

    async def _reply(self, event, text: str, **kwargs):
        """Ответить пользователю и зафиксировать сообщение."""
        msg = await event.respond(text, **kwargs)
        target_user = event.sender_id or event.chat_id
        self._log_outgoing(
            user_id=target_user,
            message_id=msg.id,
            text=text,
            reply_to=event.message.id if hasattr(event, "message") else None,
        )
        return msg

    async def _send_and_log(
        self,
        chat_id: int,
        text: str,
        *,
        user_id: Optional[int] = None,
        reply_to: Optional[int] = None,
        **kwargs,
    ):
        """Отправить сообщение от имени бота и записать его в базу."""
        msg = await self.client.send_message(chat_id, text, **kwargs)
        self._log_outgoing(
            user_id=user_id or chat_id,
            message_id=msg.id,
            text=text,
            reply_to=reply_to,
        )
        return msg

    # ------------------------------------------------------------------ #
    # Команды
    # ------------------------------------------------------------------ #
    async def _start_help(self, event: events.NewMessage.Event):
        await event.respond(
            "👋 Я VibeCoderz Bot. Запускаю вайбкодинг-пати, собираю ответы и считаю голоса.\n\n"
            "Что умею:\n"
            "• Запустить лобби и собрать игроков (/vibe_game, /vibe_join)\n"
            "• Давать промпты и собирать ответы (/vibe_round, /vibe <текст> или >vibe <текст>)\n"
            "• Запускать голосование кнопками и выбирать победителя\n"
            "• Вести счёт и показывать таблицу (/vibe_score)\n"
            "• Останавливать игру (/vibe_stop)\n\n"
            "Как начать: введите /vibe_game, затем приглашайте игроков командой /vibe_join и стартуйте раунд /vibe_round.",
            link_preview=False,
        )

    async def _start_game(self, event: events.NewMessage.Event):
        sender_id = event.sender_id
        if not self.check_access(sender_id):
            await event.respond("⛔ Доступ запрещен для этой команды.")
            return

        chat_id = event.chat_id
        async with self.chat_locks[chat_id]:
            host_name = self._display_name(event.message)
            state = GameState(host_id=sender_id, host_name=host_name)
            state.players[sender_id] = host_name
            self._reset_state(chat_id)
            self.games[chat_id] = state

        await event.respond(
            f"🚀 Вайб-пати запущена. Хост: {host_name}\n"
            "Жмите /vibe_join, чтобы зайти. Хост стартует раунды командой /vibe_round.",
            link_preview=False,
        )

    async def _join_game(self, event: events.NewMessage.Event):
        chat_id = event.chat_id
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state:
                await event.respond("Сначала запусти игру командой /vibe_game.")
                return

            user_id = event.sender_id
            user_name = self._display_name(event.message)
            state.players[user_id] = user_name

        await event.respond(f"🤝 {user_name} в лобби. Готовим вайбы!")

    async def _start_round(self, event: events.NewMessage.Event):
        chat_id = event.chat_id
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state:
                await event.respond("Сначала запусти игру: /vibe_game.")
                return

            if event.sender_id != state.host_id:
                await event.respond("Только хост может стартовать раунд.")
                return

            prompt = random.choice(PROMPTS)
            round_id = uuid.uuid4().hex[:8]

            # Завершаем предыдущие задачи, если они были
            self._cancel_task(state.round_task)
            self._cancel_task(state.vote_task)

            state.round = VibeRound(round_id=round_id, prompt=prompt)
            state.round_task = asyncio.create_task(self._close_round_after(chat_id, round_id, ROUND_DURATION))

        await event.respond(
            f"🎯 Новый раунд!\n"
            f"Промпт: {prompt}\n\n"
            "Отправь свой ответ: /vibe <текст> или >vibe <текст>\n"
            f"У тебя {ROUND_DURATION} секунд.",
            link_preview=False,
        )

    async def _submit_vibe(self, event: events.NewMessage.Event):
        chat_id = event.chat_id
        match = event.pattern_match
        if not match:
            return

        text = match.group(match.lastindex or 1)
        if not text:
            return

        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state or not state.round or state.round.status != "collecting":
                await event.respond("Сейчас нет активного сбора ответов. Хост: /vibe_round.")
                return

            user_id = event.sender_id
            user_name = self._display_name(event.message)
            state.players[user_id] = user_name
            state.round.submissions[user_id] = text.strip()

        await event.respond(f"✅ {user_name}, твой вайб записан.")

    async def _stop_game(self, event: events.NewMessage.Event):
        chat_id = event.chat_id
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state:
                await event.respond("Игра ещё не запущена.")
                return

            if event.sender_id != state.host_id:
                await event.respond("Только хост может завершить игру.")
                return

            self._reset_state(chat_id)

        await event.respond("🛑 Игра остановлена.")

    async def _show_scores(self, event: events.NewMessage.Event):
        chat_id = event.chat_id
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state:
                await event.respond("Игра не запущена. /vibe_game чтобы начать.")
                return

            scoreboard = self._format_scores(state.scores, state.players)

        await event.respond(f"🏆 Таблица очков:\n{scoreboard}", link_preview=False)

    # ------------------------------------------------------------------ #
    # Голосование
    # ------------------------------------------------------------------ #
    async def _handle_vote(self, event: events.CallbackQuery.Event):
        data = event.data.decode()
        parts = data.split("|")
        if len(parts) != 3:
            await event.answer()
            return

        _, round_id, target_raw = parts
        try:
            target_id = int(target_raw)
        except ValueError:
            await event.answer()
            return

        chat_id = event.chat_id
        voter_id = event.sender_id

        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            round_state = state.round if state else None
            if not state or not round_state or round_state.round_id != round_id or round_state.status != "voting":
                await event.answer("Раунд уже закрыт.")
                return

            round_state.voter_choice[voter_id] = target_id
            await self._refresh_vote_message(chat_id, state)

        await event.answer("Голос принят ✅")

    # ------------------------------------------------------------------ #
    # Внутренние функции раунда
    # ------------------------------------------------------------------ #
    async def _close_round_after(self, chat_id: int, round_id: str, delay: int):
        await asyncio.sleep(delay)
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            if not state or not state.round or state.round.round_id != round_id:
                return

            await self._start_voting(chat_id, state)

    async def _start_voting(self, chat_id: int, state: GameState):
        round_state = state.round
        if not round_state:
            return

        round_state.status = "voting"

        if not round_state.submissions:
            state.round = None
            await self.client.send_message(chat_id, "⏳ Никто не прислал ответы. Запусти /vibe_round заново.")
            return

        vote_text = self._vote_message_text(state)
        buttons = self._vote_buttons(state)
        msg = await self.client.send_message(chat_id, vote_text, buttons=buttons, link_preview=False)
        round_state.vote_message_id = msg.id

        state.vote_task = asyncio.create_task(self._close_vote_after(chat_id, round_state.round_id, VOTE_DURATION))

    async def _close_vote_after(self, chat_id: int, round_id: str, delay: int):
        await asyncio.sleep(delay)
        async with self.chat_locks[chat_id]:
            state = self.games.get(chat_id)
            round_state = state.round if state else None
            if not state or not round_state or round_state.round_id != round_id or round_state.status != "voting":
                return

            round_state.status = "closed"
            await self._finalize_round(chat_id, state)

    async def _refresh_vote_message(self, chat_id: int, state: GameState):
        round_state = state.round
        if not round_state or round_state.vote_message_id is None:
            return

        vote_text = self._vote_message_text(state)
        buttons = self._vote_buttons(state)
        await self.client.edit_message(
            chat_id,
            round_state.vote_message_id,
            vote_text,
            buttons=buttons,
            link_preview=False,
        )

    async def _finalize_round(self, chat_id: int, state: GameState):
        round_state = state.round
        if not round_state:
            return

        # Подсчёт голосов
        tally = Counter(round_state.voter_choice.values())
        if not tally:
            summary = "Голоса не получены. Очки не начислены."
        else:
            max_votes = max(tally.values())
            winners = [uid for uid, votes in tally.items() if votes == max_votes]
            for uid in winners:
                state.scores[uid] = state.scores.get(uid, 0) + 1

            winner_names = ", ".join(state.players.get(uid, f"ID {uid}") for uid in winners)
            summary = f"🏅 Побеждает: {winner_names} ({max_votes} голосов)."

        scoreboard = self._format_scores(state.scores, state.players)
        await self.client.send_message(
            chat_id,
            f"{summary}\n\nНовый счёт:\n{scoreboard}",
            link_preview=False,
        )

        # Сброс раунда
        self._cancel_task(state.round_task)
        self._cancel_task(state.vote_task)
        state.round = None
        state.round_task = None
        state.vote_task = None

    # ------------------------------------------------------------------ #
    # Утилиты
    # ------------------------------------------------------------------ #
    def _reset_state(self, chat_id: int):
        state = self.games.pop(chat_id, None)
        if state:
            self._cancel_task(state.round_task)
            self._cancel_task(state.vote_task)

    @staticmethod
    def _cancel_task(task: Optional[asyncio.Task]):
        if task and not task.done():
            task.cancel()

    @staticmethod
    def _display_name(message: Message) -> str:
        sender = message.sender
        if not sender:
            return "Игрок"

        if getattr(sender, "first_name", None):
            return sender.first_name
        if getattr(sender, "username", None):
            return sender.username
        return str(sender.id)

    def _vote_buttons(self, state: GameState):
        round_state = state.round
        if not round_state:
            return None

        buttons = []
        votes = Counter(round_state.voter_choice.values())

        for user_id, submission in round_state.submissions.items():
            label = state.players.get(user_id, f"ID {user_id}")
            count = votes.get(user_id, 0)
            text = f"За {label} ({count})"
            buttons.append(Button.inline(text, data=f"vote|{round_state.round_id}|{user_id}".encode()))

        # по две кнопки в ряд
        rows = [buttons[i : i + 2] for i in range(0, len(buttons), 2)]
        return rows

    def _vote_message_text(self, state: GameState) -> str:
        round_state = state.round
        if not round_state:
            return "Раунд закрыт."

        lines = ["🗳 Голосование за лучший вайб", f"Промпт: {round_state.prompt}", "", "Участники:"]

        for user_id, submission in round_state.submissions.items():
            name = state.players.get(user_id, f"ID {user_id}")
            preview = textwrap.shorten(submission, width=SUBMISSION_PREVIEW, placeholder="…")
            lines.append(f"• {name}: {preview}")

        lines.append("")
        lines.append("Нажми кнопку, чтобы проголосовать.")
        return "\n".join(lines)

    @staticmethod
    def _format_scores(scores: Dict[int, int], players: Dict[int, str]) -> str:
        if not scores:
            return "Пока 0:0. Бросай /vibe_round, чтобы начать."

        items = sorted(scores.items(), key=lambda kv: kv[1], reverse=True)
        return "\n".join(f"{players.get(uid, uid)} — {pts}" for uid, pts in items)


async def main():
    bot = VibeCodingGameBot()
    await bot.run()


if __name__ == "__main__":
    asyncio.run(main())
