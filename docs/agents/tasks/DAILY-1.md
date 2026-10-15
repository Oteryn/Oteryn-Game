# DAILY-1 — daily reward (skeleton task record)

```yaml
task_id: DAILY-1
title: "DAILY-1: daily reward"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row DAILY-1 (row 47); §7 'DAILY-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/daily-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Reward state, joker, claim and line tables; claim transaction; item binding table; line delivery to Inbox and wildcard balance (DAILY-1) with content, bind refusals and `DAILY_REWARD_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

DAILY-CONTENT-1, DAILY-1 (hard), DAILY-BIND-1, DAILY-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| NPC-ECON-1 | BLOCKING |
| MARKET-1 (`CharacterInbox` usage) | BLOCKING (Wave 4) |
| HUNT-1 / PREY-1 (wildcard balance) | BLOCKING (Wave 4) |
| ADR-0021 World reset epoch | ADR present on main (docs/architecture/ADR-0021-world-map-runtime-loading.md); reset-epoch runtime: verify |
| PREMIUM-1 | MERGED (#2110) |
| WO-1 | MERGED |
| EXERCISE-CONTENT-1, RUNE-USE content | verify (RUNE-USE-0 accepted, archive OTV2-20261014-accept-rune-use-0) |
| WORLDINT-USE-1 (DAILY-WIRE-1) | verify (WORLDINT-WIRE-1 archived) |
| acceptance of DAILY_REWARD0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/daily_reward.rs`
- `apps/game-server/src/durability/daily_reward.rs`
- `apps/game-server/src/durability/daily_reward_tests.rs`
- `apps/game-server/src/gameplay_transport/daily_reward.rs`
- `crates/protocol-oteryn/src/daily_reward.rs`
- `crates/protocol-oteryn/src/daily_reward_tests.rs`
- `apps/game-server/tests/daily_reward_postgres.rs`
- content authoring inputs for shrine bindings, rune/potion lists, training weapons, scrolls
- `docs/agents/tasks/DAILY-1.md` (this record: status and notes only)

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

- durability/character_inbox.rs delivery
- PREY-1 wildcard balance writer
- connection.rs dispatch; capabilities.rs `GATED` (`DAILY_REWARD_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + claim writer + durability
- claim = line MINT into Inbox + wildcard credit + ledger in one transaction
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] One claim per reset epoch; streak and joker rules per §3.
- [ ] Bound items refused at every §6 site.
- [ ] Claim idempotent across reconnect.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server daily
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md §3-§7
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
