# SCOPE-HANDOFF-2

```yaml
task_id: SCOPE-HANDOFF-2
title: "SCOPE-HANDOFF-2: cross-node house entry, any-node fallback (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/scope-handoff-2
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 PREREQ-1; App. 7 Archive follow-ups SCOPE-HANDOFF-2; wave 2 (PREREQ-1 residue child)"
class: "hard (persistence)"
migration: maybe
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/house_scope/
  - apps/game-server/tests/house_scope_*.rs (new)
slot_only_paths:
  - migration number (if needed)
  - mod.rs composition
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "SCOPE-HANDOFF-1: MERGED (archive record OTV2-20261004-scope-handoff-1; house_scope/handoff.rs on main)"
  - "HOUSE-RUNTIME-1a: BLOCKING: not merged (Wave 3, other agent)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

A character entering a house owned by another node is handed off across nodes; when the owner node is unavailable an any-node fallback applies, under the session fence.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| SCOPE-HANDOFF-1 | MERGED (archive record OTV2-20261004-scope-handoff-1; house_scope/handoff.rs on main) |
| HOUSE-RUNTIME-1a | BLOCKING: not merged (Wave 3, other agent) |

**BLOCKING:** HOUSE-RUNTIME-1a. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/house_scope/`
- apps/game-server/tests/house_scope_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (if needed)
- mod.rs composition
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `house_scope/handoff.rs` (SCOPE-HANDOFF-1 entry point)
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability in one PR.

## Acceptance criteria

- [ ] Handoff fenced by session generation; stale node cannot admit.
- [ ] Fallback chosen deterministically; restart/replay tests.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

archive record OTV2-20261004-scope-handoff-1; HOUSE_RUNTIME0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 PREREQ-1; App. 7 Archive follow-ups SCOPE-HANDOFF-2.

## Builder instructions

- Work on branch `skeleton/scope-handoff-2`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/scope-handoff-2
blocker: HOUSE-RUNTIME-1a
next_action: CP rechecks BLOCKING rows
```
