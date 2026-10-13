# Sales Battlecard Creator - Competitive Intelligence

## Prompt Template

```
You are an expert sales enablement strategist specializing in competitive intelligence. Your task is to create a comprehensive sales battlecard that helps sales reps win deals against specific competitors.

CONTEXT:
- Your Product: {your_product}
- Your Key Differentiators: {differentiators}
- Competitor Name: {competitor_name}
- Competitor Strengths: {competitor_strengths}
- Target Customer: {target_customer}
- Deal Size: {deal_size}

BATTLECARD STRUCTURE:

**1. COMPETITOR OVERVIEW (2-3 sentences)**
- What they do
- Market position
- Typical customer profile

**2. WHY CUSTOMERS CHOOSE THEM (3-4 bullets)**
- Their genuine strengths (be honest)
- What they do well
- Why they win deals

**3. WHY CUSTOMERS LEAVE THEM (3-4 bullets)**
- Common pain points
- Gaps in their offering
- Customer complaints (cite sources when possible)

**4. YOUR ADVANTAGES (4-5 bullets)**
- Specific differentiators with proof points
- Use metrics, case studies, or customer quotes
- Focus on outcomes, not features

**5. TRAP-SETTING QUESTIONS (5-6 questions)**
- Questions that expose competitor's weaknesses
- Frame as "important considerations" not attacks
- Make prospect think critically

**6. OBJECTION HANDLING (3-4 common objections)**
Format: "Objection → Response (with proof)"

**7. COMPETITIVE LANDMINES (2-3 bullets)**
- Topics to avoid (where competitor is genuinely better)
- How to redirect conversation

**8. CLOSING TIPS (2-3 bullets)**
- When to discount (if ever)
- When to walk away
- Ideal positioning

CONSTRAINTS:
- Be honest about competitor strengths (builds trust)
- Never badmouth competitor directly
- Always have proof points (data, quotes, case studies)
- Focus on customer outcomes, not feature comparison
- Keep battlecard to 2 pages MAX (800-1000 words)

OUTPUT FORMAT:
# Sales Battlecard: {Your Product} vs {Competitor}

## Competitor Overview
[2-3 sentences]

## Why Customers Choose Them
- [Strength 1]
- [Strength 2]
- [Strength 3]

## Why Customers Leave Them
- [Pain point 1 with source]
- [Pain point 2 with source]
- [Pain point 3 with source]

## Your Advantages
- **[Differentiator 1]**: [Proof point with metric]
- **[Differentiator 2]**: [Proof point with case study]
- **[Differentiator 3]**: [Proof point with customer quote]

## Trap-Setting Questions
1. "[Question exposing gap 1]"
2. "[Question exposing gap 2]"
3. "[Question exposing gap 3]"

## Objection Handling
**Objection**: "[Common objection]"
**Response**: "[Your response with proof]"

## Competitive Landmines
- ⚠️ [Topic to avoid]
- ⚠️ [Topic to avoid]

## Closing Tips
- ✅ [Best practice 1]
- ✅ [Best practice 2]

Generate 1 comprehensive battlecard.
```

## Input Parameters

| Parameter | Type | Format | Required |
|-----------|------|--------|----------|
| your_product | String | Product/service name | Yes |
| differentiators | String | 3-5 key differentiators (comma-separated) | Yes |
| competitor_name | String | Competitor's name | Yes |
| competitor_strengths | String | Their known strengths (comma-separated) | Yes |
| target_customer | Dropdown | SMB, Mid-Market, Enterprise, Startup | Yes |
| deal_size | Dropdown | <$10K, $10K-$50K, $50K-$250K, $250K+ | Yes |

## Example Usage

### Example 1: CRM Platform vs Salesforce

**Input**:
```
your_product: SimpleCRM
differentiators: Easier onboarding (2 hours vs 2 weeks), 50% lower cost, built-in AI without add-ons, mobile-first design, unlimited users
competitor_name: Salesforce
competitor_strengths: Market leader, extensive integrations, enterprise features, brand recognition
target_customer: SMB
deal_size: $10K-$50K
```

