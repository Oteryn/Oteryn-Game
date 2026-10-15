# NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1

```yaml
task_id: NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1
title: "NPC-ECON-1+NPC-CMD-1b+NPC-TRADE-WIRE-1: bank, promotion purchase, NPC trade wire (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/npc-econ-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 NPC-ECON-1 (rows 39, 81); App. 7 NPC-ECON-1; wave 2"
class: "hard (value)"
migration: yes
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/npc_trade_command.rs
  - apps/game-server/src/gameplay_transport/npc_command.rs
  - apps/game-server/src/interaction/bank_npc.rs (new)
  - apps/game-server/tests/npc_econ_*.rs (new)
slot_only_paths:
  - connection.rs (`npc_trade` l.696 arm)
  - mod.rs
  - capabilities.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
  - migration number
  - content NPC pins
public_contracts: []
depends_on:
  - "#2040 NPC trade writer: MERGED"
  - "NPC-CMD-1b read window: MERGED #2035 (read part); residue confirmed by builder"
  - "BANK-1: MERGED #1766"
  - "GOLD-FEE-2: MERGED #1781"
  - "BANK-FEE-0 / BANK_FEE0 ACCEPTED: BLOCKING: CANDIDATE"
  - "NPC0 ACCEPTED: BLOCKING: CANDIDATE"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Bank NPC service, promotion purchase, and the NPC trade intent (command 8) wired to the merged writer (#2040).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2040 NPC trade writer | MERGED |
| NPC-CMD-1b read window | MERGED #2035 (read part); residue confirmed by builder |
| BANK-1 | MERGED #1766 |
| GOLD-FEE-2 | MERGED #1781 |
| BANK-FEE-0 / BANK_FEE0 ACCEPTED | BLOCKING: CANDIDATE |
| NPC0 ACCEPTED | BLOCKING: CANDIDATE |

**BLOCKING:** BANK-FEE-0 / BANK_FEE0 ACCEPTED; NPC0 ACCEPTED. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/gameplay_transport/npc_trade_command.rs`
- `apps/game-server/src/gameplay_transport/npc_command.rs`
- apps/game-server/src/interaction/bank_npc.rs (new)
- apps/game-server/tests/npc_econ_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs (`npc_trade` l.696 arm)
- mod.rs
- capabilities.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- migration number
- content NPC pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `npc_trade` (l.696)
- `durability/bank.rs`, `bank_audit.rs`
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

'+' bundle as one draft; mint/burn+ledger (gold); wire+registry+capability+handler.

## Acceptance criteria

- [ ] Trade command commits one DUR-03 transaction; bank fees burned through the ledger; idempotent retry.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

NPC0; BANK0; BANK_FEE0; GOLD_FEE_BOUNDARY. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 2 NPC-ECON-1 (rows 39, 81); App. 7 NPC-ECON-1.

## Builder instructions

- Work on branch `skeleton/npc-econ-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/npc-econ-1
blocker: BANK-FEE-0 / BANK_FEE0 ACCEPTED; NPC0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```
