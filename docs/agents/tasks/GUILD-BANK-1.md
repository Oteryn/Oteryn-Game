# GUILD-BANK-1 — guild balance, ledger and disband payout (skeleton task record)

```yaml
task_id: GUILD-BANK-1
title: "GUILD-BANK-1: guild balance, ledger and disband payout"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-BANK-1 (row 113); §7 'GUILD-*' row GUILD-BANK-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/guild-bank-1
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Guild balance and ledger, deposit and withdraw, disband payout and its custody claims (GUILD0 §5).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

single child GUILD-BANK-1 (hard, persistence and economy review).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| GUILD-1 | BLOCKING (Wave 4, skeleton only) |
| NPC-ECON-1 (BANK-1) | BANK-1 MERGED (#1766); NPC-ECON-1 family BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/guild_bank.rs`
- `apps/game-server/src/durability/guild_bank_audit.rs`
- `apps/game-server/src/durability/guild_bank_tests.rs`
- `apps/game-server/tests/guild_bank_postgres.rs`
- `docs/agents/tasks/GUILD-BANK-1.md` (this record: status and notes only)

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

- durability/bank.rs (BANK-1) ledger kinds: add guild ledger kinds per BANK-0 §3 amendment
- durability/mod.rs registration
- GAME_EVENT_FOUNDATION_REGISTRY.json / RESOURCE_LIMITS_REGISTRY.json rows

## Inseparable bundles (never split across PRs)

- migration + guild bank writer + durability tests
- deposit/withdraw = account balance delta + guild balance delta + ledger in one transaction
- disband payout + custody claims in the disband transaction of GUILD-1

## Acceptance criteria

- [ ] Deposit/withdraw conserve gold across account and guild balances (property test).
- [ ] Disband pays out once; custody claims survive restart.
- [ ] Junior and Premium rules of BANK-0 apply.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server guild
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md §5
- apps/game-server/src/durability/bank.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
