# ITEM-LOOK-1

```yaml
task_id: ITEM-LOOK-1
title: "ITEM-LOOK-1: look/examine message (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/item-look-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 ITEM-LOOK-1 (row 75); wave 1"
class: "impl"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/item_look.rs (new)
  - apps/game-server/src/gameplay_transport/item_look_tests.rs (new)
  - apps/game-server/src/interaction/look.rs (new)
  - crates/protocol-oteryn/src/item_look.rs (new codec)
  - crates/protocol-oteryn/src/item_look_tests.rs (new)
slot_only_paths:
  - connection.rs dispatch
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json (look rate row)
public_contracts: []
depends_on:
  - "none (plan row): MERGED/none"
  - "item description content (item_description_promotion.rs, item_description_wiki_promotion.rs): MERGED on main"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

A player can look at an item (inventory, container, ground) or a creature and receives the server-authored examine text (name, article, count, weight, description, readable text where admitted). Server-authoritative; no client-side text assembly.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| none (plan row) | MERGED/none |
| item description content (item_description_promotion.rs, item_description_wiki_promotion.rs) | MERGED on main |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- apps/game-server/src/gameplay_transport/item_look.rs (new)
- apps/game-server/src/gameplay_transport/item_look_tests.rs (new)
- apps/game-server/src/interaction/look.rs (new)
- crates/protocol-oteryn/src/item_look.rs (new codec)
- crates/protocol-oteryn/src/item_look_tests.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs dispatch
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json (look rate row)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `trait FreshAdmissionAuthority` (l.331): new authority method (pattern: `npc_trade` l.696, `fight_modes` l.804)
- `connection.rs` `serve_admitted_actor` (l.1396): command-type dispatch arm and delta/snapshot frame (pattern: `quest_log_delta` l.3574, `chat_line_delta` l.3595)
- `mod.rs` `impl FreshAdmissionAuthority for ComposedFreshAdmission` (l.2866) and the `mod` list (l.5-53)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`
- `crates/protocol-oteryn/src/lib.rs`: `pub mod` + re-export of the new codec module; constants live in the module (pattern: `world_object.rs` `CAPABILITY_RUNE_USE_V1 = 22`; highest offered capability on main is 23, PRESENT-WIRE-1 #2124; numbers leased on #162 by CP)
- target resolution: reuse `gameplay_transport/item_ref_admission.rs` and `item_view.rs` (domain 9) for the looked-at item reference

## Inseparable bundles

wire+registry+capability+handler+tests. Registry order (plan §5): ITEM-LOOK-1, then SEC-1, then WEATHER-1.

## Acceptance criteria

- [ ] LOOK command on a visible item/creature returns text built only from admitted content facts.
- [ ] Unknown/unadmitted target -> typed refusal, no leak of hidden facts.
- [ ] Rate limit row registered and enforced (max vs max+1).
- [ ] No brief exists: builder records a short packet in this file before code; architect review at queue turn.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

Reference look text format (Tibia 'You see ...'), item description promotion modules on main. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 ITEM-LOOK-1 (row 75).

## Builder instructions

- Work on branch `skeleton/item-look-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/item-look-1
blocker: null
next_action: builder confirms residue against main and starts owned paths
```
