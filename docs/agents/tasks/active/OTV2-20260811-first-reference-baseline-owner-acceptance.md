# OTV2-20260811-first-reference-baseline-owner-acceptance

```yaml
task_id: OTV2-20260811-first-reference-baseline-owner-acceptance
title: Persist owner-accepted first Reference baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-first-reference-baseline-owner-acceptance
pr: 181
base_sha: 3106590eee22227b81228d2930b5d90e2053c9e9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T22:09:00+02:00
updated_at: 2026-08-11T22:23:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-first-reference-baseline-owner-acceptance.md
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
public_contracts:
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_DECISION_DOSSIER.md
  - docs/architecture/GAME-VISION-01_MINIMUM_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
  - docs/architecture/GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md
  - docs/architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md
blocks:
  - canonical recording of the owner-selected first Reference behavior cut
  - GAME-CHAR-01 Stage-B evidence reconciliation
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the product owner's explicit acceptance of the complete recommended package in section 21 of `GAME-VISION-01_FIRST_REFERENCE_BASELINE_DECISION_DOSSIER.md`.

## Source of truth

- `PROVEN`: trusted base is `main@3106590eee22227b81228d2930b5d90e2053c9e9`.
- `USER_SOURCE`: on 2026-08-11 at 22:09 +02:00 the product owner explicitly answered `Tak` to the complete recommended first-Reference-baseline package.
- `PROVEN`: the accepted target is Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary, not a proprietary source/binary snapshot and not a continuously moving upstream target.
- `PROVEN`: target selection and evidence completeness remain separate; unknown mechanics remain `UNKNOWN` until evidenced.
- `PROVEN`: accepting the target unblocks evidence-backed GAME-CHAR Stage B but does not accept Stage B, a physical schema, runtime/content implementation or production rollout.
- `PROVEN`: open PR #162 owns disjoint CI/repository-governance paths and is out of scope.

## Acceptance criteria

- [x] Added canonical owner baseline accepting the complete ten-item package from dossier section 21.
- [x] Recorded the 2026-07-28 post-server-save behavior cut as immutable first Reference target semantics.
- [x] Preserved evidence states `PROVEN`, `OBSERVED`, `DERIVED`, `UNKNOWN`, `CONFLICT`, `DECLARED_DIFFERENCE` and the accepted evidence-source hierarchy.
- [x] Preserved the rule that patch-note/search absence is not evidence of no upstream change.
- [x] Preserved lawful/provenance-safe native implementation and security/integrity/legal overrides.
- [x] Updated current status/register/horizon so the target is no longer shown as deferred/hard-blocking.
- [x] Kept overall `GAME-CHAR-01` `PROPOSED / PLANNED / NOT_STARTED`; Stage B becomes unblocked but remains unaccepted.
- [x] Kept final character-bearing DUR-02 schema blocked on accepted Stage B/full GAME-CHAR closure.
- [x] Chose no final Reference revision naming syntax, Stage-B mechanics/formulas or implementation.
- [x] Kept PR #162 and external repositories untouched.
- [ ] Perform full exact-head self-review and repository-required documentation CI before merge.

## Implementation / findings

- Added `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` with the owner-selected immutable 2026-07-28 behavior cut and the complete evidence/provenance package.
- Synchronized current status, global register and gameplay/product horizon so the exact-target prerequisite is satisfied while GAME-CHAR Stage B remains unaccepted.
- `GAME-VISION-01` delivery status is `OPEN` only for this concrete owner-baseline delivery and must return to `LIFECYCLE_CLOSED` during lifecycle closeout; implementation remains `NOT_STARTED`.
- Bounded DUR-02 discovery may use the target only as question/compatibility context before Stage-B acceptance; it gains no final schema authority.
- Full-diff review found two material register regressions before final freeze: the first edit accidentally removed the existing DUR-03 rule `Define item instance identity and ownership.` and introduced noncanonical status text `ACCEPTED PARTIAL SCOPE`. Repair cycle 1 restored the DUR-03 rule and represented Stage-A partial scope with ordinary `ACCEPTED` status plus explicit scope wording, without changing owner-approved semantics.
- No runtime/client/protocol/physical-schema/content/Platform/production authority is created.

## Excluded scope

This task does not:

- accept GAME-CHAR Stage B or the complete GAME-CHAR gate;
- infer unknown July-28 mechanics from current Global behavior without continuity evidence;
- copy proprietary code/protocol/assets or promote Canary/OTS code to Global authority;
- create final Reference revision identifier syntax;
- create physical database schema, runtime/client/protocol/content implementation or production behavior;
- modify governance/CI or external repositories.

## Validation

### Focused

- command/run: reconcile owner baseline against the delivered dossier, accepted GAME-VISION Reference-first/hybrid/parity-precedence baselines, GAME-CHAR Stage A and `ARCHITECTURE_STATUS_MODEL.md`
- result: **PASS after repair cycle 1**; owner target, evidence hierarchy, GAME-CHAR/DUR boundaries and no-runtime authority are preserved

### Component/integration

- command/run: `NOT_APPLICABLE` — paper-only product/architecture owner baseline
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable/player-visible behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending immutable PR head after this bookkeeping commit
- trigger source: pending
- workflow/run/job: pending
- classification: documentation/governance validation
- result: pending

## Self-review

- exact head: pending immutable PR head after this bookkeeping commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: `2` repaired before final freeze; final-head findings pending
- verdict: pending final-head review

## Independent review

- required: `NO` under trusted-base risk policy unless final review discovers unusual/high-risk semantic expansion; this package records an explicit owner-approved paper-only product/evidence target and changes no executable security/protocol/persistence/production behavior
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- delivery PR: #181
- changed-file review: five declared documentation paths; exact-head review pending
- unresolved review threads: pending
- related PR: #162 parallel/disjoint
- merge result: pending
- ownership release: pending lifecycle closeout

## Context checkpoint

```yaml
last_progress: Owner-accepted first Reference target and evidence model are in PR #181; repair cycle 1 corrected two register regressions while preserving the owner-approved 2026-07-28 target and GAME-CHAR Stage-B boundary.
status: validating
branch: docs/OTV2-20260811-first-reference-baseline-owner-acceptance
pr: 181
final_head_sha: null
final_head_frozen_at: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
runner_assignment_state: unknown
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: null
next_action: Freeze this PR head, repeat full-diff self-review on the exact head, run exact-head documentation CI, merge only if all gates pass, then archive the task and return GAME-VISION delivery status to LIFECYCLE_CLOSED.
```
