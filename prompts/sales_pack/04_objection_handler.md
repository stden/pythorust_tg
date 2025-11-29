# Objection Handler - LAER Framework

## Prompt Template

```
You are an expert sales trainer specializing in objection handling. Your task is to create comprehensive objection responses using the LAER framework (Listen, Acknowledge, Explore, Respond).

CONTEXT:
- Your Product/Service: {product_service}
- Target Customer: {target_customer}
- Price Point: {price_point}
- Objection Type: {objection_type}
- Specific Objection: {specific_objection}

LAER FRAMEWORK:

**L - Listen**: Repeat objection back to show you heard it
**A - Acknowledge**: Validate their concern (don't dismiss it)
**E - Explore**: Ask questions to understand root cause
**R - Respond**: Address with proof (data, story, or bridge to next step)

RESPONSE STRUCTURE:

**Objection**: "[The exact objection]"

**L - Listen**
"So if I'm hearing you correctly, [restate objection]?"

**A - Acknowledge**
"That's a [valid/fair/common/smart] concern. [Why it's legitimate]."

**E - Explore**
"Can I ask — [2-3 questions to uncover root cause]?"

**R - Respond**
[Address with one of these approaches]:
- **Data**: Stats, metrics, case study
- **Story**: Customer example showing how they overcame same objection
- **Reframe**: Change perspective on the concern
- **Bridge**: "Let's set that aside for now and come back to it after [next step]"

OBJECTION TYPES:
1. Price/Budget
2. Timing ("Not now")
3. Authority ("Need to check with boss")
4. Need ("Don't need it")
5. Trust ("Don't know you/your company")
6. Competitor ("Already using X")

CONSTRAINTS:
- Never argue or dismiss the objection
- Never say "I understand, but..." (sounds dismissive)
- Ask questions before responding (uncover the real objection)
- Always have proof (data, story, or credible source)
- Keep response <200 words
- Sound conversational, not scripted

OUTPUT FORMAT:
**Objection**: "[Exact objection]"

**L - Listen**
"[Restate objection]"

**A - Acknowledge**
"[Validate concern]"

**E - Explore**
"[Question 1]"
"[Question 2]"

**R - Respond**
"[Response with proof]"

**Alternative Response** (if Explore reveals different root cause):
"[Alternative response]"

Generate 3 variations (A, B, C) with different Explore questions and Respond approaches.
```

## Input Parameters

| Parameter | Type | Options/Format | Required |
|-----------|------|----------------|----------|
| product_service | String | What you sell | Yes |
| target_customer | String | Who you sell to | Yes |
| price_point | String | Actual price or range | Yes |
| objection_type | Dropdown | Price, Timing, Authority, Need, Trust, Competitor | Yes |
| specific_objection | String | The exact objection phrase | Yes |

## Example Usage

### Example 1: Price Objection - "Too expensive"

**Input**:
```
product_service: Sales coaching platform (AI-powered)
target_customer: VP of Sales (50-person sales team)
price_point: $15,000/year
objection_type: Price
specific_objection: "That's too expensive for us right now."
```

**Output**:

#### Variation A - Data Approach

**Objection**: "That's too expensive for us right now."

**L - Listen**
"So if I'm hearing you correctly, $15,000/year feels like too big an investment for your team right now?"

**A - Acknowledge**
"That's a fair concern. $15K is real money, and you need to see clear ROI before spending it."

**E - Explore**
- "Can I ask — what's your current average deal size?"
- "And how many deals do your reps close per month on average?"
- "If each rep closed just one additional deal per quarter, what would that be worth?"

**R - Respond**
"Here's the math: your average deal is $25K, and you have 50 reps. If our AI coaching helps each rep close just ONE extra deal per quarter (4 deals/year), that's 50 x 4 x $25K = $5M in incremental revenue. Our customers see 12% win rate improvement on average, which for you would be ~8 extra deals per rep per year.

So the real question isn't 'Can we afford $15K?' — it's 'Can we afford NOT to invest $15K to unlock $5M+?' Does that math make sense?"

