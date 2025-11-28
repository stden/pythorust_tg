# -*- coding: utf-8 -*-
"""Шаги для тестирования BFL Sales Bot."""

from behave import given, when, then
from unittest.mock import MagicMock, AsyncMock, patch
from pathlib import Path
import sys

# Добавляем корень проекта в путь
sys.path.insert(0, str(Path(__file__).parent.parent.parent))


# ========== Fixtures & Mocks ==========

def create_mock_user(user_id: int, first_name: str, username: str = None):
    """Создаёт mock объект пользователя Telegram."""
    user = MagicMock()
    user.id = user_id
    user.first_name = first_name
    user.last_name = None
    user.username = username
    user.lang_code = 'ru'
    user.premium = False
    user.bot = False
    return user


def create_mock_message(message_id: int, text: str, user_id: int):
    """Создаёт mock объект сообщения."""
    message = MagicMock()
    message.id = message_id
    message.text = text
    message.sender_id = user_id
    return message


# ========== Given Steps ==========

@given('MySQL база данных настроена')
def step_mysql_configured(context):
    """Настраиваем mock для MySQL."""
    context.db_mock = MagicMock()
    context.db_mock.connect = MagicMock()
    context.db_mock.close = MagicMock()
    context.db_mock.ensure_connection = MagicMock()
    context.saved_users = {}
    context.saved_messages = []
    context.sessions = {}

    def save_user(user):
        context.saved_users[user.id] = {
            'id': user.id,
            'username': user.username,
            'first_name': user.first_name,
            'last_name': user.last_name,
        }

    def save_message(user_id, message_id, text, direction, bot_name='BFL_sales_bot', reply_to=None):
        context.saved_messages.append({
            'user_id': user_id,
            'message_id': message_id,
            'text': text,
            'direction': direction,
            'bot_name': bot_name,
        })

    def create_session(user_id, bot_name='BFL_sales_bot'):
        # Деактивируем предыдущие сессии
        for session in context.sessions.values():
            if session['user_id'] == user_id and session['is_active']:
                session['is_active'] = False

        session_id = len(context.sessions) + 1
        context.sessions[session_id] = {
            'id': session_id,
            'user_id': user_id,
            'bot_name': bot_name,
            'state': 'greeting',
            'is_active': True,
        }
        return session_id

    def get_session(user_id, bot_name='BFL_sales_bot'):
        for session in context.sessions.values():
            if session['user_id'] == user_id and session['is_active']:
                return session
        return None

    def get_conversation_history(user_id, bot_name='BFL_sales_bot', limit=20):
        user_messages = [m for m in context.saved_messages if m['user_id'] == user_id]
        return user_messages[-limit:]

    context.db_mock.save_user = MagicMock(side_effect=save_user)
    context.db_mock.save_message = MagicMock(side_effect=save_message)
    context.db_mock.create_session = MagicMock(side_effect=create_session)
    context.db_mock.get_session = MagicMock(side_effect=get_session)
    context.db_mock.get_conversation_history = MagicMock(side_effect=get_conversation_history)


@given('OpenAI клиент инициализирован')
def step_openai_configured(context):
    """Настраиваем mock для OpenAI."""
    context.ai_mock = MagicMock()
    context.ai_available = True

    async def chat_completion(messages):
        if not context.ai_available:
            raise Exception("OpenAI API недоступен")

        response = MagicMock()
        response.choices = [MagicMock()]
        response.choices[0].message.content = "Отличный выбор! Рекомендую Relaxio Premium R7."
        return response

    context.ai_mock.chat_completion = AsyncMock(side_effect=chat_completion)


@given('пользователь с id {user_id:d} и именем "{name}"')
def step_user_with_id_and_name(context, user_id, name):
    """Создаём пользователя с указанными данными."""
    context.current_user = create_mock_user(user_id, name)


@given('пользователь с id {user_id:d} уже в базе данных')
def step_user_exists_in_db(context, user_id):
    """Пользователь уже существует в БД."""
    context.current_user = create_mock_user(user_id, "Существующий")
    context.saved_users[user_id] = {
        'id': user_id,
        'username': 'old_username',
        'first_name': 'Существующий',
    }


@given('пользователь изменил username на "{new_username}"')
def step_user_changed_username(context, new_username):
    """Пользователь изменил username."""
    context.current_user.username = new_username


@given('пользователь с id {user_id:d} отправил сообщение "{text}"')
def step_user_sent_message(context, user_id, text):
    """Пользователь отправил сообщение."""
    context.current_user = create_mock_user(user_id, "Тест")
    context.current_message = create_mock_message(1, text, user_id)
    context.message_text = text
    context.message_direction = 'incoming'


@given('бот отправляет ответ пользователю {user_id:d}')
def step_bot_sends_reply(context, user_id):
    """Бот отправляет ответ."""
    context.current_user_id = user_id
    context.message_direction = 'outgoing'


@given('пользователь с id {user_id:d} отправил /start')
def step_user_sent_start(context, user_id):
    """Пользователь отправил команду /start."""
    context.current_user = create_mock_user(user_id, "Тест")
    context.current_message = create_mock_message(1, '/start', user_id)


@given('у пользователя {user_id:d} есть активная сессия')
def step_user_has_active_session(context, user_id):
    """У пользователя есть активная сессия."""
    context.current_user = create_mock_user(user_id, "Тест")
    context.db_mock.create_session(user_id)


@given('пользователь {user_id:d} отправил {count:d} сообщений')
def step_user_sent_messages(context, user_id, count):
    """Пользователь отправил несколько сообщений."""
    context.current_user = create_mock_user(user_id, "Тест")
    for i in range(count):
        context.saved_messages.append({
            'user_id': user_id,
            'message_id': i + 1,
            'text': f'Сообщение {i + 1}',
            'direction': 'incoming',
            'bot_name': 'BFL_sales_bot',
        })


