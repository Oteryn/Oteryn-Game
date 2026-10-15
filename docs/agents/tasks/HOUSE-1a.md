# HOUSE-1a — house ownership, auction and rent (skeleton task record)

```yaml
task_id: HOUSE-1a
title: "HOUSE-1a: house ownership, auction and rent (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-1a
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-1a (row 119); App. 7 HOUSE-*; §5 Wave 5"
class: "hard (persistence, economy, security)"
needs_migration: yes
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house/ (new: ownership, auction, rent)"
  - "apps/game-server/src/durability/house_ownership.rs (new)"
  - "apps/game-server/src/house/*_tests.rs"
  - "apps/game-server/tests/house_ownership_*.rs"
  - "docs/agents/tasks/HOUSE-1a.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_house_ownership.sql (number leased by CP at queue turn)"
  - "apps/game-server/src/lib.rs / mod.rs module wiring"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

House ownership, auction (bid escrow) and rent per HOUSE-OWN-0 §3-§9 and SOCIAL_MAP_PACKETS HOUSE-1a.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BANK-1 (NPC-ECON-1) | MERGED (archive OTV2-20261004-bank-1) | ok |
| HOUSE-RUNTIME-1a | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-RUNTIME-1b | NOT MERGED (Wave 3) | **BLOCKING** |
| HOUSE-RUNTIME-1c | NOT MERGED (Wave 3) | **BLOCKING** |
| PREM-WIRE-1 (PREMIUM-1) | MERGED (#2110) | ok |
| HOUSE_OWN0 acceptance (brief CANDIDATE; packet accepted via SOCIAL_MAP_PACKETS) | verify before ready | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house/ (new: ownership, auction, rent)
- apps/game-server/src/durability/house_ownership.rs (new)
- `apps/game-server/src/house/*_tests.rs`
- `apps/game-server/tests/house_ownership_*.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_house_ownership.sql (number leased by CP at queue turn)
- apps/game-server/src/lib.rs / mod.rs module wiring
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- docs/contracts/RESOURCE_LIMITS_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/house_scope/mod.rs, handoff.rs (house scope)
- apps/game-server/src/durability/world_house_instance.rs
- apps/game-server/src/content/project/native_house_tiles.rs
- bank writer from BANK-1 (archive record)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- mint/burn + ledger (bid escrow, rent debit through the bank ledger)

## Acceptance criteria

- [ ] auction bid escrow and refund are exactly-once under the session fence
- [ ] rent debit idempotent per period; eviction path deterministic
- [ ] Premium gate fails closed
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house
cargo test -p oteryn-game-server --test '*house*' (Postgres e2e)
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md §3-§9
- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (HOUSE-1a)
- `docs/reference/tibia-manual/houses.md`
- `docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
