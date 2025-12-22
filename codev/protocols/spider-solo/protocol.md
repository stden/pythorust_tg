# SPIDER-SOLO Protocol

## Prerequisites

**No external dependencies required**:
- SPIDER-SOLO is a single-agent variant and does not require Zen MCP
- All review and validation is performed via rigorous self-review
- Best when multi-agent infrastructure is unavailable

## Protocol configuration

### Self-review only (NO MULTI-AGENT CONSULTATION)

**DEFAULT BEHAVIOR:**
SPIDER-SOLO uses self-review and human approval only.

**KEY DIFFERENCE FROM SPIDER:**
- No multi-agent consultation at any checkpoint
- All review is performed via thorough self-review
- Strong emphasis on self-validation before presenting work to the user

**REVIEW APPROACH:**
- Self-review at every checkpoint where SPIDER would consult external agents
- Use critical thinking to find gaps, risks, and missing requirements
- Transparently document self-review results
- Rely on human feedback for final validation

## Overview

SPIDER-SOLO is a single-agent variant of the SPIDER protocol that emphasizes specification-driven development with iterative implementation and continuous self-review. It preserves the same structured approach, but without multi-agent interaction.

**Core principle**: Each feature is tracked through exactly THREE documents — a specification, a plan, and a review with lessons learned — all sharing the same filename and sequential identifier.

## When to use SPIDER-SOLO

### Use SPIDER-SOLO for:
- New feature development (when Zen MCP is unavailable)
- Architecture changes (single-agent review)
- Complex refactoring (self-validation)
- System design decisions (human approval)
- API design and implementation
- Performance optimization initiatives

### Skip SPIDER-SOLO for:
- Simple bug fixes (< 10 lines)
- Documentation-only updates
- Configuration changes
- Dependency updates
- Emergency hotfixes (but do a lightweight retrospective after)

## Protocol phases

### S - Specify (Design exploration with self-review)

**Purpose**: Thoroughly explore the problem space and solution options before committing to an approach.

**Workflow overview**:
1. User provides a prompt describing what they want built
2. Agent generates the initial specification document
3. **COMMIT**: "Initial specification draft"
4. Critical self-review of the specification
5. Agent updates the specification based on self-review
6. **COMMIT**: "Specification after self-review"
7. Human reviews and provides comments for changes
8. Agent applies changes and lists what was modified
9. **COMMIT**: "Specification with user feedback"
10. Final self-review of the updated document
11. Final updates based on self-review
12. **COMMIT**: "Final approved specification"
13. Iterate steps 7–12 until the user approves and says to proceed to planning

**Important**: Keep documentation minimal — use only THREE core files with the same name:
- `specs/####-descriptive-name.md` — the specification
- `plans/####-descriptive-name.md` — the implementation plan
- `reviews/####-descriptive-name.md` — review and lessons learned (created during Review phase)

**Process**:
1. **Clarifying questions** (ALWAYS START HERE)
   - Ask the user/stakeholder questions to understand the problem
   - Probe for hidden requirements and constraints
   - Understand the business context and goals
   - Identify what is in scope vs. out of scope
   - Continue asking until the problem is crystal clear

2. **Problem analysis**
   - Clearly articulate the problem being solved
   - Identify stakeholders and their needs
   - Document current state and desired state
   - List assumptions and constraints

3. **Solution exploration**
   - Generate multiple solution approaches (as many as appropriate)
   - For each approach, document:
     - Technical design
     - Trade-offs (pros/cons)
     - Estimated complexity
     - Risk assessment

4. **Open questions**
   - List all uncertainties that need resolution
   - Categorize as:
     - Critical (blocks progress)
     - Important (affects design)
     - Nice-to-know (optimization)

5. **Success criteria**
   - Define measurable acceptance criteria
   - Include performance requirements
   - Specify quality metrics
   - Document test scenarios