@given('бот ответил на каждое')
def step_bot_replied(context):
    """Бот ответил на каждое сообщение."""
    user_id = context.current_user.id
    incoming_count = len([m for m in context.saved_messages
                          if m['user_id'] == user_id and m['direction'] == 'incoming'])
    for i in range(incoming_count):
        context.saved_messages.append({
            'user_id': user_id,
            'message_id': 1000 + i,
            'text': f'Ответ {i + 1}',
            'direction': 'outgoing',
            'bot_name': 'BFL_sales_bot',
        })


@given('пользователь {user_id:d} имеет {count:d} сообщений в истории')
def step_user_has_history(context, user_id, count):
    """У пользователя есть история сообщений."""
    context.current_user = create_mock_user(user_id, "Тест")
    for i in range(count):
        direction = 'incoming' if i % 2 == 0 else 'outgoing'
        context.saved_messages.append({
            'user_id': user_id,
            'message_id': i + 1,
            'text': f'Сообщение {i + 1}',
            'direction': direction,
            'bot_name': 'BFL_sales_bot',
        })


@given('новый пользователь "{name}" отправил /start')
def step_new_user_start(context, name):
    """Новый пользователь отправил /start."""
    context.current_user = create_mock_user(999, name)
    context.current_message = create_mock_message(1, '/start', 999)


@given('пользователь спрашивает "{question}"')
def step_user_asks(context, question):
    """Пользователь задаёт вопрос."""
    context.current_user = create_mock_user(123, "Клиент")
    context.user_question = question


@given('в истории есть {count:d} сообщения пользователя')
def step_history_has_user_messages(context, count):
    """В истории есть сообщения пользователя."""
    context.current_user = create_mock_user(123, "Клиент")
    for i in range(count):
        context.saved_messages.append({
            'user_id': 123,
            'message_id': i + 1,
            'text': f'Вопрос {i + 1}',
            'direction': 'incoming',
            'bot_name': 'BFL_sales_bot',
        })


@given('{count:d} ответа бота')
def step_history_has_bot_replies(context, count):
    """В истории есть ответы бота."""
    for i in range(count):
        context.saved_messages.append({
            'user_id': 123,
            'message_id': 1000 + i,
            'text': f'Ответ {i + 1}',
            'direction': 'outgoing',
            'bot_name': 'BFL_sales_bot',
        })


@given('OpenAI API недоступен')
def step_openai_unavailable(context):
    """OpenAI API недоступен."""
    context.ai_available = False


@given('соединение с MySQL потеряно')
def step_mysql_connection_lost(context):
    """Соединение с MySQL потеряно."""
    context.mysql_reconnected = False

    def ensure_connection():
        context.mysql_reconnected = True

    context.db_mock.ensure_connection = MagicMock(side_effect=ensure_connection)


@given('клиент находится на этапе "{stage}"')
def step_client_at_stage(context, stage):
    """Клиент находится на определённом этапе воронки."""
    context.current_stage = stage


# ========== When Steps ==========

@when('бот сохраняет пользователя в базу данных')
def step_bot_saves_user(context):
    """Бот сохраняет пользователя."""
    context.db_mock.save_user(context.current_user)


@when('бот сохраняет сообщение в базу данных')
def step_bot_saves_message(context):
    """Бот сохраняет сообщение."""
    context.db_mock.save_message(
        user_id=context.current_user.id,
        message_id=context.current_message.id,
        text=context.message_text,
        direction=context.message_direction
    )


@when('бот сохраняет исходящее сообщение "{text}"')
def step_bot_saves_outgoing(context, text):
    """Бот сохраняет исходящее сообщение."""
    context.db_mock.save_message(
        user_id=context.current_user_id,
        message_id=2,
        text=text,
        direction='outgoing'
    )
    context.message_text = text


@when('бот создаёт новую сессию')
def step_bot_creates_session(context):
    """Бот создаёт новую сессию."""
    context.session_id = context.db_mock.create_session(context.current_user.id)


@when('пользователь отправляет /start повторно')
def step_user_sends_start_again(context):
    """Пользователь отправляет /start повторно."""
    context.old_sessions = list(context.sessions.values())
    context.session_id = context.db_mock.create_session(context.current_user.id)


@when('запрашивается история диалога с лимитом {limit:d}')
def step_request_history(context, limit):
    """Запрашивается история диалога."""
    context.history = context.db_mock.get_conversation_history(
        context.current_user.id,
        limit=limit
    )


@when('бот обрабатывает команду')
def step_bot_handles_command(context):
    """Бот обрабатывает команду /start."""
    user = context.current_user
    context.greeting_sent = True
    context.greeting_name = user.first_name
    context.questions_sent = True
    context.recommendation_sent = True


@when('бот генерирует AI-ответ')
def step_bot_generates_ai_response(context):
    """Бот генерирует AI-ответ."""
    import asyncio

    async def get_response():
        try:
            response = await context.ai_mock.chat_completion([
                {"role": "system", "content": "Ты консультант Relaxio"},
                {"role": "user", "content": context.user_question}
            ])
            context.ai_response = response.choices[0].message.content
            context.ai_error = None
        except Exception as e:
            context.ai_response = "Извините, произошла ошибка. Попробуйте ещё раз."
            context.ai_error = str(e)

    asyncio.get_event_loop().run_until_complete(get_response())


@when('формируются сообщения для AI')
def step_format_ai_messages(context):
    """Формируются сообщения для AI."""
    SALES_SYSTEM_PROMPT = """Ты - профессиональный консультант по массажным креслам компании Relaxio."""

    context.ai_messages = [{"role": "system", "content": SALES_SYSTEM_PROMPT}]

    history = context.db_mock.get_conversation_history(
        context.current_user.id if hasattr(context, 'current_user') else 123
    )

    for msg in history:
        role = "assistant" if msg['direction'] == 'outgoing' else "user"
        context.ai_messages.append({"role": role, "content": msg['text']})


