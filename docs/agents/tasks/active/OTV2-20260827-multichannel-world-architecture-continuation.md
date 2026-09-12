# OTV2-20260827-multichannel-world-architecture-continuation

```yaml
task_id: OTV2-20260827-multichannel-world-architecture-continuation
title: Continue owner decisions for multichannel world topology and housing
mode: CONTRACT
status: handed_off
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/multichannel-world-owner-checkpoint-20260827
issue: 220
pr: 221
base_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
head_sha: 9392ffde156bea8093a135fe2527b714db6ac0b1
final_head_sha: 9392ffde156bea8093a135fe2527b714db6ac0b1
final_head_frozen_at: null
owner: Oteryn architecture continuation
worker_alias: "Oteryn: architektura"
created_at: 2026-08-27T22:11:06+02:00
updated_at: 2026-09-04T19:46:00+02:00
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

## Live lifecycle reconciliation

`PROVEN`: PR #221 is merged and closed. Its task-branch head was `9392ffde156bea8093a135fe2527b714db6ac0b1`; merge commit is `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`.

The historical checkpoint remains authoritative provenance for the 2026-08-27 owner-selected World/Channel/PvP/multi-region direction. It must not be reopened or rewritten as if later decisions had been present at that time.

Remaining owner-decision continuation has been handed off to:

`OTV2-20260904-rested-death-architecture-continuation`

on branch:

`arch/rested-death-owner-checkpoint-20260904`

under the same Issue #220.

The handoff intentionally keeps housing deferred for later and persists the subsequent Rested/death-recovery decision sequence in a new dated checkpoint rather than rewriting this historical checkpoint.

## Outcome

Preserve the owner-selected multichannel/world/multi-region direction as a durable, explicitly non-final checkpoint and continue the remaining architecture decisions without losing provenance or silently contradicting existing accepted contracts.

## Architecture and source of truth

- `PROVEN`: admission protected main was `6e6e37852b7a050a1c7117ab2a9f316907d09daf`.
- `PROVEN`: `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` is reusable with short invocation `Oteryn: architektura`.
- `PROVEN`: current accepted `GAME-CHANNEL-01` treats `WorldId` as the persistent product/economy/community/ruleset boundary and `ChannelId` as a parallel simulation identity.
- `CONFLICT`: accepted product wording also requires different profile/ruleset families to use distinct WorldIds / one inherited profile family across all Channels of one World, while the owner selected Channel-scoped PvP mode as the intended direction.
- `OWNER_SELECTED_DIRECTION`: decisions and non-decisions are recorded in `docs/architecture/reviews/OTERYN_GAME_MULTICHANNEL_WORLD_OWNER_DECISION_CHECKPOINT_2026-08-27.md`.
- `UNKNOWN` at this checkpoint: the correct final housing/auction/scarcity topology for a 1k/2k+ population and multi-region Channels.

## Acceptance criteria

- [x] Issue #220 records the exact owner-selected direction and known architecture conflict.
- [x] Durable architecture checkpoint records every selected decision from the 2026-08-27 continuation session without inventing a housing solution.
- [ ] Housing receives a dedicated deep analysis covering scarcity, auctions, item authority, channel presence, ACL/ownership, cross-region failure and scalable alternatives. **HANDED OFF / currently intentionally deferred by owner.**
- [ ] Owner selects or explicitly defers a housing model only after that analysis. **HANDED OFF; owner currently chose deferral.**
- [ ] Exact current-contract clauses affected by Channel-scoped PvP mode are identified before any canonical supersession. **HANDED OFF.**
- [ ] Remaining architecture horizon continues one material owner decision at a time, skipping already accepted decisions. **HANDED OFF.**
- [ ] Final coherent amendment package receives repository-required architecture/governance validation and independent review before canonical merge. **HANDED OFF.**

## Excluded scope

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas implementation. No permanent World Project/Bundle physical format selection. No implicit acceptance of apartments, duplicated houses, world-global house interiors or any other housing topology from this historical checkpoint.

## Implementation / findings

Owner-selected direction preserved by PR #221:

- one durable World with many parallel Channels;
- repeated/different PvP modes may exist per Channel inside one World;
- cross-mode same-World move is an ordinary fenced Channel switch;
- Channel PvP mode is immutable for the ChannelId lifetime;
- initial XP/loot/spawn/progression economics are equal across modes;
- Channel selection remains player-controlled and exposes mode, occupancy/capacity, runtime health/load, hosting region and player-relative latency, with a non-binding recommendation;
- Channels of one World may be hosted in multiple geographic regions;
- initial durable topology uses one World DurableHomeRegion and one authoritative PostgreSQL write domain, while Channel simulation stays region-local and WAN-free in the authoritative tick;
- permanent World Project/Bundle format remains deferred pending representative real E2E evidence;
- housing remained unresolved at this historical checkpoint.

## Validation

### Focused

- historical result: documentation consistency review delivered through merged PR #221.

### Component/integration

- `NOT_APPLICABLE` — architecture checkpoint only.

### E2E

- `NOT_APPLICABLE` — no executable behavior changes.

### Exact-head CI

- historical PR lifecycle completed through protected integration; live current-state authority is the merged PR/main state, not stale pending prose in the original task draft.

## Self-review

Historical checkpoint is preserved as merged provenance. Later Rested/death decisions are not backfilled into the 2026-08-27 checkpoint.

## Independent review

Any future canonical supersession of accepted profile/channel semantics remains subject to the then-current repository review policy.

## PR and closeout

- PR #221: `MERGED`.
- task branch head: `9392ffde156bea8093a135fe2527b714db6ac0b1`.
- merge commit: `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`.
- unresolved continuation scope: handed off to `OTV2-20260904-rested-death-architecture-continuation`.

## Context checkpoint

```yaml
last_progress: PR #221 merged the 2026-08-27 multichannel checkpoint; later owner decisions are intentionally preserved in a new dated continuation checkpoint rather than rewriting history
status: handed_off
branch: arch/multichannel-world-owner-checkpoint-20260827
head_sha: 9392ffde156bea8093a135fe2527b714db6ac0b1
pr: 221
final_head_sha: 9392ffde156bea8093a135fe2527b714db6ac0b1
final_head_frozen_at: null
owner_action_required: null
blocker: null
next_action: continue under OTV2-20260904-rested-death-architecture-continuation / arch/rested-death-owner-checkpoint-20260904
```
