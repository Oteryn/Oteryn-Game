# OTV2-20260825-fnd04-verifier-allocation

```yaml
task_id: OTV2-20260825-fnd04-verifier-allocation
title: Allocate production FND-04 verifier consumer
mode: COORDINATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/allocate-fnd04-verifier-115
issue: 115
pr: null
base_sha: c1020b2db62ecfa18c411bee56fa004430b28923
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator for Issue #131
created_at: 2026-08-25T10:52:00+02:00
updated_at: 2026-08-25T10:52:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-close-next-wave-blockers.md
  - docs/agents/tasks/active/OTV2-20260825-fnd04-verifier-allocation.md
  - docs/agents/tasks/active/OTV2-20260825-fnd04-verifier-consumer.md
public_contracts:
  - docs/agents/prompts/OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md
depends_on:
  - issue:115
  - issue:128
  - issue:131
  - main:c1020b2db62ecfa18c411bee56fa004430b28923
blocks:
  - task:OTV2-20260825-fnd04-verifier-consumer
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Merge one docs-only coordinator allocation that grants Issue #115 a single bounded Foundation/Cargo implementation lease. The worker converts authenticated FND-04 material plus caller-provided current authoritative evidence into typed trusted facts; it does not implement a listener or consume production secrets.

## Acceptance criteria

- [ ] Worker owns only the exact module/Cargo/docs/task paths declared in live allocations.
- [ ] Shared Cargo/workspace lease is exclusive to #115 only after this allocation merges.
- [ ] Existing registry lease is released and no registry mutation is authorized.
- [ ] No listener, port, certificate/key material, production config/deployment, Platform or external-repository authority is granted.
- [ ] TDD and genuinely independent exact-head security review are mandatory before #115 merge.
- [ ] Governance, diff check, whole-diff self-review and exact-head `game-gate` pass before allocation merge.

## Excluded scope

No Server Seam listener, Durability/gameplay/client implementation, production KMS/HSM/key distribution, secrets, live account/session/data mutation, Platform write or external-repository write.
## Validation

- focused: `python tools/agents/validate_governance.py` — pending final allocation diff
- component: `git diff --check` — pending final allocation diff
- E2E: `NOT_APPLICABLE` — allocation only
- independent review: `NOT_REQUIRED` for allocation; the implementation worker requires independent exact-head security review

## Context checkpoint

```yaml
last_progress: Current main proves #93/#116/#123 closed; #115 is the sole remaining next-wave blocker and its exact implementation lease is being prepared.
status: implementing
branch: coord/allocate-fnd04-verifier-115
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
owner_action_required: null
blocker: null
next_action: Persist live allocation and worker task, validate the docs-only diff, then merge the allocation PR before any code/Cargo mutation.
```
