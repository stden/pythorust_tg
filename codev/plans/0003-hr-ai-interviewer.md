# Plan: HR/AI продукт для автоматизации собеседований

> Протокол: SPIDER-SOLO  
> Связанная спецификация: `codev/specs/0003-hr-ai-interviewer.md`  
> Revenue Potential: 200,000 - 600,000 руб/мес

## Цели
- Автоматизировать первичный скрининг кандидатов
- Сократить время HR на 60%+
- Стандартизировать процесс оценки
- Улучшить кандидатский опыт
- Обеспечить GDPR compliance

## Фазы

### Фаза 1: Core Interview Engine (Status: pending)
**Objective**: Создать движок диалога для собеседований

**Dependencies**: None

**Tasks**:
- Спроектировать interview state machine (intro → questions → evaluation → outro)
- Реализовать question bank система (категории: experience, technical, behavioral)
- Создать conversation manager (контекст, history, follow-ups)
- Добавить evaluation framework (scoring по критериям)
- Написать unit tests для state transitions
- Benchmark latency диалога (<2 сек между репликами)

**Success Metrics**:
- State machine работает для всех сценариев
- Question bank поддерживает 100+ вопросов с категоризацией
- Evaluation framework настраиваемый под разные роли
- Test coverage >85%
- Latency <2 сек p95

**Evaluation**: State machine review, question quality check, latency tests

**Commit**: Single commit with interview engine core

---

### Фаза 2: Audio Pipeline (STT + TTS) (Status: pending)
**Objective**: Реализовать голосовой интерфейс

**Dependencies**: Phase 1 (Core Engine)

**Tasks**:
- Интегрировать Speech-to-Text (Whisper или Google Speech API)
- Интегрировать Text-to-Speech (ElevenLabs или Azure TTS)
- Реализовать audio preprocessing (noise reduction, normalization)
- Создать streaming pipeline (real-time processing)
- Добавить voice activity detection (когда кандидат закончил говорить)
- Написать integration tests с тестовыми audio файлами
- Benchmark STT accuracy и TTS naturalness

**Success Metrics**:
- STT accuracy ≥95% WER на тестовых данных
- TTS naturalness rated ≥4.0/5.0
- Audio latency <1 сек (STT + TTS)
- Voice activity detection accuracy ≥90%
- Test coverage >80%

**Evaluation**: Audio quality tests, latency benchmarks, user perception testing

**Commit**: Single commit with audio pipeline

---

### Фаза 3: Video Streaming (WebRTC) (Status: pending)
**Objective**: Добавить видео интерфейс для собеседований

**Dependencies**: Phase 2 (Audio Pipeline)

**Tasks**:
- Настроить WebRTC сервер (Janus/mediasoup)
- Реализовать frontend video interface (React + WebRTC API)
- Добавить video recording и storage
- Создать AI avatar или video feed (статичное изображение + lip-sync опционально)
- Обработать edge cases (low bandwidth, connection drops)
- Написать E2E tests для video flow
- Load test: 20 одновременных видео сессий

**Success Metrics**:
- Video streaming stable (<5% packet loss)
- Connection recovery после network issues
- Recording работает для всех сессий
- Load test: 20 concurrent sessions без деградации
- Test coverage >75%

**Evaluation**: Video quality review, stability tests, load testing

**Commit**: Single commit with video streaming

---

### Фаза 4: Evaluation & Scoring System (Status: pending)
**Objective**: Автоматическая оценка кандидатов

**Dependencies**: Phase 1 (Core Engine)

**Tasks**:
- Создать scoring model (competency-based evaluation)
- Реализовать answer analysis (completeness, relevance, depth)
- Добавить red flags detection (inconsistencies, evasion)
- Создать report generator (summary, scores, recommendation)
- Fine-tune LLM на примерах HR оценок (опционально)
- Написать evaluation accuracy tests (agreement с HR)
- Добавить bias detection и mitigation

**Success Metrics**:
- Evaluation accuracy ≥85% agreement с HR
- Report completeness (все критерии покрыты)
- No bias по защищённым характеристикам
- Report generation <5 мин
- Test coverage >85%

**Evaluation**: HR validation tests, bias audit, report quality review

**Commit**: Single commit with evaluation system

---

### Фаза 5: ATS Integration (Status: pending)
**Objective**: Интеграция с Applicant Tracking Systems

**Dependencies**: Phase 4 (Evaluation System)

**Tasks**:
- Реализовать ATS connectors (Greenhouse, Lever, BambooHR)
- Создать bidirectional sync (pull candidates, push results)
- Добавить webhook support для real-time updates
- Реализовать error handling и retry logic
- Написать integration tests с mock ATS APIs
- Документировать setup для разных ATS систем

**Success Metrics**:
- Integration работает для топ-3 ATS (Greenhouse, Lever, BambooHR)
- Sync latency <1 мин после завершения интервью
- Error recovery работает (retry до 3х раз)
- Test coverage >80%

**Evaluation**: Integration tests with real ATS sandboxes, sync timing tests

**Commit**: Single commit with ATS integration

---

### Фаза 6: Candidate Portal (Status: pending)
**Objective**: Интерфейс для кандидатов

**Dependencies**: Phase 3 (Video Streaming)

**Tasks**:
- Создать candidate portal (scheduling, interview start, feedback)
- Реализовать authentication (magic link, одноразовый token)
- Добавить pre-interview instructions и system check (mic, camera)
- Создать post-interview feedback form
- Реализовать rescheduling и resume functionality
- Написать E2E tests для candidate journey
- UX testing с реальными пользователями

