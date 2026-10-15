# MONSTER-META-1 — monster meta: bestiary D15, bosses, raids (skeleton task record)

```yaml
task_id: MONSTER-META-1
title: "MONSTER-META-1: monster meta: bestiary D15, bosses, raids"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row MONSTER-META-1; §7 'MONSTER-META-1' (BOSS_RAID0, MONSTER_D15)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/monster-meta-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Bestiary D15 completion, boss encounters with rooms and rewards, the Bosstiary, and scheduled raids with announcements.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

MONSTER-D15B-1; BOSS-1 -> BOSS-ROOM-1 (SCOPE-HANDOFF-1) -> BOSS-REWARD-1 (PARTY-XP-1) -> BOSSTIARY-1; RAID-CONTENT-1 -> RAID-1 (CHAT-1); BOSS-WIRE-1 (Reg).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-MONSTER-1 (family entry gate) | BLOCKING (Wave 2, not merged) |
| acceptance of BOSS_RAID0 / MONSTER_D15 (plan §7 gate) | BLOCKING |
| CHAT-1 / CHAT-1b-2b (RAID-1) | MERGED (#2093) |
| PARTY-XP-1 (BOSS-REWARD-1) | BLOCKING (Wave 3, not merged) |
| SCOPE-HANDOFF-1 (BOSS-ROOM-1) | MERGED (#1782) |
| D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2 | MERGED (archive records) |
| ENCOUNTER-RT-0 (encounter runtime) | ACCEPTED (2026-10-14); runtime merge state to be confirmed by CP |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/bosses/**, content/raids/** (sources only)`
- `apps/game-server/src/durability/character_bosstiary.rs`
- `apps/game-server/src/durability/boss_encounter.rs`
- `apps/game-server/src/durability/boss_encounter_tests.rs`
- `apps/game-server/src/gameplay_transport/raid.rs`
- `apps/game-server/src/gameplay_transport/raid_tests.rs`
- `crates/protocol-oteryn/src/bosstiary.rs`
- `apps/game-server/tests/monster_meta_postgres.rs`
- `docs/agents/evidence/MONSTER-META-1-*.md`
- `docs/agents/tasks/MONSTER-META-1.md` (this record: status and notes only)

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

- kill_reward.rs (boss reward hook)
- world_runtime.rs raid scheduler registration
- connection.rs dispatch chain; capabilities.rs `GATED` l.165
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- BOSS-REWARD-1: reward split + loot + bosstiary progress in one transaction
- RAID-1: schedule + spawn + broadcast + restart resume

## Acceptance criteria

- [ ] Boss cooldowns and Bosstiary progress are durable and restart-safe.
- [ ] Boss room entry/exit uses SCOPE-HANDOFF-1; no duplicate rewards on reconnect.
- [ ] Raids fire on schedule once per window even across restart.
- [ ] Bestiary D15 values match reference.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server bosstiary
cargo test -p oteryn-game-server boss
cargo test -p oteryn-game-server raid
cargo test -p oteryn-game-server bestiary
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md
- PR #1782
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
