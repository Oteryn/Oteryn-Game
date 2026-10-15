# ITEM-LOOP-1

```yaml
task_id: ITEM-LOOP-1
title: "ITEM-LOOP-1: equip, drop/pickup, containers, stacks, cap, starter pack on the bundle (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/item-loop-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 ITEM-LOOP-1 (rows 62-66, 68); App. 7 ITEM-LOOP-1; wave 1 (migration chain #1)"
class: "hard (durable item)"
migration: yes
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/item_move.rs
  - apps/game-server/src/gameplay_transport/equipment_move.rs
  - apps/game-server/src/gameplay_transport/container_view.rs
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/ (new item-loop writers)
  - apps/game-server/tests/item_loop_*.rs (new)
slot_only_paths:
  - migration number (chain #1: ITEM-LOOP-1 -> DAWNPORT-1 -> LOOP-PROGRESS-1 -> DEATH-ML-LOSS-1)
  - connection.rs
  - mod.rs
  - capabilities.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
  - content/starter pins if STARTER-1 changes content
public_contracts: []
depends_on:
  - "EQUIP-0: MERGED (ACCEPT-EQUIP-0 #2090)"
  - "#2039 ITEM-USE-1a/1b: MERGED"
  - "ITEM-USE-1c: MERGED #2082"
  - "ITEM-MOVE-2a: PARTIAL: EQUIP-WIRE-1 slice #2009; residue here"
  - "BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED: BLOCKING: all CANDIDATE (plan §7 status list)"
  - "MAP-PICKUP-1 (rows 62-63 base-map pickup): BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

The item loop works end to end on the bundle World: equip/unequip, drop/pickup (base-map pickup gated on MAP-PICKUP-1), nested containers, stacks, capacity and weight refusal, starter pack.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| EQUIP-0 | MERGED (ACCEPT-EQUIP-0 #2090) |
| #2039 ITEM-USE-1a/1b | MERGED |
| ITEM-USE-1c | MERGED #2082 |
| ITEM-MOVE-2a | PARTIAL: EQUIP-WIRE-1 slice #2009; residue here |
| BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED | BLOCKING: all CANDIDATE (plan §7 status list) |
| MAP-PICKUP-1 (rows 62-63 base-map pickup) | BLOCKING: not merged |

**BLOCKING:** BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED; MAP-PICKUP-1 (rows 62-63 base-map pickup). The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/item_move.rs`
- `apps/game-server/src/gameplay_transport/equipment_move.rs`
- `apps/game-server/src/gameplay_transport/container_view.rs`
- `apps/game-server/src/durability/item_transfer.rs`
- apps/game-server/src/durability/ (new item-loop writers)
- apps/game-server/tests/item_loop_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (chain #1: ITEM-LOOP-1 -> DAWNPORT-1 -> LOOP-PROGRESS-1 -> DEATH-ML-LOSS-1)
- connection.rs
- mod.rs
- capabilities.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- content/starter pins if STARTER-1 changes content
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/item_move.rs` (command type 9, `committed_item_move` connection.rs l.594)
- `equipment_move.rs` (`move_equipment` l.571)
- `durability/item_transfer.rs` (DUR-03 TRANSFER)
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability; mint/burn+ledger for starter pack (STARTER-1 MINT). Children split into separate PRs in App. 7 order: ITEM-MOVE-2a residue, BAGS-1, ITEM-MOVE-2b, STACK-WIRE-1, STACK-1, STARTER-1, B3-3.

## Acceptance criteria

- [ ] Each child: RED then GREEN, PostgreSQL 17.6 durability tests, restart/replay.
- [ ] Overweight refusal (B3-3) only after B3 acceptance.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

plan App. 7 ITEM-LOOP-1; Reference item move rules. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 ITEM-LOOP-1 (rows 62-66, 68); App. 7 ITEM-LOOP-1.

## Builder instructions

- Work on branch `skeleton/item-loop-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/item-loop-1
blocker: BAGS0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0, B3 ACCEPTED; MAP-PICKUP-1 (rows 62-63 base-map pickup)
next_action: CP rechecks BLOCKING rows
```
