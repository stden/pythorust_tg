# AI Prompts Sales Pack - Implementation Plan

**Проект**: Sales Prospecting Autopilot
**Цель**: Создать и продать первый промпт-пак за 14 дней
**Target Revenue**: $5,000 в первый месяц
**Pricing**: $49 за пакет из 5 промптов

---

## 🎯 ДЕНЬ 1-3: RESEARCH & CREATION

### День 1: Исследование рынка

**Задачи**:
- [ ] Изучить top-10 selling prompts на PromptBase
- [ ] Проанализировать конкурентов (pricing, описания, reviews)
- [ ] Определить gap в текущих предложениях
- [ ] Составить список must-have features для Sales Pack

**Deliverable**: Competitive analysis document (Google Doc)

---

### День 2-3: Создание промптов

#### Промпт 1: Cold Email Generator

**Input Parameters**:
```
- Company name: [String]
- Industry: [Dropdown: SaaS, E-commerce, Finance, Healthcare, Manufacturing]
- Pain point: [String, max 100 chars]
- Your product/service: [String, max 50 chars]
- Tone: [Dropdown: Professional, Casual, Urgent, Consultative]
```

**Промпт Template**:
```
You are an expert B2B sales copywriter specializing in cold email outreach. Your task is to write a personalized cold email that follows the SPIN Selling framework (Situation, Problem, Implication, Need-Payoff).

CONTEXT:
- Target Company: {company_name}
- Industry: {industry}
- Pain Point: {pain_point}
- Solution: {product_service}
- Tone: {tone}

REQUIREMENTS:
1. Subject line (max 50 chars, personalized, curiosity-driven)
2. Opening line (reference their company/industry)
3. Problem statement (2-3 sentences showing you understand their pain)
4. Solution hint (1 sentence, don't reveal everything)
5. Soft CTA (ask question or offer value, NOT hard sell)
6. Length: 80-120 words MAX

CONSTRAINTS:
- No buzzwords ("cutting-edge", "revolutionary", "game-changer")
- No fake urgency ("limited time offer")
- No generic flattery ("I love your company")
- Use specific data/insights when possible

OUTPUT FORMAT:
Subject: [Subject line]

Hi {First Name},

[Email body]

[Your name]

Generate 3 variations (A, B, C) with different hooks.
```

**Test Cases** (5 реальных примеров):
1. SaaS HR platform → recruiting agencies
2. E-commerce analytics → Shopify stores
3. Финтех решение → бухгалтерии малого бизнеса
4. Healthcare CRM → частные клиники
5. Manufacturing automation → заводы

**Expected Output Quality**:
- ✅ Персонализация: упоминание индустрии/болей
- ✅ Короткость: 80-120 слов
- ✅ CTA: вопрос или value offer
- ✅ 3 варианта с разными hooks

---

#### Промпт 2: LinkedIn Outreach Template

**Input Parameters**:
```
- Prospect name: [String]
- Prospect title: [String]
- Prospect company: [String]
- Connection reason: [Dropdown: Mutual connection, Same industry, Event attendee, Content follower]
- Your value proposition: [String, max 100 chars]
```

**Промпт Template**:
```
You are a LinkedIn networking expert. Create a connection request message and 3-message follow-up sequence.

CONTEXT:
- Prospect: {name}, {title} at {company}
- Connection basis: {reason}
- What you offer: {value_prop}

MESSAGE 1 (Connection Request - 300 chars MAX):
Requirements:
- Reference connection reason
- Show genuine interest (NOT selling)
- No generic "I'd love to connect"
- Make it about THEM, not you

MESSAGE 2 (Day 3 after acceptance):
- Thank for connecting
- Share 1 relevant insight/resource (article, tool, data)
- Ask thought-provoking question
- Length: 150 words MAX

MESSAGE 3 (Day 7 - Value Offer):
- Reference their response (or lack of)
- Share specific case study/example
- Soft pitch (offer demo, consultation, resource)
- Clear next step

MESSAGE 4 (Day 14 - Final Touch):
- Acknowledge if no response
- Leave door open
- Provide alternative (follow on Twitter, join newsletter, etc.)

OUTPUT:
[Connection Request]
[Message 2]
[Message 3]
[Message 4]

Include [PERSONALIZATION TAGS] where rep should customize.
```

---

#### Промпт 3: Sales Battlecard Creator

**Input Parameters**:
```
- Your product: [String]
- Top 3 features: [List]
- Competitor name: [String]
- Competitor strengths: [List, max 3]
```

