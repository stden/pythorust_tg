# Specification: HR/AI продукт для автоматизации собеседований

## Metadata
- **ID**: 0003-hr-ai-interviewer
- **Status**: draft
- **Created**: 2025-11-23
- **Revenue Potential**: 200,000 - 600,000 руб/мес

## Problem Statement
HR-отделы и рекрутеры тратят огромное количество времени на первичные собеседования с кандидатами, многие из которых не проходят базовый фильтр. Процесс скрининга неэффективен, субъективен и не масштабируется. Компаниям нужен инструмент для автоматизации первичного отбора с сохранением качества оценки и кандидатского опыта.

## Current State
- Ручные первичные собеседования занимают 30-60 мин на кандидата
- HR тратит 70% времени на отсев неподходящих кандидатов
- Нет стандартизации вопросов и оценки
- Субъективность в оценке (зависит от настроения рекрутера)
- Нет записи и анализа собеседований
- Сложно масштабировать при высоком потоке кандидатов

## Desired State
- AI-агент проводит первичное видео/аудио собеседование (15-20 мин)
- Стандартизированные вопросы по компетенциям и опыту
- Автоматическая оценка по заданным критериям
- Запись и транскрипция всех собеседований
- Детальный отчёт для HR с рекомендацией (пройти/отказ)
- Интеграция с ATS (Applicant Tracking System)
- Кандидатский опыт: дружелюбный AI, гибкое расписание, быстрая обратная связь

## Stakeholders
- **Primary Users**: HR-менеджеры, рекрутеры, кандидаты
- **Secondary Users**: Hiring managers (получают filtered кандидатов)
- **Technical Team**: DevOps, ML engineer, backend разработчик
- **Business Owners**: Head of HR, CEO (ROI на найм)

## Success Criteria
- [ ] Pass-through rate оптимизирован (40-60% false positive допустимо, 0% false negative)
- [ ] Время собеседования 15-20 мин
- [ ] Candidate satisfaction ≥4.0/5.0
- [ ] HR time saved ≥60%
- [ ] Evaluation accuracy ≥85% согласия с HR оценкой
- [ ] Audio/video quality acceptable (четкая речь, минимум артефактов)
- [ ] ATS integration работает (автоматическое обновление статуса)
- [ ] All tests pass with >80% coverage
- [ ] GDPR compliance (хранение персональных данных)

## Constraints
### Technical Constraints
- Video/audio streaming с low latency (<2 сек round-trip)
- Speech-to-text для точной транскрипции (Whisper/Google Speech API)
- Text-to-speech для natural voice (ElevenLabs/Azure TTS)
- Video processing для facial expression analysis (опционально)
- ATS integration (Greenhouse/Lever/BambooHR/...)
- Storage для записей собеседований (GDPR-compliant)

### Business Constraints
- GDPR/CCPA compliance для персональных данных
- Согласие кандидата на запись (обязательно)
- Нет дискриминации по защищённым характеристикам
- Прозрачность критериев оценки
- Возможность appeal (обжалование решения)

## Assumptions
- Кандидаты согласны на AI собеседование
- Интернет-соединение кандидатов достаточное для видео
- Компания предоставляет job descriptions и критерии оценки
- ATS API доступен для интеграции
- HR готовы обучить систему на примерах (fine-tuning)

## Solution Approaches

### Approach 1: Text-based Interview (Chat)
**Description**: Собеседование в текстовом чате, без аудио/видео. Быстрый старт.

**Pros**:
- Проще реализация (нет STT/TTS)
- Ниже latency
- Работает даже при плохом интернете
- Дешевле инфраструктура

**Cons**:
- Хуже кандидатский опыт
- Нет оценки коммуникативных навыков
- Проще обмануть (ChatGPT помощь)

**Estimated Complexity**: Low  
**Risk Level**: Low

### Approach 2: Audio-only Interview (Voice)
**Description**: Голосовое собеседование через телефон/VoIP. Средний вариант.

**Pros**:
- Оценка коммуникативных навыков
- Естественнее для кандидатов
- Дешевле чем видео

**Cons**:
- Нет визуального контакта
- STT/TTS latency критичен
- Требует SIP интеграцию

**Estimated Complexity**: Medium  
**Risk Level**: Medium

### Approach 3: Video Interview (Рекомендуется)
**Description**: Полноценное видео собеседование с AI. Best candidate experience.

**Pros**:
- Максимальная эмуляция реального собеседования
- Можно анализировать body language (опционально)
- Лучший кандидатский опыт
- Визуальный контакт повышает доверие

**Cons**:
- Сложнее реализация (WebRTC, streaming)
- Требует хороший интернет у кандидата
- Выше инфраструктурные затраты

**Estimated Complexity**: High  
**Risk Level**: Medium

### Approach 4: Hybrid (Гибкий выбор)
**Description**: Кандидат выбирает формат (text/audio/video).

**Pros**:
- Максимальная гибкость
- Подходит для разных ситуаций
- Не теряем кандидатов из-за технических проблем

**Cons**:
- Самая сложная реализация
- Сложнее стандартизировать оценку

**Estimated Complexity**: Very High  
**Risk Level**: Medium

