# ITEM-FORGE-1 — exaltation forge (skeleton task record)

```yaml
task_id: ITEM-FORGE-1
title: "ITEM-FORGE-1: exaltation forge"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row ITEM-FORGE-1 (row 73); §7 'ITEM-TIMED-1' FORGE-*/TIER-EFFECT-1/MARKET-TIER-1 rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/item-forge-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Exaltation forge on top of FORGE-1a (dust asset, merged): tiers, fusion, transfer, conversions and ForgeCause (FORGE-1b), content, `FORGE_V1` wire, influenced/fiendish creatures, tier effects, Market tier wares.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

FORGE-CONTENT-1, FORGE-1b (hard), FORGE-WIRE-1 (Reg), FORGE-CREATURE-1 (combat), TIER-EFFECT-1 (combat), MARKET-TIER-1 (after MARKET-1). FORGE-1a MERGED (#1691).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | BLOCKING |
| ITEM-MOVE-2a | BLOCKING |
| FORGE-1a | MERGED (#1691, migration 0059) |
| GOLD-FEE-2 | MERGED |
| GAME-AI-01 runtime | MERGED (foundation) |
| ATTACK-1b | MERGED |
| SPELL-D3 | MERGED |
| PREMIUM-1 | MERGED (#2110) |
| MARKET-1 (MARKET-TIER-1) | BLOCKING |
| acceptance of IMBUE_FORGE0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/forge.rs`
- `apps/game-server/src/durability/forge.rs`
- `apps/game-server/src/durability/forge_tests.rs`
- `apps/game-server/src/combat/tier_effects.rs`
- `apps/game-server/src/gameplay_transport/forge.rs`
- `apps/game-server/src/gameplay_transport/forge_tests.rs`
- `crates/protocol-oteryn/src/forge.rs`
- `crates/protocol-oteryn/src/forge_tests.rs`
- `apps/game-server/tests/forge_postgres.rs`
- `docs/agents/tasks/ITEM-FORGE-1.md` (this record: status and notes only)

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

- apps/game-server/src/domain/forge_dust.rs, durability/character_forge_dust.rs, character_forge_dust_audit.rs (FORGE-1a; extend, do not fork)
- apps/game-server/src/content/item_forge289_promotion.rs, item_forge3332_promotion.rs
- monster loot/death chain for FORGE-CREATURE-1
- connection.rs dispatch; capabilities.rs `GATED` (`FORGE_V1`)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + forge writer + durability
- fusion = item BURN + tier MINT + dust/gold ledger in one transaction
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Fusion/transfer conserve value per IMBUE_FORGE0 §8-§10; failure paths deterministic and replay-stable.
- [ ] Tier caps per classification enforced.
- [ ] Forge history bounded by `IMBFORGE0-RL-12`.
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

- docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md §8-§14
- archive OTV2-20261003-forge-1a.md
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