**Промпт Template**:
```
Create a sales battlecard to help reps position {product} against {competitor}.

BATTLECARD STRUCTURE:

1. QUICK FACTS
- Our positioning statement (1 sentence)
- Their positioning statement (1 sentence)
- Market segment overlap

2. FEATURE COMPARISON
| Feature | Us | Them | Talking Point |
|---------|----|----|--------------|
[Generate comparison table]

3. OUR STRENGTHS (Top 3)
For each:
- What it is
- Why it matters to customer
- Proof point (data, testimonial, case study)
- How to position in conversation

4. THEIR STRENGTHS (Top 3)
For each:
- What they claim
- How to reframe (don't bash, redirect to our value)
- Trap questions to ask prospect
- When to concede (if true)

5. OBJECTION PLAYBOOK
Common objections when losing to {competitor}:
- "They're cheaper"
  Response: [TCO angle, value justification]
- "They have more features"
  Response: [Feature bloat angle, focus on outcomes]
- "We already use them"
  Response: [Migration support, dual-run, integration]

6. CLOSING STRATEGIES
- Best-fit customer profile (when we win)
- Red flags (when they're better fit)
- Competitive deal tactics

OUTPUT: Structured markdown document, 1-2 pages MAX.
```

---

#### Промпт 4: Objection Handler

**Input Parameters**:
```
- Objection type: [Dropdown: Price, Timing, Authority, Need, Competition]
- Specific objection: [String]
- Your product context: [String]
- Deal stage: [Dropdown: Discovery, Demo, Proposal, Negotiation]
```

**Промпт Template**:
```
You are a sales objection handling coach. Generate 3 response approaches for this objection.

OBJECTION: {objection}
CONTEXT: {product}, {deal_stage}

For each approach, provide:

APPROACH 1: EMOTIONAL
- Acknowledge feeling
- Empathize
- Reframe around outcome
- Question to uncover real concern

APPROACH 2: LOGICAL
- Use data/proof
- ROI calculation
- Risk vs. reward analysis
- Comparison to status quo cost

APPROACH 3: SOCIAL PROOF
- Reference similar customer
- Industry trend/stat
- FOMO angle (what competitors do)
- Success story

RESPONSE FRAMEWORK:
1. Acknowledge (never dismiss)
2. Clarify (is this the only concern?)
3. Respond (using one of 3 approaches)
4. Confirm (did that address it?)
5. Advance (next step)

DO NOT:
- Argue or get defensive
- Discount immediately
- Use "but" (use "and" instead)
- Claim they're wrong

SCRIPT FORMAT:
"I hear you're concerned about {objection}.
[Acknowledge + validate]
[Clarify] - 'Is {X} the main thing holding you back, or is there something else?'
[Response]
[Confirm] - 'Does that make sense? What else would you need to see?'
[Advance] - '[Next step]'"
```

---

#### Промпт 5: Closing Script Generator

**Input Parameters**:
```
- Deal size: [Number + Currency]
- Decision maker: [String, title]
- Timeline: [Dropdown: This week, This month, This quarter, Unclear]
- Buying signals: [Checkboxes: Asked about pricing, Requested demo, Involved others, Asked about implementation]
- Concerns mentioned: [Text area]
```

**Промпт Template**:
```
Generate 3 closing scripts based on buying readiness level.

DEAL CONTEXT:
- Value: {deal_size}
- Decision maker: {decision_maker}
- Timeline: {timeline}
- Signals: {buying_signals}
- Concerns: {concerns}

SCRIPT 1: TRIAL CLOSE (Low-Medium Buying Intent)
Purpose: Test readiness without pressure
Format:
- Summary of value discussed
- Trial close question ("On a scale of 1-10, how close are you to making a decision?")
- If < 7: "What would get you to a 10?"
- If 7-9: "What's missing?"
- If 10: Move to Script 2

SCRIPT 2: ASSUMPTIVE CLOSE (High Buying Intent)
Purpose: Assume sale, discuss implementation
Format:
- "It sounds like this is a good fit. Let's talk about getting you started."
- Implementation timeline question
- Team introduction
- Contract send + next steps
- Handle last-minute concerns

SCRIPT 3: NOW OR NEVER (Urgent Timeline)
Purpose: Create urgency (use sparingly, must be genuine)
Format:
- Recap ROI/value
- Mention constraint (pricing change, limited slots, end of quarter, etc.)
- Calculate cost of delay ("Every week you wait costs you $X")
- Direct ask: "Can we move forward today?"
- Silence (don't fill it)

INCLUDE:
- Word-for-word scripts
- Pause points [PAUSE]
- Listening cues [LISTEN]
- Objection responses
- Next steps for each outcome

AVOID:
- Manipulation tactics
- False scarcity
- High pressure
- Burning bridges if they say no
```

---

## 🎯 ДЕНЬ 4-5: TESTING & REFINEMENT

### Тестирование промптов

**Process**:
1. Run каждый промпт 10 раз с разными inputs
2. Оценить output quality (1-10 шкала):
   - Relevance (релевантность ответа)
   - Actionability (можно ли сразу использовать)
   - Personalization (персонализация)
   - Length (оптимальная длина)
   - Tone (соответствие tone запросу)

