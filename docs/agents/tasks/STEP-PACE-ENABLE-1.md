# STEP-PACE-ENABLE-1 — enable step-wait receipts rollout switch (skeleton task record)

```yaml
task_id: STEP-PACE-ENABLE-1
title: "STEP-PACE-ENABLE-1: enable step-wait receipts rollout switch (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/step-pace-enable-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row STEP-PACE-ENABLE-1 (rows 3, 4); §5 Wave 5"
class: "ops (rollout switch)"
needs_migration: no
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "deploy/synology-game/README.md (rollout section)"
  - "deploy/synology-game/supervisor.sh"
  - "deploy/synology-game/test-supervisor-npc.sh"
  - "docs/agents/tasks/STEP-PACE-ENABLE-1.md"
slot_only_paths:
  - "none expected (no content, registry, migration or wiring change); if a default flip in `world_spatial.rs` is chosen instead of the host marker, that file becomes slot-only"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Turn on `OTERYN_STEP_WAIT_RECEIPTS=1` for the serve process (residue of STEP-PACE-WIRE-1, #2041) once the client with the capability 13 field 2 decoder ships and the 9c and 6c3 builds are retired.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| STEP-PACE-WIRE-1 | MERGED (#2041) | ok |
| STEP-PACE-CLIENT-1 / client release with capability 13 field 2 decoder (client-owned #1942) | OPEN (#1942 draft) | **BLOCKING** |
| retirement of the 9c and 6c3 client builds | NOT DONE | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- deploy/synology-game/README.md (rollout section)
- `deploy/synology-game/supervisor.sh`
- `deploy/synology-game/test-supervisor-npc.sh`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- none expected (no content, registry, migration or wiring change); if a default flip in `world_spatial.rs` is chosen instead of the host marker, that file becomes slot-only
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/world_spatial.rs:530 `STEP_WAIT_RECEIPTS_ENV`
- apps/game-server/src/gameplay_transport/connection.rs:497 (read once per connection)
- deploy/synology-game/supervisor.sh:68 (marker file `BASE/run/step-wait-receipts`)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- switch + deploy docs + supervisor test in one PR; the host marker itself is an owner (1001) production action, not a repo change

## Acceptance criteria

- [ ] receipts are emitted only for sessions that selected capability 13 when the switch is on; byte-identical step results when off
- [ ] client release and build retirement evidenced by link before ready
- [ ] no production action taken by the builder
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server step_wait
sh deploy/synology-game/test-supervisor-npc.sh
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/agents/tasks/archive/OTV2-20261013-step-pace-wire-1.md`
- `deploy/synology-game/README.md:127`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
