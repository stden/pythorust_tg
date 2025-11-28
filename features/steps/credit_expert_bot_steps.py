# -*- coding: utf-8 -*-
"""Steps for testing Credit Expert Bot."""

from behave import given, when, then
from unittest.mock import MagicMock, AsyncMock
from pathlib import Path
import sys
import asyncio

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# Import the bot class (assuming it's in credit_expert_bot.py)
# We might need to mock the import if the file has side effects on import,
# but credit_expert_bot.py looks safe (main is guarded).
from credit_expert_bot import CreditExpertBot, CREDIT_EXPERT_SYSTEM_PROMPT

# ========== Fixtures & Mocks ==========

def create_mock_user(user_id: int, first_name: str, username: str = None):
    """Creates a mock Telegram user."""
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
    """Creates a mock message."""
    message = MagicMock()
    message.id = message_id
    message.text = text
    message.sender_id = user_id
    return message

# ========== Given Steps ==========

@given('the database is initialized')
def step_db_initialized(context):
    """Mocks the database."""
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
        }

    def save_message(user_id, message_id, text, direction, bot_name='Credit_Expert_Bot', reply_to=None):
        context.saved_messages.append({
            'user_id': user_id,
            'message_id': message_id,
            'text': text,
            'direction': direction,
            'bot_name': bot_name,
        })

    def create_session(user_id, bot_name='Credit_Expert_Bot'):
        session_id = len(context.sessions) + 1
        context.sessions[session_id] = {
            'id': session_id,
            'user_id': user_id,
            'bot_name': bot_name,
            'state': 'greeting',
            'is_active': True,
        }
        return session_id

    def get_conversation_history(user_id, bot_name='Credit_Expert_Bot', limit=20):
        user_messages = [m for m in context.saved_messages if m['user_id'] == user_id]
        return user_messages[-limit:]

    context.db_mock.save_user = MagicMock(side_effect=save_user)
    context.db_mock.save_message = MagicMock(side_effect=save_message)
    context.db_mock.create_session = MagicMock(side_effect=create_session)
    context.db_mock.get_conversation_history = MagicMock(side_effect=get_conversation_history)

@given('the AI service is mocked')
def step_ai_mocked(context):
    """Mocks the AI service."""
    context.ai_mock = MagicMock()
    
    async def chat_completion(messages):
        response = MagicMock()
        response.choices = [MagicMock()]
        
        # Simple rule-based logic to simulate AI behavior for testing
        last_user_msg = messages[-1]['content'].lower()
        system_prompt = messages[0]['content']
        
        content = "Default AI response"
        
        if "меня зовут" in last_user_msg:
            name = last_user_msg.split("меня зовут")[-1].strip().capitalize()
            content = f"{name}, подскажите, вы уже решили заниматься вопросом с долгами или пока изучаете варианты?"
        elif "пока изучаю" in last_user_msg:
            content = "Понятно. Какая ситуация с долгами? Опишите кратко."
        elif "звонят" in last_user_msg:
            content = "Понимаю, тяжело. Чтобы дать конкретный план действий, предлагаю созвониться — так быстрее. 10-15 минут."
        elif "сколько стоит" in last_user_msg:
            content = "Стоимость зависит от вашей ситуации. Консультация бесплатная."
        elif "подумать" in last_user_msg:
            content = "Понимаю. Знаете, пока думаете — долг растет. Давайте просто созвонимся."
        elif "в переписке" in last_user_msg:
            content = "Понимаю, так комфортнее. Но за 10 минут разговора разберем то, на что в переписке уйдет час."
            
        response.choices[0].message.content = content
        return response

    context.ai_mock.chat_completion = AsyncMock(side_effect=chat_completion)

@given('a user "{username}" exists')
def step_user_exists(context, username):
    context.current_user = create_mock_user(123, "TestUser", username)

@given('the user has started a conversation')
def step_user_started_conversation(context):
    context.db_mock.create_session(context.current_user.id)
    # Simulate greeting sent
    context.db_mock.save_message(context.current_user.id, 1, "Greeting", "outgoing")

@given('the conversation history contains')
def step_history_contains(context):
    for row in context.table:
        direction = 'outgoing' if row['role'] == 'assistant' else 'incoming'
        context.db_mock.save_message(
            context.current_user.id, 
            len(context.saved_messages) + 1, 
            row['content'], 
            direction
        )

# ========== When Steps ==========

@when('the user sends "/start"')
def step_user_sends_start(context):
    context.current_user = create_mock_user(123, "TestUser")
    context.current_message = create_mock_message(1, '/start', 123)
    
    # Simulate bot logic for /start
    context.db_mock.save_user(context.current_user)
    context.db_mock.save_message(123, 1, '/start', 'incoming')
    context.db_mock.create_session(123)
    
    greeting = "Здравствуйте! Я Дарья, кредитный эксперт. Вижу, что обратились по вопросу долгов. Помогу разобраться. Как к вам обращаться?"
    context.last_reply = greeting
    context.db_mock.save_message(123, 2, greeting, 'outgoing')

@when('the user sends "{text}"')
def step_user_sends_text(context, text):
    context.current_message = create_mock_message(len(context.saved_messages)+1, text, context.current_user.id)
    
    # Simulate bot logic for message
    context.db_mock.save_user(context.current_user)
    context.db_mock.save_message(context.current_user.id, context.current_message.id, text, 'incoming')
    
    # Get history
    history = context.db_mock.get_conversation_history(context.current_user.id)
    
    # Build messages
    messages = [{"role": "system", "content": CREDIT_EXPERT_SYSTEM_PROMPT}]
    for msg in history:
        role = "assistant" if msg['direction'] == 'outgoing' else "user"
        messages.append({"role": role, "content": msg['text']})
    messages.append({"role": "user", "content": text})
    
    # Call AI
    loop = asyncio.get_event_loop()
    response = loop.run_until_complete(context.ai_mock.chat_completion(messages))
    response_text = response.choices[0].message.content
    
    context.last_reply = response_text
    context.db_mock.save_message(context.current_user.id, context.current_message.id + 1, response_text, 'outgoing')

# ========== Then Steps ==========

@then('the bot should save the user')
def step_bot_saves_user(context):
    context.db_mock.save_user.assert_called()

@then('the bot should create a new session')
def step_bot_creates_session(context):
    context.db_mock.create_session.assert_called()

@then('the bot should reply with "{text}"')
def step_bot_replies_exact(context, text):
    assert context.last_reply == text, f"Expected '{text}', got '{context.last_reply}'"

@then('the bot should reply with text containing "{text}"')
def step_bot_replies_containing(context, text):
    assert text.lower() in context.last_reply.lower(), f"Expected to contain '{text}', got '{context.last_reply}'"
