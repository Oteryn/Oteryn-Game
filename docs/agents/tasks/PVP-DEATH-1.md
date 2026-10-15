# PVP-DEATH-1

```yaml
task_id: PVP-DEATH-1
title: "PVP-DEATH-1: PvP death persistence (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-death-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Death module D; App. 7 PARTY/PVP PVP-DEATH-1; wave 2"
class: "hard (persistence)"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_death.rs (PvP part)
  - apps/game-server/tests/pvp_death_*.rs (new)
slot_only_paths:
  - migration number
public_contracts: []
depends_on:
  - "DEATH-1: MERGED"
  - "PVP-1a: MERGED"
  - "DEATH-3: BLOCKING: not merged (death module C)"
  - "PVP-1b: BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

PvP death outcomes (frags, skull consequences, unjust loss) persisted in the death transaction.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| DEATH-1 | MERGED |
| PVP-1a | MERGED |
| DEATH-3 | BLOCKING: not merged (death module C) |
| PVP-1b | BLOCKING: not merged |

**BLOCKING:** DEATH-3; PVP-1b. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/durability/character_death.rs (PvP part)
- apps/game-server/tests/pvp_death_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_death.rs` `commit_character_death`

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] PvP death and its receipt commit as one transaction; reconcile after restart.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0; DEATH-COMPLETE-0 #2043. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Death module D; App. 7 PARTY/PVP PVP-DEATH-1.

## Builder instructions

- Work on branch `skeleton/pvp-death-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/pvp-death-1
blocker: DEATH-3; PVP-1b
next_action: CP rechecks BLOCKING rows
```