## Open Questions

### Critical (Blocks Progress)
- [ ] Формат собеседования (text/audio/video/hybrid)
- [ ] ATS система клиента (Greenhouse/Lever/...)
- [ ] Критерии оценки кандидатов (competency framework)
- [ ] GDPR compliance requirements (где хранить данные, retention policy)

### Important (Affects Design)
- [ ] Многоязычность (RU/EN/...)
- [ ] Facial expression analysis (нужен ли, этичность)
- [ ] Длительность собеседования (15/20/30 мин)
- [ ] Типы вопросов (behavioral/technical/case studies)

### Nice-to-Know (Optimization)
- [ ] A/B тестирование разных interview flows
- [ ] Персонализация вопросов на основе резюме
- [ ] Real-time подсказки для HR (live assistance mode)

## Performance Requirements
- **Audio/Video Latency**: <2 сек round-trip (question → answer → next question)
- **Speech Recognition Accuracy**: ≥95% WER (Word Error Rate)
- **Interview Duration**: 15-20 мин в среднем
- **Concurrent Interviews**: 20-50 одновременных сессий
- **Report Generation**: <5 мин после завершения
- **Availability**: 99% (кандидаты могут выбирать удобное время)

## Security Considerations
- **GDPR Compliance**: 
  - Explicit consent для записи
  - Right to deletion (удаление по запросу)
  - Data minimization (только необходимые данные)
  - Encryption at rest and in transit
- **Authentication**: Secure link для каждого кандидата (одноразовый token)
- **Recording Storage**: Encrypted storage, access control
- **Bias Prevention**: Audit logs, no protected characteristic analysis
- **API Security**: Rate limiting, authentication для ATS integration

## Test Scenarios
### Functional Tests
1. Кандидат проходит собеседование (happy path) → отчёт с оценкой и рекомендацией
2. Кандидат отвечает неполно → follow-up вопросы от AI
3. Технические проблемы (плохой звук) → fallback на текст или rescheduling
4. Кандидат хочет прервать → graceful exit, возможность возобновить
5. ATS integration → автоматическое обновление статуса после собеседования

### Non-Functional Tests
1. Latency test: 20 одновременных видео интервью → p95 <2 сек
2. Speech recognition: тестовые записи → accuracy ≥95%
3. GDPR test: запрос удаления → данные удалены в течение 24 часов
4. Bias test: одинаковые ответы разных голосов → одинаковая оценка

## Dependencies
- **External Services**: 
  - Speech-to-text (Whisper/Google Speech/Azure)
  - Text-to-speech (ElevenLabs/Azure TTS/Coqui)
  - Video streaming (WebRTC/Agora/Twilio)
  - ATS API (Greenhouse/Lever/BambooHR)
- **Internal Systems**: 
  - Question bank и evaluation framework
  - Candidate database
  - Recording storage
- **Libraries/Frameworks**: 
  - WebRTC (video), SIP (audio)
  - openai/anthropic (LLM для диалога)
  - whisper/google-cloud-speech (STT)
  - elevenlabs/azure-tts (TTS)
  - pytest, selenium (testing)

## Revenue Model
### Pricing
- **Pay-per-interview**: 500 - 2,000 руб/собеседование
  - Entry level: 500 руб (text-based)
  - Standard: 1,000 руб (audio)
  - Premium: 2,000 руб (video + detailed report)
- **Monthly Subscription** (для крупных компаний):
  - Small: 50,000 руб/мес (до 50 интервью)
  - Medium: 100,000 руб/мес (до 200 интервью)
  - Enterprise: 200,000+ руб/мес (unlimited + custom)

### Target Market
- HR агентства (outsource screening)
- Крупные компании с высоким потоком найма (tech, retail, call centers)
- Стартапы с ограниченным HR (экономия времени)

### Expected Volume
- 100-300 интервью/мес → 50,000 - 300,000 руб/мес
- Enterprise клиенты: 2-3 × 200,000 руб = 400,000 - 600,000 руб/мес

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|-------------|--------|---------------------|
| Плохая кандидатская experience | High | Critical | UX testing, feedback loop, fallback options |
| Bias в оценке | Medium | Critical | Bias audits, no protected characteristics, diverse training data |
| GDPR нарушения | Low | Critical | Legal review, data retention policy, encryption |
| Низкая accuracy оценки | Medium | High | Fine-tuning на примерах, HR feedback loop, continuous improvement |
| Технические проблемы (latency) | Medium | Medium | Infrastructure scaling, CDN, fallback на audio/text |

## References
- `integrations/openai_client.py` - LLM integration
- `integrations/yandex_tts.py` - TTS option
- WebRTC documentation
- ATS API docs (конкретной системы)

## Expert Consultation
N/A (SPIDER-SOLO)

## Approval
- [ ] Technical Lead Review
- [ ] Legal Review (GDPR compliance)
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off

## Notes
- Рекомендуется начать с Approach 2 (Audio) для баланса UX и complexity
- После MVP добавить video (Approach 3)
- Critical: GDPR compliance review перед запуском
- Consider partnership с ATS провайдерами для distribution
