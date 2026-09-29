# OTV2-20260929-spell-part-a-state-contracts

```yaml
task_id: OTV2-20260929-spell-part-a-state-contracts
title: "Part A state contracts: monk Harmony/Serene (SPELL-D8) and Wheel of Destiny state"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-part-a-state-contracts
issue: 162
pr: 1205
base_sha: 0bd0fd75480fc70bb4629958b22b66a0e1e253f4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01LphUANMfC2q2WKdfEb39eC
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md
  - docs/agents/tasks/active/OTV2-20260929-spell-part-a-state-contracts.md
public_contracts:
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/architecture/OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md
depends_on: []
blocks:
  - OTV2-20260929-spell-part-a-wheel-harmony
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation #162 comment 5884682203, after the Part A runtime worker stopped fail-closed (no owner for
Harmony, Serene or Wheel state). This task gives that state its owners, as design only:

- SPELL-D8, an amendment candidate to the spell cast wire and vitals contract (§8.2):
  - Harmony 0..5 is a durable Character field, following Canary;
  - Serene is runtime-actor-local and non-durable;
  - the death, logout and login rules follow the sources;
  - `ActorVitalsV1` gains `harmony` and `serene` and stays within 32 bytes through value bounds.
- A new Wheel of Destiny state contract candidate:
  - a persisted 36-slot allocation, mirroring Canary `player_wheeldata`;
  - revelation stages and augment stages derived per §A.1;
  - a playable-first slice W-R, W-1 and W-2, with explicit later items.

No runtime, migration, proto, registry or content change.

## Architecture and source of truth

- `PROVEN`: the Canary `99902524` and Crystal `ff7ede5` references cited in both documents; the Fandom
  `Harmony` r1136128 and `Serene` r1104593 pages at the target date (silent on death and logout); on
  main, no `ACTOR_VITALS` proto, codec or registry entry.
- `DERIVED`: the V1 save points (actor end and death); the `ActorVitalsV1` value bounds.
- `UNKNOWN`: whether the Wheel is Premium-gated in Oteryn V1; the Global rule when levels are lost
  below the allocated points; whether a forced Serene should persist.

## High-risk authority/recovery qualification

Not applicable to this design task. The Harmony write (H-1) and the Wheel allocation write (W-1) are
fenced durable Character writes. Their implementing children carry the qualification.

## Acceptance criteria

- [ ] Both documents are on an exact frozen head with passing validators.
- [ ] Independent exact-head review, with no P1 open.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Runtime, migrations, proto and registry entries, content, the virtue slot (Part C), the party
  service, gems, scrolls and the Wheel UI.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `python3 tools/architecture/semantic_contract_audit.py --base-sha <base> --head-sha <head>`

## Context checkpoint

```yaml
last_progress: authored both documents; PR #1205 open
status: validating
branch: claude/spell-part-a-state-contracts
pr: 1205
owner_action_required: "Q1-Q2 of SPELL-D8 and Q1-Q3 of the Wheel candidate"
blocker: null
next_action: exact-head review and Merge Queue integration; then the Part A runtime resumes
```