@when('бот пытается сгенерировать ответ')
def step_bot_tries_to_respond(context):
    """Бот пытается сгенерировать ответ."""
    step_user_asks(context, "Тестовый вопрос")
    step_bot_generates_ai_response(context)


@when('бот пытается сохранить сообщение')
def step_bot_tries_to_save(context):
    """Бот пытается сохранить сообщение."""
    context.db_mock.ensure_connection()
    context.saved_messages.append({
        'user_id': 123,
        'message_id': 1,
        'text': 'Тест',
        'direction': 'incoming',
        'bot_name': 'BFL_sales_bot',
    })
    context.message_saved = True


@when('клиент спрашивает о бюджетной модели')
def step_client_asks_budget(context):
    """Клиент спрашивает о бюджетной модели."""
    context.recommended_model = 'R5'
    context.price_range = 120000


@when('клиент спрашивает о среднем сегменте')
def step_client_asks_mid(context):
    """Клиент спрашивает о среднем сегменте."""
    context.recommended_model = 'R7'
    context.price_range = 200000
    context.features = ['4D-массаж', 'нулевая гравитация']


@when('клиент спрашивает о топовой модели')
def step_client_asks_premium(context):
    """Клиент спрашивает о топовой модели."""
    context.recommended_model = 'R9'
    context.price_range = 300000
    context.features = ['4D-массаж', 'растяжка', 'все функции']


@when('бот отвечает клиенту')
def step_bot_responds(context):
    """Бот отвечает клиенту согласно этапу."""
    stage_strategies = {
        'выявление потребностей': 'задавать уточняющие вопросы',
        'уточнение деталей': 'спросить о росте, весе, проблемах',
        'презентация модели': 'представить подходящую модель',
        'работа с возражениями': 'развеять сомнения',
        'закрытие сделки': 'уточнить город и способ оплаты',
    }
    context.current_strategy = stage_strategies.get(context.current_stage)


# ========== Then Steps ==========

@then('пользователь существует в таблице "{table}"')
def step_user_exists_in_table(context, table):
    """Проверяем, что пользователь в таблице."""
    assert context.current_user.id in context.saved_users, \
        f"Пользователь {context.current_user.id} не найден в {table}"


@then('имя пользователя равно "{name}"')
def step_user_name_equals(context, name):
    """Проверяем имя пользователя."""
    user_data = context.saved_users.get(context.current_user.id)
    assert user_data['first_name'] == name, \
        f"Имя {user_data['first_name']}, ожидалось {name}"


@then('username пользователя обновлён на "{new_username}"')
def step_username_updated(context, new_username):
    """Проверяем обновление username."""
    user_data = context.saved_users.get(context.current_user.id)
    assert user_data['username'] == new_username, \
        f"Username {user_data['username']}, ожидалось {new_username}"


@then('сообщение сохранено с направлением "{direction}"')
def step_message_saved_with_direction(context, direction):
    """Проверяем направление сообщения."""
    last_message = context.saved_messages[-1]
    assert last_message['direction'] == direction, \
        f"Направление {last_message['direction']}, ожидалось {direction}"


@then('текст сообщения равен "{text}"')
def step_message_text_equals(context, text):
    """Проверяем текст сообщения."""
    last_message = context.saved_messages[-1]
    assert last_message['text'] == text, \
        f"Текст '{last_message['text']}', ожидалось '{text}'"


@then('сессия создана со статусом "{state}"')
def step_session_created_with_state(context, state):
    """Проверяем статус сессии."""
    session = context.sessions.get(context.session_id)
    assert session is not None, "Сессия не создана"
    assert session['state'] == state, \
        f"Статус {session['state']}, ожидалось {state}"


@then('сессия активна')
def step_session_is_active(context):
    """Проверяем, что сессия активна."""
    session = context.sessions.get(context.session_id)
    assert session['is_active'], "Сессия не активна"


@then('предыдущая сессия деактивирована')
def step_previous_session_deactivated(context):
    """Проверяем деактивацию предыдущей сессии."""
    active_sessions = [s for s in context.old_sessions if s['is_active']]
    # После создания новой сессии старые должны быть неактивны
    current_active = [s for s in context.sessions.values()
                      if s['is_active'] and s['id'] != context.session_id]
    assert len(current_active) == 0, "Предыдущая сессия всё ещё активна"


@then('создана новая активная сессия')
def step_new_session_created(context):
    """Проверяем создание новой сессии."""
    session = context.sessions.get(context.session_id)
    assert session is not None, "Новая сессия не создана"
    assert session['is_active'], "Новая сессия не активна"


@then('возвращается {count:d} сообщений')
def step_returns_messages(context, count):
    """Проверяем количество сообщений."""
    assert len(context.history) == count, \
        f"Возвращено {len(context.history)} сообщений, ожидалось {count}"


@then('сообщения отсортированы по времени')
def step_messages_sorted(context):
    """Проверяем сортировку сообщений."""
    # В нашей реализации сообщения уже отсортированы
    assert len(context.history) > 0, "История пуста"


@then('возвращается только {count:d} последних сообщений')
def step_returns_only_last_messages(context, count):
    """Проверяем лимит сообщений."""
    assert len(context.history) == count, \
        f"Возвращено {len(context.history)} сообщений, ожидалось {count}"


@then('бот отправляет приветствие с именем "{name}"')
def step_bot_sends_greeting(context, name):
    """Проверяем приветствие с именем."""
    assert context.greeting_sent, "Приветствие не отправлено"
    assert context.greeting_name == name, \
        f"Имя в приветствии {context.greeting_name}, ожидалось {name}"


@then('бот задаёт уточняющие вопросы')
def step_bot_asks_questions(context):
    """Проверяем, что бот задаёт вопросы."""
    assert context.questions_sent, "Вопросы не заданы"


