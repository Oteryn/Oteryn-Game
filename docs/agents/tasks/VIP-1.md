# VIP-1 — VIP list store, consent lifecycle and wire (skeleton task record)

```yaml
task_id: VIP-1
title: "VIP-1: VIP list store, consent lifecycle and wire"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row VIP-1 (row 114); §7 'VIP-*' row VIP-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/vip-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

VIP store, command receipts, add-by-name invitation, accept/reject/revoke/withdraw, `vip_status` setting on PARTY-1's social settings row, groups, Premium switch-over, rate bucket and `VIP_V1` wire (not yet offered).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

VIP-1 (impl; persistence, protocol, privacy). VIP-2 (presence) is Wave 5.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| VIP-0 acceptance | BLOCKING (VIP0 CANDIDATE) |
| PARTY-1 (`game_character_social_settings`) | BLOCKING (Wave 2) |
| PREMIUM-1 | MERGED (#2110) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/domain/vip.rs`
- `apps/game-server/src/durability/vip.rs`
- `apps/game-server/src/durability/vip_tests.rs`
- `apps/game-server/src/gameplay_transport/vip.rs`
- `apps/game-server/src/gameplay_transport/vip_tests.rs`
- `crates/protocol-oteryn/src/vip.rs`
- `crates/protocol-oteryn/src/vip_tests.rs`
- `apps/game-server/tests/vip_postgres.rs`
- `docs/agents/tasks/VIP-1.md` (this record: status and notes only)

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

- PARTY-1 social settings table (add `vip_status` column in this migration)
- connection.rs dispatch chain; capabilities.rs `GATED` (`VIP_V1`, NOT in offered sets)
- crates/protocol-oteryn/src/lib.rs `pub mod`

## Inseparable bundles (never split across PRs)

- migration + VIP writer + durability tests
- wire + registry + capability + handler + tests

## Acceptance criteria

- [ ] Invitation ids never reused; consent required before status is visible.
- [ ] Only the inviting character sees status (V2).
- [ ] Caps and rate bucket from RESOURCE_LIMITS_REGISTRY; `VIP_V1` not offered.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server vip
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md §3-§7, §9, §13, §14
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
