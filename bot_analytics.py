#!/usr/bin/env python3
"""
Bot analytics dashboard: conversions, funnel, retention.

Collects session/message stats from MySQL tables:
- bot_users
- bot_sessions
- bot_messages
"""

import argparse
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import pymysql
from dotenv import load_dotenv

# Load environment once
load_dotenv()


MYSQL_CONFIG = {
    "host": os.getenv("MYSQL_HOST", "localhost"),
    "port": int(os.getenv("MYSQL_PORT", 3306)),
    "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
    "user": os.getenv("MYSQL_USER", "pythorust_tg"),
    "password": os.getenv("MYSQL_PASSWORD"),
    "charset": "utf8mb4",
    "cursorclass": pymysql.cursors.DictCursor,
}

GRACE_SECONDS = 30  # Small window to attach /start message written before session row


@dataclass
class SessionStats:
    """Aggregated stats for a single session."""

    id: int
    user_id: int
    bot_name: str
    start: datetime
    end: Optional[datetime]
    messages_in: int = 0
    messages_out: int = 0
    non_command_in: int = 0
    phone_shared: bool = False
    last_message: Optional[datetime] = None

    @property
    def total_messages(self) -> int:
        return self.messages_in + self.messages_out

    @property
    def engaged(self) -> bool:
        return self.non_command_in >= 1

    @property
    def multi_turn(self) -> bool:
        return self.non_command_in >= 2


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Bot analytics dashboard: conversions, funnel, retention")
    parser.add_argument(
        "--bots",
        nargs="+",
        help="Bot names to include (default: all bot_names from DB)",
    )
    parser.add_argument(
        "--days",
        type=int,
        default=30,
        help="Time window in days (default: 30)",
    )
    parser.add_argument(
        "--output",
        type=str,
        help="Path to save Markdown dashboard (default: analysis_results/bot_analytics_<timestamp>.md)",
    )
    return parser.parse_args()


def connect_db():
    try:
        return pymysql.connect(**MYSQL_CONFIG)
    except pymysql.MySQLError as exc:
        sys.exit(f"Failed to connect to MySQL: {exc}")


def get_available_bots(conn) -> List[str]:
    """Return distinct bot names from sessions/messages."""
    with conn.cursor() as cursor:
        cursor.execute("SELECT DISTINCT bot_name FROM bot_sessions")
        session_bots = {row["bot_name"] for row in cursor.fetchall()}

        cursor.execute("SELECT DISTINCT bot_name FROM bot_messages")
        message_bots = {row["bot_name"] for row in cursor.fetchall()}

    bots = sorted(session_bots | message_bots)
    if not bots:
        sys.exit("No bots found in database (bot_sessions/bot_messages are empty).")
    return bots


def build_placeholders(items: List[str]) -> str:
    return ",".join(["%s"] * len(items))


def fetch_sessions(conn, bot_names: List[str], start_dt: datetime) -> List[dict]:
    placeholders = build_placeholders(bot_names)
    sql = f"""
        SELECT id, user_id, bot_name, state, is_active, session_start, session_end
        FROM bot_sessions
        WHERE bot_name IN ({placeholders})
          AND session_start >= %s
        ORDER BY session_start ASC
    """
    with conn.cursor() as cursor:
        cursor.execute(sql, [*bot_names, start_dt])
        return cursor.fetchall()


def fetch_messages(conn, bot_names: List[str], start_dt: datetime) -> List[dict]:
    placeholders = build_placeholders(bot_names)
    sql = f"""
        SELECT user_id, bot_name, direction, message_text, created_at
        FROM bot_messages
        WHERE bot_name IN ({placeholders})
          AND created_at >= %s
        ORDER BY created_at ASC
    """
    with conn.cursor() as cursor:
        cursor.execute(sql, [*bot_names, start_dt])
        return cursor.fetchall()


def fetch_first_message_map(conn, bot_names: List[str]) -> Dict[str, Dict[int, datetime]]:
    """Fetch first message timestamp per user for each bot."""
    placeholders = build_placeholders(bot_names)
    sql = f"""
        SELECT bot_name, user_id, MIN(created_at) AS first_message
        FROM bot_messages
        WHERE bot_name IN ({placeholders})
        GROUP BY bot_name, user_id
    """
    first_map: Dict[str, Dict[int, datetime]] = defaultdict(dict)
    with conn.cursor() as cursor:
        cursor.execute(sql, bot_names)
        for row in cursor.fetchall():
            first_map[row["bot_name"]][row["user_id"]] = row["first_message"]
    return first_map


def is_meaningful_user_message(text: Optional[str]) -> bool:
    if not text:
        return False
    cleaned = text.strip()
    if not cleaned:
        return False
    return not cleaned.startswith("/")