6. **Self-review (MANDATORY)**
   - **First self-review** (after initial draft):
     - Check: problem clarity, solution completeness, missing requirements, risks
     - Update specification with improvements
     - Document key findings and changes in a “Self-Review Notes” section
   - **Second self-review** (after human comments):
     - Validate changes, ensure alignment with feedback
     - Re-check completeness and edge cases
     - Update “Self-Review Notes” with additional findings

**⚠️ IMPORTANT**: Comprehensive self-review is required before proceeding.

**Output**: Single specification document in `codev/specs/####-descriptive-name.md`
- All self-review results incorporated directly into this document
- Include a “Self-Review Notes” section summarizing key issues found and what changed
- Version control captures evolution through commits
**Template**: `templates/spec.md`
**Review required**: Yes — human approval AFTER self-review

### P - Plan (Structured decomposition)

**Purpose**: Transform the approved specification into an executable roadmap with clear phases.

**⚠️ CRITICAL: No time estimates in the AI age**
- **NEVER include time estimates** (hours, days, weeks, story points)
- AI-driven development makes traditional time estimates meaningless
- Delivery speed depends on iteration cycles, not calendar time
- Focus on logical dependencies and phase ordering instead
- Measure progress by completed phases, not elapsed time
- The only valid metrics are: “done” or “not done”

**Workflow overview**:
1. Agent creates the initial plan document
2. **COMMIT**: "Initial plan draft"
3. Thorough self-review of the plan
4. Agent updates the plan based on self-review
5. **COMMIT**: "Plan after self-review"
6. User reviews and requests modifications
7. Agent updates the plan based on user feedback
8. **COMMIT**: "Plan with user feedback"
9. Final self-review of the updated plan
10. Final updates based on self-review
11. **COMMIT**: "Final approved plan"
12. Iterate steps 6–11 until agreement is reached

**Phase design goals**:
Each phase should be:
- A separate piece of work that can be committed as a unit
- A complete set of functionality
- Self-contained and independently valuable

**Process**:
1. **Phase definition**
   - Break work into logical phases
   - Each phase must:
     - Have a clear, single objective
     - Be independently testable
     - Deliver observable value
     - Be a complete unit that can be committed
     - End with evaluation discussion and a single commit
   - Note dependencies inline, for example:
     ```markdown
     Phase 2: API Endpoints
     - Depends on: Phase 1 (Database Schema)
     - Objective: Create /users and /todos endpoints
     - Evaluation: Test coverage, API design review, performance check
     - Commit: Will create a single commit after user approval
     ```

2. **Success metrics**
   - Define “done” for each phase
   - Include test coverage requirements
   - Specify performance benchmarks
   - Document acceptance tests

3. **Self-review (MANDATORY)**
   - **First self-review** (after plan creation):
     - Evaluate feasibility and phase breakdown
     - Check completeness and missing dependencies
     - Update the plan based on gaps found
   - **Second self-review** (after human review):
     - Verify adjustments match user feedback
     - Confirm approach remains sound
     - Final refinement of the plan

   **Note**: Self-review only — no multi-agent consultation in the SOLO variant

**⚠️ IMPORTANT**: Comprehensive self-review is required before moving on.

**Output**: Single plan document in `codev/plans/####-descriptive-name.md`
- Same filename as the specification, different directory
- All self-review results included directly
- Include phase status tracking within this document
- **DO NOT include time estimates** — focus on deliverables and dependencies, not hours/days
- Version control captures evolution through commits
**Template**: `templates/plan.md`
**Review required**: Yes — technical lead approval AFTER self-review

### (IDE) - Implementation loop

Execute for each phase in the plan. This is a strict cycle that must be completed in order.

**⚠️ MANDATORY**: The I-D-E cycle MUST be completed for EACH PHASE, not just at the end of all phases. Skipping D (Defend) or E (Evaluate) for any phase is a PROTOCOL VIOLATION.

**CRITICAL PRECONDITION**: Before starting any phase, verify the previous phase was committed to git. No phase can begin without the prior phase’s commit.

**Phase completion process**:
1. **Implement** — build the code for this phase
2. **Defend** — write comprehensive tests that guard functionality
3. **Evaluate** — assess and discuss with the user
4. **Commit** — single atomic commit for the phase (MANDATORY before next phase)
5. **Proceed** — move to next phase only after commit