3. Собрать feedback от 3-5 sales professionals:
   - Поделиться промптами в LinkedIn/Twitter
   - Попросить оценить и дать suggestions
   - Внести правки на основе feedback

4. A/B тестирование:
   - Создать 2 версии каждого промпта
   - Сравнить outputs
   - Выбрать лучшую версию

**Acceptance Criteria**:
- ✅ 8/10+ quality rating
- ✅ 90%+ outputs usable without edits
- ✅ Положительный feedback от тестеров

---

## 🎯 ДЕНЬ 6-7: PACKAGING & LISTING

### Создание продукта на PromptBase

**Listing Components**:

1. **Title** (60 chars max):
   "Sales Prospecting Autopilot: 5 AI Prompts for Cold Outreach"

2. **Subtitle** (120 chars):
   "Generate personalized cold emails, LinkedIn sequences, battlecards, objection responses & closing scripts in seconds"

3. **Description** (Markdown format):
```markdown
# 🎯 Sales Prospecting Autopilot

Stop spending hours crafting sales messages. This pack includes 5 battle-tested AI prompts that generate high-converting sales content in seconds.

## What's Included:

### 1. Cold Email Generator
- 3 variations per run (A/B/C testing ready)
- SPIN Selling framework
- Industry-specific personalization
- 80-120 word sweet spot
- NO buzzwords or spam triggers

**Use case**: First outreach to cold prospects
**Output**: 3 email variations + subject lines

---

### 2. LinkedIn Outreach Sequence
- Connection request (300 chars)
- 3-message follow-up sequence
- Value-first approach (no hard selling)
- Personalization tags included

**Use case**: Building LinkedIn pipeline
**Output**: 4-message sequence ready to send

---

### 3. Sales Battlecard Creator
- Feature comparison table
- Competitive positioning
- Objection handling playbook
- Strengths/weaknesses analysis

**Use case**: Competing against specific rival
**Output**: 1-2 page battlecard (markdown)

---

### 4. Objection Handler
- 3 response approaches (Emotional, Logical, Social Proof)
- 5-step response framework
- Customized by objection type
- Deal stage-specific

**Use case**: Handling "too expensive", "no time", etc.
**Output**: 3 scripted responses

---

### 5. Closing Script Generator
- Trial close (low intent)
- Assumptive close (high intent)
- Urgent close (deadline scenarios)
- Word-for-word scripts with pause cues

**Use case**: Moving deals to signature
**Output**: 3 closing scripts

---

## Who This Is For:

✅ SDRs & BDRs doing cold outreach
✅ Account Executives closing deals
✅ Founders/solopreneurs selling their product
✅ Sales managers training their team
✅ Anyone doing B2B prospecting

## What You Get:

- 5 copy-paste prompts (works with ChatGPT, Claude, Gemini)
- Input parameter guides
- 10+ real-world examples
- Best practices document
- Bonus: Sales email templates

## Results You Can Expect:

📈 3x faster content creation
💰 Higher response rates (personalized = better)
🎯 Consistent messaging across team
⏰ Save 5-10 hours/week on copywriting

## Requirements:

- ChatGPT Plus, Claude Pro, or Gemini Advanced
- Basic understanding of B2B sales
- Your product/service details

---

## FAQ:

**Q: Do these prompts work with free ChatGPT?**
A: Yes, but paid versions (GPT-4, Claude Sonnet) produce better results.

**Q: Can I customize the prompts?**
A: Absolutely! We encourage tweaking for your specific use case.

**Q: Do you provide examples?**
A: Yes, the pack includes 10+ real-world examples with outputs.

**Q: Refund policy?**
A: If prompts don't work as described, full refund within 7 days.

---

## Instant Delivery:

✅ Download immediately after purchase
✅ PDF format (copy-paste ready)
✅ Free updates (we improve prompts based on feedback)

---

**Price: $49** (80% margin after PromptBase fee)

**Satisfaction Guarantee**: If these prompts don't save you time or improve your outreach, email us for a full refund.
```

4. **Category**: Business > Sales & Marketing

5. **Tags**:
   - sales
   - cold email
   - prospecting
   - B2B
   - outreach
   - LinkedIn
   - ChatGPT
   - sales enablement
   - objection handling
   - closing scripts

6. **Pricing**: $49

7. **Demo Output** (include in listing):
   - 1 example cold email
   - 1 LinkedIn connection request
   - 1 objection response

8. **Thumbnail** (Canva):
   - Clean design
   - "Sales Prospecting Autopilot"
   - "5 AI Prompts"
   - "$49"
   - Professional color scheme (blue/white)

---

