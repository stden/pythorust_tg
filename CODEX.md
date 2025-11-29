# Codex Instructions - Telegram Bots Testing

## 📍 Расположение проектов

| Проект | Путь | Описание |
|--------|------|----------|
| **Telegram CLI** | `/srv/pythorust_tg/` | Rust CLI для экспорта чатов |
| **BFL Sales Bot** | `/srv/BFL_sales_bot/` | Rust бот для ФЦБ (банкротство) |
| **Credit Expert Bot** | `/srv/pythorust_tg/credit_expert_bot.py` | Python бот (legacy) |

## 📂 Где брать диалоги для тестирования

### Экспортированные чаты
```
/srv/pythorust_tg/chats/
├── Хара.txt              # 572KB - основной канал
├── вайбкодеры.md         # Сообщество разработчиков
├── Hobbitkn.md           # Личный чат
├── Golang_GO.txt         # Golang чат
├── iriy5.txt             # Личный чат
└── ValTarobot.txt        # Чат с ботом
```

### Экспорт нового чата
```bash
cd /srv/pythorust_tg/rust_telegram
cargo run --release --bin tg -- "Chat Name" --limit 500

# Или через конфиг
cargo run --release --bin read_chat -- dasha5 --limit 300
```

### Конфигурация чатов
```yaml
# /srv/pythorust_tg/config.yml
chats:
  dasha5:
    type: user
    id: 5551302260
    title: "Даша 5"
  Хара:
    type: channel
    id: 2325284588
```

## 🧪 Тестирование BFL Sales Bot

### Расположение
```
/srv/BFL_sales_bot/
├── src/main.rs           # Основной код бота
├── src/openai.rs         # OpenAI интеграция
├── prompts/              # Промпты для AI
│   ├── consultant.md     # RAG консультант
│   ├── contact.md        # Установление контакта
│   ├── greeting.md       # Приветствие
│   └── objection_handling.md
├── .env                  # Конфигурация
└── target/release/bfl_sales_bot  # Бинарник
```

### Проверка статуса
```bash
# Проверить что бот запущен
ps aux | grep bfl_sales_bot

# Логи
journalctl -u bfl_sales_bot -f

# Перезапуск
systemctl restart bfl_sales_bot
# или
pkill bfl_sales_bot && cd /srv/BFL_sales_bot && ./target/release/bfl_sales_bot &
```

### Тестирование в Telegram
1. Найти бота: `@BFL_sales_bot`
2. Отправить `/start`
3. Проверить сценарии:
   - Приветствие и запрос имени
   - Валидация имени (не принимать "привет", "ок")
   - Сбор информации о долгах
   - Запрос телефона
   - Отработка возражений

### Известные проблемы (из тестов Даши)
1. **Эмодзи** - бот добавляет эмодзи когда не нужно
2. **Имена** - принимает "привет" как имя
3. **Тон** - слишком "тёплый", нужен профессиональный
4. **Сессии** - не всегда продолжает диалог

### Unit тесты
```bash
cd /srv/BFL_sales_bot
cargo test

# Конкретный тест
cargo test test_name_validation
```

## 🧪 Тестирование Credit Expert Bot (Python)

### Запуск
```bash
cd /srv/pythorust_tg
python credit_expert_bot.py
```

### Требуемые переменные (.env)
```
CREDIT_EXPERT_BOT_TOKEN=<token from @BotFather>
MYSQL_HOST=localhost
MYSQL_DATABASE=pythorust_tg
OPENAI_API_KEY=sk-...
```

### Тесты
```bash
cd /srv/pythorust_tg
pytest tests/test_mysql_logger.py -v
```

## 📊 MySQL база данных

### Подключение
```bash
mysql -u pythorust_tg -p pythorust_tg
# Пароль в /srv/pythorust_tg/.env
```

