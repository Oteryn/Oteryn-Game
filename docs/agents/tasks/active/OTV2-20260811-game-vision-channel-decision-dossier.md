# OTV2-20260811-game-vision-channel-decision-dossier

```yaml
task_id: OTV2-20260811-game-vision-channel-decision-dossier
title: Prepare GAME-VISION-01 and GAME-CHANNEL-01 owner decision dossiers
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-game-vision-channel-decision-dossier
pr: 152
base_sha: c1f115621acd7ba87fc47954f0e8b7d94f63e037
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T08:40:00+02:00
updated_at: 2026-08-11T08:56:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-game-vision-channel-decision-dossier.md
  - docs/architecture/GAME-VISION-01_PREDECISION_ANALYSIS.md
  - docs/architecture/GAME-VISION-01_PREDECISION_ECONOMY_SCARCITY_ADDENDUM.md
  - docs/architecture/GAME-CHANNEL-01_PREDECISION_ANALYSIS.md
  - docs/architecture/GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md
public_contracts: []
depends_on:
  - ADR-0010-reference-and-evolved-world-product-profiles.md
  - PRODUCT_DIRECTION_BASELINE.md
  - ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md
  - MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - FOUNDATION_PROGRAMME_CURRENT_STATUS.md
blocks: []
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories:
  - blakinio/canary # read-only comparative proposal inventory only
```

## Outcome

Produce nonbinding, owner-decision-ready architecture analyses for `GAME-VISION-01` and `GAME-CHANNEL-01`. The dossier separates accepted invariants from open product choices, evaluates realistic options, states risks/trade-offs, gives explicit recommendations, identifies the exact owner decisions that remain, and does not convert recommendations into accepted product policy.

## Architecture and source of truth

- `PROVEN`: ADR-0010 accepts Reference and Evolved profile families over one engine/client/`protocol-oteryn`, but does not require simultaneous launch.
- `PROVEN`: `PRODUCT_DIRECTION_BASELINE.md` accepts Global Tibia observable behavior as the initial reference direction while leaving exact parity target, launch sequence and first Oteryn differences unresolved.
- `PROVEN`: `ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` requires a minimum `GAME-VISION-01` before broad gameplay/content production and a dedicated `GAME-CHANNEL-01` before multichannel becomes a player-facing feature. It explicitly requires economy source/sink/scarcity goals and channel creation/removal/capacity-trigger policy.
- `PROVEN`: `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` already freezes system ownership such as channel-local simulation, world-shared market/social state, shared character durability and channel-switch safety gates; this task does not redesign those accepted boundaries.
- `PROVEN`: task base `main@c1f115621acd7ba87fc47954f0e8b7d94f63e037` had no open Oteryn-v2 PR.
- `PROVEN`: read-only comparative evidence from Blizzard documents sharding/layering trade-offs with persistent-world/event semantics; ESO support documents player-visible phasing/co-location failure and a regroup mechanism.
- `PROVEN`: `blakinio/canary` future-gameplay classification is a read-only proposal inventory, not accepted Oteryn architecture or a launch checklist.
- `DERIVED`: the missing work is product-policy selection and measurable launch scope, not another foundation technology choice.

## Acceptance criteria

- [x] `GAME-VISION-01_PREDECISION_ANALYSIS.md` contains problem, constraints, realistic options, trade-offs, risks, recommendation, future impact and mandatory decision-timing test.
- [x] Vision analysis gives a recommended launch sequencing and parity-baseline policy without marking it owner-accepted.
- [x] Vision dossier includes an explicit economy source/sink/scarcity decision frame and owner choice without guessing rates/prices/formulas.
- [x] Vision analysis identifies the minimum owner decisions necessary to unblock `GAME-CHAR-01`, broad gameplay/content work and later alpha milestone design.
- [x] `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` separates already accepted system ownership from unresolved player-facing channel policy.
- [x] Channel analysis covers assignment/co-location, visibility/manual switching, anti-hopping, spawn/resource multiplication, boss/event eligibility, PvP, social fragmentation, recovery and capacity lifecycle.
- [x] Channel dossier defines semantic create/drain/abort/remove trigger conditions while deferring numeric thresholds/hysteresis/orchestration to `PERF-01`/`OPS-CHANNEL-01`.
- [x] Channel analysis states which decisions block `VSL-MULTICHANNEL-01` versus which can remain deferred while single-channel vertical slices proceed.
- [x] Relevant external MMO observations are used only as comparative evidence, not copied as policy or implementation.
- [x] Existing disconnect/forensics active checkpoints are audited for lifecycle relevance; direct evidence does not justify archiving them in this task.
- [x] No runtime code, wire schema, persistence schema, production policy or owner-only gameplay rule is implemented or silently accepted.
- [ ] Repaired exact-head self-review and repository-required CI pass before merge.

## Excluded scope

- no owner decision on Reference versus Evolved launch;
- no exact Global Tibia patch/version selection;
- no final death/progression/economy/PvP formulas;
- no final numeric channel capacity thresholds or cooldown values;
- no runtime/client/server implementation;
- no Platform or other external-repository writes;
- no acceptance of GAME-VISION-01 or GAME-CHANNEL-01 through this analysis task.

