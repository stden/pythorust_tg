Feature: Credit Expert Bot Logic
  As a user with debt issues
  I want to get advice from a credit expert
  So that I can solve my financial problems

  Background:
    Given the database is initialized
    And the AI service is mocked

  Scenario: Initial greeting
    When the user sends "/start"
    Then the bot should save the user
    And the bot should create a new session
    And the bot should reply with "Здравствуйте! Я Дарья, кредитный эксперт. Вижу, что обратились по вопросу долгов. Помогу разобраться. Как к вам обращаться?"

  Scenario: Qualification flow - User provides name
    Given a user "User123" exists
    And the user has started a conversation
    When the user sends "Меня зовут Иван"
    Then the bot should reply with text containing "Иван, подскажите, вы уже решили заниматься вопросом с долгами или пока изучаете варианты?"

  Scenario: Qualification flow - User is studying options
    Given a user "User123" exists
    And the conversation history contains:
      | role      | content |
      | assistant | Иван, подскажите, вы уже решили заниматься вопросом с долгами или пока изучаете варианты? |
    When the user sends "Пока изучаю"
    Then the bot should reply with text containing "Какая ситуация с долгами?"

  Scenario: Transition to call
    Given a user "User123" exists
    And the conversation history contains:
      | role      | content |
      | user      | Долг 800 тысяч, просрочки есть |
      | assistant | Понимаю, тяжело. Коллекторы звонят? |
    When the user sends "Да, звонят"
    Then the bot should reply with text containing "предлагаю созвониться"
    And the bot should reply with text containing "10-15 минут"

  Scenario: Objection - How much does it cost
    Given a user "User123" exists
    When the user sends "Сколько стоит?"
    Then the bot should reply with text containing "Стоимость зависит от вашей ситуации"
    And the bot should reply with text containing "Консультация бесплатная"

  Scenario: Objection - I need to think
    Given a user "User123" exists
    When the user sends "Мне нужно подумать"
    Then the bot should reply with text containing "пока думаете — долг растет"
    And the bot should reply with text containing "Давайте просто созвонимся"

  Scenario: Objection - Can we do this in chat
    Given a user "User123" exists
    When the user sends "Можно в переписке?"
    Then the bot should reply with text containing "за 10 минут разговора разберем то, на что в переписке уйдет час"
