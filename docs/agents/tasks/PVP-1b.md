# PVP-1b

```yaml
task_id: PVP-1b
title: "PVP-1b: skull evaluation, World cleanup job, pvp/party rulesets (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/pvp-1b
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 PARTY/PVP PVP-1b; wave 2"
class: "hard, persistence and security review"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/pvp/
  - apps/game-server/src/durability/pvp_state.rs
  - rulesets/pvp/ (new rows)
  - rulesets/party/ (new rows)
  - apps/game-server/tests/pvp_1b_*.rs (new)
slot_only_paths:
  - migration number (if any)
  - mod.rs (cleanup job composition)
  - ruleset pins
public_contracts: []
depends_on:
  - "PVP-1a: MERGED #2032 (migration 0081)"
  - "DEATH-1: MERGED"
  - "PARTY-1: BLOCKING: not merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Skull evaluation in the death transaction (points and revenge-mark inserts), the node World cleanup job calling the 1a cleanup function, `rulesets/pvp/` and `rulesets/party/` rows (`pvp_type` OPTIONAL).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PVP-1a | MERGED #2032 (migration 0081) |
| DEATH-1 | MERGED |
| PARTY-1 | BLOCKING: not merged |

**BLOCKING:** PARTY-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/combat/pvp/`
- `apps/game-server/src/durability/pvp_state.rs`
- rulesets/pvp/ (new rows)
- rulesets/party/ (new rows)
- apps/game-server/tests/pvp_1b_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (if any)
- mod.rs (cleanup job composition)
- ruleset pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_death.rs` death transaction
- `durability/pvp_state.rs` 1a cleanup fn

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Skull points written atomically with the death.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0 l.33, §6, §9. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` App. 7 PARTY/PVP PVP-1b.

## Builder instructions

- Work on branch `skeleton/pvp-1b`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/pvp-1b
blocker: PARTY-1
next_action: CP rechecks BLOCKING rows
```
