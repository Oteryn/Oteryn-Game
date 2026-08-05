# OTV2-20260805-game-intelligence-architecture

```yaml
task_id: OTV2-20260805-game-intelligence-architecture
title: Accept Oteryn Game Intelligence, analytics and audit architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-intelligence-architecture-20260805
pr: "#17"
base_sha: 521531a38ad98845a1147d6220c204ddb88e4911
head_sha: null
owner: GPT-5.6 Thinking
created_at: 2026-08-05T14:11:00+02:00
updated_at: 2026-08-05T14:35:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
  - docs/agents/tasks/active/OTV2-20260805-game-intelligence-architecture.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
public_contracts:
  - docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
depends_on:
  - ADR-0001 through ADR-0005
  - Canary Gameplay Analytics evidence at PRs #30 and #330
  - Otheryn OAM-048 disposition at PR #109
blocks:
  - detailed ANL-01 through ANL-04 package execution until this direction is accepted
cross_repository_coordination_id: OTV2-GAME-INTELLIGENCE
external_repositories:
  - blakinio/canary (read-only evidence)
  - blakinio/Otheryn (read-only evidence)
```

## Outcome

Accept and durably record a first-class Oteryn Game Intelligence architecture covering gameplay/balance analysis, world/content analysis, economy and item integrity, security analytics and a read-only AI investigation layer without implementing runtime code.

## Architecture and source of truth

- `PROVEN`: Canary PR #30 merged as `684ff1b520a5d296bc4018e32fb9e1c068cea0b6` and implemented the original optional Gameplay Analytics subsystem.
- `PROVEN`: Canary PR #330 merged as `d061dbe72265c89df9ab683717b18b598a106964` and recorded the Oteryn handoff and limits.
- `PROVEN`: Otheryn PR #109 merged as `a6e2993ed32b1316168045ad0b97ddebb50a2128` and classified direct migration as `EXPERIMENTAL_ONLY`.
- `DERIVED`: Oteryn should preserve the proven batching, idempotency, schema, retention and test patterns while replacing Lua hooks and best-effort anti-duplication assumptions with explicit Rust events and atomic audit/outbox evidence.

## Acceptance criteria

- [x] Game Intelligence is defined as a first-class umbrella subsystem.
- [x] Operational metrics, best-effort gameplay telemetry and durable economy/security audit are separated.
- [x] Gameplay/balance, world/content, economy/item, security and read-only AI responsibilities are explicit.
- [x] A common event-envelope vocabulary and item/currency event horizon are recorded without freezing the final IDL.
- [x] Privacy, pseudonymization, access, retention and prohibited AI actions are explicit.
- [x] Canary reuse/adapt/rewrite/reject decisions are recorded.
- [x] Stable gates `ANL-01` through `ANL-04` and dependencies on existing gates are registered.
- [ ] Governance validation passes on the exact final head.
- [ ] Independent audit has zero open material findings.
- [ ] PR is squash-merged and this task is archived separately.

## Excluded scope

- No Rust workspace or crate creation.
- No event schema/IDL implementation.
- No PostgreSQL schema, outbox or analytical datastore implementation.
- No runtime collector, dashboard, detector, enforcement or AI agent.
- No writes to Canary, Otheryn, Platform or otclient.
- No production data collection or access-policy activation.

## Implementation / findings

- The prior Canary subsystem is retained as design and regression evidence, not copied as the target runtime.
- Anti-duplication prevention remains owned by `DUR-03`; analytics supplies independent evidence and monitoring.
- Durable item/currency/security records cannot silently degrade to best-effort telemetry.
- Investigation and AI remain outside authoritative runtime and are read-only by contract.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head GitHub Actions evidence

### Component/integration

- result: `NOT_APPLICABLE` — architecture/documentation package only; no executable product component changed.

### E2E

- result: `NOT_APPLICABLE` — no runtime behavior was introduced.

### Exact-head CI

- head: pending
- workflow/run: `Agent governance`
- result: pending

## Independent audit

- exact head: pending
- method/auditor: adversarial full-diff architecture review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #17 contains ADR-0006, ANL-01 through ANL-04 integration, named failure scenarios and repaired audit findings.
status: validating
branch: docs/game-intelligence-architecture-20260805
head_sha: null
pr: "#17"
ci_check_generation: null
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Run exact-head Agent governance, complete the final full-diff audit and close PR #17 through squash merge and separate archive.
```