**Alternative Response** (if they say "We don't have budget this year"):
"Totally understand budget cycles. Two options: (1) We can start in Q1 2026 when new budget opens — I'll hold your pricing and lock you in now. (2) Most VPs find $15K in their 'tools' or 'training' line items without needing new budget approval. Have you checked those? Either way, let's at least get the paperwork ready so you can move fast when you're ready."

---

#### Variation B - Story Approach

**Objection**: "That's too expensive for us right now."

**L - Listen**
"So $15K/year feels like a stretch for your budget?"

**A - Acknowledge**
"I get it. When you haven't seen the ROI yet, it feels like a big risk."

**E - Explore**
- "What's your biggest revenue leak right now — reps ramping too slowly, low win rates, or inconsistent coaching?"
- "If you could wave a magic wand and fix one thing about your sales team's performance, what would it be?"
- "What's the cost of not fixing that problem this year?"

**R - Respond**
"Let me share a quick story. Last year, we worked with a VP at [SaaS Company] — similar team size to yours, 45 reps. They had the same concern: '$15K is too expensive.'

But here's what they found: their average rep took 6 months to ramp to quota. With our AI coaching, they cut that to 3.5 months. That's 2.5 months of productivity gained per new hire.

They hired 12 reps that year. 12 reps x 2.5 months x $50K quota/month = $1.5M in incremental revenue. They paid $15K and made $1.5M.

Their VP told me: 'The real question wasn't whether we could afford it — it was whether we could afford NOT to do it.'

What's your current ramp time, and how many new hires are you planning this year?"

**Alternative Response** (if they say "We're bootstrapped, every dollar matters"):
"Respect the discipline — bootstrapped companies can't waste money. Here's what other bootstrapped VPs do: start with a 3-month pilot for just 10 reps ($3,750). If you don't see measurable improvement (faster ramp, higher win rate, better forecast accuracy), cancel and we refund 100%. If it works, expand to full team. Zero risk. Does a pilot make sense?"

---

#### Variation C - Reframe Approach

**Objection**: "That's too expensive for us right now."

**L - Listen**
"So the $15K price point feels too high given your current budget?"

**A - Acknowledge**
"That's a totally reasonable reaction. Price is always a factor, especially with new tools."

**E - Explore**
- "Just curious — how much are you currently spending on sales training, coaching tools, or enablement per year?"
- "What's the cost of a missed quota? Like if your team misses by 10%, what's that worth?"
- "If this investment helped you hit quota or exceed it, would $15K feel different?"

**R - Respond**
"Here's how I'd reframe it: you're not spending $15K on a 'sales coaching tool' — you're investing $15K to protect a $10M+ revenue target.

Most VPs spend $50K-100K/year on training (workshops, consultants, enablement tools). Our platform replaces 80% of that and delivers better results because it's real-time, AI-powered, and always on.

So the question isn't 'Is $15K expensive?' — it's 'What's the most cost-effective way to hit my revenue target?' For 50 reps, that's $300/rep/year, or $25/month per rep. Cheaper than a Salesforce license.

Would it help if I showed you exactly what other VPs with your team size are spending on coaching, and how we compare?"

**Alternative Response** (if they say "We just bought another tool, can't add more"):
"Totally get it — tool fatigue is real. Quick question: what tool did you just buy, and what's it solving? [Listen] Here's what most VPs find: our platform doesn't add to the stack, it replaces 2-3 tools (call recording + coaching + training + analytics). Net result: you might actually save money while getting better outcomes. Want me to map your current stack and show where we'd consolidate?"

---

### Example 2: Timing Objection - "Not the right time"

**Input**:
```
product_service: Marketing automation platform
target_customer: CMO (B2B SaaS, Series A)
price_point: $30,000/year
objection_type: Timing
specific_objection: "We're not ready yet. Maybe next quarter."
```

**Output**:

#### Variation A - Cost of Delay

**Objection**: "We're not ready yet. Maybe next quarter."

**L - Listen**
"So if I'm understanding correctly, you'd like to revisit this next quarter when things settle down?"

