# Specification: Neuro Sales Agent (Universal Sales Agent with Learning)

## Metadata
- **ID**: spec-2025-11-23-neuro-sales-agent
- **Status**: draft
- **Created**: 2025-11-23

## Goal & Positioning
Universal sales agent for Telegram (chat-first, voice as an option) that knows inventory/prices/stock, conducts dialogue following sales scripts, self-learns from real conversations/calls, and supports different verticals without manual prompt rewrites.

## Problem Statement
- Sales rely on manual operators: slow response, script inconsistencies, no unified catalog and quality control.
- Scripts diverge between niches, training new operators is expensive; history and contacts are lost, leads are poorly tracked.
- No continuous learning system: feedback from chats doesn't translate into prompt/knowledge improvements, conversion and reaction metrics aren't collected.

## Current State
- Basic auto-responder (`autoanswer.py`) without sales scripts and catalog, without CRM/Sheets lead logging.
- Telegram reading (`read.py`, `tg.py`) without sales triggers and CTAs.
- No voice in production: no SIP connection and stable STT/TTS pipeline (there are prototypes in `integrations/`).
- Catalog and prices are managed manually (sheets/messages), no source of truth and stock verification.

## Desired State
- Chat-first AI salesperson responds in ≤3s p95, initiates offers based on triggers (new subscriber, price question, silence >N min).
- Knows inventory/prices/stock/promotions; does upsell/downsell/bundles; when data is unknown responds "let me check" and escalates.
- Records leads/deals in CRM/Google Sheet with contacts, amount, dialogue link, status; eliminates duplicates.
- Logs and metrics (conversions, response speed, escalations, rejections) are saved and visible to owner.
- Escalation to human for stop-words, empty catalog, response delay, N failed payment/delivery attempts.
- Profile configured per-chat (tone, aggressiveness, language, niche, script), can quickly switch verticals without code rewrite.
- Learning: new scripts/catalog/chat analysis automatically added to knowledge base; offline A/B and auto-eval on reference dialogues available.
- Voice (MVP option): SIP + STT→LLM→TTS, response ≤1.5s p95, unified logic with chats.

## Stakeholders
- **Primary**: owners/operators of sales chats; end buyers.
- **Secondary**: sales/marketing managers (metrics, leads).
- **Tech**: developer/DevOps of the project (`my_telegram` repo).

## Scope
- In scope: sales chatbot with catalog and CTA; lead logging; learning from chat/catalog; per-chat config; knowledge base for scripts; optional voice worker.
- Out of scope (MVP): in-Telegram payments, auto-dialing database without consent, omnichannel (WA/VK) except Telegram/SIP.

## Success Criteria
- [ ] p95 time to first response in chat ≤3s; in voice round-trip ≤1.5s.
- [ ] ≥90% responses contain CTA or explicit next step.
- [ ] 0 hallucinations on price/availability (validation against catalog); when gap exists — "let me check" + escalation.
- [ ] ≥95% leads with purchase intent recorded in CRM/Sheet with dialogue link.
- [ ] Learning: new item/price in catalog appears in responses without restart; offline auto-eval ≥80% pass on test set.
- [ ] Logs/metrics accessible to owner (conversions, speed, escalations, rejection/hallucinations).
- [ ] All bot logic tests pass; load test meets Performance Requirements.

## Constraints
### Technical
- Python 3 + Telethon; LLM via `integrations/openai_client.py` or `integrations/ollama_client.py`.
- Source of truth for catalog: Google Sheet/CSV/Notion API (choose 1, others sync).
- .env for keys; can't log secrets/PII; mask personal data in logs.
- Voice: SIP provider (Zadarma/Exolve/Twilio) + local STT/TTS (Whisper/Silero) for cost control.

### Business
- Compliance with Telegram and SIP provider ToS; no spam or cold outreach.
- Bilingual RU/EN minimum; sales aggressiveness is adjustable.
- Cost: local models preferred with stable quality.

## Assumptions
- Catalog available in structured format and updated ≥1 time/day.
- OpenAI keys or local LLM ready; SIP keys available for testing.
- Access to Google Sheet/CRM for lead recording.
- Auto-responder allowed in target chats; operator available for escalations.

## Functional Requirements
### Core Sales Behavior
- Response ≤3s p95; short replies, always CTA (payment/delivery/contact/next step).
- Per-chat personalization: bot name, tone, language, aggressiveness/urgency, discount limits, niche.
- Triggers: new message/subscription, silence >N minutes, price/availability question, catalog view, abandoned dialogue.
- Escalation: stop-words, missing data, 2+ failed payment/address attempts, long LLM response.

### Catalog & Knowledge
- Integration with single source of truth (Sheet/CSV/Notion); cache + update TTL.
- Fields: SKU/name, category, price, availability/stock, promotions/discounts, bundles for upsell/combos.
- Catalog used for price/availability validation; on mismatch — "let me check" + escalation.
- Script base: greeting, qualification, objections, closing, follow-up. Stored in file/kv-storage, versioned.

