# LinkedIn Outreach Template - 4-Touch Sequence

## Prompt Template

```
You are an expert B2B sales professional specializing in LinkedIn outreach. Your task is to write a 4-message LinkedIn outreach sequence that builds rapport, provides value, and moves the prospect toward a conversation.

CONTEXT:
- Prospect Name: {prospect_name}
- Prospect Title: {prospect_title}
- Prospect Company: {prospect_company}
- Your Solution: {solution}
- Connection Reason: {connection_reason}
- Shared Context: {shared_context} (optional: mutual connection, same group, event)

SEQUENCE STRUCTURE:
**Message 1 - Connection Request (300 chars MAX)**
- Reference shared context or genuine reason to connect
- NO pitch, NO selling
- Curiosity-driven

**Message 2 - Value Delivery (Day 3 after connection)**
- Thank for accepting
- Share relevant resource (article, tool, insight)
- Ask thoughtful question about their work
- 100-150 words

**Message 3 - Soft Transition (Day 7)**
- Reference their response or LinkedIn activity
- Introduce your solution indirectly (case study, customer story)
- No direct ask yet
- 80-120 words

**Message 4 - Meeting Request (Day 14)**
- Clear, specific CTA
- Time-bound offer (15-min call, demo, resource)
- Make it easy to say yes
- 60-80 words

CONSTRAINTS:
- NO templates that scream "sales sequence"
- NO generic compliments ("I was impressed by your profile")
- NO immediate pitch in connection request
- Use prospect's actual LinkedIn activity when possible
- Sound human, not robotic

OUTPUT FORMAT:
**MESSAGE 1 - Connection Request**
[Text within 300 chars]

**MESSAGE 2 - Value Delivery (Day 3)**
Hi {First Name},

[Body]

[Your name]

**MESSAGE 3 - Soft Transition (Day 7)**
[Body with reference to their response]

**MESSAGE 4 - Meeting Request (Day 14)**
[Clear CTA]

Generate 2 variations (A, B) for each message.
```

## Input Parameters

| Parameter | Type | Format | Required |
|-----------|------|--------|----------|
| prospect_name | String | First + Last Name | Yes |
| prospect_title | String | Job title | Yes |
| prospect_company | String | Company name | Yes |
| solution | String | What you sell (max 50 chars) | Yes |
| connection_reason | Dropdown | Same industry, Mutual connection, LinkedIn group, Event attendee, Content engagement | Yes |
| shared_context | String | Specific detail (mutual name, event, article) | No |

## Example Usage

### Example 1: SaaS Sales Tool → VP of Sales

**Input**:
```
prospect_name: Michael Chen
prospect_title: VP of Sales
prospect_company: CloudSync Technologies
solution: AI-powered sales coaching platform
connection_reason: Content engagement
shared_context: Commented on his post about remote sales team challenges
```

**Output**:

#### Variation A

**MESSAGE 1 - Connection Request**
Hi Michael, saw your post about managing remote sales teams — resonated with our experience at [Your Company]. Would love to connect and exchange ideas on what's working.

---

**MESSAGE 2 - Value Delivery (Day 3)**
Hi Michael,

Thanks for connecting! Your post about remote team challenges got me thinking.

I put together a quick guide on "5 Tactics High-Performing Remote Sales Teams Use" based on data from 200+ sales orgs. Thought you might find it useful: [link]

Curious — what's been your biggest win with the CloudSync team this quarter?

Best,
[Your name]

---

**MESSAGE 3 - Soft Transition (Day 7)**
Michael,

Glad the remote sales tactics resonated! Your point about async coaching is spot-on.

We recently worked with a SaaS company (similar size to CloudSync) that had the same challenge. Their VP implemented AI-powered call reviews and cut ramp time from 90 days to 45 days.

Happy to share the full case study if you're curious how they did it. No strings attached.

Cheers,
[Your name]

---

**MESSAGE 4 - Meeting Request (Day 14)**
Michael,

Would a 15-min call next week make sense? I can walk you through how [Similar Company] built their remote coaching system.

Tuesday 2pm or Wednesday 10am work?

