# DAWNPORT-1

```yaml
task_id: DAWNPORT-1
title: "DAWNPORT-1: Dawnport content and vocation choice (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/dawnport-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 DAWNPORT-1 (row 38); App. 7 Supporting (A13); wave 1 (migration chain #2)"
class: "impl (content)"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - content/world/ Dawnport inputs (new family files only)
  - content/npcs/ Dawnport vocation NPC dialogue (new files)
  - apps/game-server/src/interaction/vocation_choice.rs (new)
  - apps/game-server/tests/dawnport_*.rs (new)
slot_only_paths:
  - regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins (one author, one commit)
  - migration number if a vocation-choice receipt is needed
public_contracts: []
depends_on:
  - "CHAR-BUILD-1: MERGED #1393 + 1b writer"
  - "W2b: MERGED #2109"
  - "A13 ACCEPTED: BLOCKING: A13 CANDIDATE (plan §7)"
  - "ITEM-LOOP-1 (migration chain order): BLOCKING: chain order, migration slot after ITEM-LOOP-1"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Dawnport content and the vocation-choice interaction; choosing a vocation keeps cumulative progress (SKILLS-0 §3.3).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CHAR-BUILD-1 | MERGED #1393 + 1b writer |
| W2b | MERGED #2109 |
| A13 ACCEPTED | BLOCKING: A13 CANDIDATE (plan §7) |
| ITEM-LOOP-1 (migration chain order) | BLOCKING: chain order, migration slot after ITEM-LOOP-1 |

**BLOCKING:** A13 ACCEPTED; ITEM-LOOP-1 (migration chain order). The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- content/world/ Dawnport inputs (new family files only)
- content/npcs/ Dawnport vocation NPC dialogue (new files)
- apps/game-server/src/interaction/vocation_choice.rs (new)
- apps/game-server/tests/dawnport_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins (one author, one commit)
- migration number if a vocation-choice receipt is needed
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `durability/character_progression.rs` (build state writer)
- `content/character_progression_content.rs`
- `tools/content-migration/regenerate_content.py` then `content/content.lock.json`, `content/manifest.json`, `content/world/pins/*` rotated by one author in one commit

## Inseparable bundles

content+regeneration+all pins by one author; migration+writer if needed.

## Acceptance criteria

- [ ] Vocation choice commits through the build-state writer; progress kept.
- [ ] Content validators pass; pins rotated once.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

A13 §; SKILLS-0 §3.3; Reference Dawnport map/NPC data. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 DAWNPORT-1 (row 38); App. 7 Supporting (A13).

## Builder instructions

- Work on branch `skeleton/dawnport-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/dawnport-1
blocker: A13 ACCEPTED; ITEM-LOOP-1 (migration chain order)
next_action: CP rechecks BLOCKING rows
```