## 🎯 ДЕНЬ 8-10: MARKETING & LAUNCH

### Pre-Launch (Day 8)

**Build Hype**:
- [ ] Twitter thread previewing prompts (without revealing full prompt)
- [ ] LinkedIn post with free sample (Prompt 1)
- [ ] Reddit post in r/sales, r/Entrepreneur (no spam, provide value first)
- [ ] Email list teaser (if you have one)

**Create Supporting Content**:
- [ ] Blog post: "5 AI Prompts That Generated $50K in Sales Pipeline"
- [ ] YouTube short: Demo of Cold Email Generator
- [ ] Twitter thread: "How I use AI to write all my sales emails"

---

### Launch Day (Day 9)

**Morning**:
- [ ] Publish on PromptBase (8am EST)
- [ ] Tweet launch announcement
- [ ] LinkedIn post with link
- [ ] Reddit "Show HN" / "I made this" posts
- [ ] Email subscribers (if any)

**Afternoon**:
- [ ] DM 50 sales professionals on LinkedIn/Twitter
  - "Hey {name}, just launched a sales prompt pack. Would love your feedback: {link}"
- [ ] Post in relevant Slack/Discord communities
- [ ] Comment on sales/AI posts with link (non-spammy)

**Evening**:
- [ ] Monitor comments/DMs
- [ ] Respond to questions
- [ ] Share early sales wins ("3 sold in first hour!")

---

### Post-Launch (Day 10-14)

**Organic Growth**:
- [ ] Daily Twitter/LinkedIn content:
  - Monday: Share customer testimonial
  - Tuesday: Demo video (Prompt 2)
  - Wednesday: Free tip from pack
  - Thursday: Case study ("How one SDR 3x'd their pipeline")
  - Friday: Weekend sale (20% off)

**Outreach**:
- [ ] Reach out to sales podcasts for interview
- [ ] Write guest post for sales blogs
- [ ] Collaborate with sales influencers (affiliate deal)

**Iterate**:
- [ ] Collect feedback from buyers
- [ ] Update prompts based on reviews
- [ ] Add new prompts (upsell to v2 later)

---

## 🎯 SUCCESS METRICS

### Week 1 Goals:
- [ ] 20 sales (20 × $49 × 0.8 = $784 revenue)
- [ ] 4+ star rating on PromptBase
- [ ] 5+ testimonials/reviews
- [ ] 1,000+ impressions on launch posts

### Month 1 Goals:
- [ ] 100 sales ($3,920 revenue)
- [ ] 50+ Twitter followers from launch
- [ ] 10+ customer testimonials
- [ ] Start work on Pack 2 (CRM Intelligence)

---

## 💰 REVENUE FORECAST

**Pricing**: $49
**PromptBase Commission**: 20%
**Net per sale**: $39.20

**Conservative (Month 1)**:
- 100 sales × $39.20 = **$3,920**

**Optimistic (Month 1)**:
- 200 sales × $39.20 = **$7,840**

**Month 2-6 (with Pack 2, 3, 4)**:
- Pack 1: 50 sales/month × $39.20 = $1,960
- Pack 2: 30 sales/month × $79.20 = $2,376
- Pack 3: 20 sales/month × $239.20 = $4,784
- **Total**: $9,120/month

**6-Month Total**: $3,920 + (5 × $9,120) = **$49,520**

---

## 🚀 NEXT STEPS

After Pack 1 success:
1. **Pack 2: CRM Intelligence Hub** ($99)
2. **Pack 3: Investor Fundraising Kit** ($299)
3. **Pack 4: Content Creation Engine** ($39)
4. **Pack 5: HR Recruiting Toolkit** ($79)

**Bundle Strategy** (Month 3):
- All 5 packs: $449 → Sale price $299 (save $150)

---

## 📋 DELIVERABLES CHECKLIST

- [ ] 5 Prompts (finalized & tested)
- [ ] PDF document (formatted, branded)
- [ ] Examples document (10+ use cases)
- [ ] PromptBase listing (approved)
- [ ] Thumbnail image (Canva)
- [ ] Marketing copy (tweets, posts, email)
- [ ] Demo video (2-3 minutes)
- [ ] Launch strategy (calendar)

---

## ⏱ TIMELINE SUMMARY

| Day | Tasks | Deliverables |
|-----|-------|-------------|
| 1 | Research competitors, define positioning | Competitive analysis |
| 2-3 | Create 5 prompts, test with examples | Prompt templates |
| 4-5 | Get feedback, refine prompts | Final prompts |
| 6-7 | Package product, create listing | PromptBase listing |
| 8 | Pre-launch marketing | Social posts, emails |
| 9 | Launch! | Live on PromptBase |
| 10-14 | Post-launch marketing, iteration | Sales, reviews |

**Total**: 14 days from start to first revenue! 🚀
