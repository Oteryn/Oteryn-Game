# OTV2-20260825-fnd04-verifier-allocation

```yaml
task_id: OTV2-20260825-fnd04-verifier-allocation
title: Allocate production FND-04 verifier consumer
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/allocate-fnd04-verifier-115
issue: 115
pr: 145
base_sha: c1020b2db62ecfa18c411bee56fa004430b28923
head_sha: 43ddf1001fcf82c11afc1b38bbcd734e85c29ea4
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT coordinator for Issue #131
created_at: 2026-08-25T10:52:00+02:00
updated_at: 2026-08-25T12:33:00+02:00
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

- [x] Worker owns only the exact module/Cargo/docs/task paths declared in live allocations.
- [x] Shared Cargo/workspace lease is exclusive to #115 only after this allocation merges.
- [x] Existing registry lease is released and no registry mutation is authorized.
- [x] No listener, port, certificate/key material, production config/deployment, Platform or external-repository authority is granted.
- [x] TDD and genuinely independent exact-head security review are mandatory before #115 merge.
- [x] Governance, diff check and whole-diff self-review passed on allocation head `43ddf1001fcf82c11afc1b38bbcd734e85c29ea4`; exact-head CI generation for that head also completed successfully before this handoff metadata commit.

## Excluded scope

No Server Seam listener, Durability/gameplay/client implementation, production KMS/HSM/key distribution, secrets, live account/session/data mutation, Platform write or external-repository write.

## Validation

- focused: `python tools/agents/validate_governance.py` — PASS (25 required policy documents / 9 lanes)
- component: `git diff --check` — PASS
- whole-diff allocation review — PASS; exactly four coordinator/task documentation paths, no code/Cargo mutation
- E2E: `NOT_APPLICABLE` — allocation only
- independent review: `NOT_REQUIRED` for allocation; the implementation worker requires a genuinely independent exact-head security review
- prior exact-head CI on `43ddf1001fcf82c11afc1b38bbcd734e85c29ea4`: Agent governance run `32837590459` SUCCESS; Architecture semantic audit `32837590474` SUCCESS; Merge authority audit `32837590490` SUCCESS; Merge gate `32837590463` SUCCESS
- note: this handoff metadata commit moves PR #145 head, so the next agent must require the new exact-head CI generation to complete before merging PR #145.

## Handoff

Current durable state:

- resource blocker lifecycle is terminal on `main@c1020b2db62ecfa18c411bee56fa004430b28923`;
- Issues #93, #116 and #123 are CLOSED after a current-main mechanical recheck;
- Issue #115 is the sole remaining next-wave blocker;
- allocation PR #145 is OPEN on `coord/allocate-fnd04-verifier-115`;
- no FND-04 production code or Cargo dependency has been written yet, intentionally preserving TDD;
- worker task `OTV2-20260825-fnd04-verifier-consumer.md` already records the exact authorized implementation paths, exclusions, required tests and independent-review obligation.

## Context checkpoint

```yaml
last_progress: PR #145 is open for the sole remaining blocker #115. Allocation content is complete and the previous exact-head generation was green; this metadata commit records the handoff and requires fresh exact-head CI before merge.
status: validating
branch: coord/allocate-fnd04-verifier-115
head_sha: 43ddf1001fcf82c11afc1b38bbcd734e85c29ea4
pr: 145
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request_145
ci_check_generation: prior_head_43ddf100
ci_checks_for_current_head: 0
owner_action_required: null
blocker: null
next_action: Wait for the new exact-head PR #145 CI generation created by this metadata commit. If all required checks including game-gate are green and review threads are empty, squash-merge PR #145; then create `feat/fnd04-verifier-consumer-115` from the exact allocation merge SHA, baseline-test it, and begin implementation with a failing verifier test before any production code.
```