def contains_phone(text: Optional[str]) -> bool:
    """Detect if message likely contains a phone number (10-15 digits)."""
    if not text:
        return False
    digits = re.sub(r"\D", "", text)
    if len(digits) >= 10 and len(digits) <= 15:
        return True
    return bool(re.search(r"\+?\d[\d\-\s\(\)]{8,}\d", text))


def sessions_lookup(sessions: List[dict]) -> Dict[str, Dict[int, List[SessionStats]]]:
    """Group sessions by bot and user for quick lookup."""
    lookup: Dict[str, Dict[int, List[SessionStats]]] = defaultdict(lambda: defaultdict(list))
    for row in sessions:
        stat = SessionStats(
            id=row["id"],
            user_id=row["user_id"],
            bot_name=row["bot_name"],
            start=row["session_start"],
            end=row["session_end"],
        )
        lookup[stat.bot_name][stat.user_id].append(stat)

    # Ensure chronological order to attach messages correctly
    for bot_sessions in lookup.values():
        for session_list in bot_sessions.values():
            session_list.sort(key=lambda s: s.start)
    return lookup


def attach_messages_to_sessions(
    messages: List[dict],
    lookup: Dict[str, Dict[int, List[SessionStats]]],
    grace: timedelta,
) -> Dict[str, Dict[int, List[dict]]]:
    """
    Update SessionStats in-place and return messages grouped by bot/user
    (for retention calculations).
    """
    messages_by_bot_user: Dict[str, Dict[int, List[dict]]] = defaultdict(lambda: defaultdict(list))

    for msg in messages:
        bot = msg["bot_name"]
        user = msg["user_id"]
        messages_by_bot_user[bot][user].append(msg)

        sessions = lookup.get(bot, {}).get(user, [])
        if not sessions:
            continue

        assigned = False
        for session in sessions:
            start_with_grace = session.start - grace
            session_end = session.end or datetime.max
            if start_with_grace <= msg["created_at"] <= session_end:
                if msg["direction"] == "incoming":
                    session.messages_in += 1
                    if is_meaningful_user_message(msg["message_text"]):
                        session.non_command_in += 1
                        if contains_phone(msg["message_text"]):
                            session.phone_shared = True
                else:
                    session.messages_out += 1
                session.last_message = msg["created_at"]
                assigned = True
                break

        # If message is outside all known sessions (e.g., older), skip counting.
        if not assigned:
            continue

    return messages_by_bot_user


def safe_div(num: int, denom: int) -> float:
    return (num / denom * 100) if denom else 0.0


def average(values: List[int]) -> float:
    return sum(values) / len(values) if values else 0.0


def compute_retention(
    messages_by_user: Dict[int, List[dict]],
    first_message_map: Dict[int, datetime],
    window_start: datetime,
) -> Dict[str, Tuple[int, int, float]]:
    """
    Compute D1/D7 retention for users whose first message falls inside the window.
    Returns dict with entries like {"d1": (base, returned, rate)}.
    """
    today = datetime.utcnow().date()
    d1_base = d1_return = d7_base = d7_return = 0

    for user_id, first_dt in first_message_map.items():
        if first_dt < window_start:
            continue  # not a new user in the selected window

        message_days = {m["created_at"].date() for m in messages_by_user.get(user_id, [])}
        first_day = first_dt.date()

        d1_target = first_day + timedelta(days=1)
        if d1_target <= today:
            d1_base += 1
            if d1_target in message_days:
                d1_return += 1

        d7_target = first_day + timedelta(days=7)
        if d7_target <= today:
            d7_base += 1
            if d7_target in message_days:
                d7_return += 1

    return {
        "d1": (d1_base, d1_return, safe_div(d1_return, d1_base)),
        "d7": (d7_base, d7_return, safe_div(d7_return, d7_base)),
    }


def build_metrics(
    sessions: List[SessionStats],
    messages_by_user: Dict[int, List[dict]],
    first_message_map: Dict[int, datetime],
    window_start: datetime,
) -> dict:
    total_sessions = len(sessions)
    engaged_sessions = [s for s in sessions if s.engaged]
    multi_turn_sessions = [s for s in sessions if s.multi_turn]
    converted_sessions = [s for s in sessions if s.phone_shared]

    daily = defaultdict(lambda: {"sessions": 0, "converted": 0})
    for s in sessions:
        day = s.start.date()
        daily[day]["sessions"] += 1
        if s.phone_shared:
            daily[day]["converted"] += 1

    retention = compute_retention(messages_by_user, first_message_map, window_start)
    new_users = sum(1 for dt in first_message_map.values() if dt >= window_start)

    return {
        "sessions": total_sessions,
        "engaged": len(engaged_sessions),
        "multi_turn": len(multi_turn_sessions),
        "converted": len(converted_sessions),
        "conversion_rate": safe_div(len(converted_sessions), total_sessions),
        "engaged_rate": safe_div(len(engaged_sessions), total_sessions),
        "multi_rate": safe_div(len(multi_turn_sessions), len(engaged_sessions)),
        "avg_user_messages": average([s.messages_in for s in sessions]),
        "avg_bot_messages": average([s.messages_out for s in sessions]),
        "new_users": new_users,
        "active_users": len(messages_by_user),
        "retention": retention,
        "daily": sorted(
            [(day, stats["sessions"], stats["converted"]) for day, stats in daily.items()],
            key=lambda x: x[0],
        ),
    }