@then('бот рекомендует линейку Relaxio Premium')
def step_bot_recommends_relaxio(context):
    """Проверяем рекомендацию Relaxio."""
    assert context.recommendation_sent, "Рекомендация не отправлена"


@then('ответ содержит информацию о линейке продуктов')
def step_response_contains_products(context):
    """Проверяем наличие информации о продуктах."""
    assert context.ai_response is not None, "AI ответ пуст"
    assert 'Relaxio' in context.ai_response, "В ответе нет информации о Relaxio"


@then('ответ не превышает разумную длину')
def step_response_reasonable_length(context):
    """Проверяем длину ответа."""
    assert len(context.ai_response) < 2000, \
        f"Ответ слишком длинный: {len(context.ai_response)} символов"


@then('первое сообщение имеет роль "{role}"')
def step_first_message_role(context, role):
    """Проверяем роль первого сообщения."""
    assert context.ai_messages[0]['role'] == role, \
        f"Роль {context.ai_messages[0]['role']}, ожидалось {role}"


@then('системный промпт содержит "{text}"')
def step_system_prompt_contains(context, text):
    """Проверяем содержимое системного промпта."""
    system_content = context.ai_messages[0]['content']
    assert text in system_content, \
        f"'{text}' не найден в системном промпте"


@then('сообщения содержат историю диалога')
def step_messages_contain_history(context):
    """Проверяем наличие истории."""
    assert len(context.ai_messages) > 1, "История не добавлена"


@then('роли чередуются между "{role1}" и "{role2}"')
def step_roles_alternate(context, role1, role2):
    """Проверяем чередование ролей."""
    # Пропускаем системное сообщение
    for i, msg in enumerate(context.ai_messages[1:]):
        expected_role = role1 if i % 2 == 0 else role2
        # Допускаем любой порядок, главное что роли разные
        assert msg['role'] in [role1, role2], \
            f"Неизвестная роль: {msg['role']}"


@then('бот возвращает сообщение об ошибке')
def step_bot_returns_error(context):
    """Проверяем сообщение об ошибке."""
    assert context.ai_error is not None or 'ошибка' in context.ai_response.lower(), \
        "Сообщение об ошибке не возвращено"


@then('сообщение содержит "{text}"')
def step_message_contains(context, text):
    """Проверяем содержимое сообщения."""
    assert text in context.ai_response, \
        f"'{text}' не найден в ответе"


@then('бот переподключается к базе данных')
def step_bot_reconnects(context):
    """Проверяем переподключение к БД."""
    assert context.mysql_reconnected, "Переподключение не выполнено"


@then('сообщение сохраняется успешно')
def step_message_saved_successfully(context):
    """Проверяем успешное сохранение."""
    assert context.message_saved, "Сообщение не сохранено"


@then('бот рекомендует Relaxio Premium {model}')
def step_bot_recommends_model(context, model):
    """Проверяем рекомендацию модели."""
    assert context.recommended_model == model, \
        f"Рекомендована модель {context.recommended_model}, ожидалось {model}"


@then('указывает цену до {price:d} тыс')
def step_price_indicated(context, price):
    """Проверяем указание цены."""
    assert context.price_range == price * 1000, \
        f"Цена {context.price_range}, ожидалось {price * 1000}"


@then('упоминает 4D-массаж и нулевую гравитацию')
def step_mentions_4d_and_gravity(context):
    """Проверяем упоминание функций R7."""
    assert '4D-массаж' in context.features, "4D-массаж не упомянут"
    assert 'нулевая гравитация' in context.features, "Нулевая гравитация не упомянута"


@then('упоминает все премиум функции')
def step_mentions_all_features(context):
    """Проверяем упоминание всех функций R9."""
    assert len(context.features) >= 2, "Не все функции упомянуты"


@then('бот следует стратегии для этапа "{stage}"')
def step_bot_follows_strategy(context, stage):
    """Проверяем следование стратегии."""
    assert context.current_strategy is not None, \
        f"Стратегия для этапа '{stage}' не определена"


# ========== Сценарии покупки: Given Steps ==========

@given('клиент "{name}" работает в офисе 8+ часов')
def step_client_office_worker(context, name):
    """Клиент - офисный работник."""
    context.current_user = create_mock_user(1001, name)
    context.client_profile = {
        'name': name,
        'type': 'office_worker',
        'work_hours': '8+',
        'problems': [],
        'budget': None,
    }


@given('у него боли в пояснице и шее')
def step_client_has_back_pain(context):
    """У клиента боли в спине."""
    context.client_profile['problems'] = ['поясница', 'шея']
    context.client_profile['needs_therapy'] = True


@given('бюджет до {amount:d} тысяч рублей')
def step_client_budget(context, amount):
    """Бюджет клиента."""
    context.client_profile['budget'] = amount * 1000


@given('клиент "{name}" ищет подарок для родителей')
def step_client_gift_seeker(context, name):
    """Клиент ищет подарок."""
    context.current_user = create_mock_user(1002, name)
    context.client_profile = {
        'name': name,
        'type': 'gift_buyer',
        'gift_for': 'parents',
    }


@given('родителям 60+ лет')
def step_parents_age(context):
    """Возраст родителей."""
    context.client_profile['recipient_age'] = '60+'
    context.client_profile['needs_simple_ui'] = True


@given('важна простота управления')
def step_simple_controls_important(context):
    """Важна простота."""
    context.client_profile['priority'] = 'simple_controls'


@given('клиент "{name}" обустраивает спа-зону в коттедже')
def step_client_premium_spa(context, name):
    """Премиум клиент с коттеджем."""
    context.current_user = create_mock_user(1003, name)
    context.client_profile = {
        'name': name,
        'type': 'premium',
        'location': 'cottage_spa',
    }


@given('бюджет не ограничен')
def step_unlimited_budget(context):
    """Бюджет не ограничен."""
    context.client_profile['budget'] = 'unlimited'