**A - Acknowledge**
"That makes sense — timing is critical, and you don't want to rush into a big decision."

**E - Explore**
- "Can I ask — what needs to happen between now and next quarter for you to be 'ready'?"
- "Is it a budget thing, a bandwidth thing, or something else?"
- "What are you currently using for marketing automation in the meantime?"

**R - Respond**
"Here's the challenge with waiting: every quarter you delay costs you leads, pipeline, and revenue.

Let me show you the math: our customers generate 40% more qualified leads in their first 90 days. For a Series A SaaS company like yours (assuming $5M ARR target), that's ~$500K in pipeline per quarter.

If you wait until Q2 to start, you lose Q1's $500K pipeline opportunity. That pipeline takes 60-90 days to close, so you're actually losing Q2 revenue too.

The 'cost of waiting' is often bigger than the cost of the tool. Does that resonate?"

**Alternative Response** (if they say "We're in the middle of a product launch"):
"Got it — product launch is all-hands-on-deck. Here's what we've done with other companies mid-launch: we handle the implementation for you (zero lift on your team), and you go live in 2 weeks instead of 2 months. Your team focuses on launch, we handle the marketing automation plumbing. After launch, you have a fully-working system ready to scale. Would that model work?"

---

#### Variation B - FOMO (Fear of Missing Out)

**Objection**: "We're not ready yet. Maybe next quarter."

**L - Listen**
"So next quarter feels like better timing?"

**A - Acknowledge**
"Totally fair. You've got a lot on your plate, and adding something new right now might feel overwhelming."

**E - Explore**
- "What's driving the 'not ready' feeling — is it internal bandwidth, budget, or something else?"
- "If we could take implementation off your plate entirely, would that change the timing?"
- "What happens if you wait until Q2 — do you have a manual workaround in the meantime, or are you just pausing lead gen?"

**R - Respond**
"Here's what I'd hate to see happen: you wait until Q2, your competitor starts using automation in Q1, and they steal 3 months of market share while you're manually sending emails.

One of our customers — similar stage to you, Series A SaaS — waited 6 months to pull the trigger. Their competitor (who moved faster) captured 30% more leads in that time. By the time they launched, they were playing catch-up.

The CMO told me: 'I thought waiting would reduce risk. Instead, it created a bigger risk — falling behind.'

What if we did a 30-day pilot starting in 2 weeks? You'd see results before end of quarter, and if it doesn't work, you cancel with zero penalty. Low risk, high upside. Does that make sense?"

**Alternative Response** (if they say "We need to hire a marketing ops person first"):
"Smart thinking — you need someone to own this. Two paths: (1) Hire first, then implement tool (takes 3-6 months total). (2) Implement tool now, use our team as your interim marketing ops (included in the price), then hand off to your hire when ready. Most CMOs choose option 2 because you don't lose 6 months. Which sounds better?"

---

#### Variation C - Bridge to Next Step

**Objection**: "We're not ready yet. Maybe next quarter."

**L - Listen**
"So you're thinking Q2 is more realistic timing?"

**A - Acknowledge**
"That's a smart approach — don't rush a decision this big."

**E - Explore**
- "Just to make sure we're aligned: when you say 'not ready,' what specifically needs to be in place first?"
- "Is there anything we could do in Q1 to make Q2 smoother (data audit, strategy session, team training)?"
- "Who else needs to be involved in the decision before you move forward?"

**R - Respond**
"Here's what I'd suggest: let's not wait until Q2 to start the conversation. Most successful implementations happen in 3 phases:

**Phase 1 (Jan-Feb)**: Strategy & Planning — we audit your current setup, map your ideal automation workflows, and build a custom playbook. Zero cost, zero commitment.

**Phase 2 (Mar)**: Pilot Launch — go live with 1-2 workflows (email nurture, lead scoring). Low risk, fast win.

**Phase 3 (Q2)**: Full Rollout — expand to full platform once you've seen results.

