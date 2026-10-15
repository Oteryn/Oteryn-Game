# GM-1 — GM tools, mute, kick, staff audit and reports (skeleton task record)

```yaml
task_id: GM-1
title: "GM-1: GM tools, mute, kick, staff audit and reports (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/gm-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 6 row GM-1 (row 136); App. 7 GM-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: yes
needs_registry: yes
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/staff/ (new: audit, sanctions, roster, reports) + tests"
  - "tools/oteryn-game-ops (new, signed requests)"
  - "docs/agents/tasks/GM-1.md"
slot_only_paths:
  - "migrations: game_character_sanctions, game_staff_roles, game_player_reports"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (`staff_action`)"
  - "apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs, crates/protocol-oteryn/src/lib.rs"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Game side of OPS-GM-01: OPS-GM-AUDIT-1, SANCTION-2, REPORT-3, ROSTER-4, STAFF-5, PREPORT-6. No value-creating staff action.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| LOGOUT-WIRE-1 | OPEN (#2127) | **BLOCKING** |
| WRITE-MOD-1 (REPORT-3 commands) | NOT MERGED | **BLOCKING** |
| OPS-GM-AUDIT-1 | none (first child, can start) | ok |
| OPS-GM-P1 (Platform), ARCH-CONTROL-ANCHOR-0 | external gate for EXT phase only | ok |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/staff/ (new: audit, sanctions, roster, reports) + tests
- tools/oteryn-game-ops (new, signed requests)

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- migrations: game_character_sanctions, game_staff_roles, game_player_reports
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (staff panel)
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (`staff_action`)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs, crates/protocol-oteryn/src/lib.rs
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (mute)
- logout path (LOGOUT-WIRE-1)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability per child
- wire + registry + capability + handler + tests (STAFF-5)

## Acceptance criteria

- [ ] every staff action emits an audited signed `staff_action` event
- [ ] sanctions are staff records, not Character aggregate writes
- [ ] no value-creating staff action
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server staff
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md §1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