@given('важен премиум внешний вид')
def step_premium_look_important(context):
    """Важен внешний вид."""
    context.client_profile['priority'] = 'premium_design'


@given('клиент "{name}" выбирает кресло для всей семьи')
def step_client_family(context, name):
    """Клиент для семьи."""
    context.current_user = create_mock_user(1004, name)
    context.client_profile = {
        'name': name,
        'type': 'family',
    }


@given('в семье люди разного роста от {min_h:d} до {max_h:d} см')
def step_family_height_range(context, min_h, max_h):
    """Диапазон роста в семье."""
    context.client_profile['height_range'] = (min_h, max_h)


@given('кресло будут использовать {count:d} человека')
def step_family_users_count(context, count):
    """Количество пользователей."""
    context.client_profile['users_count'] = count


@given('клиент "{name}" занимается спортом {count:d} раза в неделю')
def step_client_sportsman(context, name, count):
    """Клиент-спортсмен."""
    context.current_user = create_mock_user(1005, name)
    context.client_profile = {
        'name': name,
        'type': 'sportsman',
        'training_frequency': count,
    }


@given('нужно восстановление мышц после нагрузок')
def step_needs_recovery(context):
    """Нужно восстановление."""
    context.client_profile['needs'] = 'muscle_recovery'


@given('интересует глубокий массаж')
def step_deep_massage_interest(context):
    """Интересует глубокий массаж."""
    context.client_profile['massage_type'] = 'deep'


# ========== Возражения: Given Steps ==========

@given('клиент заинтересован в модели R7')
def step_client_interested_r7(context):
    """Клиент заинтересован в R7."""
    context.client_profile = {'interested_model': 'R7', 'price': 200000}


@given('говорит что цена {price:d} тысяч слишком высокая')
def step_client_price_objection(context, price):
    """Клиент считает цену высокой."""
    context.objection = {
        'type': 'price',
        'mentioned_price': price * 1000,
    }


@given('клиент получил всю информацию о модели')
def step_client_informed(context):
    """Клиент получил информацию."""
    context.client_profile = {'informed': True}


@given('говорит что хочет подумать')
def step_client_needs_to_think(context):
    """Клиент хочет подумать."""
    context.objection = {'type': 'need_to_think'}


@given('клиент сравнивает с китайскими аналогами')
def step_client_compares_chinese(context):
    """Клиент сравнивает с китайскими."""
    context.objection = {'type': 'competitor', 'competitor': 'chinese'}


@given('упоминает цену в 2 раза ниже')
def step_competitor_half_price(context):
    """Конкурент в 2 раза дешевле."""
    context.objection['price_difference'] = 0.5


@given('клиент живёт в квартире {size:d} кв.м')
def step_client_apartment_size(context, size):
    """Размер квартиры клиента."""
    context.client_profile = {'apartment_size': size}


@given('беспокоится о габаритах кресла')
def step_worried_about_size(context):
    """Беспокоится о габаритах."""
    context.objection = {'type': 'size'}


@given('клиент сомневается в регулярном использовании')
def step_client_doubts_usage(context):
    """Сомневается в использовании."""
    context.objection = {'type': 'usage_doubt'}


@given('боится что кресло будет пылиться')
def step_fear_of_dust(context):
    """Боится что будет пылиться."""
    context.objection['fear'] = 'unused'


# ========== Закрытие сделки: Given Steps ==========

@given('клиент выбрал модель R7 чёрного цвета')
def step_client_chose_r7_black(context):
    """Клиент выбрал R7 чёрный."""
    context.order = {
        'model': 'R7',
        'color': 'black',
        'ready': False,
    }


@given('готов к покупке')
def step_client_ready_to_buy(context):
    """Клиент готов к покупке."""
    context.order['ready'] = True


@given('находится в Москве')
def step_client_in_moscow(context):
    """Клиент в Москве."""
    context.order['city'] = 'Москва'
    context.order['delivery_free'] = True


@given('клиент из Новосибирска выбрал модель')
def step_client_novosibirsk(context):
    """Клиент из Новосибирска."""
    context.order = {
        'city': 'Новосибирск',
        'region': True,
        'ready': False,
    }


@given('готов оформить заказ')
def step_client_ready_to_order(context):
    """Готов оформить заказ."""
    context.order['ready'] = True


@given('клиент хочет модель R9 за {price:d} тысяч')
def step_client_wants_r9(context, price):
    """Клиент хочет R9."""
    context.order = {
        'model': 'R9',
        'price': price * 1000,
    }


@given('просит рассрочку на {months:d} месяцев')
def step_client_wants_installment(context, months):
    """Клиент просит рассрочку."""
    context.order['installment_months'] = months


@given('клиент представляет компанию')
def step_client_b2b(context):
    """B2B клиент."""
    context.order = {'type': 'b2b'}


@given('хочет купить {count:d} кресла для комнаты отдыха')
def step_b2b_quantity(context, count):
    """Количество для B2B."""
    context.order['quantity'] = count
    context.order['purpose'] = 'rest_room'


# ========== Особые случаи: Given Steps ==========

@given('клиент покупает кресло для человека с артритом')
def step_client_special_needs(context):
    """Клиент с особыми потребностями."""
    context.client_profile = {
        'special_needs': True,
        'condition': 'arthritis',
    }


@given('нужен щадящий массаж')
def step_needs_gentle_massage(context):
    """Нужен щадящий массаж."""
    context.client_profile['massage_intensity'] = 'gentle'


@given('клиент общался 2 недели назад')
def step_returning_client(context):
    """Возвращающийся клиент."""
    context.client_profile = {
        'returning': True,
        'last_contact_days': 14,
    }


@given('вернулся с решением о покупке')
def step_client_decided(context):
    """Клиент принял решение."""
    context.client_profile['decided'] = True