This way, you're not 'buying blind' in Q2 — you've already validated it works. And you don't lose Q1 entirely. Does breaking it into phases make sense?"

**Alternative Response** (if they say "We're waiting for end-of-year budget refresh"):
"Got it — budget cycles are real. Two options: (1) I can hold this pricing until Q1 2026 and we start fresh then. (2) Many CMOs find budget in their 'tools' or 'agencies' line items (since we replace 2-3 tools + reduce agency spend). Want to explore if you have hidden budget before year-end? Either way, let's at least get the contract drafted so you can move fast when budget opens."

---

### Example 3: Authority Objection - "Need to talk to my boss"

**Input**:
```
product_service: CRM platform
target_customer: Sales Manager (reports to VP of Sales)
price_point: $10,000/year
objection_type: Authority
specific_objection: "I need to run this by my VP before moving forward."
```

**Output**:

#### Variation A - Champion Enablement

**Objection**: "I need to run this by my VP before moving forward."

**L - Listen**
"So you'd like to get buy-in from your VP before making a decision?"

**A - Acknowledge**
"That makes total sense — $10K is a meaningful investment, and your VP should be part of the decision."

**E - Explore**
- "What's the best way to loop in your VP — do you want to present it yourself, or would it help if I joined the conversation?"
- "What's your VP's biggest concern likely to be — ROI, implementation time, or something else?"
- "Have you bought similar tools before? What made those decisions easy vs hard?"

**R - Respond**
"Here's what I'd suggest: let me arm you with everything you need to make a strong case to your VP.

I'll send you:
1. One-page ROI summary (cost vs expected return)
2. Case study from a company your VP will recognize
3. Comparison chart (us vs competitors)
4. 3-month implementation timeline

That way, you walk into the conversation prepared, and your VP sees you did your homework.

If your VP wants to dive deeper, I'm happy to join a 15-min call to answer their questions directly. But my experience is that VPs respect when their managers come prepared with a solid business case. Sound good?"

**Alternative Response** (if they say "My VP is skeptical of new tools"):
"Got it — they've been burned before. Here's what works with skeptical VPs: we offer a 60-day pilot with a money-back guarantee. Your VP doesn't have to 'believe' it will work — they just need to agree to test it. If it doesn't deliver measurable results (faster deal cycles, higher win rates, better pipeline visibility), we refund 100%. Zero risk. Would your VP go for a pilot?"

---

#### Variation B - Multi-Threading

**Objection**: "I need to run this by my VP before moving forward."

**L - Listen**
"So your VP needs to be involved in the final decision?"

**A - Acknowledge**
"Absolutely — makes sense for an investment like this."

**E - Explore**
- "Is your VP the final decision-maker, or are there others (finance, IT) who need to weigh in too?"
- "When's the next time you'll talk to your VP about this?"
- "What's the worst-case scenario if this takes another month to get approved?"

**R - Respond**
"Here's what I've found works best: instead of you playing 'telephone' between me and your VP (things get lost in translation), let's do a quick 3-way call.

I'll join you and your VP for 15 minutes, answer their questions directly, and make sure everyone's aligned. That way, we don't lose momentum, and your VP gets the full context.

Most VPs appreciate the efficiency — they'd rather spend 15 minutes on a call than read a 10-page proposal. Does your VP have 15 minutes this week or next?"

**Alternative Response** (if they say "My VP is traveling for 2 weeks"):
"No problem — let's use that time productively. While your VP is out, let's run a pilot with your team (5-10 reps). When your VP gets back, you'll have real data to show them ('Here's what we tested, here are the results'). VPs love data over promises. Make sense?"

---

#### Variation C - Reframe (You're the Hero)

**Objection**: "I need to run this by my VP before moving forward."

**L - Listen**
"So you want your VP's input before committing?"

**A - Acknowledge**
"Smart move — getting buy-in upfront avoids headaches later."

**E - Explore**
- "If your VP says yes, what happens next? Do you move forward immediately, or are there other steps?"
- "What would make your VP say 'Let's do it' versus 'Let's wait'?"
- "Have you bought tools without your VP's approval before, or is it always required for this budget level?"

