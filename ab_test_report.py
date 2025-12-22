#!/usr/bin/env python3
"""
Report helper for A/B тестирования промптов.

Собирает метрики из таблицы bot_experiments:
- количество сессий на вариант
- число конверсий и конверсию
- причины конверсий (детектированы/заданы кодом)
"""

import argparse
import os
from collections import defaultdict
from typing import Dict, List, Optional

import pymysql
from dotenv import load_dotenv


def build_mysql_config() -> Dict:
    return {
        "host": os.getenv("MYSQL_HOST", "localhost"),
        "port": int(os.getenv("MYSQL_PORT", 3306)),
        "database": os.getenv("MYSQL_DATABASE", "pythorust_tg"),
        "user": os.getenv("MYSQL_USER", "pythorust_tg"),
        "password": os.getenv("MYSQL_PASSWORD"),
        "charset": "utf8mb4",
        "cursorclass": pymysql.cursors.DictCursor,
    }


def fetch_metrics(
    conn: pymysql.connections.Connection,
    bot_name: str,
    experiment: str,
    days: Optional[int],
) -> List[Dict]:
    where = ["bot_name = %s", "experiment_name = %s"]
    params: List = [bot_name, experiment]
    if days:
        where.append("assigned_at >= DATE_SUB(NOW(), INTERVAL %s DAY)")
        params.append(days)
    where_clause = " AND ".join(where)

    with conn.cursor() as cursor:
        cursor.execute(
            f"""
            SELECT variant,
                   COUNT(*) AS sessions,
                   SUM(conversion) AS conversions,
                   SUM(conversion_value) AS conversion_value_sum,
                   SUM(conversion_value IS NOT NULL) AS conversions_with_value
            FROM bot_experiments
            WHERE {where_clause}
            GROUP BY variant
            ORDER BY variant
            """,
            params,
        )
        rows = cursor.fetchall()

        cursor.execute(
            f"""
            SELECT variant, conversion_reason, COUNT(*) AS cnt
            FROM bot_experiments
            WHERE {where_clause} AND conversion = 1 AND conversion_reason IS NOT NULL
            GROUP BY variant, conversion_reason
            """,
            params,
        )
        reasons_raw = cursor.fetchall()

    reasons_map: Dict[str, Dict[str, int]] = defaultdict(dict)
    for item in reasons_raw:
        reasons_map[item["variant"]][item["conversion_reason"]] = item["cnt"]

    for row in rows:
        variant = row["variant"]
        row["reason_breakdown"] = reasons_map.get(variant, {})
        sessions = max(row["sessions"], 1)
        conversions = row["conversions"] or 0
        row["conversion_rate"] = round((conversions / sessions) * 100, 2)
        if row["conversions_with_value"]:
            row["avg_conversion_value"] = round((row["conversion_value_sum"] or 0) / row["conversions_with_value"], 2)
        else:
            row["avg_conversion_value"] = None
    return rows


def print_report(rows: List[Dict], bot_name: str, experiment: str, days: Optional[int]):
    header = f"🧪 A/B отчет для {bot_name} / {experiment}"
    if days:
        header += f" (последние {days} дн.)"
    print(header)
    print("-" * len(header))
    if not rows:
        print("Нет данных в bot_experiments по заданным фильтрам.")
        return

    print(f"{'Variant':22} {'Sessions':8} {'Conv':6} {'Rate %':7} {'Avg value':10}")
    for row in rows:
        avg_val = row["avg_conversion_value"]
        avg_text = f"{avg_val:.2f}" if avg_val is not None else "-"
        print(
            f"{row['variant'][:22]:22} "
            f"{row['sessions']:8} "
            f"{row['conversions'] or 0:6} "
            f"{row['conversion_rate']:7.2f} "
            f"{avg_text:10}"
        )
        if row["reason_breakdown"]:
            reasons = ", ".join(f"{reason}:{count}" for reason, count in row["reason_breakdown"].items())
            print(f"  reasons: {reasons}")


def main():
    parser = argparse.ArgumentParser(description="A/B отчет по промптам")
    parser.add_argument("--bot-name", default="BFL_sales_bot", help="Имя бота в БД")
    parser.add_argument(
        "--experiment",
        default=os.getenv("BFL_PROMPT_EXPERIMENT", "bfl_prompt_ab"),
        help="Имя эксперимента (experiment_name в bot_experiments)",
    )
    parser.add_argument(
        "--days",
        type=int,
        default=None,
        help="Фильтр по дате (последние N дней), по умолчанию все записи",
    )

    args = parser.parse_args()

    load_dotenv()
    conn = pymysql.connect(**build_mysql_config())

    try:
        rows = fetch_metrics(conn, args.bot_name, args.experiment, args.days)
        print_report(rows, args.bot_name, args.experiment, args.days)
    except pymysql.err.ProgrammingError as exc:
        print("bot_experiments не найдена. Запустите бота с A/B менеджером и повторите.")
        print(f"SQL error: {exc}")
    finally:
        conn.close()


if __name__ == "__main__":
    main()
