# MAGIC-SUMMON-1

```yaml
task_id: MAGIC-SUMMON-1
title: "MAGIC-SUMMON-1: summons, familiars, magic fields (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/magic-summon-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 3 MAGIC-SUMMON-1 (rows 57, 58); App. 7 LOOP-MONSTER-1, MAGIC-SUMMON-1; wave 2"
class: "hard (child graph)"
migration: maybe
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/monster_summon.rs
  - apps/game-server/src/gameplay_transport/familiar_cast*.rs
  - apps/game-server/src/gameplay_transport/world_item_cast.rs
  - apps/game-server/tests/summon_*.rs (new)
slot_only_paths:
  - connection.rs
  - mod.rs
  - capabilities.rs (SUMMON-WIRE-1)
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - migration number (FAMILIAR-1)
  - content spell/creature pins
public_contracts: []
depends_on:
  - "MAGIC-CAST-1: MERGED #2112, #2119"
  - "SPELL_NPC_MAP0: ACCEPTED #2098"
  - "CREATURE_AI0 ACCEPTED (summon children): BLOCKING: CANDIDATE"
  - "FAMILIARS0 ACCEPTED: BLOCKING: CANDIDATE"
  - "CREATURE-MOVE-1 residue (SUMMON-1): BLOCKING: draft skeleton/creature-move-1"
  - "HOUSE-1b (`house_access`): BLOCKING: Wave 5"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Summons, familiars and magic fields; children MONSTER-SUMMON-1, SUMMON-1, SUMMON-WIRE-1, FAMILIAR-CONTENT-1, FAMILIAR-1, SPELL-RT-SUMMON-1, SPELL-RT-FAMILIAR-1, SPELL-RT-WORLD-1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAGIC-CAST-1 | MERGED #2112, #2119 |
| SPELL_NPC_MAP0 | ACCEPTED #2098 |
| CREATURE_AI0 ACCEPTED (summon children) | BLOCKING: CANDIDATE |
| FAMILIARS0 ACCEPTED | BLOCKING: CANDIDATE |
| CREATURE-MOVE-1 residue (SUMMON-1) | BLOCKING: draft skeleton/creature-move-1 |
| HOUSE-1b (`house_access`) | BLOCKING: Wave 5 |

**BLOCKING:** CREATURE_AI0 ACCEPTED (summon children); FAMILIARS0 ACCEPTED; CREATURE-MOVE-1 residue (SUMMON-1); HOUSE-1b (`house_access`). The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/monster_summon.rs`
- `apps/game-server/src/gameplay_transport/familiar_cast*.rs`
- `apps/game-server/src/gameplay_transport/world_item_cast.rs`
- apps/game-server/tests/summon_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs
- mod.rs
- capabilities.rs (SUMMON-WIRE-1)
- PROTOCOL_OTERYN_V1_REGISTRY.json
- migration number (FAMILIAR-1)
- content spell/creature pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/spell_parameter_cast.rs` closed gates `vertical_move`, `house_access` (BUNDLE-GATES-1 inventory `bundle_gates_tests.rs`)
- `monster_summon.rs`

## Inseparable bundles

SUMMON-WIRE-1 wire bundle; FAMILIAR-1 migration+writer. First unblocked child: SPELL-RT-WORLD-1 minus `house_access`.

## Acceptance criteria

- [ ] Every closed bundle gate it opens is updated in `bundle_gates_tests.rs` inventory.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

CREATURE_AI0; FAMILIARS0; SPELL_NPC_MAP0 §2-§3. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 3 MAGIC-SUMMON-1 (rows 57, 58); App. 7 LOOP-MONSTER-1, MAGIC-SUMMON-1.

## Builder instructions

- Work on branch `skeleton/magic-summon-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/magic-summon-1
blocker: CREATURE_AI0 ACCEPTED (summon children); FAMILIARS0 ACCEPTED; CREATURE-MOVE-1 residue (SUMMON-1); HOUSE-1b (`house_access`)
next_action: CP rechecks BLOCKING rows
```
