# ITEM-IMBUE-1 — imbuing (skeleton task record)

```yaml
task_id: ITEM-IMBUE-1
title: "ITEM-IMBUE-1: imbuing"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-IMBUE-1 (row 72); §7 'ITEM-TIMED-1' IMBUE-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-imbue-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Imbuements: content (24 types x 3 tiers, slots, shrines), item-state table with apply/clear/ImbueCause/checkpoints/expiry, ticking runtime and effects, scrolls and etcher, `IMBUE_V1` wire.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

IMBUE-CONTENT-1 (content), IMBUE-1 (hard), IMBUE-RT-1 (hard, combat), IMBUE-SCROLL-1 (hard), IMBUE-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| NPC-ECON-1 (per child) | BLOCKING |
| GOLD-FEE-2 | MERGED |
| ATTACK-1b | MERGED (#1798) |
| SPEED-1 | MERGED |
| PREMIUM-1 | MERGED (#2110) |
| NPC-TRADE-1 | MERGED |
| ITEM-SEM-USE / USE-WIRE-V1 / ITEM-MOVE-WIRE-1 | MERGED (archive records) |
| acceptance of IMBUE_FORGE0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/imbuement.rs`
- `apps/game-server/src/durability/imbuement.rs`
- `apps/game-server/src/durability/imbuement_tests.rs`
- `apps/game-server/src/combat/imbuement_effects.rs`
- `apps/game-server/src/gameplay_transport/imbue.rs`
- `apps/game-server/src/gameplay_transport/imbue_tests.rs`
- `crates/protocol-oteryn/src/imbue.rs`
- `crates/protocol-oteryn/src/imbue_tests.rs`
- `apps/game-server/tests/imbuement_postgres.rs`
- content authoring inputs for imbuement types/shrines (source side)
- `docs/agents/tasks/ITEM-IMBUE-1.md` (this record: status and notes only)

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

- apps/game-server/src/domain/timed_item.rs and timed_item_host.rs (checkpoint/expiry pattern)
- ability/combat pipeline hook for IMBUE-RT-1
- connection.rs dispatch; capabilities.rs `GATED` (`IMBUE_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + imbue writer + durability
- gold fee burn + imbuement row + ledger (mint/burn + ledger)
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Apply charges fee and writes state atomically; clear is idempotent.
- [ ] Ticking checkpoints survive restart; expiry writes exactly once.
- [ ] Effects pass combat parity tests.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server item
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md §3-§7, §14
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