Best,
[Your name]

---

#### Variation B

**MESSAGE 1 - Connection Request**
Michael, your insights on remote sales management are 🔥. We're solving similar problems at [Your Company]. Let's connect!

---

**MESSAGE 2 - Value Delivery (Day 3)**
Hi Michael,

Appreciate the connection! Been following your content on remote sales — always actionable.

Question for you: how are you handling call reviews with a distributed team? Most VPs tell us it's their #1 bottleneck.

We published a breakdown of what top remote teams do differently: [link]. Let me know if it's useful.

Best,
[Your name]

---

**MESSAGE 3 - Soft Transition (Day 7)**
Totally hear you on the call review challenge, Michael.

One of our customers (SaaS VP, 25-person team) automated their entire call coaching workflow with AI. Reps now get feedback within 2 hours instead of 2 weeks.

The ROI was crazy — 40% faster ramp for new hires. Want me to intro you to their VP? He's happy to share what worked.

Cheers,
[Your name]

---

**MESSAGE 4 - Meeting Request (Day 14)**
Michael,

Quick question: would seeing a 10-min demo of the AI coaching platform be valuable?

I can show you exactly how it works with your team's call structure. This Thursday or Friday?

Best,
[Your name]

---

### Example 2: Marketing Agency → CMO

**Input**:
```
prospect_name: Sarah Williams
prospect_title: CMO
prospect_company: GreenTech Innovations
solution: Performance marketing for climate tech
connection_reason: Same industry
shared_context: Both in "Climate Tech CMOs" LinkedIn group
```

**Output**:

#### Variation A

**MESSAGE 1 - Connection Request**
Hi Sarah, saw we're both in the Climate Tech CMOs group. Love what GreenTech is doing with carbon tracking. Would be great to connect!

---

**MESSAGE 2 - Value Delivery (Day 3)**
Hi Sarah,

Thanks for connecting! Just saw GreenTech's Series A announcement — congrats! 🎉

I'm sending over a resource that might help with scaling: "Climate Tech CAC Benchmarks (2025)" based on 50+ companies. Spoiler: your industry's CAC is 3x higher than SaaS 😅

Link: [resource]

What's your biggest growth lever right now — paid, content, or partnerships?

Best,
[Your name]

---

**MESSAGE 3 - Soft Transition (Day 7)**
Sarah,

Glad the CAC benchmarks were helpful! Your point about content SEO is exactly what we saw with [Climate Tech Company].

They were burning $50K/month on paid with 8% conversion. We shifted 60% to SEO + thought leadership, and their demo requests doubled in 90 days.

Want the full breakdown? I can send the case study (no pitch, just numbers).

Cheers,
[Your name]

---

**MESSAGE 4 - Meeting Request (Day 14)**
Sarah,

Would a quick 15-min call make sense to walk through the climate tech SEO playbook?

I can show you the exact content strategy that's working for [Similar Company]. Tuesday or Thursday next week?

Best,
[Your name]

---

#### Variation B

**MESSAGE 1 - Connection Request**
Sarah, we're both in Climate Tech CMOs — love GreenTech's approach to carbon tracking. Let's connect and share insights!

---

**MESSAGE 2 - Value Delivery (Day 3)**
Hi Sarah,

Thanks for connecting! Saw your Series A news — exciting times ahead.

Quick question: as you scale, how are you thinking about CAC for B2B climate tech? Most CMOs we talk to struggle with the long sales cycles.

Sharing a resource we built: "Climate Tech Marketing Playbook 2025" [link]. Let me know if it's useful!

Best,
[Your name]

---

**MESSAGE 3 - Soft Transition (Day 7)**
Love your take on enterprise sales cycles, Sarah.

One of our clients (climate tech SaaS, similar to GreenTech) had the same challenge. They cut CAC by 40% in 6 months by switching from paid ads to content + partnerships.

Their CMO shared the full strategy on a podcast we did. Want the link? It's 20 minutes and packed with tactical stuff.

Cheers,
[Your name]

---

**MESSAGE 4 - Meeting Request (Day 14)**
Sarah,