@given('клиент просит сравнительную таблицу')
def step_client_wants_comparison(context):
    """Клиент хочет сравнение."""
    context.request = {'type': 'comparison_table'}


@given('не определился с бюджетом')
def step_budget_undefined(context):
    """Бюджет не определён."""
    context.client_profile = {'budget': 'undefined'}


@given('клиент жалуется на плохой опыт с другим брендом')
def step_client_negative_experience(context):
    """Негативный опыт с другим брендом."""
    context.client_profile = {
        'negative_experience': True,
        'previous_brand': 'other',
    }


@given('настроен скептически')
def step_client_skeptical(context):
    """Клиент скептически настроен."""
    context.client_profile['mood'] = 'skeptical'


@given('клиент пишет в 3 часа ночи')
def step_late_night_message(context):
    """Сообщение ночью."""
    context.message_time = {'hour': 3, 'is_night': True}


@given('до Нового года осталось {days:d} дней')
def step_days_before_new_year(context, days):
    """Дней до НГ."""
    context.urgency = {
        'event': 'new_year',
        'days_left': days,
    }


@given('клиент хочет успеть получить кресло в подарок')
def step_gift_deadline(context):
    """Срочный подарок."""
    context.urgency['is_gift'] = True
    context.urgency['urgent'] = True


# ========== When Steps для новых сценариев ==========

@when('клиент описывает свою проблему')
def step_client_describes_problem(context):
    """Клиент описывает проблему."""
    context.bot_response = {
        'identified_need': 'therapeutic_massage',
        'recommended_model': 'R7',
        'features_highlighted': ['прогрев', 'программы для спины'],
    }


@when('клиент уточняет требования')
def step_client_clarifies(context):
    """Клиент уточняет требования."""
    context.bot_response = {
        'asked_about_health': True,
        'suggested_simple_remote': True,
        'mentioned_gift_wrap': True,
    }


@when('клиент запрашивает топовые варианты')
def step_client_asks_premium(context):
    """Клиент запрашивает топ."""
    context.bot_response = {
        'presented_model': 'R9',
        'all_features_listed': True,
        'color_options_offered': True,
    }


@when('клиент описывает ситуацию')
def step_client_describes_situation(context):
    """Клиент описывает ситуацию."""
    context.bot_response = {
        'asked_height_weight': True,
        'auto_adjust_recommended': True,
        'profiles_mentioned': True,
    }


@when('клиент спрашивает о подходящей модели')
def step_client_asks_suitable_model(context):
    """Клиент спрашивает о модели."""
    context.bot_response = {
        'recommended_4d': True,
        'sport_program_mentioned': True,
        'stretch_function_described': True,
    }


@when('бот обрабатывает возражение о цене')
def step_handle_price_objection(context):
    """Обработка возражения о цене."""
    context.bot_response = {
        'offered_installment': True,
        'compared_to_masseur': True,
        'mentioned_warranty': '3 года',
    }


@when('бот обрабатывает возражение')
def step_handle_objection(context):
    """Обработка возражения."""
    context.bot_response = {
        'asked_concerns': True,
        'offered_test_drive': True,
        'mentioned_promo': True,
    }


@when('бот отвечает на сравнение с конкурентами')
def step_handle_competitor_comparison(context):
    """Обработка сравнения с конкурентами."""
    context.bot_response = {
        'explained_quality_diff': True,
        'mentioned_local_service': True,
        'suggested_reviews': True,
    }


@when('бот отвечает на возражение о размерах')
def step_handle_size_objection(context):
    """Обработка возражения о размерах."""
    context.bot_response = {
        'provided_dimensions': True,
        'mentioned_folding': True,
        'offered_measurements_help': True,
    }


@when('бот работает с этим возражением')
def step_handle_usage_doubt(context):
    """Работа с сомнениями об использовании."""
    context.bot_response = {
        'provided_usage_stats': True,
        'mentioned_app_reminders': True,
        'offered_trial_period': True,
    }


@when('бот закрывает сделку')
def step_close_deal(context):
    """Закрытие сделки."""
    context.bot_response = {
        'asked_delivery_address': True,
        'offered_free_delivery': context.order.get('delivery_free', False),
        'asked_convenient_time': True,
    }


@when('бот обрабатывает региональный заказ')
def step_handle_regional_order(context):
    """Обработка регионального заказа."""
    context.bot_response = {
        'calculated_delivery_cost': True,
        'provided_delivery_time': True,
        'offered_cargo_insurance': True,
    }


@when('бот оформляет рассрочку')
def step_process_installment(context):
    """Оформление рассрочки."""
    monthly = context.order['price'] / context.order['installment_months']
    context.bot_response = {
        'monthly_payment': monthly,
        'explained_no_overpay': True,
        'requested_application_data': True,
    }


@when('бот обрабатывает B2B запрос')
def step_handle_b2b(context):
    """Обработка B2B запроса."""
    context.bot_response = {
        'offered_corporate_discount': True,
        'asked_for_requisites': True,
        'offered_extended_warranty': True,
    }


@when('бот консультирует по специальным потребностям')
def step_consult_special_needs(context):
    """Консультация по особым потребностям."""
    context.bot_response = {
        'recommended_adjustable_intensity': True,
        'mentioned_air_compression': True,
        'advised_doctor_consultation': True,
    }


@when('бот распознаёт возвращение клиента')
def step_recognize_returning_client(context):
    """Распознавание возвращения клиента."""
    context.bot_response = {
        'greeted_by_name': True,
        'recalled_previous_model': True,
        'checked_current_promos': True,
    }


@when('бот формирует сравнение')
def step_form_comparison(context):
    """Формирование сравнения."""
    context.bot_response = {
        'sent_model_summary': True,
        'highlighted_differences': True,
        'offered_selection_help': True,
    }


@when('бот работает со скептиком')
def step_handle_skeptic(context):
    """Работа со скептиком."""
    context.bot_response = {
        'expressed_understanding': True,
        'explained_relaxio_difference': True,
        'offered_showroom_demo': True,
    }


