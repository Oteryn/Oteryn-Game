# OTV2-20260811-solo-viable-party-rewarded-owner-baseline

```yaml
task_id: OTV2-20260811-solo-viable-party-rewarded-owner-baseline
title: Persist GAME-VISION-01 solo-viable party-rewarded owner baseline
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-solo-viable-party-rewarded-owner-baseline
pr: 167
base_sha: 470f269d338e085f40910ca79e60a5a8d8ea4abf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T12:22:00+02:00
updated_at: 2026-08-11T12:27:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-solo-viable-party-rewarded-owner-baseline.md
  - docs/architecture/GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md
depends_on:
  - GAME-VISION-01_PLAYER_PROMISE_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_REFERENCE_HYBRID_TRACKING_OWNER_BASELINE.md
  - GAME-VISION-01_EVOLVED_RELIABILITY_UX_FIRST_OWNER_BASELINE.md
  - GAME-VISION-01_PVP_SECONDARY_PILLAR_OWNER_BASELINE.md
  - GAME-VISION-01_PREDECISION_ANALYSIS.md
  - ADR-0010-reference-and-evolved-world-product-profiles.md
blocks:
  - GAME-VISION-01 owner decision packet item 7 closure
  - GAME-CHAR-01 product-level solo/party progression assumption
  - party/shared-XP and encounter/content-sizing product intent before exact mechanics are frozen
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Persist the owner's explicit acceptance of the Oteryn product baseline **solo viable, party rewarded**: meaningful ordinary progression and normal sessions remain viable without permanent party dependence, while coordinated party play has real gameplay value.

## Architecture and source of truth

- `USER_SOURCE`: on 2026-08-11, after the recommendation was explained as “solo viable, party rewarded” — normal solo progression remains meaningful, while organized parties gain real benefits such as greater efficiency, access to harder coordinated goals, profession synergy and safety without forcing a party for every ordinary session — the owner answered `tak`.
- `PROVEN`: `GAME-VISION-01_PREDECISION_ANALYSIS.md` identifies relative solo/party importance as owner-decision packet item 7 and explicitly says the baseline is needed so `GAME-CHAR-01`, shared XP and content sizing do not guess.
- `PROVEN`: accepted Reference-first and hybrid-tracking baselines keep Reference mechanics tied to a named immutable parity target.
- `PROVEN`: accepted Evolved reliability/UX-first does not automatically authorize early systemic progression/party redesign.
- `PROVEN`: accepted PvP secondary-pillar baseline leaves detailed gameplay mechanics to their owning gates.
- `DERIVED`: the owner acceptance is product emphasis, not a numeric party-reward formula or a requirement that all content be solo-completable.

## Acceptance criteria

- [x] Record `solo viable, party rewarded` as the owner-accepted product-level solo/party emphasis.
- [x] Define solo viability as meaningful ordinary progression without permanent party dependence.
- [x] Define party reward as real cooperative gameplay value without selecting a specific bonus formula.
- [x] State that not every boss/quest/event/endgame objective must be soloable.
- [x] State that routine play must not require constant party formation.
- [x] Keep shared-XP formula, party bonus values, party-size/level/vocation rules and loot distribution unresolved.
- [x] Keep party finder, channel/co-location, instance admission and multibox/account policy unresolved in their owning gates.
- [x] Preserve Reference parity authority and explicit/versioned/measurable Evolved differences.
- [x] Preserve server authority, durable-value integrity, anti-duplication and abuse-resistant reward eligibility without inventing enforcement policy.
- [x] Include decision-timing, concrete blocked work, cost-of-delay and supersession-evidence records.
- [x] Keep full `GAME-VISION-01` explicitly `NOT ACCEPTED`.
- [x] No runtime/client/server/content/production implementation is authorized.
- [ ] Exact-head self-review and repository-required CI pass before merge.

## Excluded scope

This task must not:

- implement party, shared-XP, progression, combat, boss, quest, loot, client-social or content behavior;
- define numeric XP/loot/efficiency bonuses or penalties;
- define party size, level range, vocation composition or synergy formulas;
- require that all content be soloable;
- select which exact bosses/quests/hunts require or reward groups;
- define personal/shared loot policy;
- define party finder/matchmaking or invite/kick/leadership rules;
- define cross-channel co-location, preferred-channel assignment or channel-switch semantics;
- define instance ownership/admission rules;
- define multi-account/multibox policy;
- alter accepted Reference-first, hybrid-tracking, player-promise, Evolved reliability/UX-first or PvP secondary-pillar decisions;
- modify parallel draft PR #162 repository-engineering/governance paths;
- authorize production rollout.

## Implementation / findings

- Current `main` at task start: `470f269d338e085f40910ca79e60a5a8d8ea4abf`.
- Open PR at task start: draft #162 (`ci: enforce aggregate pull request merge gate`), owning disjoint repository-engineering/governance paths and based on an older main; this task does not modify those paths.
- Repository search found no existing owner baseline for `solo viable, party rewarded`.
- Read `GAME-VISION-01_PREDECISION_ANALYSIS.md`; owner-decision packet item 7 is explicitly `Solo/party emphasis` and exists to prevent `GAME-CHAR-01`, shared XP and content sizing from guessing.
- Added `docs/architecture/GAME-VISION-01_SOLO_VIABLE_PARTY_REWARDED_OWNER_BASELINE.md` as a separate owner-accepted partial baseline.
- Opened PR #167 with the mandatory `## Summary`, `## Scope` and `## Validation` metadata headings.
- The baseline distinguishes product intent from mechanics: solo viability does not mean every content item is soloable, and party reward does not imply a specific XP multiplier.
- Full `GAME-VISION-01` remains `NOT ACCEPTED`; no executable implementation or rollout is authorized.

