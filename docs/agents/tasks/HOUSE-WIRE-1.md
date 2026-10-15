# HOUSE-WIRE-1 — HOUSE_V1 capability, house list, bids, move-out (skeleton task record)

```yaml
task_id: HOUSE-WIRE-1
title: "HOUSE-WIRE-1: HOUSE_V1 capability, house list, bids, move-out (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-wire-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-WIRE-1 (rows 119, 121); App. 7 HOUSE-*; §5 Wave 5"
class: "impl (protocol review)"
needs_migration: no
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "crates/protocol-oteryn/src/house.rs (new)"
  - "crates/protocol-oteryn/src/house_tests.rs"
  - "apps/game-server/src/gameplay_transport/house_intent.rs (new)"
  - "apps/game-server/src/gameplay_transport/house_intent_tests.rs"
  - "docs/contracts/protocol-oteryn/v1/house_v1.proto (new)"
  - "client House Management panel (coordinate with client lane #1942)"
  - "docs/agents/tasks/HOUSE-WIRE-1.md"
slot_only_paths:
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability and command numbers leased at queue turn)"
  - "crates/protocol-oteryn/src/lib.rs (module export)"
  - "apps/game-server/src/gameplay_transport/connection.rs (dispatch arm)"
  - "apps/game-server/src/gameplay_transport/mod.rs"
  - "apps/game-server/src/gameplay_transport/capabilities.rs (+capabilities_tests.rs)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Capability `HOUSE_V1`, `HOUSE_INTENT` commands (house list, bids, move-out, ACL edits) and the House Management panel.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1b | NOT MERGED | **BLOCKING** |
| HOUSE-ACL-1 | NOT MERGED | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- crates/protocol-oteryn/src/house.rs (new)
- `crates/protocol-oteryn/src/house_tests.rs`
- apps/game-server/src/gameplay_transport/house_intent.rs (new)
- `apps/game-server/src/gameplay_transport/house_intent_tests.rs`
- docs/contracts/protocol-oteryn/v1/house_v1.proto (new)
- client House Management panel (coordinate with client lane #1942)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability and command numbers leased at queue turn)
- crates/protocol-oteryn/src/lib.rs (module export)
- apps/game-server/src/gameplay_transport/connection.rs (dispatch arm)
- apps/game-server/src/gameplay_transport/mod.rs
- apps/game-server/src/gameplay_transport/capabilities.rs (+capabilities_tests.rs)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/connection.rs command dispatch
- apps/game-server/src/gameplay_transport/capabilities.rs offered set
- crates/protocol-oteryn/src/lib.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- wire + registry + capability + handler + tests (one commit at slot)

## Acceptance criteria

- [ ] older clients without HOUSE_V1 receive nothing new; commands rejected without capability
- [ ] flat message bounds in RESOURCE_LIMITS
- [ ] handlers call HOUSE-1a/1b/ACL writers only
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-protocol-oteryn house
cargo test -p oteryn-game-server capabilities house_intent
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
- docs/agents/tasks/archive/WORLDINT-WIRE-1.md (wire pattern)
- `docs/reference/tibia-manual/houses.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
