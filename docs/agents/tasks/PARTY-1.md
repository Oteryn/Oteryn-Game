# PARTY-1

```yaml
task_id: PARTY-1
title: "PARTY-1: party membership and state (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/party-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 PARTY-1 (row 112); App. 7 PARTY/PVP; wave 2"
class: "hard (shared state)"
migration: yes
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/world_party.rs
  - apps/game-server/src/foundation/runtime_actor_party.rs
  - apps/game-server/src/party/ (new)
  - crates/protocol-oteryn/src/party*.rs (new)
  - apps/game-server/tests/party_*.rs (new)
slot_only_paths:
  - migration number
  - connection.rs
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "CHAT-2: BLOCKING: CHAT-2A merged #2076; CHAT-2B #2128 / 2C in flight"
  - "PARTY_PVP0: CONFLICT: plan §7 lists ACCEPTED, decision file Status line CANDIDATE; CP verifies"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Party invite/join/leave, succession, shared-XP toggle, `PartyView` cache (membership only).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CHAT-2 | BLOCKING: CHAT-2A merged #2076; CHAT-2B #2128 / 2C in flight |
| PARTY_PVP0 | CONFLICT: plan §7 lists ACCEPTED, decision file Status line CANDIDATE; CP verifies |

**BLOCKING:** CHAT-2; PARTY_PVP0. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/durability/world_party.rs`
- `apps/game-server/src/foundation/runtime_actor_party.rs`
- apps/game-server/src/party/ (new)
- crates/protocol-oteryn/src/party*.rs (new)
- apps/game-server/tests/party_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- connection.rs
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `trait FreshAdmissionAuthority` (l.331): new authority method (pattern: `npc_trade` l.696, `fight_modes` l.804)
- `connection.rs` `serve_admitted_actor` (l.1396): command-type dispatch arm and delta/snapshot frame (pattern: `quest_log_delta` l.3574, `chat_line_delta` l.3595)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`
- `gameplay_transport/party_spell_owner.rs` reads membership

## Inseparable bundles

wire+registry+capability+handler+tests; migration+writer+durability.

## Acceptance criteria

- [ ] Membership consistent across nodes; leader succession deterministic.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PARTY_PVP0 §30-33. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 5 PARTY-1 (row 112); App. 7 PARTY/PVP.

## Builder instructions

- Work on branch `skeleton/party-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/party-1
blocker: CHAT-2; PARTY_PVP0
next_action: CP rechecks BLOCKING rows
```
