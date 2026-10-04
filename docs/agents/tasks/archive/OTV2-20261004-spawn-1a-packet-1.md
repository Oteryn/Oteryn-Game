# OTV2-20261004-spawn-1a-packet-1

```yaml
task_id: OTV2-20261004-spawn-1a-packet-1
title: "SPAWN-1A-PACKET-1: the D116 fixture spawn and entry-room revision 2"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spawn-1a-packet-1-20261004
issue: 162
pr: 1745
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_NATIVE_ENTRY_ROOM_PRODUCT_BINDINGS_DECISION_2026-09-26.md
  - docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_04.md
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_NATIVE_ENTRY_QUALIFICATION_AMENDMENT_02.md
  - docs/agents/tasks/archive/OTV2-20261004-spawn-1a-packet-1.md
public_contracts: []
depends_on: []
blocks: [SPAWN-1a]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Control plane D492 (owner 7a): SPAWN-1a moves out of #1735 (ARCH-CORE-LOOP-PACKETS-2 §1.4 room
  revision 2 and spawn inputs, §2.7 packet) into this decision. The bindings "Amendment
  2026-10-04" comes with it. #1735 keeps the SPAWN-1a/1b split ruling and SPAWN-1b.
- #1735 P1 4177035356: room r2 is a new immutable package and lock identity
  (`oteryn:package-rev/entry-r2`, `lock:oteryn:package-rev/entry-r2`); definition revisions follow
  their bytes, so the unchanged definitions keep `oteryn:rev/entry-r1`; mixed r1/r2 tuples are
  refused (§1.2).
- #1735 P1 4177035359: `gameplay_transport/qualification.rs` is owned as the second
  `into_channel_parts` consumer; the spawn source enters through an additive constructor, so the
  other call sites compile unchanged (§1.3). The file is serialized with SESSION-PUSH-1 (#1736, §0).
- #1735 P1 4177035362: `tools/monster-lab/test_arena_map.py` is owned and expects six cells and
  the rats' slots at (2, 0, 0) and (2, -1, 0) (§2.1).
- #1745 P1 4177068268: D116 is one spawn of 2 rats. Revision 2 adds two non-proof den cells, an
  ordered `cell_keys` list and population 2, and the boot test requires both rats (§1.1, §2.1).
- #1745 P1 4177087017 and P1 4177087019: the owning profiles are amended in this PR.
  - FirstProduction Amendment 04: population 1..=2 and aggregate 2, `cell_keys`, a spawn-cell
    record and eight recomputed registry maxima.
  - Native source Amendment 02: five room cells, six placements, `cell_keys` and the spawn inputs.
  - SPAWN-1a owns `production.rs`, the registry rows and the governance registry test, with max
    and max+1 tests (§1.4, §2.1).
- #1745 P1 4177134666 (owner D498 8a): `RECORD_SPAWN_CELL` takes kind 19, the first unused one,
  because 18 is `RECORD_RNG_PURPOSE`. SPAWN-1a adds a test that every record kind is distinct
  (FirstProduction Amendment 04 §3).
- #1745 P1 4177181274 (control plane D501 1a): revision 2 makes the rat hostile and authors its
  inputs in content.
  - `oteryn:behavior/rat-hostile` replaces passive-idle, and the rat creature profile authors
    `health` and `initial_health` 20 and `speed` 67. Both records are at `oteryn:rev/entry-r2`.
  - The inputs go through the two admitted v2 authoring profiles and the qualified spawn source to
    the carrier, with no hardcoded 20.
  - Perception, think interval and wander follow CREATURE-AI-0 R1, §10 and §5.4.
  - Native source Amendment 02 and the bindings amendment record the change (§1.2, §1.6).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
