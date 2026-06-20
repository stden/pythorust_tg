-- Create viral questions database schema
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    expected_reactions_min INTEGER,
    expected_reactions_max INTEGER,
    expected_comments INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS viral_questions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_id INTEGER NOT NULL,
    question_text TEXT NOT NULL,
    status TEXT CHECK(status IN ('draft', 'active', 'sent', 'archived')) DEFAULT 'active',
    priority INTEGER DEFAULT 0,
    tags TEXT,  -- JSON array of tags
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    sent_at TIMESTAMP,
    reactions_received INTEGER DEFAULT 0,
    comments_received INTEGER DEFAULT 0,
    FOREIGN KEY (chat_id) REFERENCES chats(id)
);

CREATE INDEX IF NOT EXISTS idx_viral_questions_chat_id ON viral_questions(chat_id);
CREATE INDEX IF NOT EXISTS idx_viral_questions_status ON viral_questions(status);
CREATE INDEX IF NOT EXISTS idx_viral_questions_priority ON viral_questions(priority DESC);

-- Insert default chats
INSERT OR IGNORE INTO chats (name, description, expected_reactions_min, expected_reactions_max, expected_comments)
VALUES
    ('Golang GO', 'Go programming community', 60, 120, 40),
    ('вайбкодеры', 'Vibe coders community', 50, 100, 30),
    ('Хара', 'Spiritual community', 30, 60, 20);

-- Insert current viral questions
INSERT INTO viral_questions (chat_id, question_text, status, priority)
SELECT
    c.id,
    'Реально ли попасть в Яндекс/Авито на Go без олимпиадных регалий в 2025?

Или там только ICPC финалисты?

Кто проходил собесы недавно — что спрашивали, сколько этапов, какие алгоритмы?',
    'active',
    1
FROM chats c WHERE c.name = 'Golang GO';

INSERT INTO viral_questions (chat_id, question_text, status, priority)
SELECT
    c.id,
    'Claude Haiku 4.5 vs GPT-4.5-mini: кто реально выиграл?

Anthropic говорят что "лучше всех на рынке", OpenAI молчит. Кто тестил обе модели на реальных задачах (не бенчмарки)? Поделитесь примерами где одна слила другую.',
    'active',
    1
FROM chats c WHERE c.name = 'вайбкодеры';

INSERT INTO viral_questions (chat_id, question_text, status, priority)
SELECT
    c.id,
    'Какая самая безумная синхрония случалась в вашей жизни?

У меня: читала книгу про лотерею → получила ''случайные'' числа → поставила → выиграла ровно столько, сколько нужно было на книги.

Поделитесь своими историями 🙏✨',
    'active',
    1
FROM chats c WHERE c.name = 'Хара';