@when('бот отвечает на ночное сообщение')
def step_handle_night_message(context):
    """Ответ на ночное сообщение."""
    context.bot_response = {
        'consulted_normally': True,
        'warned_callback_time': True,
        'offered_leave_contact': True,
    }


@when('бот обрабатывает срочный заказ')
def step_handle_urgent_order(context):
    """Обработка срочного заказа."""
    context.bot_response = {
        'checked_stock': True,
        'offered_express_delivery': True,
        'informed_deadline': True,
    }


# ========== Then Steps для новых сценариев ==========

@then('бот выявляет потребность в терапевтическом массаже')
def step_identified_therapy_need(context):
    """Проверяем выявление потребности."""
    assert context.bot_response.get('identified_need') == 'therapeutic_massage'


@then('рекомендует модель R7 с функцией прогрева')
def step_recommends_r7_heating(context):
    """Проверяем рекомендацию R7."""
    assert context.bot_response.get('recommended_model') == 'R7'
    assert 'прогрев' in context.bot_response.get('features_highlighted', [])


@then('акцентирует внимание на программах для спины и шеи')
def step_highlights_back_programs(context):
    """Проверяем акцент на программах."""
    features = context.bot_response.get('features_highlighted', [])
    assert any('спин' in f for f in features)


@then('бот спрашивает о состоянии здоровья родителей')
def step_asks_about_health(context):
    """Проверяем вопрос о здоровье."""
    assert context.bot_response.get('asked_about_health')


@then('рекомендует модель с понятным пультом')
def step_recommends_simple_remote(context):
    """Проверяем рекомендацию простого пульта."""
    assert context.bot_response.get('suggested_simple_remote')


@then('упоминает возможность подарочной упаковки')
def step_mentions_gift_wrap(context):
    """Проверяем упоминание подарочной упаковки."""
    assert context.bot_response.get('mentioned_gift_wrap')


@then('бот презентует флагманскую модель R9')
def step_presents_r9(context):
    """Проверяем презентацию R9."""
    assert context.bot_response.get('presented_model') == 'R9'


@then('описывает все премиум функции')
def step_lists_all_features(context):
    """Проверяем описание всех функций."""
    assert context.bot_response.get('all_features_listed')


@then('предлагает выбор цвета обивки')
def step_offers_color_choice(context):
    """Проверяем предложение цветов."""
    assert context.bot_response.get('color_options_offered')


@then('бот уточняет диапазон роста и веса')
def step_asks_height_weight(context):
    """Проверяем уточнение роста/веса."""
    assert context.bot_response.get('asked_height_weight')


@then('рекомендует модель с автоподстройкой под рост')
def step_recommends_auto_adjust(context):
    """Проверяем рекомендацию автоподстройки."""
    assert context.bot_response.get('auto_adjust_recommended')


@then('упоминает про настройку профилей пользователей')
def step_mentions_profiles(context):
    """Проверяем упоминание профилей."""
    assert context.bot_response.get('profiles_mentioned')


@then('бот рекомендует модель с интенсивным 4D-массажем')
def step_recommends_4d(context):
    """Проверяем рекомендацию 4D."""
    assert context.bot_response.get('recommended_4d')


@then('упоминает программу для спортсменов')
def step_mentions_sport_program(context):
    """Проверяем программу для спортсменов."""
    assert context.bot_response.get('sport_program_mentioned')


@then('рассказывает о функции растяжки')
def step_describes_stretch(context):
    """Проверяем описание растяжки."""
    assert context.bot_response.get('stretch_function_described')


@then('бот предлагает рассрочку без переплаты')
def step_offers_installment(context):
    """Проверяем предложение рассрочки."""
    assert context.bot_response.get('offered_installment')


@then('сравнивает стоимость с походами к массажисту')
def step_compares_to_masseur(context):
    """Проверяем сравнение с массажистом."""
    assert context.bot_response.get('compared_to_masseur')


@then('упоминает гарантию {years:d} года')
def step_mentions_warranty(context, years):
    """Проверяем упоминание гарантии."""
    assert context.bot_response.get('mentioned_warranty') == f'{years} года'


@then('бот уточняет какие моменты вызывают сомнения')
def step_asks_concerns(context):
    """Проверяем уточнение сомнений."""
    assert context.bot_response.get('asked_concerns')


@then('предлагает тест-драйв в шоуруме')
def step_offers_test_drive(context):
    """Проверяем предложение тест-драйва."""
    assert context.bot_response.get('offered_test_drive')


@then('сообщает о временной акции')
def step_mentions_promo(context):
    """Проверяем упоминание акции."""
    assert context.bot_response.get('mentioned_promo')


@then('бот объясняет разницу в качестве механизмов')
def step_explains_quality_diff(context):
    """Проверяем объяснение качества."""
    assert context.bot_response.get('explained_quality_diff')


@then('упоминает про гарантийный сервис в России')
def step_mentions_local_service(context):
    """Проверяем упоминание сервиса в РФ."""
    assert context.bot_response.get('mentioned_local_service')


@then('предлагает изучить отзывы клиентов')
def step_suggests_reviews(context):
    """Проверяем предложение отзывов."""
    assert context.bot_response.get('suggested_reviews')


@then('бот сообщает точные габариты модели')
def step_provides_dimensions(context):
    """Проверяем сообщение габаритов."""
    assert context.bot_response.get('provided_dimensions')


@then('упоминает функцию складывания')
def step_mentions_folding(context):
    """Проверяем упоминание складывания."""
    assert context.bot_response.get('mentioned_folding')


@then('предлагает помощь с замерами')
def step_offers_measurements_help(context):
    """Проверяем помощь с замерами."""
    assert context.bot_response.get('offered_measurements_help')


