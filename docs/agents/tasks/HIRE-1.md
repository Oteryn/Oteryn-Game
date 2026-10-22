# HIRE-1 — hirelings (skeleton task record)

```yaml
task_id: HIRE-1
title: "HIRE-1: hirelings (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/hire-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 2 row HIRE-1 (row 90); §5 Wave 5"
class: "impl"
needs_migration: yes
needs_registry: yes  # HIRE1-RL-01..04 (RESOURCE_LIMITS_REGISTRY) and the HIRE-WIRE-1 capability/wire limits
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/hireling/ (new) + tests"
  - "content/npcs hireling inputs (source only)"
  - "docs/agents/tasks/HIRE-1.md"
slot_only_paths:
  - "apps/game-server/migrations/00NN_hirelings.sql"
  - "regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in crates/gameplay/dispatch/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Hirelings (house NPC servants: purchase, placement in the house, services).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1b | NOT MERGED | **BLOCKING** (HIRE-DATA-1 extends the HOUSE-1b rent, move-out, eviction and disposition transaction; HOUSE-1a excludes it) |
| NPC-ECON-1 | NOT MERGED (Wave 2) | **BLOCKING** |
| PREMIUM-1 | MERGED (#2110) | ok |
| acceptance of the HIRE1 brief (`OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md`) | ACCEPTED by the owner 2026-10-15 ("akceptuje hire i craft"), recorded in ACCEPT-BATCH-2026-10-15 §7; effective when this PR is integrated. Delivery still waits for the Platform Store contract (#1625) | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/hireling/ (new) + tests
- content/npcs hireling inputs (source only)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_hirelings.sql
- regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- house ownership and disposition (HOUSE-1b)
- NPC trade/bank (NPC-ECON-1)
- premium gate
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- content + regeneration + pins by one author

## Acceptance criteria

- [ ] only house owner may place a hireling
- [ ] delivery consumed from the Platform Store: a signed, idempotent Store delivery is applied once per delivery id and reported by status callback; no price, currency or payment data exists in Game, and Game never sells a hireling for gold
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server hireling
python3 tools/content-migration/regenerate_content.py && git diff --exit-code
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 row HIRE-1
- `docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