## Implementation / findings

- Added `GAME-VISION-01_PREDECISION_ANALYSIS.md` with realistic launch-profile options, parity-baseline options, recommended Reference-first/hybrid-release-train path, candidate player promise, design pillars/anti-pillars, core-loop frame, success-measure categories and an owner decision packet.
- Added `GAME-CHANNEL-01_PREDECISION_ANALYSIS.md` with explicit separation between accepted ownership semantics and open player policy. It recommends soft-visible sticky channels, automatic assignment with social co-location preference, safe-state manual switching, server-authoritative anti-hopping eligibility, explicit encounter/reward scope, no PvP escape via channel switching, world-level social continuity and same-channel recovery semantics.
- Comparative evidence is deliberately non-normative. Blizzard's Classic layering discussion demonstrates that capacity partitioning can conflict with world-event semantics; ESO's phasing support demonstrates that invisible phase separation creates real co-location UX problems.
- Read-only `blakinio/canary` proposal classification confirms there is a large Evolved-feature inventory. The vision dossier explicitly rejects treating it as an initial launch checklist and recommends reliability/UX-first differences before broad systemic redesign.
- Audited `docs/agents/tasks/active/OTV2-20260807-disconnect-forensic-evidence-analysis.md` and `OTV2-20260807-lag-disconnect-protection-analysis.md` against their merged architecture packages and later FND-04/ANL work. They still preserve unresolved downstream questions including PvP disconnect semantics, post-grace behavior, detector/enforcement policy and remaining forensic policy/detail. They are therefore not proven stale or fully superseded; no lifecycle mutation is justified here.
- Initial self-review repaired task metadata only: PR #152 and the read-only Canary evidence repository were declared.
- After the first frozen head passed self-review and CI, an automatically triggered Codex review surfaced two useful P2 completeness findings. The merge was correctly blocked by unresolved conversations. Repair cycle 2 adds only nonbinding pre-decision completeness:
  - `GAME-VISION-01_PREDECISION_ECONOMY_SCARCITY_ADDENDUM.md` supplies the missing launch-level source/sink/scarcity options and ninth owner decision, recommending Reference-rule mechanical economy first while explicitly rejecting fabricated price/supply parity and deferring numeric economics;
  - `GAME-CHANNEL-01_PREDECISION_CAPACITY_TRIGGERS_ADDENDUM.md` freezes candidate semantic `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, low-load consolidation, unhealthy/recovery, drain-abort and terminal-remove predicates while leaving numbers and orchestration later.
- No accepted ADR, runtime, dependency, workflow or public contract is changed.

## Validation

### Focused

- source reconciliation: PASS against ADR-0010, product baseline, 2026-08-10 architecture refinements, multichannel scope matrix, current programme status and read-only comparative sources
- pre-repair exact head `8eefbcce9ffefb902882f2cd65612ad669211ff1`: exact-head self-review PASS and Agent Governance / Dependency Review / CodeQL all PASS, but terminal merge blocked by two newly surfaced review P2s; therefore that evidence is superseded for merge readiness
- repair scope: task metadata + two pre-decision addenda only; accepted architecture remains untouched
- authority classification: every analysis/addendum remains `PRE-DECISION ANALYSIS / NOT ACCEPTED`

### Component/integration

- result: `NOT_APPLICABLE` — architecture analysis only

### E2E

- result: `NOT_APPLICABLE` — no executable behavior

### Exact-head CI

- final head: pending repair-cycle-2 checkpoint commit
- result: pending

## Self-review

- pre-repair exact head `8eefbcce9ffefb902882f2cd65612ad669211ff1`: PASS, superseded by review-driven repair
- exact repaired head: pending current checkpoint commit
- method/reviewer: implementing/coordinating agent full-diff architecture review
- material findings: economy/scarcity completeness and semantic capacity-trigger completeness repaired; final recheck pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — this task remains explicitly nonbinding pre-decision analysis; it changes no accepted authority/security/protocol/persistence/product contract and cannot authorize implementation. Automatic Codex review activity is incidental and is not relied upon as a required gate, although its valid findings are repaired.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE` as a required independent gate
- verdict: `NOT_APPLICABLE`

## PR and closeout

- PR: #152
- changed-file review: pending repaired final head; intended scope now 5 declared paths
- unresolved review threads: two P2 threads pending verification/resolution after repaired head is visible
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Repaired two automatic-review completeness findings with nonbinding economy/scarcity and semantic channel-capacity trigger addenda; no owner decision or accepted architecture was changed.
status: validating
branch: docs/OTV2-20260811-game-vision-channel-decision-dossier
head_sha: null
pr: 152
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request/synchronize
ci_check_generation: repair-cycle-2-head-pending
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the repaired exact head, perform mandatory full-diff self-review, verify exact-head CI, resolve the two repaired conversations and squash-merge only if the unchanged head is clean.
```