**Success Metrics**:
- Candidate satisfaction ≥4.0/5.0
- System check успешен для ≥95% кандидатов
- Drop-off rate <10% после начала интервью
- Rescheduling работает корректно
- Test coverage >80%

**Evaluation**: UX testing feedback, drop-off analysis, E2E tests

**Commit**: Single commit with candidate portal

---

### Фаза 7: GDPR Compliance & Data Security (Status: pending)
**Objective**: Обеспечить соответствие GDPR и безопасность данных

**Dependencies**: Phase 5 (ATS Integration), Phase 6 (Candidate Portal)

**Tasks**:
- Реализовать consent management (explicit opt-in для записи)
- Создать data retention policy и auto-deletion
- Добавить right to deletion (candidate request handling)
- Шифрование данных (at rest and in transit)
- Реализовать access control и audit logs
- Провести security audit и penetration testing
- Подготовить GDPR documentation (privacy policy, DPA templates)

**Success Metrics**:
- All GDPR requirements implemented
- Data encryption verified
- Audit logs for all data access
- Penetration testing passed
- Legal review approved

**Evaluation**: Legal compliance review, security audit, penetration testing report

**Commit**: Single commit with GDPR compliance features

---

### Фаза 8: HR Dashboard & Analytics (Status: pending)
**Objective**: Инструменты для HR команды

**Dependencies**: Phase 4 (Evaluation System)

**Tasks**:
- Создать HR dashboard (pending interviews, completed, metrics)
- Реализовать candidate comparison view
- Добавить analytics (pass rate, time saved, evaluation accuracy)
- Создать export functionality (reports в PDF/CSV)
- Реализовать interview review (playback, transcript, notes)
- Написать E2E tests для HR workflow
- UX testing с HR пользователями

**Success Metrics**:
- Dashboard отображает все ключевые метрики
- HR workflow efficient (navigation, filters, search)
- Export работает корректно
- Interview review удобен для анализа
- HR satisfaction ≥4.5/5.0

**Evaluation**: HR user testing, dashboard functionality review, workflow efficiency

**Commit**: Single commit with HR dashboard

---

### Фаза 9: Pilot Launch (Status: pending)
**Objective**: Пилот с 2-3 клиентами

**Dependencies**: Phase 8 (All core features ready)

**Tasks**:
- Onboard 2-3 pilot клиентов
- Настроить их ATS integration и competency frameworks
- Провести 50-100 собеседований per клиент
- Собрать feedback от HR и кандидатов
- Измерить ключевые метрики (accuracy, time saved, satisfaction)
- Провести weekly review с каждым клиентом
- Итерировать на основе feedback

**Success Metrics**:
- 50-100 интервью per pilot client completed
- HR time saved ≥60%
- Candidate satisfaction ≥4.0/5.0
- Evaluation accuracy ≥85%
- Pilot clients satisfied и готовы к full rollout

**Evaluation**: Pilot metrics review, client feedback, iteration plan

**Commit**: Single commit with pilot results and improvements

---

### Фаза 10: Scale & Productize (Status: pending)
**Objective**: Подготовка к масштабированию

**Dependencies**: Phase 9 (Pilot Success)

**Tasks**:
- Оптимизировать infrastructure для 100+ concurrent interviews
- Создать self-service onboarding для новых клиентов
- Реализовать billing system (pay-per-interview, subscriptions)
- Добавить multi-language support (EN, RU, ...)
- Создать knowledge base и support system
- Запустить marketing campaign
- Подготовить sales materials и demos

**Success Metrics**:
- Infrastructure handles 100+ concurrent interviews
- Self-service onboarding works end-to-end
- Billing system operational
- 10+ paying clients onboarded
- Monthly revenue >200,000 руб

**Evaluation**: Scale tests, onboarding funnel analysis, revenue tracking

**Commit**: Single commit with productization features

---

## Открытые вопросы
- [ ] Приоритет формата: audio-first или video-first MVP?
- [ ] Выбор ATS для первой интеграции (Greenhouse recommended)
- [ ] STT/TTS провайдер (cost vs quality trade-off)
- [ ] Facial analysis - нужен ли? (этические вопросы)
- [ ] Multi-language priority (RU+EN достаточно для старта?)

## Success Metrics (Overall)
- HR time saved ≥60%
- Candidate satisfaction ≥4.0/5.0
- Evaluation accuracy ≥85% agreement с HR
- 100-300 interviews/month
- Monthly revenue 200,000-600,000 руб

## Risks & Mitigation
- **Риск**: Poor candidate experience → **Mitigation**: Extensive UX testing, fallback options
- **Риск**: Bias in evaluation → **Mitigation**: Bias audits, diverse training data
- **Риск**: GDPR violations → **Mitigation**: Legal review, proper consent flow
- **Риск**: Low adoption → **Mitigation**: Pilot with friendly clients, iterate fast

## Артефакты
- Спецификация: `codev/specs/0003-hr-ai-interviewer.md`
- План: `codev/plans/0003-hr-ai-interviewer.md` (текущий документ)
- Ревью: `codev/reviews/0003-hr-ai-interviewer.md` (создать после завершения)

## Timeline Note
⚠️ **NO TIME ESTIMATES** - Фокус на deliverables, не на календарные сроки.
