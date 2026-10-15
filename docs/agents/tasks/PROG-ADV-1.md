# PROG-ADV-1 — advanced progression: proficiency, wheel, gems (skeleton task record)

```yaml
task_id: PROG-ADV-1
title: "PROG-ADV-1: advanced progression: proficiency, wheel, gems"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row PROG-ADV-1; §7 'PROG-ADV-1' (PROFICIENCY1/1B, PROF_EFFECT0, WHEEL0, WHEEL_GEM0)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1_PERK_MODIFICATION_AND_CATALYSTS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1B_PERK_MODIFICATION_VALUE_SHAPES_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_PROF_EFFECT0_KILL_CREDIT_AND_PERK_EFFECTS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_WHEEL_GEM0_GEM_ATELIER_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/prog-adv-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Weapon proficiency perks, the Wheel of Destiny and the gem atelier work end to end: orb burn, shape wiring, wheel spell gates, gem reveal/vessel/crush and their effects.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

PROF-ORB-BURN-1 -> PROF-SHAPE-WIRE-1 -> PROF-CONTENT-2 -> PROF-PARITY-1; SPELL-WHEEL-GATE-1 -> W-2 -> W-FX-1; GEM-R -> GEM-CONTENT-1 -> GEM-1 -> GEM-VESSEL-1 -> GEM-WIRE-1 -> GEM-FX-1 -> GEM-CRUSH-1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING (Wave 1 sequential, not merged) |
| PREMIUM-1 | MERGED (#2110) |
| acceptance of PROFICIENCY1, PROFICIENCY1B, PROF_EFFECT0, WHEEL0, WHEEL_GEM0 (plan §7 gate) | BLOCKING |
| W-R, W-1, PROF-SHAPE-CONTENT-1, PROF-SHAPE-1b slice, FORGE-1a | MERGED (migration 0070, #1670, #1704, #1691) |
| CHAR-REV-SEQ-1, GOLD-FEE-2, CONDITIONS-0 | MERGED / ACCEPTED (archive, #2106) |
| PROF-2 | MERGED (archive record) |
| PROF-1, PROF-WIRE-1 (PROF-* children) | BLOCKING until CP confirms (no exact record on main) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/character_proficiency.rs`
- `apps/game-server/src/durability/character_proficiency_codec.rs`
- `apps/game-server/src/durability/character_proficiency_modification.rs`
- `apps/game-server/src/durability/character_wheel.rs`
- `apps/game-server/src/durability/character_wheel_gem.rs`
- `apps/game-server/src/durability/character_wheel_gem_tests.rs`
- `apps/game-server/src/gameplay_transport/wheel.rs`
- `apps/game-server/src/gameplay_transport/wheel_gem.rs`
- `crates/protocol-oteryn/src/wheel_gem.rs`
- `apps/game-server/tests/prog_adv_postgres.rs`
- `docs/agents/evidence/PROG-ADV-1-*.md`
- `docs/agents/tasks/PROG-ADV-1.md` (this record: status and notes only)

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

- connection.rs dispatch chain (wheel/gem intents)
- capabilities.rs `GATED` l.165
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568
- spell cast gate (SPELL-WHEEL-GATE-1) in the spell owner path

## Inseparable bundles (never split across PRs)

- PROF-ORB-BURN-1: orb consume + perk write + audit
- GEM-1/GEM-VESSEL-1: migration + gem writer + vessel slot
- each *-WIRE-1: wire + registry + capability + tests

## Acceptance criteria

- [ ] Perk and wheel point changes are durable, revision-sequenced and restart-safe.
- [ ] Gem reveal/crush consume and mint atomically; values match WHEEL_GEM0A reference amendment.
- [ ] Wheel spell gates refuse with typed code when points are missing.
- [ ] Effects applied only through PROF_EFFECT0 kill credit rules.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server proficiency
cargo test -p oteryn-game-server wheel
cargo test -p oteryn-game-server gem
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_WHEEL_GEM0A_REFERENCE_VALUES_AMENDMENT_DECISION_2026-10-01.md
- PRs #1670, #1704, #1691, #2106, #2110
- migration 0070 (W-1)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.
