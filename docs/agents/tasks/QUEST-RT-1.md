# QUEST-RT-1

```yaml
task_id: QUEST-RT-1
title: "QUEST-RT-1: quest runtime on the bundle, chest rewards, cap 16 (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/quest-rt-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1; wave 2"
class: "hard (rewards are mints)"
migration: maybe
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/quest/
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/quest_rt_*.rs (new)
slot_only_paths:
  - capabilities.rs (cap 16)
  - connection.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - content quest pins
public_contracts: []
depends_on:
  - "QUEST-LOWER-1: MERGED #1727"
  - "ITEM-LOOP-1: BLOCKING: not merged"
  - "MAP-PICKUP-1: BLOCKING: not merged"
  - "QUEST-XP-ADMISSION-1: BLOCKING: not merged"
  - "QUEST_GATE0, QUEST_STATE0 ACCEPTED: BLOCKING: CANDIDATE"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Quest runtime on the bundle World, chest rewards, capability 16 offer.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| QUEST-LOWER-1 | MERGED #1727 |
| ITEM-LOOP-1 | BLOCKING: not merged |
| MAP-PICKUP-1 | BLOCKING: not merged |
| QUEST-XP-ADMISSION-1 | BLOCKING: not merged |
| QUEST_GATE0, QUEST_STATE0 ACCEPTED | BLOCKING: CANDIDATE |

**BLOCKING:** ITEM-LOOP-1; MAP-PICKUP-1; QUEST-XP-ADMISSION-1; QUEST_GATE0, QUEST_STATE0 ACCEPTED. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/quest/`
- `apps/game-server/src/interaction/chest_use.rs`
- apps/game-server/tests/quest_rt_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- capabilities.rs (cap 16)
- connection.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- content quest pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/quest_log.rs` (domain, `observe_quest_log` l.633)
- `interaction/chest_use.rs` MINT
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`

## Inseparable bundles

mint+ledger; wire+registry+capability+handler.

## Acceptance criteria

- [ ] Chest reward minted once per character; replay safe.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

QUEST_GATE0; QUEST_STATE0. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1.

## Builder instructions

- Work on branch `skeleton/quest-rt-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/quest-rt-1
blocker: ITEM-LOOP-1; MAP-PICKUP-1; QUEST-XP-ADMISSION-1; QUEST_GATE0, QUEST_STATE0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```