**Handling failures**:
- If **Defend** reveals gaps → return to **Implement** to fix
- If **Evaluate** reveals unmet criteria → return to **Implement**
- If the user requests changes → return to **Implement**
- If fundamental plan flaws are found → mark phase as `blocked` and revise the plan

**Commit requirements**:
- Each phase MUST end with a git commit before proceeding
- Commit message format: `[Spec ####][Phase: name] type: Description`
- No work on the next phase until the current phase is committed
- If changes are needed after a commit, create a new commit with fixes

#### I - Implement (Build with discipline)

**Purpose**: Transform the plan into working code with high quality standards.

**Precondition**: Previous phase must be committed (verify with `git log`)

**Requirements**:
1. **Pre-implementation**
   - Verify previous phase is committed to git
   - Review the phase plan and success criteria
   - Set up the development environment
   - Create a feature branch following naming convention
   - Document any plan deviations immediately

2. **During implementation**
   - Write self-documenting code
   - Follow the project style guide strictly
   - Implement incrementally with frequent commits
   - Each commit must:
     - Be atomic (single logical change)
     - Include a descriptive message
     - Reference the phase
     - Pass basic syntax checks

3. **Code quality standards**
   - No commented-out code
   - No debug prints in final code
   - Handle all error cases explicitly
   - Include necessary logging
   - Follow security best practices

4. **Documentation requirements**
   - Update API documentation
   - Add inline comments for complex logic (only when needed)
   - Update README if needed
   - Document configuration changes

**Evidence required**:
- Link to commits
- Code review approval (if applicable)
- No linting errors
- CI pipeline pass link (build/test/lint)

**Self-review (MANDATORY)**:
- Perform a thorough self-review after implementation
- Focus: code quality, patterns, security, best practices
- Fix issues found during self-review before proceeding

#### D - Defend (Write comprehensive tests)

**Purpose**: Create comprehensive automated tests that safeguard intended behavior and prevent regressions.

**CRITICAL**: Tests must be written IMMEDIATELY after implementation, NOT retroactively at the end of all phases. This is MANDATORY.

**Requirements**:
1. **Defensive test creation**
   - Write unit tests for all new functions
   - Create integration tests for feature flows
   - Develop edge case coverage
   - Build error condition tests
   - Establish performance benchmarks (if required by the spec)

2. **Test validation** (ALL MANDATORY)
   - All new tests must pass
   - All existing tests must pass
   - No reduction in overall coverage
   - Performance benchmarks met (if applicable)
   - Security scans pass (if configured)
   - **Avoid overmocking**:
     - Test behavior, not implementation details
     - Prefer integration tests over unit tests with heavy mocking
     - Only mock external dependencies (APIs, databases, file systems)
     - Never mock the system under test itself
     - Use real implementations for internal module boundaries

3. **Test suite documentation**
   - Document test scenarios
   - Explain complex test setups
   - Note any flaky tests
   - Record performance baselines

**Evidence required**:
- Test execution logs
- Coverage report (show no reduction)
- Performance test results (if applicable per spec)
- Security scan results (if configured)
- CI test run link with artifacts

**Self-review (MANDATORY)**:
- Self-review test strategy after writing tests
- Focus: coverage completeness, edge cases, defensive patterns
- Add missing defensive tests based on self-review before proceeding
- Share self-review findings during the Evaluation discussion

#### E - Evaluate (Assess objectively)

**Purpose**: Verify the implementation fully satisfies the phase requirements and maintains system quality. This is where the critical discussion happens before committing the phase.

**Requirements**:
1. **Functional evaluation**
   - All acceptance criteria met
   - User scenarios work as expected
   - Edge cases handled properly
   - Error messages are helpful

2. **Non-functional evaluation**
   - Performance requirements satisfied
   - Security standards maintained
   - Code maintainability assessed
   - Technical debt documented

