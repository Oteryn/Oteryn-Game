# ACH-1 — achievements, badges, titles (skeleton task record)

```yaml
task_id: ACH-1
title: "ACH-1: achievements, badges, titles"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row ACH-1 (rows 125, 129); §7 'ACH-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/ach-1
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Achievement grant runtime: quest-transition grant, encounter-outcome grant with durable receipt, batch grant API and counter thresholds, coverage report. ACH-NOTIFY-1 is MERGED.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

ACH-QUEST-1, ACH-ENC-1 (hard), ACH-COUNTER-1, ACH-COVERAGE-1 (content tooling). ACH-NOTIFY-1 MERGED.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING (Wave 1 sequential) |
| QUEST-STATE-1 | MERGED |
| ENC-OUTCOME-1 (ACH-ENC-1) | BLOCKING |
| XP receipt; SKILLS-0 build receipts (ACH-COUNTER-1) | verify |
| FND-02 / ACH-NOTIFY-1 | MERGED |
| acceptance of ACHIEVEMENT0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/achievement/grant.rs`
- `apps/game-server/src/achievement/counters.rs`
- `apps/game-server/src/achievement/grant_tests.rs`
- `apps/game-server/src/durability/achievement_outcome_receipt.rs`
- `apps/game-server/tests/achievement_grant_postgres.rs`
- `tools/content-schema/achievement-authoring/validate_achievements.py and test_validate_achievements.py (ACH-COVERAGE-1 coverage report)`
- `docs/agents/tasks/ACH-1.md` (this record: status and notes only)

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

- apps/game-server/src/durability/account_achievement.rs (single writer of the account fact; call its grant entry point only)
- apps/game-server/src/achievement_catalogue.rs
- quest transition transaction (quest/)
- crates/protocol-oteryn/src/account_achievements.rs and achievement_notices.rs (existing; no new capability)

## Inseparable bundles (never split across PRs)

- migration (outcome receipt table) + writer + durability
- each granter calls the one grant entry point from its own transaction

## Acceptance criteria

- [ ] Grant idempotent per (account, achievement).
- [ ] Encounter outcome receipt prevents double grant after restart.
- [ ] Notice delta emitted after `Granted` commit only.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ach
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md §3-§5
- apps/game-server/src/durability/account_achievement.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
