# OTV2-20260811-game-char-predecision-analysis

```yaml
task_id: OTV2-20260811-game-char-predecision-analysis
title: Prepare baseline-neutral GAME-CHAR-01 owner decision dossier
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-game-char-predecision-analysis
pr: 175
base_sha: 9510a93b024b92a761176b18373c8853c30a6617
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T17:03:00+02:00
updated_at: 2026-08-11T17:12:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-game-char-predecision-analysis.md
  - docs/architecture/GAME-CHAR-01_PREDECISION_ANALYSIS.md
public_contracts: []
depends_on:
  - docs/architecture/GAME-VISION-01_MINIMUM_OWNER_BASELINE.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
  - docs/architecture/DUR-01_DURABLE_IDENTIFIER_REPRESENTATION_CONTRACT.md
blocks:
  - product-owner decision on the baseline-neutral GAME-CHAR package
  - later Reference-sensitive GAME-CHAR closure on the exact first Reference baseline
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one nonbinding `GAME-CHAR-01` pre-decision dossier that separates decisions that can safely be made now from Reference-sensitive progression/name/death/creation decisions that must remain fail-closed until the exact first Reference baseline is selected.

## Architecture and source of truth

- `PROVEN`: current task base is `main@9510a93b024b92a761176b18373c8853c30a6617`.
- `PROVEN`: minimum `GAME-VISION-01` is `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` and makes `GAME-CHAR-01` the next product-sensitive gate.
- `PROVEN`: Character Authority owns CharacterId issuance, character aggregate lifecycle, current AccountId ownership, current WorldId membership and final name reservation; Platform remains AccountId authority/orchestrator and cannot directly mutate native character tables.
- `PROVEN`: CharacterId is a global durable UUIDv7, stable across rename/world transfer/account transfer and never reused.
- `PROVEN`: one AccountId may hold at most one authoritative online character; FND-04 owns GameSession/CharacterLease/control continuity.
- `PROVEN`: exact Reference gameplay semantics must not be guessed; concrete Reference decisions that cannot remain baseline-neutral are blocked on the exact named first Reference baseline.
- `DERIVED`: a staged GAME-CHAR closure is required: baseline-neutral lifecycle/aggregate/mutation/revision decisions may proceed now, while Reference-sensitive formulas/namespace/creation/death choices remain a hard-gated later package.
- `PROVEN`: open PR #162 remains disjoint CI/repository-governance work and is out of scope.

## Acceptance criteria

- [x] Inventory already accepted character identity/authority/session invariants and do not reopen them.
- [x] Separate baseline-neutral GAME-CHAR decisions from Reference-sensitive decisions that require the exact first Reference target.
- [x] Recommend a minimal character aggregate boundary that does not absorb inventory/economy/social/house/market authority.
- [x] Recommend a minimal lifecycle model and explicit deletion/restore/retirement semantics without guessing retention durations.
- [x] Recommend first-generation safe mutation rules for rename, deletion finalization, world transfer and account transfer without live-authority ambiguity.
- [x] Define progression facts versus derived-stat/revision semantics without freezing formulas or physical schema.
- [x] Define ruleset/profile migration and offline-progression boundaries without silent reinterpretation.
- [x] Map consequences to DUR-02, GAME-ITEM, combat, Platform projections and FND-04 without implementing them.
- [x] Produce a compact owner decision package for the baseline-neutral portion and name the exact remaining hard blocker for full GAME-CHAR closure.
- [x] Do not modify runtime, protocol, persistence schema, content, external repositories or accepted owner baselines.
- [ ] Perform exact-head self-review and repository-required documentation CI before merge.

## Excluded scope

This task does not:

- accept `GAME-CHAR-01` or any recommendation in the analysis;
- select the exact first Global Tibia Reference baseline;
- choose exact level/XP/skill/stat formulas, death penalties, blessings, vocation roster, skill list, creation choices or offline-training formulas;
- choose exact name namespace/recycling/redirect rules or slot/quota numbers where Reference parity matters;
- design PostgreSQL DDL, locks, indexes, outbox tables or migrations;
- implement Character Authority commands, client UI, gameplay runtime or Platform consumers;
- authorize world/account transfer as a launch feature;
- modify PR #162 or external repositories.

## Implementation / findings

- Added `docs/architecture/GAME-CHAR-01_PREDECISION_ANALYSIS.md` as a nonbinding dossier.
- The analysis concludes that full GAME-CHAR closure must be staged: Stage A baseline-neutral semantics may be owner-accepted now, while Stage B remains hard-blocked on the exact first Reference target wherever concrete character mechanics/durable vocabulary are target-sensitive.
- Recommended Stage A uses a bounded Character aggregate rather than a giant MMO aggregate; inventory/economy/social/house/market/session authorities remain separate.
- Recommended lifecycle is `ACTIVE -> DELETION_SCHEDULED -> RETIRED`, with restore before terminal retirement, CharacterId non-reuse and privacy erasure kept separate from semantic retirement.
- Recommended first-generation safety requires actor `ABSENT` and no playable CharacterLease before terminal retirement, world transfer or account ownership transfer; live transfer is deferred.
- Recommended semantic concurrency separates character-state revision/fencing from FND-04 GameSession/CharacterLease fencing.
- Progression is modeled as ruleset-defined authoritative facts versus derived values; exact formulas and persistent target-specific vocabulary remain blocked rather than guessed.
- The dossier makes name namespace, slot/quota values, creation choices, exact progression/death/vocation/offline-training rules and final Reference-sensitive DUR-02 fields explicit Stage-B work.
- No current accepted source was superseded and PR #162 remains disjoint.

## Validation

### Focused

- command/run: reconcile the dossier against accepted GAME-VISION, ADR-0012 / Character Authority Platform boundary, FND-ID character/account and single-online-character baselines, FND-04 session/lease authority, DUR-01 durability split and current programme ordering
- result: **PASS before final-head freeze**; no identity/authority conflict, Reference hard-gate bypass or runtime/schema authority leakage found

### Component/integration

- command/run: `NOT_APPLICABLE` — nonbinding architecture analysis only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable/player-visible runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending immutable PR/check evidence after final repository commit
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: documentation/governance validation
- result: pending

## Self-review

- exact head: pending immutable PR evidence after final repository commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: pending final-head review
- verdict: pending

## Independent review

- required: `NO` under trusted-base risk policy — final intended diff is nonbinding architecture/task analysis only and changes no accepted security/protocol/persistence/multichannel authority/product rule
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: exactly two declared documentation paths; final-head verification pending
- unresolved review threads: 0 before final freeze
- related/superseded PRs: PR #162 parallel/disjoint; no supersession
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Baseline-neutral GAME-CHAR predecision dossier is in draft PR #175; focused reconciliation passed and the package is ready for final-head self-review and documentation CI.
status: validating
branch: docs/OTV2-20260811-game-char-predecision-analysis
head_sha: null
pr: 175
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
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
next_action: Freeze the final PR head, perform exact-head self-review and documentation CI, merge/archive the nonbinding dossier, then request the product-owner Stage-A decision.
```
