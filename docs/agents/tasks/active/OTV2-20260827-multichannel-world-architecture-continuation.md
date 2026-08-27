# OTV2-20260827-multichannel-world-architecture-continuation

```yaml
task_id: OTV2-20260827-multichannel-world-architecture-continuation
title: Continue owner decisions for multichannel world topology and housing
mode: CONTRACT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/multichannel-world-owner-checkpoint-20260827
issue: 220
pr: null
base_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn architecture continuation
worker_alias: "Oteryn: architektura"
created_at: 2026-08-27T22:11:06+02:00
updated_at: 2026-08-27T22:11:06+02:00
execution_budget_minutes: 240
large_budget_reason: owner-driven architecture sequence spans multichannel product semantics, cross-region topology, housing/economy/anti-duplication and explicit supersession analysis
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MULTICHANNEL_WORLD_OWNER_DECISION_CHECKPOINT_2026-08-27.md
  - docs/agents/tasks/active/OTV2-20260827-multichannel-world-architecture-continuation.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTV2-MULTICHANNEL-WORLD-OWNER-DECISIONS
external_repositories: []
```

## Outcome

Preserve the owner-selected multichannel/world/multi-region direction as a durable, explicitly non-final checkpoint and continue the remaining architecture decisions without losing provenance or silently contradicting existing accepted contracts.

## Architecture and source of truth

- `PROVEN`: admission protected main is `6e6e37852b7a050a1c7117ab2a9f316907d09daf`.
- `PROVEN`: `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` is reusable with short invocation `Oteryn: architektura`.
- `PROVEN`: current accepted `GAME-CHANNEL-01` treats `WorldId` as the persistent product/economy/community/ruleset boundary and `ChannelId` as a parallel simulation identity.
- `CONFLICT`: current accepted product wording also requires different profile/ruleset families to use distinct WorldIds / one inherited profile family across all Channels of one World, while the owner has selected Channel-scoped PvP mode as the intended direction.
- `OWNER_SELECTED_DIRECTION`: decisions and non-decisions are recorded in `docs/architecture/reviews/OTERYN_GAME_MULTICHANNEL_WORLD_OWNER_DECISION_CHECKPOINT_2026-08-27.md`.
- `UNKNOWN`: the correct final housing/auction/scarcity topology for a 1k/2k+ population and multi-region Channels.

## Acceptance criteria

- [x] Issue #220 records the exact owner-selected direction and known architecture conflict.
- [x] Durable architecture checkpoint records every selected decision from the current continuation session without inventing a housing solution.
- [ ] Housing receives a dedicated deep analysis covering scarcity, auctions, item authority, channel presence, ACL/ownership, cross-region failure and scalable alternatives.
- [ ] Owner selects or explicitly defers a housing model only after that analysis.
- [ ] Exact current-contract clauses affected by Channel-scoped PvP mode are identified before any canonical supersession.
- [ ] Remaining architecture horizon continues one material owner decision at a time, skipping already accepted decisions.
- [ ] Final coherent amendment package receives repository-required architecture/governance validation and independent review before canonical merge.

## Excluded scope

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas implementation. No permanent World Project/Bundle physical format selection. No implicit acceptance of apartments, duplicated houses, world-global house interiors or any other housing topology before the housing deep-dive.

## Implementation / findings

Current owner-selected direction:

- one durable World with many parallel Channels;
- repeated/different PvP modes may exist per Channel inside one World;
- cross-mode same-World move is an ordinary fenced Channel switch;
- Channel PvP mode is immutable for the ChannelId lifetime;
- initial XP/loot/spawn/progression economics are equal across modes;
- Channel selection remains player-controlled and exposes mode, occupancy/capacity, runtime health/load, hosting region and player-relative latency, with a non-binding recommendation;
- Channels of one World may be hosted in multiple geographic regions;
- initial durable topology uses one World DurableHomeRegion and one authoritative PostgreSQL write domain, while Channel simulation stays region-local and WAN-free in the authoritative tick;
- permanent World Project/Bundle format remains deferred pending representative real E2E evidence;
- housing remains unresolved and is the next required deep-dive.

## Validation

### Focused

- command/run: documentation consistency review against Issue #220 and current accepted architecture sources
- result: pending after PR creation

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture checkpoint only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted repository policy
- classification: architecture/documentation checkpoint
- result: pending

## Self-review

- exact head: pending
- method/reviewer: architecture continuation agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES before any merge that canonically supersedes accepted profile/channel semantics; checkpoint-only draft may remain open while analysis continues
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known at admission
- protected auto-merge: disabled while owner-decision sequence is incomplete
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: owner-selected World/Channel/PvP/multi-region direction persisted; housing explicitly held for deep analysis
status: investigating
branch: arch/multichannel-world-owner-checkpoint-20260827
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
blocker: housing model requires deeper analysis before owner disposition
next_action: perform the housing/auction/scarcity/multichannel deep-dive from current GitHub state, then present one decision question to the owner
```