### Learning Loop
- Dialogue logs + outcome labels (lead/no, rejection reason, escalation) saved.
- Auto dataset collection: successful/unsuccessful responses, objection handling, unknown SKUs.
- Offline auto-eval on reference dialogues before deploying new prompt/model version.
- Support for A/B prompt profiles; conversion/speed metrics compared.
- Knowledge updates without restart: catalog/script updates pulled via watcher (polling or webhook).

### Lead & CRM Flow
- Lead fields: name/username, contact (phone/username), request/product, amount/currency, status (new/qualified/won/lost/escalated), dialogue/message link.
- Lead deduplication by chat_id + username/phone; status update instead of duplicate.
- Sync to Google Sheet/CRM via service account; retries + dead-letter for failed writes.

### Voice (MVP option)
- SIP inbound/outbound, minimal IVR (greeting → operator → AI).
- STT (Whisper/Silero), text normalization (profanity/noise), LLM response, TTS (Silero/Yandex) → RTP.
- Timeouts and silence barrier; quick escalation to human on stop-words/delay.

### Operations & Admin
- Operator commands: enable/pause chat, change profile/script, test dialogue run, export logs/metrics.
- Feature flags: voice enablement, A/B profiles, auto-escalation, auto-follow-up.

## Non-Functional / Performance
- **Latency**: chat ≤3s p95; voice round-trip ≤1.5s p95.
- **Throughput**: 10 parallel chat dialogues/instance; 3 voice on 12GB GPU.
- **Availability**: 99% uptime; graceful restart without state loss (state in Redis/SQLite).
- **Cost**: limits on tokens/minutes STT/TTS; fallback to local models when budget exceeded.
- **Reliability**: lead write retries; circuit breaker for external APIs; observability (tracing + metrics).

## Data & Integrations
- Catalog: Sheet/CSV/Notion → loader + validation → cache/embeddings (for search).
- CRM/Sheets: service account, retries; error logging without PII.
- Models: OpenAI or local LLM; warming and k-shot template support.
- Log storage: local/SQLite → possible S3/Sheets export; PII masking.

## Architecture Outline
- Telethon listener → Event Router → Dialogue Engine.
- Dialogue Engine: state machine + policy rules (escalations/CTA) + LLM client + prompt templates.
- Knowledge Service: Catalog adapter + Script store + Embedding search (for clarifications and objections).
- Lead Service: CRM/Sheet writer + dedup + retry queue.
- Learning Pipeline: dialogue logger → dataset builder → auto-eval runner → deploy new prompt/profile.
- Voice Worker (option): SIP endpoint → STT → Dialogue Engine → TTS → RTP.

## Observability & Metrics
- Metrics: p50/p95 latency, conversion to lead/payment, escalations, upsell attempts, rejection/hallucinations, CRM/catalog errors.
- Logs: bot input/output (masked contacts), escalation reasons, profile/script changes.
- Alerts: latency degradation, CRM/catalog error growth, empty catalog, LLM/STT/TTS unavailability.

## Test Scenarios
### Functional
1. Price/availability request from catalog → accurate response + CTA for payment/delivery.
2. Unknown SKU → "let me check", escalation, problem logged in learning log.
3. New item added to Sheet → bot uses it without restart.
4. Abandoned dialogue (silence >N min) → follow-up with offer/delivery.
5. Price objection → downsell/discount within limit; if limit exhausted — escalation.
6. Voice: incoming call → STT → LLM → TTS ≤1.5s p95, on stop-word transfer to operator.

### Non-Functional
1. Load: 10 parallel dialogues → p95 ≤3s.
2. Failover: LLM unavailable → backup profile with template responses + escalation.
3. Security: prompt injection doesn't change system prompt; PII doesn't leak to logs/responses.
4. Learning: auto-eval on reference set ≥80% before deploying new profile.

## Open Questions
- Catalog source of truth: Sheet vs CSV vs Notion; format and update frequency.
- SIP provider and pricing (or exclude voice from first version?).
- Multi-language: RU/EN auto-detect or manual flag?
- Discount/upsell policy: strict rules vs LLM decision, limits per day/user.
- Minimum logging volume: full texts vs metadata; where to store long-term.
- Are anti-spam filters/incoming lead scoring required.

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Price/availability hallucinations | Medium | High | Strict catalog validation, "let me check" fallback, escalation. |
| High LLM/STT/TTS latency | Medium | High | Warming, short context, cache, local models, timeouts + fallbacks. |
| Blocking/spam | Low | Medium | Limit chats, follow ToS, anti-spam, manual outbound confirmation. |
| PII leakage | Medium | High | Masking in logs, log ACLs, retention. |
| Learning quality (poor dataset) | Medium | Medium | Dataset cleaning, auto-eval, A/B, manual sample review. |

## References
- `autoanswer.py`, `telegram_session.py`, `chat_export_utils.py` — current Telegram logic.
- `integrations/` — OpenAI/Ollama/tts/stt clients.
- `AGENTS.md`, `DASHA_TOOLS.md`, `ENV_SETUP.md` — general instructions and environment variables.

## Approval
- [ ] Technical Lead Review
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off
