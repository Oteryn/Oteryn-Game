# OTV2-20261001-encounter-rt0

```yaml
task_id: OTV2-20261001-encounter-rt0
title: "ENCOUNTER-RT-0 encounter runtime and boss levers"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-encounter-rt-0
pr: "assigned at PR creation; recorded in the #162 FREEZE_SHA entry"
base_sha: d3cfb246
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ENCOUNTER_RT0_ENCOUNTER_RUNTIME_AND_BOSS_LEVERS_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261001-encounter-rt0.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ENCOUNTER-RT-0 decides the runtime of the encounter vocabulary and the boss lever (#162 5929069698,
close-out plan; BOSS-RAID-0 §6.5).

- **Owner:** one encounter instance per scope owner (one per boss room; one per channel for
  `channel_shared`), runtime-only, activated only with every anchor bound (E3, E4).
- **Execution:** a FIFO trigger queue drained in the owner turn; `lethal_damage` and the
  health-change triggers inline in the damage applier (Canary's synchronous points) with a per-hit
  re-entry bound; delayed rules and timers on 50 ms windows, no firing dropped; the `ENCOUNTER_DRAW` purpose; loops fault the instance.
- **Actions:** each through its owner (creatures, GAME-ABILITY-01, overlays, Movement, CHAT-0);
  outcomes to bound consumers, idempotent by key (a reward-boss death durable with the death); `drop_item` as an `EncounterDropCause` MINT.
- **Boss lever:** the `BOSS_ENTRY` lever child feeding BOSS-RAID-0 §6.2, for `instance_per_party` only.
- **Instance floor:** Ground custody in an InstanceRuntime scope, retired by `InstanceRetire` after a
  fresh ownership generation fences the departed runtime.
- **Owner questions:** none; R1-R4 are architect rulings.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: the encounter format (§2-§13), E1-E4, BOSS-RAID-0 §6-§8, CREATURE-AI-0, WORLD-INTERACTION-0
  §5, DUR-03 §32 and §39, D3.
- `DERIVED`: Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b` (`OTS_HYPOTHESIS_ONLY`; `boss_lever.lua`,
  `lever.lua`, creature events).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. Children need determinism, combat, persistence and security review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (determinism, combat, persistence, security).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code; scripted movement; cross-party state; the Tibiadrome; World Changes; Hazard.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- `git diff --cached --check`: clean.

## Closeout

- PR: recorded in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Amendments, each pending on acceptance of ENCOUNTER-RT-0: the encounter format §10; BOSS-RAID-0
  §6.5; WORLD-INTERACTION-0 §5; CREATURE-AI-0 §3; DUR-03 §32 and §39.3.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-encounter-rt-0
owner_action_required: null
blocker: null
next_action: null
```