**Output**:

---

# Sales Battlecard: SimpleCRM vs Salesforce

## Competitor Overview

Salesforce is the #1 CRM globally with 23% market share. They dominate enterprise and mid-market segments with extensive customization, 3,000+ integrations, and a massive ecosystem. Typical customers: 100+ employees, complex sales processes, dedicated Salesforce admin.

## Why Customers Choose Them

- **Market leader**: "Nobody gets fired for buying Salesforce" — perceived safety
- **Unlimited customization**: Can build almost anything with their platform
- **Massive ecosystem**: 3,000+ integrations, huge AppExchange marketplace
- **Enterprise features**: Advanced automation, robust security, compliance certifications
- **Brand trust**: Public company, established 1999, unlikely to disappear

## Why Customers Leave Them

- **Complexity overload**: G2 review (2025): *"Took us 6 months to fully onboard. Still discovering features we're paying for but not using."* — VP of Sales, SaaS company (50 employees)
- **Hidden costs**: Base price is $75/user/mo, but AI features (+$50/user), CPQ (+$75/user), advanced analytics (+$100/user) bring real cost to $250-300/user/mo
- **Requires dedicated admin**: 78% of Salesforce customers hire full-time admin ($80K-120K salary) just to maintain the system (Source: Salesforce Admin Salary Survey 2024)
- **Mobile experience**: Rated 3.2/5 stars on iOS App Store vs competitors at 4.5+ — clunky, slow, missing key features

## Your Advantages

- **2-hour onboarding vs 2-week**: SimpleCRM is live in 2 hours with zero training. Customer quote: *"Our sales team was up and running the same day. With Salesforce, we spent $15K on implementation consultants."* — CEO, MarketingTech startup (25 reps)

- **50% lower total cost**: $45/user/mo all-in (including AI, analytics, unlimited users) vs Salesforce's real cost of $250/user/mo. ROI case study: MedTech company saved $180K/year switching from Salesforce (30 users, 3-year contract)

- **Built-in AI (no add-ons)**: Lead scoring, email automation, forecasting included. Salesforce Einstein costs extra $50/user/mo. SimpleCRM customers close deals 23% faster with AI (internal data, 500+ customers)

- **Mobile-first design**: 4.8/5 stars on app stores. 65% of SimpleCRM activity happens on mobile vs 28% for Salesforce (industry avg: 40%)

- **Unlimited users**: Most SMBs add CSM, marketing, support to CRM. SimpleCRM = flat $45/user regardless of role. Salesforce charges same price for "view-only" users.

## Trap-Setting Questions

1. **"How long did your Salesforce implementation take, and what was the total cost including consultants?"**
   *Exposes: Hidden implementation costs ($10K-50K for SMBs)*

2. **"Do you currently have a dedicated Salesforce admin on staff? If not, who manages customizations and updates?"**
   *Exposes: Need for expensive admin headcount*

3. **"What percentage of Salesforce features are you actively using? Are there modules you're paying for but not using?"**
   *Exposes: Over-engineering and waste*

4. **"How often do your reps use Salesforce mobile app? What's their biggest complaint?"**
   *Exposes: Poor mobile UX*

5. **"What's your all-in cost per user including AI, analytics, CPQ, and any add-ons?"**
   *Exposes: Real cost is 3-4x advertised price*

6. **"If you wanted to add 10 more users tomorrow (support team, contractors), how much would that cost and how long to provision?"**
   *Exposes: Per-user pricing friction*

## Objection Handling

**Objection**: "Salesforce has way more integrations than SimpleCRM."
**Response**: "Absolutely true — Salesforce has 3,000+ integrations. Quick question: how many integrations do you actually use today? Most SMBs use 5-10 core tools (Slack, Gmail, Zoom, Stripe, QuickBooks). SimpleCRM has native integrations for all the top 50 tools, plus Zapier for anything else. We've found that companies switch to us *because* Salesforce has too many options — analysis paralysis. What are your must-have integrations? Let me confirm we support them."