Would it make sense to hop on a quick call next week? I can walk you through the exact content playbook [Similar Company] used to cut CAC.

15 minutes, zero pitch. Does Wednesday 2pm or Friday 10am work?

Best,
[Your name]

---

## Quality Checklist

Before using the output, verify:

- [ ] Connection request is <300 characters
- [ ] No pitch in Message 1
- [ ] Message 2 provides genuine value (resource, insight, question)
- [ ] Message 3 references specific prospect activity or response
- [ ] Message 4 has clear, time-bound CTA
- [ ] All messages sound human (no "I hope this message finds you well")
- [ ] Variations have different hooks/angles
- [ ] Timing is realistic (Day 0, 3, 7, 14)

## Timing Best Practices

| Message | Timing | Notes |
|---------|--------|-------|
| Message 1 | Day 0 | Send connection request |
| Message 2 | Day 3 | Wait for acceptance + 2-3 days |
| Message 3 | Day 7 | Only if they responded to Message 2 |
| Message 4 | Day 14 | If engagement is high, move faster (Day 10) |

**IMPORTANT**:
- If no response to Message 2 → wait 7 days, then skip to Message 4
- If they engage deeply in Message 2 → accelerate to Message 4 (skip Message 3)
- If they ghost after Message 3 → stop. Don't be annoying.

## Tips for Better Results

1. **Research before sending**: Check their last 3 LinkedIn posts, recent company news
2. **Use mutual connections**: "I saw you know [Name] — we worked together at [Company]"
3. **Reference specific content**: "Your post about [topic] resonated because..."
4. **Provide REAL value in Message 2**: No fluff resources, send something genuinely useful
5. **Make Message 4 easy**: Specific time slots, clear value prop, low commitment (15 min)
6. **A/B test connection requests**: Track which reasons get higher acceptance rates

## Common Mistakes to Avoid

❌ **Pitching in connection request**: "I help companies like yours..." = instant ignore
❌ **Generic compliments**: "I was impressed by your profile" = spam
❌ **No value in Message 2**: Just "How are you?" = waste of time
❌ **Sending Message 3 without response**: Looks desperate
❌ **Vague CTA in Message 4**: "Let me know if you'd like to chat" = low conversion
❌ **Not personalizing**: Using {First Name} without context = robotic

## Success Metrics

Track these after sending:

| Metric | Target | What to Track |
|--------|--------|---------------|
| Connection acceptance rate | 40%+ | Message 1 effectiveness |
| Message 2 response rate | 25%+ | Value delivery quality |
| Message 3 engagement | 60%+ | If they responded to M2, keep engaging |
| Meeting booked rate | 10%+ | Of total connections made |

**If below targets, adjust**:
- **Low acceptance** → test different connection reasons, improve personalization
- **No M2 responses** → resource not valuable enough, question not engaging
- **M3 ignored** → pivot timing or messaging
- **No meetings booked** → CTA unclear, offer not compelling

## Advanced Tactics

### Tactic 1: LinkedIn Activity Trigger
Instead of time-based sequence, trigger Message 2 when prospect:
- Likes your post
- Comments on industry content
- Changes job
- Company announces news

### Tactic 2: Multi-Threading
If Message 3 gets ignored, find another person at the company and start new sequence.

### Tactic 3: Content Retargeting
After sending Message 2 resource, tag them in related LinkedIn post 2 days later. Keeps you top-of-mind.

### Tactic 4: Voice Note in Message 3
For high-value prospects, send 30-second LinkedIn voice note instead of text. Stands out + builds rapport faster.

## Integration with Other Channels

**LinkedIn + Email**:
- Day 0: LinkedIn connection request
- Day 2: Email with same value resource
- Day 5: LinkedIn Message 2
- Day 10: Email follow-up
- Day 14: LinkedIn Message 4

**LinkedIn + Phone**:
- After Message 2 response, offer: "Want to continue this over a quick call?"
- Converts 2x better than email → call

---

**Created**: 2025-11-25
**Version**: 1.0
**Part of**: Sales Prospecting Autopilot Pack
