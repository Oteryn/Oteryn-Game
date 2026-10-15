# BOOT-RAT-POLICY-1

```yaml
task_id: BOOT-RAT-POLICY-1
title: "BOOT-RAT-POLICY-1: boot-spawned rat binds an AI policy and qualifies for melee (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/boot-rat-policy-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 PREREQ-1; App. 7 Archive follow-ups BOOT-RAT-POLICY-1; wave 2 (PREREQ-1 residue child)"
class: "hard (combat), combat review"
migration: no
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/behaviour_profile.rs
  - apps/game-server/src/ai/spawn.rs
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - apps/game-server/src/gameplay_transport/qualification_wild_spawn.rs
  - apps/game-server/tests/boot_rat_policy*.rs (new)
slot_only_paths:
  - content/world/pins/*, goldens and qualified digests if any entry-room fact changes (one author, one commit)
  - mod.rs (only if the boot spawn source composition moves)
public_contracts: []
depends_on:
  - "CREATURE-AI-1: MERGED #1813"
  - "ATTACK-1b: MERGED #1798"
  - "SPAWN-1b: MERGED #1981"
  - "ENTRY-SPELL-TILES-1: MERGED #2005"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

A rat spawned at boot on a no-manifest boot binds a creature AI policy and qualifies for melee in the entry room, so entry-room rats fight a player; also closes the 'bundle World monster cycle' deferral of CREATURE-MOVE-1a.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CREATURE-AI-1 | MERGED #1813 |
| ATTACK-1b | MERGED #1798 |
| SPAWN-1b | MERGED #1981 |
| ENTRY-SPELL-TILES-1 | MERGED #2005 |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- `apps/game-server/src/ai/behaviour_profile.rs`
- `apps/game-server/src/ai/spawn.rs`
- `apps/game-server/src/gameplay_transport/monster_ai_cycle.rs`
- `apps/game-server/src/gameplay_transport/qualification_wild_spawn.rs`
- apps/game-server/tests/boot_rat_policy*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- content/world/pins/*, goldens and qualified digests if any entry-room fact changes (one author, one commit)
- mod.rs (only if the boot spawn source composition moves)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `ai/spawn.rs` spawn realization -> policy binding
- `gameplay_transport/monster_ai_cycle.rs` monster think cycle
- entry-room facts `content/project/native_entry.rs` (read-only unless pins rotate)

## Inseparable bundles

content+regeneration+all pins by one author if facts change.

## Acceptance criteria

- [ ] Reproduce the current refusal first (RED).
- [ ] Boot rat attacks and is attackable in the entry room on a no-manifest boot.
- [ ] Deterministic replay unchanged for existing spawn tests.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

archive records OTV2-20261004-spawn-1a, OTV2-20261011-accept-chat-attack-death-0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 PREREQ-1; App. 7 Archive follow-ups BOOT-RAT-POLICY-1.

## Builder instructions

- Work on branch `skeleton/boot-rat-policy-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/boot-rat-policy-1
blocker: null
next_action: builder confirms residue against main and starts owned paths
```