def render_markdown(metrics: Dict[str, dict], days: int, window_start: datetime, output_path: Path) -> None:
    lines: List[str] = []
    lines.append(f"# Bot Analytics Dashboard (last {days} days)")
    lines.append("")
    lines.append(
        f"- Period: {window_start.strftime('%Y-%m-%d')} → {datetime.utcnow().strftime('%Y-%m-%d %H:%M:%S')} UTC"
    )
    lines.append("- Definitions:")
    lines.append("  - Engaged: session has ≥1 осмысленных входящих сообщений (не команды).")
    lines.append("  - Multi-turn: ≥2 осмысленных входящих сообщений.")
    lines.append("  - Conversion: сообщение содержит телефон (10-15 цифр).")
    lines.append("")

    for bot_name, data in metrics.items():
        lines.append(f"## {bot_name}")
        lines.append(
            f"- Sessions: {data['sessions']} | Engaged: {data['engaged']} "
            f"({data['engaged_rate']:.1f}%) | Multi-turn: {data['multi_turn']} "
            f"({data['multi_rate']:.1f}% от engaged)"
        )
        lines.append(f"- Conversions (phone shared): {data['converted']} ({data['conversion_rate']:.1f}% of sessions)")
        lines.append(
            f"- Users: new {data['new_users']}, active {data['active_users']}, "
            f"avg user msgs/session {data['avg_user_messages']:.1f}, "
            f"avg bot msgs/session {data['avg_bot_messages']:.1f}"
        )

        d1_base, d1_ret, d1_rate = data["retention"]["d1"]
        d7_base, d7_ret, d7_rate = data["retention"]["d7"]
        lines.append(f"- Retention: D1 {d1_rate:.1f}% ({d1_ret}/{d1_base}), D7 {d7_rate:.1f}% ({d7_ret}/{d7_base})")
        lines.append("")

        lines.append("**Funnel**")
        lines.append("| Stage | Sessions | Rate |")
        lines.append("| --- | --- | --- |")
        lines.append(f"| Start | {data['sessions']} | 100% |")
        lines.append(f"| Engaged | {data['engaged']} | {data['engaged_rate']:.1f}% |")
        lines.append(f"| Multi-turn | {data['multi_turn']} | {data['multi_rate']:.1f}% of engaged |")
        lines.append(f"| Phone shared | {data['converted']} | {data['conversion_rate']:.1f}% |")
        lines.append("")

        if data["daily"]:
            lines.append("**Daily conversion**")
            lines.append("| Date | Sessions | Conversions | Rate |")
            lines.append("| --- | --- | --- | --- |")
            for day, sessions, conv in data["daily"]:
                rate = safe_div(conv, sessions)
                lines.append(f"| {day.isoformat()} | {sessions} | {conv} | {rate:.1f}% |")
            lines.append("")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"✅ Saved dashboard to {output_path}")


def main():
    args = parse_args()
    conn = connect_db()

    bots = args.bots or get_available_bots(conn)
    window_start = datetime.utcnow() - timedelta(days=args.days)
    grace = timedelta(seconds=GRACE_SECONDS)

    sessions_rows = fetch_sessions(conn, bots, window_start)
    messages_rows = fetch_messages(conn, bots, window_start - grace)
    first_message_map = fetch_first_message_map(conn, bots)

    lookup = sessions_lookup(sessions_rows)
    messages_by_bot_user = attach_messages_to_sessions(messages_rows, lookup, grace)

    metrics: Dict[str, dict] = {}
    for bot in bots:
        bot_sessions = [s for user_sessions in lookup.get(bot, {}).values() for s in user_sessions]
        metrics[bot] = build_metrics(
            bot_sessions,
            messages_by_bot_user.get(bot, {}),
            first_message_map.get(bot, {}),
            window_start,
        )

    timestamp = datetime.utcnow().strftime("%Y%m%d_%H%M%S")
    output_path = Path(args.output) if args.output else Path("analysis_results") / f"bot_analytics_{timestamp}.md"
    render_markdown(metrics, args.days, window_start, output_path)


if __name__ == "__main__":
    main()