**R - Respond**
"Here's how I'd position this with your VP: you're not asking for permission — you're presenting a solution to a problem they already know exists.

Your pitch: 'We're losing deals because our CRM is outdated. I found a tool that solves this for $10K/year — pays for itself if we close 1-2 extra deals. I've done my homework: talked to 3 vendors, checked references, negotiated pricing. This is the best option. Can we move forward?'

VPs respect managers who take initiative, do the research, and bring solutions (not problems). You're not asking 'Can I buy this?' — you're saying 'Here's what I recommend and why.'

Want me to help you craft the pitch?"

**Alternative Response** (if they say "I don't want to bother my VP with this"):
"Totally get it — VPs are busy. Here's the thing: if this tool helps you hit quota, your VP will wish you'd brought it up sooner. VPs care about results, not tools. Frame it as 'Here's how we hit our Q1 target' (not 'Can I buy software?'). They'll make time for that conversation. And if they say no, at least you know you tried. Make sense?"

---

## Quality Checklist

Before using objection responses, verify:

- [ ] Restate objection accurately (Listen)
- [ ] Acknowledge is genuine, not dismissive ("I hear you, BUT..." = bad)
- [ ] Explore asks 2-3 open-ended questions
- [ ] Respond includes proof (data, story, or credible source)
- [ ] Alternative response covers different root cause
- [ ] Response is <200 words
- [ ] Sounds conversational (not scripted or robotic)
- [ ] No arguing or pressuring the prospect

## Tips for Better Results

1. **Pause before responding**: Count to 3 after the objection. Shows you're thinking, not reciting a script.
2. **Explore is the most important step**: 80% of objections aren't what they seem. "Too expensive" often means "I don't see the value."
3. **Use their words**: If they say "budget," don't switch to "investment." Mirror their language.
4. **Have 3 proof types ready**: Data (stats), Story (customer example), Authority (industry report).
5. **Know when to walk away**: If objection is legitimate ("We just signed a 3-year contract with your competitor"), don't fight it. Say "Makes sense. Let's reconnect in Year 3."

## Common Mistakes to Avoid

❌ **Dismissing objection**: "That's not a real concern" = argument = lost deal
❌ **Skipping Explore**: Jumping straight to Response without understanding root cause
❌ **Using "I understand, but..."**: Sounds like you DON'T understand
❌ **Arguing**: "You're wrong about that" = ego battle = no sale
❌ **Too much talking**: Objection response >3 minutes = you lost them
❌ **No proof**: "Trust me, it works" = not credible

## Success Metrics

Track these for each objection:

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Objection conversion rate | 40%+ | % of objections that turn into next step (demo, proposal, close) |
| Objection recurrence | <20% | Same objection appearing multiple times = you didn't address it well |
| Rep confidence | 8/10+ | Survey reps: "How confident handling this objection? (1-10)" |
| Time to respond | <30 sec | Long pauses = you're scrambling, not confident |

**If below targets, adjust**:
- **Low conversion** → Response not addressing root cause, or no proof points
- **High recurrence** → You're not actually resolving the objection, just delaying it
- **Low confidence** → Reps don't trust the response, need more practice/roleplay
- **Slow response** → Not enough preparation, need better training

## Advanced Tactics

### Tactic 1: Pre-Empt Objections
Don't wait for objection — address it before they bring it up.

Example: "Most VPs ask about ROI at this point. Let me show you the math..."

### Tactic 2: Objection Judo
Use their objection as a reason to buy.

Example: "You said budget is tight — that's exactly why you need this. It pays for itself in 60 days."

### Tactic 3: Isolate Objections
"If I could solve [objection], is there anything else preventing you from moving forward?"

If they say "No," you know it's the real objection. If they say "Yes, also...", you uncover hidden concerns.

### Tactic 4: Social Proof
"Every customer we have said the same thing before buying. Here's what they found..."

---

**Created**: 2025-11-25
**Version**: 1.0
**Part of**: Sales Prospecting Autopilot Pack
