# HUNT-1 — prey and hunting tasks (skeleton task record)

```yaml
task_id: HUNT-1
title: "HUNT-1: prey and hunting tasks"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row HUNT-1 (rows 45, 46); §7 'HUNT-1, RANK-1' PREY0 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/hunt-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Prey slots, resource and wildcard balances, ledger, draws, hunting-time checkpoints (PREY-1); gold reroll fee; prey effects; Task Board, delivery, shop; `PREY_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

PREY-CONTENT-1, PREY-1 (hard), PREY-FEE-1, PREY-EFFECT-1 (combat), TASKBOARD-1 (hard), TASKBOARD-DELIVERY-1, TASKSHOP-1, PREY-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-MONSTER-1 (family entry gate) | BLOCKING (Wave 2) |
| PREMIUM-1 | MERGED (#2110) |
| CHAR-REV-SEQ-1 | MERGED |
| GOLD-FEE-1b | MERGED |
| ATTACK-0 / SPELL-TARGET-1 | ATTACK-0 ACCEPTED; SPELL-TARGET-1: verify |
| ITEM-USE-1c (TASKBOARD-DELIVERY-1) | BLOCKING (Wave 0 residue) |
| acceptance of PREY0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/prey.rs`
- `apps/game-server/src/domain/task_board.rs`
- `apps/game-server/src/durability/prey.rs`
- `apps/game-server/src/durability/task_board.rs`
- `apps/game-server/src/durability/prey_tests.rs`
- `apps/game-server/src/gameplay_transport/prey.rs`
- `apps/game-server/src/gameplay_transport/prey_tests.rs`
- `crates/protocol-oteryn/src/prey.rs`
- `crates/protocol-oteryn/src/prey_tests.rs`
- `apps/game-server/tests/prey_postgres.rs`
- `content/rulesets/progression/prey/ and content/rulesets/progression/task-board/ authoring inputs`
- `docs/agents/tasks/HUNT-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- death chain kill receipt for task kills (gameplay_transport/kill_reward.rs)
- XP lane (D118) for the multiplier
- connection.rs dispatch; capabilities.rs `GATED` (`PREY_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + prey/task writers + durability
- reroll fee burn + ledger; task shop debit + item MINT + ledger
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Weekly reset settles the finished week under one barrier (TASKBOARD obligation a).
- [ ] Replay-stable overflow policy at `PREY0-RL-14`.
- [ ] Hunting-time checkpoints survive restart.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server hunt
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md §3-§11
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