---

**Objection**: "We need advanced customization that only Salesforce can provide."
**Response**: "That's a fair concern. Can you walk me through a specific customization you need? [Listen] Here's what we've found: 90% of SMBs need the same core workflows — lead routing, pipeline management, email sequences, reporting. SimpleCRM handles those out-of-the-box with zero config. The other 10% of 'custom' needs are often workarounds for Salesforce's complexity. For example, [Customer X] thought they needed custom objects for product catalog, but SimpleCRM's built-in product library did it natively. Let's map your workflow and see if SimpleCRM fits. If not, I'll be the first to tell you."

---

**Objection**: "Our parent company is a Salesforce shop — we need to match their tech stack."
**Response**: "Totally understand enterprise standardization. A few questions: (1) Are you required to use Salesforce, or just encouraged? (2) Does parent company pay for your licenses? (3) Do you share data with them daily or monthly? [If monthly] We have customers in your exact situation. They use SimpleCRM for day-to-day (faster, cheaper) and sync to Salesforce monthly via our integration for reporting up to parent company. Saves $100K+/year while keeping everyone happy. Want to see how [Similar Company] did it?"

---

**Objection**: "SimpleCRM doesn't have [obscure feature X] that Salesforce has."
**Response**: "You're right — we don't have that feature today. We built SimpleCRM for 80% of CRM use cases, not 100%. Quick question: how often would you use [feature X]? [If rarely] Here's our philosophy: Salesforce tries to be everything to everyone, which makes it complex and expensive. We focus on the 20% of features that drive 80% of results — pipeline visibility, email automation, mobile access, AI forecasting. That focus is why we're 50% cheaper and 10x easier to use. If [feature X] is truly mission-critical, Salesforce might be better fit. But most customers tell us they'd rather save $100K/year and skip the edge case features. Does that resonate?"

## Competitive Landmines

- ⚠️ **Enterprise compliance**: If they need SOC 2 Type II, HIPAA, or FedRAMP, Salesforce wins. SimpleCRM has SOC 2 Type I only. Redirect: *"For enterprise compliance, Salesforce is industry-leading. If that's table-stakes, they're great choice. Most SMBs don't need that level yet. Where are you?"*

- ⚠️ **AppExchange ecosystem**: Don't fight Salesforce's 3,000+ integrations. Redirect: *"Salesforce's ecosystem is unmatched. Question is: do you need 3,000 integrations, or the right 10? Let's map your must-haves."*

- ⚠️ **Multi-currency, multi-entity**: If they operate in 20+ countries with complex currency/tax needs, Salesforce handles it better. Redirect: *"For global enterprises, Salesforce's localization is best-in-class. Are you currently operating in multiple currencies?"*

## Closing Tips

- ✅ **Lead with TCO (Total Cost of Ownership)**: Don't compare list prices ($45 vs $75). Compare all-in costs: SimpleCRM $45/user vs Salesforce $250/user (real cost). Show 3-year savings: $180K for 30 users.

- ✅ **Offer side-by-side pilot**: "Run SimpleCRM alongside Salesforce for 30 days with 5 reps. If we're not faster and easier, we'll refund 100% and send you a $500 Amazon gift card for your time." Low risk = high conversion.

- ✅ **Know when to walk away**: If they have >200 users, need FedRAMP, or require Salesforce Einstein Advanced AI, they're not a fit. Politely exit: *"Based on your needs, Salesforce is actually better fit. Here's why... [explain]. If your needs change (new division, smaller team), let's reconnect."* Builds trust, generates referrals.

---

**Last Updated**: 2025-11-25
**Win Rate vs Salesforce (SMB)**: 67% (last 6 months, n=150 deals)
**Key Insight**: Lead with "2 hours to launch" story. Salesforce's complexity is their Achilles heel in SMB.

