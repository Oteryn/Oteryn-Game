# SEC-1

```yaml
task_id: SEC-1
title: "SEC-1: server-side client integrity and anti-bot checks (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/sec-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 SEC-1 (row 135); App. 7 SEC-1; wave 1"
class: "hard (child graph SEC-REL-1, SEC-CHAL-1, SEC-TELEM-1, SEC-DETECT-1, SEC-CHAL-2)"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/security/ (new module dir)
  - apps/game-server/tests/security_*.rs (new)
  - crates/protocol-oteryn/src/client_integrity*.rs (new)
slot_only_paths:
  - connection.rs
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "CLOCK-1 residue: MERGED #2114"
  - "ITEM-LOOK-1 (registry order): BLOCKING: not merged (draft skeleton/item-look-1)"
  - "ADMIT-CAP-1 (for SEC-CHAL-1/2): BLOCKING: not merged; ADMIT0 CANDIDATE"
  - "SEC_CLIENT01 ACCEPTED: BLOCKING: decision Status CANDIDATE"
  - "PROD-COMPAT-01 release train, ANL-01, ANL-03, DATA-PRIVACY-01 disclosure: BLOCKING/UNKNOWN: CP verifies"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Server-side integrity checks per SEC_CLIENT01 (CLIENT_INTEGRITY, INPUT_TELEMETRY). Pacing stays in LOOP-WORLD-1; 9c/6c3 retirement is separate.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CLOCK-1 residue | MERGED #2114 |
| ITEM-LOOK-1 (registry order) | BLOCKING: not merged (draft skeleton/item-look-1) |
| ADMIT-CAP-1 (for SEC-CHAL-1/2) | BLOCKING: not merged; ADMIT0 CANDIDATE |
| SEC_CLIENT01 ACCEPTED | BLOCKING: decision Status CANDIDATE |
| PROD-COMPAT-01 release train, ANL-01, ANL-03, DATA-PRIVACY-01 disclosure | BLOCKING/UNKNOWN: CP verifies |

**BLOCKING:** ITEM-LOOK-1 (registry order); ADMIT-CAP-1 (for SEC-CHAL-1/2); SEC_CLIENT01 ACCEPTED; PROD-COMPAT-01 release train, ANL-01, ANL-03, DATA-PRIVACY-01 disclosure. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/security/ (new module dir)
- apps/game-server/tests/security_*.rs (new)
- crates/protocol-oteryn/src/client_integrity*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

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
- `crates/protocol-oteryn/src/lib.rs`: `pub mod` + re-export of the new codec module; constants live in the module (pattern: `world_object.rs` `CAPABILITY_RUNE_USE_V1 = 22`; highest offered capability on main is 23, PRESENT-WIRE-1 #2124; numbers leased on #162 by CP)
- `gameplay_transport/input_gate.rs` (OPS-SAVE-2 input permit) as the per-command observation point

## Inseparable bundles

Each SEC-* child with Reg=yes is wire+registry+capability+handler+tests. Allocate children in order SEC-REL-1, SEC-CHAL-1, SEC-TELEM-1, SEC-DETECT-1, SEC-CHAL-2; split into child PRs if more than one becomes ready.

## Acceptance criteria

- [ ] Only children whose brief is ACCEPTED and deps merged are implemented.
- [ ] Privacy: telemetry collection off in production until DATA-PRIVACY-01 disclosure.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

SEC_CLIENT01 decision; plan App. 7 SEC-1. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 6 SEC-1 (row 135); App. 7 SEC-1.

## Builder instructions

- Work on branch `skeleton/sec-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/sec-1
blocker: ITEM-LOOK-1 (registry order); ADMIT-CAP-1 (for SEC-CHAL-1/2); SEC_CLIENT01 ACCEPTED; PROD-COMPAT-01 release train, ANL-01, ANL-03, DATA-PRIVACY-01 disclosure
next_action: CP rechecks BLOCKING rows
```
