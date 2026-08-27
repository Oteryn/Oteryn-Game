# OTV2-20260827-multichannel-world-architecture-continuation

```yaml
task_id: OTV2-20260827-multichannel-world-architecture-continuation
title: Continue owner decisions for multichannel world topology and housing
mode: CONTRACT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/anti-bot-player-reporting-20260827
issue: 220
pr: null
base_sha: 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn architecture continuation
worker_alias: "Oteryn: architektura"
created_at: 2026-08-27T22:11:06+02:00
updated_at: 2026-08-27T23:40:00+02:00
execution_budget_minutes: 240
large_budget_reason: owner-driven architecture sequence spans multichannel product semantics, cross-region topology, housing/economy/anti-duplication, Rested progression, anti-automation reporting and explicit supersession analysis
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_MULTICHANNEL_WORLD_OWNER_DECISION_CHECKPOINT_2026-08-27.md
  - docs/architecture/reviews/OTERYN_GAME_ANTI_AUTOMATION_PLAYER_REPORTING_OPEN_ANALYSIS_2026-08-27.md
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

- `PROVEN`: current continuation admission protected main is `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`, containing merged PR #221.
- `PROVEN`: `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` is reusable with short invocation `Oteryn: architektura`.
- `PROVEN`: current accepted `GAME-CHANNEL-01` treats `WorldId` as the persistent product/economy/community/ruleset boundary and `ChannelId` as a parallel simulation identity.
- `CONFLICT`: current accepted product wording also requires different profile/ruleset families to use distinct WorldIds / one inherited profile family across all Channels of one World, while the owner has selected Channel-scoped PvP mode as the intended direction.
- `OWNER_SELECTED_DIRECTION`: World/Channel/PvP/multi-region decisions and Rested/anti-automation open analysis are recorded in `docs/architecture/reviews/OTERYN_GAME_MULTICHANNEL_WORLD_OWNER_DECISION_CHECKPOINT_2026-08-27.md`.
- `OPEN_ANALYSIS`: player anti-bot reporting, malicious-report resistance and report-driven evidence collection are recorded in `docs/architecture/reviews/OTERYN_GAME_ANTI_AUTOMATION_PLAYER_REPORTING_OPEN_ANALYSIS_2026-08-27.md`.
- `UNKNOWN`: the correct final housing/auction/scarcity topology for a 1k/2k+ population and multi-region Channels.

## Acceptance criteria

- [x] Issue #220 records the exact owner-selected direction and known architecture conflict.
- [x] Durable architecture checkpoint records selected World/Channel/PvP/multi-region direction without inventing a housing solution.
- [x] Rested XP / `EligibleRawXP` / anti-automation interaction is preserved as explicitly open analysis rather than canonical runtime policy.
- [x] Player-report anti-bot direction is preserved with `report != guilt`, malicious/mass-report protection, reporter-independence analysis and UUIDv7/economy-provenance integration as open analysis.
- [ ] Housing receives a dedicated deep analysis covering scarcity, auctions, item authority, channel presence, ACL/ownership, cross-region failure and scalable alternatives.
- [ ] Owner selects or explicitly defers a housing model only after that analysis.
- [ ] Exact current-contract clauses affected by Channel-scoped PvP mode are identified before any canonical supersession.
- [ ] Remaining architecture horizon continues one material owner decision at a time, skipping already accepted decisions.
- [ ] Final coherent amendment package receives repository-required architecture/governance validation and independent review before canonical merge.

## Excluded scope

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas implementation. No anti-cheat product selection or anti-bot enforcement implementation. No permanent World Project/Bundle physical format selection. No implicit acceptance of apartments, duplicated houses, world-global house interiors or any other housing topology before the housing deep-dive.

## Implementation / findings

Current owner-selected / preserved direction:

- one durable World with many parallel Channels;
- repeated/different PvP modes may exist per Channel inside one World;
- cross-mode same-World move is an ordinary fenced Channel switch;
- Channel PvP mode is immutable for the ChannelId lifetime;
- initial XP/loot/spawn/progression economics are equal across modes;
- Channel selection remains player-controlled and exposes mode, occupancy/capacity, runtime health/load, hosting region and player-relative latency, with a non-binding recommendation;
- Channels of one World may be hosted in multiple geographic regions;
- initial durable topology uses one World DurableHomeRegion and one authoritative PostgreSQL write domain, while Channel simulation stays region-local and WAN-free in the authoritative tick;
- permanent World Project/Bundle format remains deferred pending representative real E2E evidence;
- housing remains unresolved and is the next required deep-dive;
- Rested is being analyzed as a positive bonus-XP-denominated pool anchored to per-Character `EligibleRawXP`, not a minute timer or low-stamina punishment;
- anti-automation detection is separate from Rested reward accounting and may consume neutral `EligibleRawXP`, behavioural telemetry and UUIDv7 value provenance;
- player reports are investigation/prioritization signals only and must be protected against spam, collusion, guild/PvP weaponization and malicious mass-reporting.

## Validation

### Focused

- command/run: documentation consistency review against Issue #220, merged PR #221 and current accepted architecture sources
- result: pending on the new anti-bot reporting continuation PR

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/open-analysis documentation only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted repository policy
- classification: architecture/documentation continuation
- result: pending

## Self-review

- exact head: pending
- method/reviewer: architecture continuation agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES before any merge that canonically supersedes accepted profile/channel/gameplay/anti-abuse semantics; open-analysis checkpoint material may remain non-canonical while owner decisions continue
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- prior checkpoint delivery: PR #221 merged as `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`
- current continuation branch: `arch/anti-bot-player-reporting-20260827`
- changed-file review: player-reporting open analysis plus this task update
- unresolved review threads: pending
- protected auto-merge: disabled while owner-decision sequence is incomplete
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: merged PR #221 checkpoint extended with a dedicated open-analysis document for player anti-bot reports, malicious-report resistance, reporter independence/reliability, enhanced observation and UUIDv7/value-provenance integration
status: investigating
branch: arch/anti-bot-player-reporting-20260827
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
blocker: housing model and final anti-automation/reporting policy remain deliberately unresolved pending further owner decisions
next_action: continue Issue #220 from current main; perform the housing/auction/scarcity/multichannel deep-dive and keep anti-automation/reporting material as open analysis until explicit owner disposition
```