---

### Example 2: Email Marketing Platform vs Mailchimp

**Input**:
```
your_product: SendGrid Pro
differentiators: Developer-friendly API, 99.9% deliverability, transactional + marketing in one, advanced segmentation, white-glove support
competitor_name: Mailchimp
competitor_strengths: Easy to use, great templates, brand recognition, all-in-one marketing suite
target_customer: Mid-Market
deal_size: $50K-$250K
```

**Output**:

---

# Sales Battlecard: SendGrid Pro vs Mailchimp

## Competitor Overview

Mailchimp is the most recognized email marketing platform with 13M+ users. Originally built for small businesses and agencies, now moving upmarket with CRM, landing pages, and social ads. Typical customers: e-commerce, agencies, content creators who want all-in-one marketing suite.

## Why Customers Choose Them

- **Ease of use**: Drag-and-drop builder, zero technical skills required
- **Template library**: 100+ pre-built email templates across industries
- **All-in-one platform**: Email, landing pages, social ads, CRM, websites in single tool
- **Brand recognition**: "The email tool everyone knows" — trusted by 13M users
- **Free tier**: Generous free plan (500 contacts, 1K emails/mo) for startups

## Why Customers Leave Them

- **Deliverability issues at scale**: G2 review (2024): *"Our open rates dropped from 28% to 14% after moving to Mailchimp. Emails landing in spam."* — Director of Growth, E-commerce (50K subscribers). Industry data: Mailchimp avg deliverability 83% vs SendGrid 99.1% (EmailToolTester, 2024)

- **Pricing explosion**: Mailchimp charges by contacts, not emails sent. Customer case: SaaS company with 100K subscribers (20K active) pays $1,000/mo on Mailchimp vs $400/mo on SendGrid. *"We're paying for dead contacts."* — CMO, B2B SaaS

- **Limited API/developer tools**: Reddit r/emailmarketing (2024): *"Mailchimp's API is a nightmare. Rate limits, poor docs, constant breaking changes."* Rated 2.8/5 by developers vs SendGrid 4.6/5 (DeveloperSurvey 2024)

- **No transactional email focus**: Mailchimp built for newsletters, not order confirmations, password resets, alerts. Customer quote: *"We had to use Mailchimp for marketing + SendGrid for transactional anyway. Why pay for two tools?"* — CTO, FinTech startup

## Your Advantages

- **99.9% deliverability guarantee**: SendGrid delivers 99.1% inbox placement (3rd party verified) vs Mailchimp 83%. Case study: SaaS company increased open rates from 14% → 32% in 30 days after switching. ROI: $400K incremental revenue from better deliverability.

- **Developer-friendly API**: Rated #1 email API by developers (2024 StackOverflow survey). Webhook support, real-time event tracking, 99.99% API uptime. Customer: *"Mailchimp's API went down during Black Friday. Cost us $50K in lost sales. SendGrid has never failed us."* — VP Eng, E-commerce ($10M GMV)

- **Transactional + Marketing in one**: Eliminate tool sprawl. Customers save $800/mo consolidating Mailchimp (marketing) + AWS SES (transactional) into SendGrid. Single dashboard, unified analytics, one vendor relationship.

- **Advanced segmentation**: Behavioral triggers (clicked link A but not B), predictive send time, AI-powered subject line optimization. Mid-market customer increased CTR by 48% using SendGrid's segmentation vs Mailchimp's basic tags.

- **White-glove support**: Dedicated CSM for $50K+ accounts, quarterly business reviews, deliverability audits. Mailchimp's support is email-only (24-48hr response) for most plans. SendGrid: <2hr response, phone/Slack for enterprise.

## Trap-Setting Questions

1. **"What's your current email deliverability rate? Are you tracking inbox placement vs spam folder?"**
   *Exposes: Mailchimp's deliverability issues (most customers don't track this)*

