# OTV2-20260825-next-wave-limit-decisions

```yaml
task_id: OTV2-20260825-next-wave-limit-decisions
title: Accept evidence-backed first-slice next-wave resource limits
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/next-wave-limit-decisions-133
issue: 133
pr: 140
base_sha: 86653375231febbf81623b4c6984a6ff1263bdc2
allocation_pr: 137
allocation_merge_sha: 86653375231febbf81623b4c6984a6ff1263bdc2
head_sha: c1fcf8aaa428028659762b9a6359819c81997a7a
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT decision/evidence worker for Issue #133
created_at: 2026-08-25T01:06:00+02:00
updated_at: 2026-08-25T09:50:00+02:00
execution_budget_minutes: 120
large_budget_reason: One reproducible evidence harness must classify and justify exact first-slice limits across three blocker issues without conflating semantic domains.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260825-next-wave-limit-decisions.md
  - tools/next-wave-limit-evidence/**
  - docs/agents/evidence/OTV2-20260825-next-wave-limit-evidence.json
  - docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_FIRST_SLICE_LIMITS_DECISION_2026-08-25.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_FIRST_SLICE_LIMITS_DECISION_2026-08-25.md
depends_on:
  - issue:93
  - issue:116
  - issue:123
  - issue:131
blocks:
  - issue:93
  - issue:116
  - issue:123
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A reproducible checked-accounting harness and reviewed decision baseline identify exact conservative first-slice maxima for Ability, Interaction, AI and TCP/TLS listener resource handling, while the journal-only Durability child explicitly excludes every DUR03 transaction row fail-closed. Canonical registration remains a separately serialized task.

## Architecture and source of truth

- `PROVEN`: Issue #133 is the child task authorized by Issue #131/#128.
- `PROVEN`: #93, #116 and #123 own the unresolved resource decisions and prohibit silent reuse of generic Foundation envelopes.
- `PROVEN`: this task owns no canonical registry, Cargo/workspace or product-code path.
- `DERIVED`: Movement-only semantic limits require a dedicated non-current successor because no exact Movement executable slice is allocated.
- `PROVEN`: exact worker base is allocation merge `86653375231febbf81623b4c6984a6ff1263bdc2`; branch `arch/next-wave-limit-decisions-133` was created from that SHA.

## Acceptance criteria

- [x] A Rust/std-only harness observes RED before the checked model exists, then GREEN at every exact maximum/max+1 boundary.
- [x] Evidence JSON records units, checked equations, representative retained-state/work costs and deterministic outcomes.
- [x] Every exercised #93/#116/#123 inventory row has one exact candidate; every omitted row is explicitly fail-closed `NOT_APPLICABLE_TO_FIRST_SLICE`.
- [x] Movement unresolved rows move to a dedicated successor issue without releasing Movement implementation.
- [x] No canonical registry, product code, production tuning/default, port/certificate/key/deployment or gameplay balance value changes.
- [ ] Whole-diff review, governance, semantic audits, exact-head `game-gate` and merge pass.

## Excluded scope

`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, Cargo/workspace/lockfile, Foundation/product code, listener bind/runtime, Durability/gameplay/client implementation, production sizing/defaults, Reference parity, secrets, deployment and external-repository write.

## Implementation / findings

Allocation PR #137 is merged and the exact-base worker branch exists. The harness now records 24 accepted candidate ceilings, 33 explicit fail-closed exclusions, exact inherited `FND02-WIRE-FRAME-BYTES` use for listener ingress, and Movement successor #139. Evidence SHA-256 is `2d6db30655c33408b8e48f80b827f864e4c7efd331ebc5cc37437a1d1851d0da`. Candidate values remain non-authoritative until this decision PR merges.

## Validation

### Focused

- command/run: `rustc --edition 2024 -D warnings --test tools/next-wave-limit-evidence/main.rs -o <temp-test> && <temp-test>`
- result: `PASS` — 11/11 tests; prior RED was observed independently before each generic boundary/overflow/domain/exclusion/metadata/margin/JSON/stress implementation step.

### Component/integration

- command/run: optimized harness emission + `--stress 64` + Python JSON invariants + `cargo run -q -p oteryn-architecture-check -- workspace .` + governance + diff check
- result: `PASS` — stress accepted/rejected 1536/1536 with 1 MiB peak fixture and checksum `14695981039346656037`; candidate records include allocation-impact/boundary/evidence-basis fields; evidence SHA-256 `2d6db30655c33408b8e48f80b827f864e4c7efd331ebc5cc37437a1d1851d0da`; workspace-boundaries/governance/diff all pass.

### E2E

- scenario: `NOT_APPLICABLE` — evidence/decision task creates no runtime path
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: contract/evidence only
- result: pending

## Self-review

- exact head: pending final PR metadata commit
- method/reviewer: implementing decision/evidence worker, complete changed-file diff plus literal Issue #93/#116/#123 acceptance re-check
- material findings: `P2=2`, both repaired before freeze: harness required `rustfmt`; evidence records lacked literal `allocation_impact`/`boundary_tests`/`evidence_basis` handoff fields. Post-repair coverage: Ability 19/19, Interaction 7/7, AI 16/16, #116 all required dimensions, #123 8/8 explicit exclusions; no unauthorized paths.
- verdict: `PASS`

## Independent review

- required: NO — this task selects bounded safety ceilings but changes no executable security/durable/runtime authority; protected semantic audits remain mandatory
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after decision/evidence merge and handoff to serialized registry task

## Context checkpoint

```yaml
last_progress: Decision/evidence PR #140 opened from rebased semantic head c1fcf8a; 24 candidates + 33 exclusions and Movement successor #139 are complete; final metadata head is being frozen for exact-head CI.
status: implementing
branch: arch/next-wave-limit-decisions-133
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze this metadata commit as final PR head, rerun governance/diff, then require exact-head GitHub CI including game-gate before squash merge.
```
