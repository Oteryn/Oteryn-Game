# LOOP-DAMAGE-1

```yaml
task_id: LOOP-DAMAGE-1
title: "LOOP-DAMAGE-1: cap 12, resistances, crits/leech, area/chain, mana shield (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-damage-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-DAMAGE-1 (rows 23-26); App. 7 LOOP-DAMAGE-1; wave 2"
class: "hard (wire)"
migration: maybe
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/ (new damage modules: resistances, crit_leech, mana_shield)
  - apps/game-server/src/ability/
  - apps/game-server/tests/loop_damage_*.rs (new)
slot_only_paths:
  - capabilities.rs (cap 12 offer)
  - connection.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "EQUIP-0: MERGED (ACCEPTED #2090)"
  - "BUNDLE-COMBAT-ENABLE-1: MERGED #2025"
  - "COND-1: MERGED #1405, #2086, #2122"
  - "EQUIP-RT-1 <- ITEM-MOVE-2a residue: BLOCKING: ITEM-MOVE-2a only partly merged (#2009)"
  - "CHARM-DEF-1 (CHARM4): BLOCKING: CHARM4 CANDIDATE"
  - "COND-DUR-1 <- FOOD-REGEN-1: BLOCKING: FOOD-REGEN-1 not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Enable capability 12 (EQUIP-0), resistances, critical hits and leech, area and chain attacks on the bundle World, mana shield.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| EQUIP-0 | MERGED (ACCEPTED #2090) |
| BUNDLE-COMBAT-ENABLE-1 | MERGED #2025 |
| COND-1 | MERGED #1405, #2086, #2122 |
| EQUIP-RT-1 <- ITEM-MOVE-2a residue | BLOCKING: ITEM-MOVE-2a only partly merged (#2009) |
| CHARM-DEF-1 (CHARM4) | BLOCKING: CHARM4 CANDIDATE |
| COND-DUR-1 <- FOOD-REGEN-1 | BLOCKING: FOOD-REGEN-1 not merged |

**BLOCKING:** EQUIP-RT-1 <- ITEM-MOVE-2a residue; CHARM-DEF-1 (CHARM4); COND-DUR-1 <- FOOD-REGEN-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/combat/ (new damage modules: resistances, crit_leech, mana_shield)
- `apps/game-server/src/ability/`
- apps/game-server/tests/loop_damage_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- capabilities.rs (cap 12 offer)
- connection.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/attack.rs` / `ordinary_combat.rs` damage composition
- `creature_chain_attack.rs`, `spell/chain.rs` (chain)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`

## Inseparable bundles

wire+registry+capability+handler+tests (cap 12 offer).

## Acceptance criteria

- [ ] Damage formulas match Reference; deterministic replay.
- [ ] Cap 12 offered only when every effect it advertises is live.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

EQUIP0; CONDITIONS0; Reference combat formulas. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-DAMAGE-1 (rows 23-26); App. 7 LOOP-DAMAGE-1.

## Builder instructions

- Work on branch `skeleton/loop-damage-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/loop-damage-1
blocker: EQUIP-RT-1 <- ITEM-MOVE-2a residue; CHARM-DEF-1 (CHARM4); COND-DUR-1 <- FOOD-REGEN-1
next_action: CP rechecks BLOCKING rows
```