## Validation

### Focused

- command/run: compare owner acceptance with `GAME-VISION-01_PREDECISION_ANALYSIS.md` item 7 and accepted Reference/player-promise/Evolved/PvP baselines; verify no existing solo/party owner baseline; inspect draft PR #162 for path collision; inspect PR #167 metadata and declared changed-file scope
- result: **PASS** for declared product scope before final-head freeze; exact-head self-review still required after this checkpoint commit

### Component/integration

- command/run: `NOT_APPLICABLE` — product/architecture/task documentation only; no executable component/integration behavior changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no runtime/client/server/content behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending this PR metadata checkpoint commit
- trigger source: `pull_request/synchronize`
- workflow/run/job: pending fresh generation for final candidate head
- runner assignment: pending
- classification: repository-required documentation checks applicable to exact PR head
- result: pending

## Audit

- scope: owner-authority fidelity, accidental shared-XP/party-bonus freeze, accidental all-content-solo requirement, Reference/Evolved conflict, GAME-CHANNEL authority leakage, reward-integrity weakening, task-template completeness and parallel ownership collision
- material findings: no material finding in focused pre-freeze review; exact-head audit pending
- unresolved findings: pending exact-head PR review-thread verification
- verdict: pending exact-head verification

## Self-review

- exact head: pending this checkpoint commit
- method/reviewer: implementing/coordinating agent full-diff product/architecture/governance review
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` under current risk policy — bounded product-priority documentation only; no authentication/session, protocol/wire, persistence/economy conservation, security, production configuration or executable multichannel authority semantics are changed
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending exact final head
- unresolved review threads: pending
- related/superseded PRs: parallel disjoint draft PR #162; no supersession
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending lifecycle archive after merge

## Context checkpoint

```yaml
last_progress: Persisted the bounded owner-accepted solo viable, party rewarded baseline and opened PR #167; parallel draft PR #162 remains disjoint and untouched.
status: validating
branch: docs/OTV2-20260811-solo-viable-party-rewarded-owner-baseline
head_sha: null
pr: 167
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
next_action: Freeze this checkpointed exact head, perform mandatory full-diff self-review, verify review threads/current main/parallel #162 state and repository-required CI, then squash-merge only if clean.
```
