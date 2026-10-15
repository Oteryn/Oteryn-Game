# PREM-2b+PREM-5

```yaml
task_id: PREM-2b+PREM-5
title: "PREM-2b+PREM-5: promotion state and promotion/Premium blessing NPC services (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/prem-2b-prem-5
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 PREREQ-1; App. 7 Supporting PREM-2b+PREM-5; wave 2"
class: "hard"
migration: maybe
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/premium.rs
  - apps/game-server/src/premium/promotion.rs (new)
  - apps/game-server/src/interaction/npc_promotion.rs (new)
  - apps/game-server/tests/promotion_*.rs (new)
slot_only_paths:
  - migration number
  - mod.rs
  - content NPC service pins
public_contracts: []
depends_on:
  - "PREM-WIRE-1 (PREMIUM-1): MERGED #2110"
  - "NPC-TALK-1: MERGED #1976"
  - "PREMIUM_ACTIVATION ACCEPTED: MERGED #2089"
  - "a promotion source: BLOCKING: held, owner/CP decision on the promotion source"
  - "spell P3b-2 vitals, progression readiness: UNKNOWN: CP verifies"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

(PREM-2b) promotion state, effective promotion at login, soul maxima, death input, Wheel promotion read; (PREM-5) promotion and Premium blessing NPC services. One allocation.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| PREM-WIRE-1 (PREMIUM-1) | MERGED #2110 |
| NPC-TALK-1 | MERGED #1976 |
| PREMIUM_ACTIVATION ACCEPTED | MERGED #2089 |
| a promotion source | BLOCKING: held, owner/CP decision on the promotion source |
| spell P3b-2 vitals, progression readiness | UNKNOWN: CP verifies |

**BLOCKING:** a promotion source. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/domain/premium.rs`
- apps/game-server/src/premium/promotion.rs (new)
- apps/game-server/src/interaction/npc_promotion.rs (new)
- apps/game-server/tests/promotion_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- mod.rs
- content NPC service pins
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `domain/premium.rs` (PREM-2a pure rules)
- `gameplay_transport/npc_talk.rs` service hook
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

PREM-2b+PREM-5 is one '+' bundle; migration+writer; gold burn+ledger for the promotion price.

## Acceptance criteria

- [ ] Promotion fail closed without Premium; price burned through the fee ledger.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

PREMIUM_ACTIVATION0 §2-§4.3; WHEEL0 §6.2. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 PREREQ-1; App. 7 Supporting PREM-2b+PREM-5.

## Builder instructions

- Work on branch `skeleton/prem-2b-prem-5`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/prem-2b-prem-5
blocker: a promotion source
next_action: CP rechecks BLOCKING rows
```
