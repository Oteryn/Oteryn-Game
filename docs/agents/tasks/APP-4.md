# APP-4 — client outfit window and other-actor rendering (skeleton task record)

```yaml
task_id: APP-4
title: "APP-4: client outfit window and other-actor rendering"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row APP-4 (rows 126, 127); §7 supporting briefs row APP-4"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/app-4
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Client outfit window and rendering of other actors' looks.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

APP-4 (impl, client).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| APP-3 | BLOCKING |
| client owner (#1942) | BLOCKING (#1942 open draft) |
| acceptance of CHARACTER_APPEARANCE (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/client/ outfit window module (new files only)`
- `crates/client-runtime/ appearance rendering (new files only)`
- `docs/agents/tasks/APP-4.md` (this record: status and notes only)

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

- client panel registry in apps/client (coordinate with #1942 owner)
- crates/protocol-oteryn appearance codec from APP-3

## Inseparable bundles (never split across PRs)

- client window + renderer + tests

## Acceptance criteria

- [ ] Outfit window lists unlocked looks only; preview matches committed look.
- [ ] Other actors render with their committed look.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server app
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md §5
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