2. **"How are you currently handling transactional emails (order confirmations, password resets)? Separate tool or same as marketing?"**
   *Exposes: Tool sprawl and complexity*

3. **"What percentage of your contact list is actually active (opened/clicked in last 90 days)? Are you paying for inactive contacts?"**
   *Exposes: Mailchimp's contact-based pricing vs SendGrid's email-based*

4. **"Have you tried integrating Mailchimp with your app via API? How was the developer experience?"**
   *Exposes: Poor API quality and docs*

5. **"What happens when an email campaign fails or gets delayed? How quickly can you troubleshoot with support?"**
   *Exposes: Slow support for non-enterprise plans*

6. **"Do you use Mailchimp's CRM, landing pages, and social ads features, or just email?"**
   *Exposes: Paying for unused features (all-in-one bloat)*

## Objection Handling

**Objection**: "Mailchimp's drag-and-drop builder is so easy. Your tool looks more technical."
**Response**: "That's a fair perception! Mailchimp nails ease of use for small teams. Here's what we've found with mid-market companies: the 'easy' builder becomes a limitation when you need advanced personalization (dynamic content based on user behavior, predictive send times, A/B testing beyond subject lines). SendGrid has a visual builder too — it's just more powerful. Let me show you: [Demo 2-min use case]. The learning curve is 1-2 hours, but the ROI is 40%+ higher engagement. Does your team have 2 hours to invest for that outcome?"

---

**Objection**: "We're already using Mailchimp for landing pages and social ads — switching means losing those features."
**Response**: "Good point. Quick question: how often do you use those features, and what results are you seeing? [Listen] Here's what most mid-market customers tell us: Mailchimp's landing page builder is fine, but they're already using Unbounce or Instapage (better conversion rates). Same with social ads — Facebook Ads Manager or LinkedIn Campaign Manager have more features. So they're paying for Mailchimp's 'all-in-one' suite but using specialized tools anyway. SendGrid focuses on one thing — email — and we're the best at it. Most customers pair us with best-in-class tools for landing pages, ads, CRM. Total cost is the same or lower, but each tool is top-tier. Does that model make sense for you?"

---

**Objection**: "Mailchimp has better templates and design options."
**Response**: "Mailchimp's template library is great — 100+ designs. SendGrid has 50+ templates, but here's the key difference: mid-market companies don't use pre-built templates for long. They create branded, custom designs. SendGrid's editor gives you full HTML/CSS control + dynamic content blocks. Mailchimp locks you into their template structure. Customer example: [E-commerce brand] wanted to personalize product recommendations in emails based on browsing history. Mailchimp couldn't do it without expensive workarounds. SendGrid handled it natively. Result: 60% higher CTR. Are you looking for out-of-box templates, or fully custom branded emails with dynamic content?"

---

**Objection**: "Your pricing is confusing — Mailchimp charges per contact, you charge per email sent. How do I compare?"
**Response**: "Great question — this trips up everyone! Here's the math: Mailchimp charges based on total contacts (active + inactive). SendGrid charges per email sent. For most mid-market companies, SendGrid is 40-60% cheaper. Example: You have 100K contacts, send 500K emails/month. Mailchimp = $1,000/mo (100K contact tier). SendGrid = $450/mo (500K email tier). Why? You're not paying for dead contacts. Want me to run your numbers? [Use pricing calculator]. I'll show exact cost comparison in 2 minutes."

## Competitive Landmines

- ⚠️ **Templates and design**: Mailchimp has more templates and better visual editor for non-technical users. Don't fight this. Redirect: *"If plug-and-play templates are priority, Mailchimp wins. For custom branded emails with advanced personalization, SendGrid is stronger. Where do you fall?"*

- ⚠️ **All-in-one platform**: If they genuinely use Mailchimp's CRM, landing pages, ads, and email actively, switching creates tool sprawl. Redirect: *"If you're deeply embedded in Mailchimp's full suite and it's working, switching might not make sense. Most mid-market teams use specialized tools anyway. What's your current stack?"*