### Таблицы ботов
```sql
-- Пользователи
SELECT * FROM bot_users ORDER BY last_seen_at DESC LIMIT 10;

-- Сообщения
SELECT * FROM bot_messages WHERE bot_name='BFL_sales_bot' ORDER BY created_at DESC LIMIT 20;

-- Сессии
SELECT * FROM bot_sessions WHERE is_active=TRUE;
```

### Просмотр диалогов из БД
```sql
SELECT 
  direction,
  message_text,
  created_at
FROM bot_messages 
WHERE user_id = 5551302260 
  AND bot_name = 'BFL_sales_bot'
ORDER BY created_at DESC 
LIMIT 50;
```

## 🔧 Отладка

### Логи Rust бота
```bash
RUST_LOG=debug ./target/release/bfl_sales_bot
```

### Логи Python бота
```bash
python credit_expert_bot.py 2>&1 | tee /tmp/credit_bot.log
```

### Отправка тестового сообщения
```bash
cd /srv/pythorust_tg/rust_telegram
cargo run --release --bin send_message -- 5551302260 "Тестовое сообщение"
```

## 📝 Промпты

### BFL Sales Bot промпты
```
/srv/BFL_sales_bot/prompts/
├── consultant.md       # Основной промпт Алины
├── contact.md          # Установление контакта
├── greeting.md         # Приветствие
├── needs_discovery.md  # Выявление потребностей
├── objection_handling.md # Отработка возражений
└── offer.md            # Презентация услуг
```

### Обновление промптов
1. Отредактировать файл в `/srv/BFL_sales_bot/prompts/`
2. Перезапустить бота (он загружает промпты при старте)
3. Или обновить в MySQL: `UPDATE prompts SET content='...' WHERE name='consultant'`

## 🚀 CI/CD

### Сборка
```bash
cd /srv/BFL_sales_bot
cargo build --release
```

### Деплой
```bash
systemctl stop bfl_sales_bot
cp target/release/bfl_sales_bot /usr/local/bin/
systemctl start bfl_sales_bot
```

## 📌 Чеклист перед релизом

- [ ] `cargo fmt` - форматирование
- [ ] `cargo clippy` - линтер без warnings
- [ ] `cargo test` - все тесты проходят
- [ ] Ручное тестирование в Telegram
- [ ] Проверка логов на ошибки
- [ ] Проверка MySQL на корректность записей

---

## 🔬 Автоматический анализ диалогов

### Утилита test_bot_dialogue

Rust утилита для автоматического анализа диалогов с ботом через OpenAI:

```bash
cd /srv/pythorust_tg

# Анализ диалога из файла
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file "@BFL_sales_bot.md"

# Анализ диалога из MySQL по user_id
./target/release/test_bot_dialogue --bot @BFL_sales_bot --user-id 5551302260

# Интерактивный режим
./target/release/test_bot_dialogue --bot @BFL_sales_bot --interactive

# JSON вывод для CI/CD
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md --json

# Только проблемы
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md --problems-only
```

### Что анализирует:

| Категория | Описание |
|-----------|----------|
| `tone` | Тон общения (профессиональный vs "подруга") |
| `emoji` | Использование эмодзи (обычно не нужны) |
| `name_validation` | Валидация имени клиента |
| `session_continuity` | Продолжение сессии |
| `response_length` | Длина ответов (2-4 предложения) |
| `call_to_action` | Призыв к действию |
| `objection_handling` | Отработка возражений |
| `jailbreak_attempt` | Защита от взлома |

### Уровни критичности:

- 🔴 `critical` - блокирующая проблема
- 🟠 `high` - серьёзная проблема
- 🟡 `medium` - желательно исправить
- 🟢 `low` - минорное замечание

### Интеграция в CI/CD:

```bash
# Выход с кодом 1 если есть critical проблемы
./target/release/test_bot_dialogue --bot @BFL_sales_bot --file dialogue.md
if [ $? -ne 0 ]; then
  echo "❌ Найдены критические проблемы!"
  exit 1
fi
```
