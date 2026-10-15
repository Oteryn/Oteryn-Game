# ADMIT-CAP-1

```yaml
task_id: ADMIT-CAP-1
title: "ADMIT-CAP-1: World capability admission authority (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/admit-cap-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 ADMIT-1 (none, security prerequisite); App. 7 ADMIT; wave 1"
class: "hard (authority, protocol); protocol and security review"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
  - apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
  - apps/game-server/tests/world_required_capabilities*.rs (new)
slot_only_paths:
  - capabilities.rs (required set)
  - connection.rs admit path (`admit_frame` l.1101, `required_capability` l.363)
  - PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "ADMIT0 decision ACCEPTED: BLOCKING: `OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md` Status CANDIDATE"
  - "numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162: BLOCKING: CP verifies reservation (WORLDINT-WIRE-1 #2094 offers capability 11)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

The World admission refuses a session that does not select the capabilities a World requires (`MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1`), per ADMIT0. One admission authority, fail closed, before SEC-CHAL-1/2.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ADMIT0 decision ACCEPTED | BLOCKING: `OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md` Status CANDIDATE |
| numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162 | BLOCKING: CP verifies reservation (WORLDINT-WIRE-1 #2094 offers capability 11) |

**BLOCKING:** ADMIT0 decision ACCEPTED; numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/gameplay_transport/world_required_capabilities.rs (new)
- apps/game-server/src/gameplay_transport/world_required_capabilities_tests.rs (new)
- apps/game-server/tests/world_required_capabilities*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- capabilities.rs (required set)
- connection.rs admit path (`admit_frame` l.1101, `required_capability` l.363)
- PROTOCOL_OTERYN_V1_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `admit_frame` (l.1101) and `FreshAdmissionAuthority::required_capability` (l.363)
- `capabilities.rs` offered sets `ITEM_VIEW_OFFERED_CAPABILITIES` / `BUNDLE_WORLD_OFFERED_CAPABILITIES` (l.100-150) and `GATED` (l.165); `capabilities_tests.rs` keeps them equal to `PROTOCOL_OTERYN_V1_REGISTRY.json`

## Inseparable bundles

wire+registry+capability+handler+tests in one PR.

## Acceptance criteria

- [ ] A session missing any World-required capability is refused at admission with the ADMIT0 code; no partial admission.
- [ ] Registry entries and `capabilities_tests.rs` agree.
- [ ] Negative cases: each required capability missing alone.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

ADMIT0 §2-§4; plan App. 7 ADMIT; Reference login capability negotiation is not applicable (Oteryn-native). Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 6 ADMIT-1 (none, security prerequisite); App. 7 ADMIT.

## Builder instructions

- Work on branch `skeleton/admit-cap-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/admit-cap-1
blocker: ADMIT0 decision ACCEPTED; numbers of MAP_STATE_V1, WORLD_SPATIAL_FIELDS, WORLD_INTERACTION_V1 reserved on #162
next_action: CP rechecks BLOCKING rows
```
