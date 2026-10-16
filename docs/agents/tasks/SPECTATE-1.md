# SPECTATE-1 — spectating

```yaml
task_id: SPECTATE-1
title: "SPECTATE-1: spectating"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/spectate-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row SPECTATE-1 (row 138); App. 7 LOOP-WORLD-1, SPECTATE-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: false
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/spectate/ (new) + tests"
  - "docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md (D1095)"
  - "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md (SPECTATE row only, D1095)"
  - "docs/agents/tasks/SPECTATE-1.md"
slot_only_paths:
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
  - "crates/protocol-oteryn/src/lib.rs"
  - "apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Spectating (read-only view of a player session).

## Accepted ruling and delivery boundaries (2026-10-16)

The [CP/architect ruling D1035–D1039](https://github.com/Oteryn/Oteryn-Game/pull/2183#issuecomment-6216592075) accepts the brief with amendments and supersedes the conflicting skeleton requirements below. The owner already chose Global parity option (b), an accepted Oteryn deviation; no owner decision is pending. O1–O12 remain unchanged. `SPECTATE_V1` is offered only after the three children merge and CP enables it.

- SPECTATE-GRANT-1 and SPECTATE-SESSION-1 land as commits on one mechanic PR. Audit uses the existing event foundation.
- SPECTATE-WIRE-1 is a separate PR with protocol review; `spectate.rs` belongs only to that child.
- Observation has no durable observer state. Any required migration is a separate PR, not a slot commit in this mechanic PR.
- O2 Gateway admission is a separate child allocated by CP; this task does not allocate or implement that boundary.
- D1095 clarifies D1038: this existing #2183 authors the accepted brief and only the SPECTATE plan row, then GRANT/SESSION. WIRE-1 and O2 Gateway are not allocated until green GRANT/SESSION. No merge-main or shared-path slot is implied.

## Dependencies (D1095 current CP confirmation)

| Dependency | State | Gate |
|---|---|---|
| Accepted SPECTATE contract | D1035–D1039; D1095 authoring on #2183 | Brief/one-row plan commit precedes runtime commits |
| FND-04B | On main, confirmed by D1095 | Satisfied for GRANT/SESSION |
| MAP-WIRE-2 | On main, confirmed by D1095 | Satisfied for GRANT/SESSION |
| CHAT-1b-2b | #2093 on main, confirmed by D1095 | Satisfied for GRANT/SESSION |
| SCOPE-HANDOFF-1 | Archived on main, confirmed by D1095 | Satisfied for GRANT/SESSION |
| O2 Gateway / WIRE-1 | CP allocates after green GRANT/SESSION | Later boundary; no invented Gateway/protocol authority |

LOOP-WORLD-1 is not a prerequisite. Existing static Candidate labels do not override D1095's prerequisite confirmation.

## Owned paths (builder edits freely)

- apps/game-server/src/spectate/ (new) + tests
- docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md (D1095)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md (SPECTATE row only, D1095)
- docs/agents/tasks/SPECTATE-1.md

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
- crates/protocol-oteryn/src/lib.rs
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

Shared module registration and wiring require the exact CP slot and its allocated author. Any migration requires a separate PR under D1036.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/gameplay_transport/world_map.rs, world_spatial.rs (view stream)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Mechanic and wire delivery (D1036)

- GRANT-1 + SESSION-1, including their tests, form one mechanic PR.
- WIRE-1, including its codec, registry, capability, projection privacy filter and tests, is a separate PR with protocol review and CP-allocated shared-path work.
- Any migration is a separate PR; Gateway O2 admission is a separately allocated child.

## Acceptance criteria

- [ ] spectator cannot issue any gameplay command
- [ ] privacy: consent of the spectated player
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] No durable observer state; restart ends observer sessions (O11). Target session-generation change ends its grant and observer streams (O3).
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] All registered gameplay command types are refused before a gameplay handler runs, with no state change.
- [ ] Projection privacy tests exclude inventory, private chat and command outcomes.
- [ ] Audit uses the existing event foundation; activation waits for all three children and CP enablement.

## Validation commands

```sh
cargo test -p oteryn-game-server spectate
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- CP/architect D1035–D1039: https://github.com/Oteryn/Oteryn-Game/pull/2183#issuecomment-6216592075
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §7 LOOP-WORLD-1, SPECTATE-1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Checkpoint

AUTHORING resumed from `77c6d68fa5f56ceecc37a2c7e65bc38cbad74886` under [D1095](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6219643346). The first commit reconciles the accepted brief and only the SPECTATE plan row. GRANT-1/SESSION-1 follow in owned runtime paths. Shared module/transport registration, real event publication, WIRE and O2 Gateway are not silently allocated. Activation, ready, queue and merge remain CP actions. No runtime completion is claimed by this architecture commit.