3. **Deviation analysis**
   - Document any changes from plan
   - Explain reasoning for changes
   - Assess impact on other phases
   - Update future phases if needed
   - **Overmocking check** (MANDATORY):
     - Verify tests focus on behavior, not implementation
     - Ensure at least one integration test per critical path
     - Check that internal module boundaries use real implementations
     - Confirm mocks are only used for external dependencies
     - Tests should survive refactoring that preserves behavior

4. **Final self-review before user evaluation** (MANDATORY)
   - Perform a thorough self-evaluation of the phase
   - Identify and fix remaining issues
   - **CRITICAL**: Reach a high confidence level in the implementation
   - Only proceed to user evaluation after self-validation
   - If doubts remain, address them first

5. **Evaluation discussion with user**
   - Present to user: “Phase X complete. Here’s what was built: [summary]”
   - Share test results and coverage metrics
   - Share self-review findings and confidence level
   - Ask: “Any changes needed before I commit this phase?”
   - Incorporate user feedback if requested
   - Get explicit approval to proceed

6. **Phase commit** (MANDATORY — NO EXCEPTIONS)
   - Create a single atomic commit for the entire phase
   - Commit message: `[Spec ####][Phase: name] type: Description`
   - Update the plan document marking this phase as complete
   - Push all changes to version control
   - Document any deviations or decisions in the plan
   - **CRITICAL**: Next phase CANNOT begin until this commit is complete
   - Verify commit with `git log` before proceeding

7. **Final verification**
   - Confirm all self-review findings were addressed
   - Verify all tests pass
   - Check that documentation is updated
   - Ensure no outstanding concerns from user

**Evidence required**:
- Evaluation checklist completed
- Test results and coverage report
- Self-review notes (implementation + tests)
- User approval from evaluation discussion
- Updated plan document with:
  - Phase marked complete
  - Evaluation discussion summary
  - Any deviations noted
- Git commit for this phase
- Final CI run link after all fixes

## 📋 Phase completion checklist (MANDATORY BEFORE NEXT PHASE)

**⚠️ STOP: DO NOT PROCEED TO NEXT PHASE UNTIL ALL ITEMS ARE ✅**

### Before starting ANY phase:
- [ ] Previous phase is committed to git (verify with `git log`)
- [ ] Plan document shows previous phase as `completed`
- [ ] No outstanding issues from previous phase

### After Implement phase:
- [ ] All code for this phase is complete
- [ ] Code follows project style guide
- [ ] No commented-out code or debug prints
- [ ] Error handling is implemented
- [ ] Documentation is updated (if needed)
- [ ] Self-review completed (critical analysis)
- [ ] Self-identified issues are fixed

### After Defend phase:
- [ ] Unit tests written for all new functions
- [ ] Integration tests written for critical paths
- [ ] Edge cases have test coverage
- [ ] All new tests are passing
- [ ] All existing tests still pass
- [ ] No reduction in code coverage
- [ ] Overmocking check completed (tests focus on behavior)
- [ ] Test coverage self-review completed
- [ ] Test gaps identified and closed

### After Evaluate phase:
- [ ] All acceptance criteria from spec are met
- [ ] Performance requirements satisfied
- [ ] Security standards maintained
- [ ] Thorough self-assessment completed
- [ ] High confidence in implementation achieved
- [ ] Evaluation discussion with user completed
- [ ] User has given explicit approval to proceed
- [ ] Plan document updated with phase status
- [ ] Phase commit created with proper message format
- [ ] Commit pushed to version control
- [ ] Commit verified with `git log`

### ❌ Phase blockers (fix before proceeding):
- Any failing tests
- Unaddressed self-review issues
- Missing user approval
- Uncommitted changes
- Incomplete documentation
- Coverage reduction
- Low confidence in the implementation

**REMINDER**: Each phase is atomic. You cannot start the next phase until the current phase is fully complete, tested, evaluated, and committed.

### R - Review/Refine/Revise (Continuous improvement)

**Purpose**: Ensure overall coherence, capture learnings, improve the methodology, and perform systematic review.