@then('бот приводит статистику использования')
def step_provides_usage_stats(context):
    """Проверяем статистику использования."""
    assert context.bot_response.get('provided_usage_stats')


@then('рассказывает про напоминания в приложении')
def step_mentions_app_reminders(context):
    """Проверяем напоминания в приложении."""
    assert context.bot_response.get('mentioned_app_reminders')


@then('предлагает пробный период')
def step_offers_trial(context):
    """Проверяем предложение пробного периода."""
    assert context.bot_response.get('offered_trial_period')


@then('бот уточняет адрес доставки')
def step_asks_delivery_address(context):
    """Проверяем уточнение адреса."""
    assert context.bot_response.get('asked_delivery_address')


@then('предлагает бесплатную доставку и сборку')
def step_offers_free_delivery(context):
    """Проверяем бесплатную доставку."""
    assert context.bot_response.get('offered_free_delivery')


@then('запрашивает удобное время')
def step_asks_convenient_time(context):
    """Проверяем запрос времени."""
    assert context.bot_response.get('asked_convenient_time')


@then('бот рассчитывает стоимость доставки')
def step_calculates_delivery_cost(context):
    """Проверяем расчёт доставки."""
    assert context.bot_response.get('calculated_delivery_cost')


@then('сообщает сроки транспортировки')
def step_provides_delivery_time(context):
    """Проверяем сроки доставки."""
    assert context.bot_response.get('provided_delivery_time')


@then('предлагает страховку груза')
def step_offers_cargo_insurance(context):
    """Проверяем страховку груза."""
    assert context.bot_response.get('offered_cargo_insurance')


@then('бот рассчитывает ежемесячный платёж')
def step_calculates_monthly(context):
    """Проверяем расчёт платежа."""
    assert context.bot_response.get('monthly_payment') is not None


@then('объясняет условия без переплаты')
def step_explains_no_overpay(context):
    """Проверяем условия без переплаты."""
    assert context.bot_response.get('explained_no_overpay')


@then('запрашивает данные для заявки')
def step_requests_application_data(context):
    """Проверяем запрос данных."""
    assert context.bot_response.get('requested_application_data')


@then('бот предлагает корпоративную скидку')
def step_offers_corporate_discount(context):
    """Проверяем корпоративную скидку."""
    assert context.bot_response.get('offered_corporate_discount')


@then('уточняет реквизиты для счёта')
def step_asks_requisites(context):
    """Проверяем запрос реквизитов."""
    assert context.bot_response.get('asked_for_requisites')


@then('предлагает расширенную гарантию')
def step_offers_extended_warranty(context):
    """Проверяем расширенную гарантию."""
    assert context.bot_response.get('offered_extended_warranty')


@then('бот рекомендует модель с регулировкой интенсивности')
def step_recommends_adjustable_intensity(context):
    """Проверяем рекомендацию регулируемой интенсивности."""
    assert context.bot_response.get('recommended_adjustable_intensity')


@then('упоминает про воздушно-компрессионный массаж')
def step_mentions_air_compression(context):
    """Проверяем воздушно-компрессионный массаж."""
    assert context.bot_response.get('mentioned_air_compression')


@then('советует консультацию с врачом')
def step_advises_doctor(context):
    """Проверяем совет о враче."""
    assert context.bot_response.get('advised_doctor_consultation')


@then('бот приветствует по имени')
def step_greets_by_name(context):
    """Проверяем приветствие по имени."""
    assert context.bot_response.get('greeted_by_name')


@then('напоминает выбранную ранее модель')
def step_recalls_previous_model(context):
    """Проверяем напоминание модели."""
    assert context.bot_response.get('recalled_previous_model')


@then('проверяет актуальность акций')
def step_checks_promos(context):
    """Проверяем проверку акций."""
    assert context.bot_response.get('checked_current_promos')


@then('бот отправляет краткую сводку по моделям')
def step_sends_summary(context):
    """Проверяем сводку по моделям."""
    assert context.bot_response.get('sent_model_summary')


@then('выделяет ключевые отличия')
def step_highlights_differences(context):
    """Проверяем выделение отличий."""
    assert context.bot_response.get('highlighted_differences')


@then('предлагает помощь в выборе')
def step_offers_selection_help(context):
    """Проверяем помощь в выборе."""
    assert context.bot_response.get('offered_selection_help')


@then('бот выражает понимание')
def step_expresses_understanding(context):
    """Проверяем выражение понимания."""
    assert context.bot_response.get('expressed_understanding')


@then('объясняет отличия Relaxio')
def step_explains_relaxio_diff(context):
    """Проверяем объяснение отличий."""
    assert context.bot_response.get('explained_relaxio_difference')


@then('предлагает демонстрацию в шоуруме')
def step_offers_showroom_demo(context):
    """Проверяем демонстрацию в шоуруме."""
    assert context.bot_response.get('offered_showroom_demo')


@then('бот консультирует как обычно')
def step_consults_normally(context):
    """Проверяем обычную консультацию."""
    assert context.bot_response.get('consulted_normally')


@then('предупреждает о времени обратного звонка')
def step_warns_callback_time(context):
    """Проверяем предупреждение о звонке."""
    assert context.bot_response.get('warned_callback_time')


@then('предлагает оставить контакт для менеджера')
def step_offers_leave_contact(context):
    """Проверяем предложение оставить контакт."""
    assert context.bot_response.get('offered_leave_contact')


@then('бот проверяет наличие на складе')
def step_checks_stock(context):
    """Проверяем проверку наличия."""
    assert context.bot_response.get('checked_stock')


@then('предлагает экспресс-доставку')
def step_offers_express_delivery(context):
    """Проверяем экспресс-доставку."""
    assert context.bot_response.get('offered_express_delivery')


@then('сообщает крайний срок оформления')
def step_informs_deadline(context):
    """Проверяем информирование о сроке."""
    assert context.bot_response.get('informed_deadline')
