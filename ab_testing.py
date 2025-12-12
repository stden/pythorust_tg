import logging
import random
import re
from dataclasses import dataclass
from typing import Any, List, Optional

logger = logging.getLogger(__name__)


@dataclass
class PromptVariant:
    name: str
    prompt: str
    description: str = ""
    weight: float = 1.0
    temperature: float = 0.7
    model: Optional[str] = None


class ABTestManager:
    """
    Minimal A/B test manager for prompt experiments.

    Responsibilities:
    - Persist variant assignment per session
    - Provide prompt/temperature for the assigned variant
    - Mark conversions for basic offline reporting
    """

    def __init__(
        self,
        db: Any,
        bot_name: str,
        experiment_name: str,
        variants: List[PromptVariant],
    ):
        if not variants:
            raise ValueError("At least one prompt variant is required")

        self.db = db
        self.bot_name = bot_name
        self.experiment_name = experiment_name
        self.variants = variants

        self._ensure_table()

    def _ensure_table(self) -> None:
        """Create experiments table if it does not exist."""
        self.db.ensure_connection()
        with self.db.conn.cursor() as cursor:
            cursor.execute(
                """
                CREATE TABLE IF NOT EXISTS bot_experiments (
                    id BIGINT AUTO_INCREMENT PRIMARY KEY,
                    bot_name VARCHAR(64) NOT NULL,
                    experiment_name VARCHAR(128) NOT NULL,
                    session_id BIGINT NULL,
                    user_id BIGINT NOT NULL,
                    variant VARCHAR(64) NOT NULL,
                    conversion TINYINT(1) DEFAULT 0,
                    conversion_reason VARCHAR(255) NULL,
                    conversion_value INT NULL,
                    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    closed_at TIMESTAMP NULL,
                    KEY idx_experiment (bot_name, experiment_name, variant),
                    KEY idx_session (session_id),
                    KEY idx_user (user_id)
                )
                """
            )
        self.db.conn.commit()

    def _choose_variant(self) -> PromptVariant:
        total_weight = sum(v.weight for v in self.variants)
        pick = random.uniform(0, total_weight)
        cumulative = 0.0
        for variant in self.variants:
            cumulative += variant.weight
            if pick <= cumulative:
                return variant
        return self.variants[-1]

    def _fetch_assigned_variant(self, session_id: Optional[int], user_id: Optional[int]) -> Optional[str]:
        """Fetch saved variant name for a session/user if present."""
        self.db.ensure_connection()
        where = ["bot_name = %s", "experiment_name = %s"]
        params: List[Any] = [self.bot_name, self.experiment_name]
        if session_id:
            where.append("session_id = %s")
            params.append(session_id)
        if user_id:
            where.append("user_id = %s")
            params.append(user_id)
        where_clause = " AND ".join(where)

        query = f"""
            SELECT variant
            FROM bot_experiments
            WHERE {where_clause}
            ORDER BY assigned_at DESC
            LIMIT 1
        """
        with self.db.conn.cursor() as cursor:
            cursor.execute(query, params)
            row = cursor.fetchone()
            return row["variant"] if row else None

    def get_or_assign_variant(self, user_id: int, session_id: Optional[int]) -> PromptVariant:
        """
        Returns an assigned variant, creating one if needed.
        """
        existing = self._fetch_assigned_variant(session_id, user_id)
        if existing:
            return self._variant_by_name(existing)

        chosen = self._choose_variant()
        self.db.ensure_connection()
        with self.db.conn.cursor() as cursor:
            cursor.execute(
                """
                INSERT INTO bot_experiments
                (bot_name, experiment_name, session_id, user_id, variant)
                VALUES (%s, %s, %s, %s, %s)
                """,
                (self.bot_name, self.experiment_name, session_id, user_id, chosen.name),
            )
        self.db.conn.commit()
        logger.info(
            "Assigned variant %s for user %s (session %s, experiment %s)",
            chosen.name,
            user_id,
            session_id,
            self.experiment_name,
        )
        return chosen

    def _variant_by_name(self, name: str) -> PromptVariant:
        for variant in self.variants:
            if variant.name == name:
                return variant
        raise ValueError(f"Unknown variant: {name}")

    def mark_conversion(
        self,
        session_id: Optional[int],
        reason: Optional[str] = None,
        conversion_value: Optional[int] = None,
    ) -> None:
        """Mark conversion for the given session if an assignment exists."""
        if not session_id:
            return

        self.db.ensure_connection()
        with self.db.conn.cursor() as cursor:
            cursor.execute(
                """
                UPDATE bot_experiments
                SET conversion = 1,
                    conversion_reason = COALESCE(%s, conversion_reason),
                    conversion_value = COALESCE(%s, conversion_value),
                    closed_at = COALESCE(closed_at, CURRENT_TIMESTAMP)
                WHERE session_id = %s AND bot_name = %s AND experiment_name = %s
                """,
                (reason, conversion_value, session_id, self.bot_name, self.experiment_name),
            )
        self.db.conn.commit()

    def detect_conversion(self, text: str) -> Optional[str]:
        """Detect simple conversion intents (phone/CTA) in user text."""
        if not text:
            return None

        phone_match = re.search(r"(?:\+?\d[\d\s\-\(\)]{8,}\d)", text)
        if phone_match:
            return "phone_shared"

        normalized = text.lower()
        intent_keywords = [
            "беру",
            "покупаю",
            "оформляем",
            "оплачиваю",
            "готов купить",
            "давай оформим",
            "давайте оформим",
            "давай заказ",
            "берем",
            "хочу купить",
        ]
        for kw in intent_keywords:
            if kw in normalized:
                return "purchase_intent"

        delivery_keywords = ["доставка", "оплата", "адрес", "курьер", "самовывоз"]
        if any(word in normalized for word in delivery_keywords) and ("давай" in normalized or "офор" in normalized):
            return "checkout_details"

        return None

    def detect_and_mark_conversion(self, session_id: Optional[int], text: str) -> Optional[str]:
        reason = self.detect_conversion(text)
        if reason:
            self.mark_conversion(session_id, reason)
        return reason

    def close_assignment(self, session_id: Optional[int]) -> None:
        if not session_id:
            return
        self.db.ensure_connection()
        with self.db.conn.cursor() as cursor:
            cursor.execute(
                """
                UPDATE bot_experiments
                SET closed_at = COALESCE(closed_at, CURRENT_TIMESTAMP)
                WHERE session_id = %s AND bot_name = %s AND experiment_name = %s
                """,
                (session_id, self.bot_name, self.experiment_name),
            )
        self.db.conn.commit()
