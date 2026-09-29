# OTV2-20260928-cw2-encounter-vocabulary-extensions

```yaml
task_id: OTV2-20260928-cw2-encounter-vocabulary-extensions
title: CW2 encounter vocabulary extension candidates for the remaining unresolved_semantics rows
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw2-encounter-vocabulary-extensions
pr: 1183
base_sha: 3dcf3c82
head_sha: be0b122964f88892f5b7c2e4f36e73f33455e5d9
final_head_sha: be0b122964f88892f5b7c2e4f36e73f33455e5d9
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-28T22:00:00Z
updated_at: 2026-09-29T06:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md (the unresolved-semantics rows of the four bosses only)
  - docs/agents/tasks/active/OTV2-20260928-cw2-encounter-vocabulary-extensions.md
public_contracts: []
depends_on:
  - "owner decision on #162, 2026-09-28, batch item 4 (the lane proposes extensions from CrystalServer/Canary behaviour; the owner accepts the finished design)"
  - "allocation on issue #162: ALLOCATIONS item B"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Docs only. The new §12 of `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` proposes CANDIDATE resolutions, pending owner
acceptance, for the `unresolved_semantics` rows of four encounters. Evidence is cited as file:line in CrystalServer
`ff7ede593c69d4c658b382c97443e8155926924a` (`data-global/`; `data-crystal/` carries none of these scripts).

| Boss | Mechanic | Proposal |
|---|---|---|
| Alptramun | `alptramun summon` escalation | no extension; D29 `ability_cast` with D45 covers it; the row is resolvable in the vocabulary, and evidence waits on Q1 |
| Gorzindel | the tome's portal to the first free knowledge room for 10 s | CW2-1: `teleport who: {triggering: true}`, with `in_anchor` of `triggering` gating the delayed return to players still in the instance |
| Melting Frozen Horror | death actions on two fixed tiles | no extension; the lever names `dragon_egg` and `solid_frozen_horror`; resolvable now |
| The Sandking | the stage counter and the brood cycle behind it | CW2-2: `stepped_on corpse_of: role`; CW2-3: `map_item remove triggering: true`; CW2-4: `random_in` with an optional `free: true` (existing `random_in` semantics unchanged) |

The extensions (DERIVED) widen parameters of existing terms. No trigger, condition or action kind is added.

Owner questions (deviations and product choices only): Q1 Alptramun (follow Canary's never-cast spell, or the
wiki), Q2 Gorzindel room and return defects, Q3 Sandking brood spawn failure, Q4 Melting Frozen Horror killed
while parked (recommended: follow Canary).

## Evidence classification

Labels follow `docs/agents/AGENTS.md` and apply to CrystalServer `ff7ede59`, path `data-global/`.

- **PROVEN** (source read, file:line):
  - Alptramun:
    - `alptramun summon` spawns 1-4 dreams by counter band while fewer than 5 summons exist
      (`scripts/spells/monster/alptramun_summon.lua:15-47`).
    - No monster casts it (`monster/quests/the_dream_courts/bosses/alptramun.lua:120-132`).
    - The counter skips mastered dreams (`creaturescripts_dream_courts_death.lua:68-70,107-114`).
  - Gorzindel:
    - The tome's portal and its 10 s return (`creaturescripts_gorzindel.lua:38-50`).
    - The first-open-room teleport and the return (`movements_gorzindel.lua:1-38`).
    - The tome cannot move (`stolen_tome_of_portals.lua:14,35`).
    - The lever tiles (`bosses_levers/gorzindel.lua:22-27`).
  - Melting Frozen Horror:
    - The death actions on two fixed tiles (`creaturescripts_bosses_kill.lua:37-48`).
    - The lever places the dragon egg and the parked melting horror (`actions_frozen_horror.lua:7-12,64-67`).
    - The swap and revert (`creaturescripts_dragon_egg.lua:1-43`).
  - The Sandking:
    - The lever sets stage 1 (`actions_bosses_levers.lua:480-481`).
    - The stage cycle (`creaturescripts_sandking.lua:1-130`).
    - Corpse healing (`movements_sandking.lua:3-16`).
    - The credit needs stage 5 (`creaturescripts_bosses_mission_cults.lua:7,24-26`).
    - The mark survives corpse decay (`src/game/game.cpp:3097,3148`).
    - `SandHealth` never acts (`creaturescripts_sandking.lua:132-152`).
  - `data-crystal/` registers none of these events.
- **DERIVED** (proposal, pending owner acceptance):
  - Alptramun and Melting Frozen Horror need no vocabulary change.
  - CW2-1..4 and their authored JSON (§12.1-12.4).
  - Existing-term JSON validates against the current schema. Only the proposed terms fail.
  - The decision test (§12.7).
- **UNKNOWN** (waits on the owner):
  - Q1: Alptramun's escalation, and whether its row becomes `approved_omission`.
  - Q2: Gorzindel's room and return defects.
  - Q3: the free-tile brood spawn, and with it CW2-4.
  - Q4: Melting Frozen Horror killed while parked.
  - The dragon egg transcription (outside this task).
- **CONFLICT** (Canary vs wiki):
  - Alptramun: Canary never escalates, while the reference-date wiki says killed summons return stronger (Q1).
  - `SandHealth`'s evident intent (reflection) against its no-op in Canary. This is a D25 wiki check, not decided
    here.

## Excluded scope

- Ferumbras Mortal Shell, which waits for a quest-domain contract.
- Any change to the schema, samples, tools, Rust or `content/`. Those follow in the implementation order of §12.5,
  after acceptance.
- The dragon egg's own events, which the Melting Frozen Horror admission still waits for.

## Validation

- The authored-JSON examples were checked against the current schema. The rules without new terms pass. The only
  failures are the new terms (CW2-1..4).
- `python3 tools/agents/validate_governance.py` and `python3 tools/repository/validate_repository_policy.py`: see
  the PR.

## Terminal integration

- **Final head:** `be0b122964f88892f5b7c2e4f36e73f33455e5d9`.
- **Integration:** merged into main as PR #1183, merge commit
  `86116adf` (2026-09-28T22:36:01Z).
- **Closeout:** the record was archived by `OTV2-20260929-cw1-od9-create-teleporters` (issue #162,
  allocation comment 5884353101).
- **Owned paths:** released.
- **Follow-up:** §12 of `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` was merged as CANDIDATE and
  awaited owner acceptance at merge. Its implementation (schema, samples, tools, Rust) follows the
  §12.5 order after acceptance, in its own allocation.

## Context checkpoint

last_progress: merged as 86116adf via PR #1183; archived as completed
jira: pending (no mapped Story resolved in this worker session)
