# VIP-2 — VIP presence on the World relay (skeleton task record)

```yaml
task_id: VIP-2
title: "VIP-2: VIP presence on the World relay (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/vip-2
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row VIP-2 (row 114); App. 7 VIP-*; §5 Wave 5"
class: "hard (security, privacy)"
needs_migration: maybe
needs_registry: maybe
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/vip/presence.rs (new) + tests"
  - "docs/agents/tasks/VIP-2.md"
slot_only_paths:
  - "apps/game-server/src/chat/ relay wiring"
  - "apps/game-server/src/gameplay_transport/connection.rs (presence push)"
  - "migration if a consent column is needed"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

VIP online presence over the World chat relay, gated by consent and `vip_status` (hide my status) per VIP0 §5.1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| VIP0 acceptance (CANDIDATE) | NOT ACCEPTED | **BLOCKING** |
| VIP-1 | NOT MERGED (Wave 4) | **BLOCKING** |
| CHAT-2 (relay; CHAT-2B) | OPEN (#2128) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/vip/presence.rs (new) + tests

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/src/chat/ relay wiring
- apps/game-server/src/gameplay_transport/connection.rs (presence push)
- migration if a consent column is needed
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (World relay from CHAT-2)
- apps/game-server/src/gameplay_transport/chat_intent.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- presence + consent gate + tests in one PR

## Acceptance criteria

- [ ] no presence leaks without mutual consent
- [ ] hidden status never broadcast
- [ ] relay rate-limited
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server vip
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md §5.1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
