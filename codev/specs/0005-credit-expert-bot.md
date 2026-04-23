# CODEV Spec: Credit Expert Bot

> Protocol: SPIDER-SOLO  
> Status: Draft

## Problem
We need to automate lead qualification for the "Personal Bankruptcy/Debt Relief" niche. The current process requires manual effort to gather basic info, which is inefficient. Goal: collect the client's phone number for a consultation while following a strict script.

## Solution
Telegram bot "Credit Expert" (Daria) that guides the dialogue with a predefined script.

### Functional requirements
1. **Greeting**:
   - Immediate response to the first message.
   - Introduction (Daria, credit expert).
   - Ask for the client's name.

2. **Qualification**:
   - Ask about decision stage (decided/researching).
   - Collect info (situation, delinquencies, collectors, debt amount, creditor types).
   - Empathy after each answer ("I understand, that's hard").

3. **Call booking**:
   - Offer a call to build an action plan.
   - Arguments: free, 10–15 minutes, no obligation.

4. **Objection handling**:
   - Handle typical objections: "Tell me first", "How much does it cost", "I'll think about it", "Where are you located", "Another company", "Can we stay in chat".
   - Always return to suggesting a call.

5. **Integration**:
   - Save user data and chat history to MySQL.
   - Use LLM (OpenAI) to generate replies matching the tone and script.

### Technical design
- **Language**: Python 3.11+
- **Libraries**: `telethon` (Telegram), `openai` (LLM), `pymysql` (DB).
- **Database**: MySQL (tables `bot_users`, `bot_messages`, `bot_sessions`).
- **Architecture**:
  - `CreditExpertBot` manages connection/events.
  - `MySQLLogger` handles persistence.
  - `OpenAIClient` generates replies using a system prompt with the script.

## Success criteria
- Bot follows the script correctly.
- All messages are saved to the DB.
- Tests (BDD) cover main scenarios and objections.
