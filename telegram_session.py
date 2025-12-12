"""
Общий модуль для управления Telegram сессиями.
Использует только SQLite файловые сессии для стабильности.

КРИТИЧЕСКИ ВАЖНО:
1. Не запускайте скрипты без существующего session файла!
2. Не запускайте несколько скриптов одновременно - только последовательно!
"""

import os
import sys
import fcntl
from telethon import TelegramClient

from dotenv import load_dotenv

load_dotenv()


def _require_env(name: str) -> str:
    """Получить обязательную переменную окружения или завершить работу."""
    value = os.getenv(name)
    if value is None or value.strip() == "":
        raise RuntimeError(f"Переменная окружения {name} не задана (добавьте в .env).")
    return value


def _require_int_env(name: str) -> int:
    """Получить обязательную числовую переменную окружения."""
    value = _require_env(name)
    try:
        return int(value)
    except ValueError as exc:
        raise RuntimeError(f"{name} должно быть целым числом.") from exc


PHONE = _require_env("TELEGRAM_PHONE")
API_ID = _require_int_env("TELEGRAM_API_ID")
API_HASH = _require_env("TELEGRAM_API_HASH")

# ВАЖНО: Python (Telethon) и Rust (grammers) используют разные форматы SQLite схемы!
# Если TELEGRAM_SESSION_NAME=telegram_session, Python автоматически добавит _py суффикс
# для избежания конфликта с Rust сессией.
_raw_session = os.getenv("TELEGRAM_SESSION_NAME", "telegram_session").strip() or "telegram_session"
SESSION_NAME = f"{_raw_session}_py" if _raw_session == "telegram_session" else _raw_session
LOCK_FILE = f"{SESSION_NAME}.lock"


def _get_user_id() -> int | None:
    raw = os.getenv("USER_ID") or os.getenv("MY_ID")
    if raw is None or raw.strip() == "":
        return None
    try:
        return int(raw)
    except ValueError:
        print("USER_ID/MY_ID должно быть целым числом. Игнорирую значение.")
        return None


USER_ID = _get_user_id()
USER_NAME = (os.getenv("USER_NAME") or os.getenv("MY_NAME") or "").strip() or "User"


class SessionLock:
    """
    Контекстный менеджер для блокировки сессии.
    Гарантирует, что только один скрипт использует сессию одновременно.
    """

    def __init__(self):
        self.lock_file = None
        self.locked = False

    def __enter__(self):
        self.lock_file = open(LOCK_FILE, "w")
        try:
            # Пытаемся получить эксклюзивную блокировку (неблокирующий режим)
            fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.locked = True
        except IOError:
            # Не удалось получить блокировку - другой скрипт уже работает
            print("""
⚠️  ОШИБКА: Telegram сессия уже используется другим скриптом!

Telegram требует последовательного выполнения операций.
Параллельное использование одной сессии может привести к конфликтам и блокировкам.

Подождите, пока завершится другой скрипт, и попробуйте снова.
            """)
            self.lock_file.close()
            sys.exit(1)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.locked and self.lock_file:
            fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_UN)
            self.lock_file.close()
            # Удаляем lock файл после освобождения
            try:
                os.remove(LOCK_FILE)
            except OSError:
                pass


def check_session_exists():
    """
    Проверяет наличие session файла.
    Если файла нет - выводит предупреждение и завершает программу.

    ВАЖНО: Это защищает от случайного создания новой сессии!
    """
    session_file = f"{SESSION_NAME}.session"
    if not os.path.exists(session_file):
        print(f"""
⚠️  ОШИБКА: Session файл '{session_file}' не найден!

Это защита от случайного создания новой сессии, которая вытеснит
активные сессии на других устройствах.

Для создания session файла:
1. Запустите скрипт один раз ЛОКАЛЬНО (не в CI/CD)
2. Введите номер телефона
3. Введите код из Telegram

Или используйте существующий session файл из резервной копии.
        """)
        sys.exit(1)


def get_client():
    """
    Возвращает настроенный Telegram клиент с SQLite сессией.

    ВАЖНО:
    - Проверяет наличие session файла перед созданием клиента
    - Используйте вместе с SessionLock для защиты от параллельного запуска

    Пример использования:
        from telegram_session import get_client, SessionLock

        client = get_client()
        with SessionLock():  # Блокировка от параллельного запуска
            with client:     # Подключение к Telegram
                # ваш код
    """
    check_session_exists()
    return TelegramClient(SESSION_NAME, API_ID, API_HASH)


# Кэш для имен отправителей (загружается из config.yml или остаётся пустым)
known_senders = {}
if USER_ID is not None:
    known_senders[USER_ID] = USER_NAME
