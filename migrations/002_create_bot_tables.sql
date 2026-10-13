-- Bot persistence schema (MySQL) — single source of truth for all bot tables.
-- Used by: sales_bot, community_game_bot, credit_expert_bot, and the A/B testing
-- analytics (bot_experiments).
--
-- sales_bot self-bootstraps these via MySqlLogger::ensure_tables() at startup.
-- community_game_bot and credit_expert_bot do NOT create tables themselves, so
-- apply this migration before running them:
--   mysql <db> < migrations/002_create_bot_tables.sql

-- Telegram users seen by any bot (id = Telegram user id, set explicitly on insert).
CREATE TABLE IF NOT EXISTS bot_users (
    id BIGINT PRIMARY KEY,
    username VARCHAR(255) NOT NULL DEFAULT '',
    first_name VARCHAR(255) NOT NULL DEFAULT '',
    last_name VARCHAR(255) NOT NULL DEFAULT '',
    language_code VARCHAR(16) NOT NULL DEFAULT '',
    is_premium TINYINT(1) NOT NULL DEFAULT 0,
    is_bot TINYINT(1) NOT NULL DEFAULT 0,
    first_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS bot_sessions (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    bot_name VARCHAR(64) NOT NULL,
    state VARCHAR(32) DEFAULT 'greeting',
    is_active TINYINT(1) DEFAULT TRUE,
    session_start TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    session_end TIMESTAMP NULL,
    KEY idx_session_user (user_id),
    KEY idx_session_bot (bot_name)
);

CREATE TABLE IF NOT EXISTS bot_messages (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    telegram_message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    bot_name VARCHAR(64) NOT NULL,
    direction VARCHAR(16) NOT NULL,
    message_text TEXT,
    reply_to_message_id BIGINT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    KEY idx_msg_user (user_id),
    KEY idx_msg_bot (bot_name)
);

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
);
