# OTV2-20260811-reference-first-owner-baseline

```yaml
task_id: OTV2-20260811-reference-first-owner-baseline
title: Persist GAME-VISION-01 Reference-first owner baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-reference-first-owner-baseline
pr: 156
base_sha: ae5b6ff9feeb2a608583cd34e16675a1f4639299
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T09:31:00+02:00
updated_at: 2026-08-11T09:36:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-reference-first-owner-baseline.md
  - docs/architecture/GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
depends_on:
  - ADR-0010-reference-and-evolved-world-product-profiles.md
  - PRODUCT_DIRECTION_BASELINE.md
  - GAME-VISION-01_PREDECISION_ANALYSIS.md
  - GAME-VISION-01_PREDECISION_ECONOMY_SCARCITY_ADDENDUM.md
blocks: []
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the owner's current-session acceptance that the first externally evaluated Oteryn build is **Reference-first**. Record only the accepted launch/evaluation ordering and its direct implications; do not silently accept the remaining `GAME-VISION-01` product packet.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11 the owner explicitly answered `tak` to the proposed `Reference-first` direction for the first externally evaluated Oteryn build.
- `PROVEN`: ADR-0010 already permits Reference and Evolved product-profile families over one canonical engine/client/`protocol-oteryn` and does not require simultaneous launch.
- `PROVEN`: `GAME-VISION-01_PREDECISION_ANALYSIS.md` recommends Reference-first specifically to reduce ambiguity while the native runtime is not yet proven.
- `DERIVED`: the owner's acceptance resolves launch/evaluation ordering but does not resolve the whole GAME-VISION gate, exact Global Tibia target revision, long-term reference tracking cadence, PvP, solo/party emphasis, economy goals or evolved-system priorities.

## Acceptance criteria

- [x] Record first externally evaluated build as one Reference profile rather than Evolved-first or simultaneous Reference+Evolved.
- [x] Record that Evolved follows later from the same engine/client/protocol foundation rather than launching in parallel with the first external build.
- [x] Preserve one canonical engine/client/`protocol-oteryn`, distinct `WorldId` values and default cross-profile gameplay-value isolation.
- [x] Require the first external Reference build to name an immutable reference baseline/revision for reproducible claims, without selecting the exact Global Tibia version/date now.
- [x] Explicitly leave long-term pinned/continuous/hybrid tracking policy unresolved.
- [x] Explicitly leave the remaining GAME-VISION owner packet unresolved and do not claim full gate acceptance.
- [x] Include mandatory decision-timing, downstream-consumer, cost-of-delay and supersession-evidence records.
- [x] No runtime/client/server implementation, content production, public branding or production rollout is authorized.
- [ ] Exact-head self-review and repository-required CI pass before merge.

## Excluded scope

- no exact Global Tibia patch/date selection;
- no long-term Reference update cadence selection;
- no final public naming/branding or `1:1` claim;
- no player-promise/design-pillar acceptance beyond already accepted architecture;
- no first Evolved-system package selection;
- no PvP, solo/party, death/progression or economy/scarcity decision;
- no gameplay/content/runtime implementation.

## Implementation / findings

- Added `docs/architecture/GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md` as an owner-accepted partial GAME-VISION baseline.
- The baseline freezes only the first external profile order: one Reference profile first; no simultaneous Reference+Evolved first evaluation; Evolved follows later on the shared foundation.
- The evaluated Reference build must name an immutable baseline/revision so parity and regression evidence remain reproducible.
- The exact Global Tibia patch/date and long-term Reference tracking cadence remain unresolved.
- The baseline includes the required decision-timing record, named downstream consumers, cost-of-delay analysis and explicit supersession evidence.
- Full `GAME-VISION-01` remains `NOT ACCEPTED`; no gameplay/content/runtime implementation is authorized by this documentation.

## Validation

### Focused

- owner decision: current-session `USER_SOURCE`, explicit acceptance
- base: `main@ae5b6ff9feeb2a608583cd34e16675a1f4639299`
- open PRs at task start: 0
- pre-PR compare: `behind_by=0`, exactly two declared documentation paths
- authority classification: accepted partial launch-order baseline only; full gate explicitly remains NOT ACCEPTED

### Component/integration

- result: `NOT_APPLICABLE` — product architecture documentation only

### E2E

- result: `NOT_APPLICABLE` — no executable behavior

### Exact-head CI

- final head: pending this metadata checkpoint commit
- result: pending

## Self-review

- exact head: pending current checkpoint commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture review
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — this is a product launch-order decision and changes no authentication/session, protocol/wire, persistence/economy conservation, security, production or multichannel-authority semantics.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- PR: #156
- changed-file review: pending exact final head
- unresolved review threads: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Persisted the owner-accepted Reference-first launch-order baseline and opened PR #156 without accepting any remaining GAME-VISION decisions.
status: validating
branch: docs/OTV2-20260811-reference-first-owner-baseline
head_sha: null
pr: 156
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: final-head-pending
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
next_action: Freeze the exact final head, perform mandatory full-diff self-review, verify repository-required CI and squash-merge only if unchanged and clean.
```
