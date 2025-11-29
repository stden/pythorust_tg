# Cold Email Generator - SPIN Selling Framework

## Prompt Template

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

## Input Parameters

| Parameter | Type | Options/Format | Required |
|-----------|------|----------------|----------|
| company_name | String | Company name | Yes |
| industry | Dropdown | SaaS, E-commerce, Finance, Healthcare, Manufacturing, Other | Yes |
| pain_point | String | Max 100 chars | Yes |
| product_service | String | Max 50 chars | Yes |
| tone | Dropdown | Professional, Casual, Urgent, Consultative | Yes |

## Example Usage

### Example 1: SaaS HR Platform → Recruiting Agencies

**Input**:
```
company_name: TalentHub Recruiting
industry: SaaS
pain_point: Manual resume screening takes 20+ hours per week
product_service: AI-powered resume screening tool
tone: Professional
```

**Output**:

**Variation A:**

Subject: 20 hours/week wasted on resume screening?

Hi Sarah,

I noticed TalentHub has been posting a lot of open roles lately — congrats on the growth! But I'm guessing your team is drowning in resumes.

Most recruiting agencies we work with spend 20+ hours/week manually screening candidates. That's time your recruiters could spend actually talking to people.

We built an AI tool that pre-screens resumes and ranks candidates by fit. One agency cut their screening time by 75%.

Would a 15-min demo make sense? I can show you how it works with your actual job descriptions.

Best,
[Your name]

---

**Variation B:**

Subject: Sarah - question about TalentHub's hiring process

Hi Sarah,

Saw you're hiring for 3 new recruiters at TalentHub. Smart move given your recent Series A.

Quick question: how are you handling the increased resume volume? Most agencies your size struggle with 100+ applications per role.

We help recruiting teams automate the first-pass screening using AI. Think of it as a junior recruiter that never sleeps.

Curious if this is even on your radar? Happy to share how similar agencies use it.

Cheers,
[Your name]

---

**Variation C:**

Subject: TalentHub + AI screening = 75% time saved?

Hi Sarah,

Your team posted 12 roles last month on TalentHub — impressive pipeline.

But here's the math: if each role gets 100 applicants, that's 1,200 resumes to screen. At 2 min per resume, that's 40 hours of manual work.

What if AI could do the first-pass screening for you? We built a tool that ranks candidates by fit in seconds, not hours.

Would love to show you a quick demo using one of your actual job descriptions. 15 minutes, zero pitch.

Best,
[Your name]

---

### Example 2: E-commerce Analytics → Shopify Stores

**Input**:
```
company_name: GreenLeaf Skincare
industry: E-commerce
pain_point: Don't know which products are actually profitable
product_service: Profit analytics for Shopify stores
tone: Casual
```

**Output**:

**Variation A:**

Subject: Which of your products are actually profitable?

Hey Emma,

Checked out GreenLeaf's store — love the minimalist branding! But I have a weird question:

Do you know which SKUs are *actually* making you money after ads, returns, and shipping?

Most Shopify stores we talk to track revenue but not true profit per product. Turns out 30% of SKUs are usually loss-leaders.

We built a simple dashboard that shows real profit (not just revenue) for every product. Takes 5 min to set up.

Want to see it in action with your store data?

Emma

---

**Variation B:**

Subject: GreenLeaf's top-selling product might be losing money

Hey Emma,

Saw your Black Friday sale crushed it (congrats on 40% rev growth!).

But here's the uncomfortable truth: your #1 best-seller might be losing you money once you factor in returns, ads, and shipping.

We plugged GreenLeaf's numbers into our profit calculator and found something interesting. Want to see it?

No strings attached — just thought you'd want to know before scaling that product further.

Cheers,
Emma

---

**Variation C:**

Subject: Emma - quick profit check for GreenLeaf

Hey Emma,

Your store is doing $50K/month — nice work. But I'm curious:

What's your actual profit margin per product after Facebook ads, shipping, and returns?

Most founders track revenue obsessively but guess at profit. We built a tool that connects to Shopify and shows true profit in real-time.

10-minute demo? I'll run your numbers live and you can decide if it's useful.

Best,
Emma

---

## Quality Checklist

Before using the output, verify:

- [ ] Subject line is <50 characters
- [ ] Opening references company/industry
- [ ] Email is 80-120 words
- [ ] No buzzwords used
- [ ] CTA is soft (question or value offer, not "book a demo now")
- [ ] Personalization is genuine (not "I love your company")
- [ ] Tone matches request
- [ ] All 3 variations have different hooks

## Tips for Better Results

1. **Be specific with pain point**: Instead of "slow processes", say "manual invoice processing takes 10 hours/week"
2. **Include numbers**: Quantify the pain and the solution
3. **Research the company**: Mention recent funding, growth, new roles, etc.
4. **Test different tones**: Professional works for enterprise, casual for startups
5. **A/B test subject lines**: Track which variation gets better open rates

## Common Mistakes to Avoid

❌ **Too long**: >150 words = instant delete
❌ **Generic**: "I noticed you're in the X industry" = spam
❌ **Hard sell**: "Book a demo now!" = pushy
❌ **Fake flattery**: "Your company is amazing!" = insincere
❌ **No personalization**: Could be sent to anyone = waste

## Success Metrics

Track these after sending:
- Open rate (target: 40%+)
- Reply rate (target: 10%+)
- Meeting booked rate (target: 3%+)

If below targets, adjust:
- **Low opens** → test different subject lines
- **Opens but no replies** → email body too salesy or not relevant
- **Replies but no meetings** → CTA unclear or value proposition weak

---

**Created**: 2025-11-24
**Version**: 1.0
**Part of**: Sales Prospecting Autopilot Pack