- ⚠️ **Small budget (<$500/mo)**: Mailchimp's free tier and low entry price beat SendGrid for very small teams. Redirect: *"For early-stage startups, Mailchimp's free tier is hard to beat. SendGrid's value shows at scale (50K+ emails/mo). Where are you today?"*

## Closing Tips

- ✅ **Lead with deliverability**: Show before/after open rates from case studies. Deliverability = revenue. Example: *"2% deliverability improvement = $50K/year for your volume. Want to see the math?"*

- ✅ **Offer deliverability audit**: "We'll audit your current Mailchimp deliverability for free (inbox rate, spam triggers, domain health). Takes 15 min. If Mailchimp is performing well, we'll tell you to stay. If not, we'll show you what's broken." Builds trust, converts 40%+ of audits.

- ✅ **Migration assistance**: Mid-market companies fear migration pain (contacts, templates, automations). Offer: *"We'll migrate everything for free — contacts, templates, automations, historical data. If anything breaks, we fix it within 24 hours or you get 3 months free."* Removes biggest objection.

---

**Last Updated**: 2025-11-25
**Win Rate vs Mailchimp (Mid-Market)**: 58% (last 6 months, n=85 deals)
**Key Insight**: Don't compete on "ease of use." Win on deliverability + developer experience + cost at scale.

---

## Quality Checklist

Before using the battlecard, verify:

- [ ] Competitor overview is factual (no spin)
- [ ] "Why customers choose them" is honest (3+ genuine strengths)
- [ ] "Why customers leave them" has sources (G2, Reddit, customer quotes)
- [ ] Your advantages have proof points (metrics, case studies, quotes)
- [ ] Trap-setting questions are neutral (not attacks)
- [ ] Objection responses include proof (data, examples)
- [ ] Competitive landmines are realistic (topics you can't win)
- [ ] Closing tips are tactical (not generic "build rapport")
- [ ] Total length <1000 words (2 pages max)

## Tips for Better Results

1. **Be brutally honest**: Admitting competitor's strengths builds trust with prospect
2. **Use real data**: G2 reviews, case studies, industry reports (not "our customers say...")
3. **Cite sources**: "According to G2 (2024)" or "CustomerX, SaaS company, 50 employees"
4. **Focus on outcomes**: Don't say "We have feature X." Say "Feature X helped CustomerY achieve Z result."
5. **Update quarterly**: Competitors change pricing, add features, fix bugs. Keep battlecards fresh.
6. **Test with sales team**: Ask reps "Which objections are missing?" and "Which responses actually work?"

## Common Mistakes to Avoid

❌ **Trash-talking competitor**: "Their product sucks" = unprofessional
❌ **Feature-dumping**: Listing 20 features without context = boring
❌ **No proof points**: "We're better" without data = not credible
❌ **Ignoring competitor strengths**: Pretending they have no advantages = dishonest
❌ **Generic objection handling**: "That's a good question..." without real answer = useless
❌ **Too long**: >2 pages = reps won't read it

## Success Metrics

Track these for each battlecard:

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Win rate vs competitor | 50%+ | CRM data (deals won vs lost against this competitor) |
| Battlecard usage | 80%+ | Survey reps: "Did you use battlecard in last 5 competitive deals?" |
| Objection handling confidence | 8/10+ | Survey reps: "Rate confidence handling objections (1-10)" |
| Time to close (competitive) | <10% slower | Compare deal cycle: competitive vs non-competitive |

**If below targets, adjust**:
- **Low win rate** → Battlecard may be inaccurate, or competitor genuinely better fit
- **Low usage** → Too long, not accessible, or reps don't trust it
- **Low confidence** → Objection responses too generic, need more proof points
- **Longer deal cycles** → Trap-setting questions not effective

---

**Created**: 2025-11-25
**Version**: 1.0
**Part of**: Sales Prospecting Autopilot Pack