**Precondition**: All implementation phases must be committed (verify with `git log --oneline | grep '\\[Phase'`)

**Process**:
1. **Comprehensive review**
   - Verify all phases have been committed to git
   - Compare final implementation to original specification
   - Assess overall architecture impact
   - Review code quality across all changes
   - Validate documentation completeness

2. **Refinement actions**
   - Refactor code for clarity if needed
   - Optimize performance bottlenecks
   - Improve test coverage gaps
   - Enhance documentation

3. **Revision requirements** (MANDATORY)
   - Update README.md with any new features or changes
   - Update AGENTS.md and CLAUDE.md with protocol improvements from lessons learned
   - Update specification and plan documents with final status
   - Revise architectural diagrams if needed
   - Update API documentation
   - Modify deployment guides as necessary
   - **CRITICAL**: Update this protocol document based on lessons learned

4. **Systematic issue review** (MANDATORY)
   - Review entire project for systematic issues:
     - Repeated problems across phases
     - Process bottlenecks or inefficiencies
     - Missing documentation patterns
     - Technical debt accumulation
     - Testing gaps or quality issues
   - Document systematic findings in lessons learned
   - Create action items for addressing systematic issues

5. **Lessons learned** (MANDATORY)
   - What went well?
   - What was challenging?
   - What would you do differently?
   - What methodology improvements are needed?

6. **Review document creation**
   - Create a single review document in `codev/reviews/####-descriptive-name.md`
   - Same filename as the spec/plan; captures the review and learnings for this feature
   - Include recommendations for methodology improvements (update protocol if needed)

**Output**:
- Single review document in `codev/reviews/####-descriptive-name.md`
- Same filename as spec/plan; captures review and lessons learned
- Methodology improvement proposals (update the protocol if needed)

**Review required**: Yes — team retrospective recommended

## File naming conventions

### Specifications and plans
Format: `####-descriptive-name.md`
- Use sequential numbering (0001, 0002, etc.)
- Same filename under `specs/` and `plans/`
- Example: `0001-user-authentication.md`

## Status tracking

Status is tracked at the **phase level** inside plan documents, not at the document level.

Each phase in the plan must have a status:
- `pending`: Not started
- `in-progress`: In progress
- `completed`: Phase done and tested
- `blocked`: Cannot proceed due to external factors

## Git integration

### Commit message format

For spec/plan docs:
```
[Spec ####] <stage>: <description>
```

Examples:
```
[Spec 0001] Initial specification draft
[Spec 0001] Specification after self-review
[Spec 0001] Specification with user feedback
[Spec 0001] Final approved specification
```

For implementation phases:
```
[Spec ####][Phase: <phase-name>] <type>: <description>

<optional detailed description>
```

Example:
```
[Spec 0001][Phase: user-auth] feat: Add password hashing service

Implements bcrypt-based password hashing with configurable rounds.
```

### Branch naming
```
spider/####-<spec-name>/<phase-name>
```

Example:
```
spider/0001-user-authentication/database-schema
```

## Best practices

### During specification
- Use clear, unambiguous language
- Include concrete examples
- Define measurable success criteria
- Reference relevant resources

### During planning
- Keep phases small and focused
- Ensure each phase delivers value
- Note dependencies inline (no formal dependency mapping required)
- Include rollback strategies

### During implementation
- Follow the plan, but document deviations
- Maintain test coverage
- Keep commits atomic and well-described
- Update documentation as you go

### During review
- Compare against the original specification
- Document lessons learned
- Propose methodology improvements
- Update evaluation heuristics for future work

## Templates

Templates for each phase are available in the `templates/` directory:
- `spec.md` — specification template
- `plan.md` — planning template (includes phase status tracking)
- `review.md` — review + lessons learned template

**Remember**: Create only THREE documents per feature — spec, plan, and review — with the same filename in different directories.

## Protocol evolution

This protocol can be customized per project:
1. Fork the protocol directory
2. Modify templates and processes
3. Document changes in `protocol-changes.md`
4. Share improvements with the community
