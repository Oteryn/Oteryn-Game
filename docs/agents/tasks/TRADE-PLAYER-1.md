# TRADE-PLAYER-1 — player-to-player trade (skeleton task record)

```yaml
task_id: TRADE-PLAYER-1
title: "TRADE-PLAYER-1: player-to-player trade"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row TRADE-PLAYER-1 (row 82); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (TRADE-WIRE-1, TRADE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/trade-player-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Two players of one channel exchange items safely: capability, trade command and domain (TRADE-WIRE-1), trade session with TRANSFERRING state and the two-Character swap transaction with every persistence delta (TRADE-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

TRADE-WIRE-1 (impl, protocol review, Reg) -> TRADE-1 (hard, persistence and security review). BAGS-TRADE-1 is an integration child released by TRADE-1 and outside this completion set.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a (TRADE-1) | BLOCKING (ITEM-LOOP-1 child) |
| ITEM-VIEW-1b | MERGED (archive OTV2-20261003-item-view-1b) |
| VIS-2 | MERGED (archive OTV2-20260930-vis-2) |
| acceptance of PLAYER_TRADE0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/player_trade.rs`
- `apps/game-server/src/durability/player_trade.rs`
- `apps/game-server/src/durability/player_trade_tests.rs`
- `apps/game-server/src/gameplay_transport/player_trade.rs`
- `apps/game-server/src/gameplay_transport/player_trade_tests.rs`
- `crates/protocol-oteryn/src/player_trade.rs`
- `crates/protocol-oteryn/src/player_trade_tests.rs`
- `apps/game-server/tests/player_trade_postgres.rs`
- `docs/agents/tasks/TRADE-PLAYER-1.md` (this record: status and notes only)

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

- connection.rs: command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (see CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs: `GATED` table l.165 and offered sets l.76/104/132 (capability `TRADE_V1` not offered until CP decides)
- crates/protocol-oteryn/src/lib.rs: `pub mod` list l.2545-2568
- durability/mod.rs and domain/mod.rs module registration
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability, command type, state domain)

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests (TRADE-WIRE-1)
- migration + swap writer + durability tests (TRADE-1)
- item move/MINT-free swap: both inventory deltas + audit in one transaction (DUR-03 trade swap amendment)

## Acceptance criteria

- [ ] A trade between two Characters on one channel either commits both item deltas or none (crash/restart test).
- [ ] TRANSFERRING state survives restart and resolves deterministically; concurrent offers lose with a typed refusal.
- [ ] Cross-channel trade is refused.
- [ ] Wire gated by capability; older clients receive no trade domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server trade
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md §3-§6
- DUR-03 §34 trade swap paragraph (amended by PLAYER_TRADE0)
- ITEM-MOVE-WIRE-1 (archive OTV2-20260930-item-move-wire1)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
