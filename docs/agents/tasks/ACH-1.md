# ACH-1 — achievements, badges, titles (consolidated task card)

```yaml
task_id: ACH-1
title: "ACH-1: achievements, badges, titles"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row ACH-1 (rows 125, 129); §7 'ACH-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2154, branch skeleton/ach-1

Status note: the draft qualified three bounded pieces on the pre-ADR-0022
`apps/game-server/src/...` layout: (1) ACH-COVERAGE-1 coverage report in the authoring
validator (23/23 Python tests); (2) a 1..64 batch grant API on the existing account
achievement writer (4 pure + 8 PG cases on PG 17.6); (3) a PENDING counter-threshold loader
with one reviewed threshold row (8 cases). No earning caller, receipt migration or notice
emission was installed. It is closing; the mechanic is re-implemented fresh on the current layout.

## Goal

Achievement grant runtime: quest-transition grant, encounter-outcome grant with durable
receipt, batch grant API and counter thresholds, coverage report. ACH-NOTIFY-1 is MERGED.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):
ACH-QUEST-1, ACH-ENC-1 (hard), ACH-COUNTER-1, ACH-COVERAGE-1 (content tooling; no
dependencies in the brief's child graph). ACH-NOTIFY-1 MERGED.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| ACHIEVEMENT0 acceptance | ACCEPTED (#2209) |
| LOOP-PROGRESS-1 (family entry gate) | MERGED (#2192, skeleton) |
| QUEST-STATE-1 | MERGED (#1684) |
| ACH-NOTIFY-1 / ACH-NOTIFY-2 | MERGED (#1653 / #1656) |
| SKILLS-0 build state (ACH-COUNTER-1) | MERGED (#1346); XP/build writer hooks: verify at allocation |
| Quest-transition schema (ACH-QUEST-1) | BLOCKING — re-implementation pending (draft #2198 closing) |
| Counter earning receipts / XP+build writer hooks (ACH-COUNTER-1) | BLOCKING (no owner record on main; verify at allocation) |
| GM-1 (named with #2192 in the draft's XP/build hook gate) | BLOCKING — re-implementation pending (draft #2181 closing) |
| ENC-OUTCOME-1 (ACH-ENC-1) | BLOCKING (no record on main) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/account_achievement.rs` (484 lines) — single
  writer of the account fact (`record_achievement_grant`); the batch API extends this owner.
- `crates/gameplay/progression/src/account_achievement.rs`,
  `crates/gameplay/progression/src/achievement_catalogue.rs`,
  `crates/gameplay/social/src/achievement_catalogue.rs` — catalogue / `AchievementCatalogue::lookup`.
- `crates/gameplay/quests/src/quest/` — quest transition transaction (ACH-QUEST-1 caller).
- `crates/gameplay/durability/src/durability/boss_encounter.rs` — encounter owner (ACH-ENC-1 caller).
- `crates/protocol-oteryn/src/account_achievements.rs`, `achievement_notices.rs` — existing
  wire; no new capability.
- `apps/game-server/migrations/0021_account_achievements.sql` — existing schema (immutable).
- `apps/game-server/tests/account_achievement_postgres.rs`,
  `apps/game-server/tests/character_authority_postgres.rs`,
  `apps/game-server/tests/support/account_achievement_postgres_cases.rs` — existing PG targets/
  support; mounts are wiring-only.
- `tools/content-schema/achievement-authoring/validate_achievements.py`,
  `test_validate_achievements.py` — ACH-COVERAGE-1 report.
- `content/achievements/index.json` + catalogue shards.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/progression/src/achievement_grant_batch.rs` (new) +
  `achievement_grant_batch_tests.rs` (new) — L3 pure batch validation.
- `crates/gameplay/progression/src/achievement_counters.rs` (new) +
  `achievement_counters_tests.rs` (new) — L3 PENDING threshold loader.
- `crates/gameplay/durability/src/durability/account_achievement_batch.rs` (new) — L4 batch
  write path delegating to the existing owner (keeps `account_achievement.rs` growth ≤ ~150 lines).
- `crates/gameplay/durability/src/durability/achievement_outcome_receipt.rs` (new) — encounter
  outcome receipt.
- `apps/game-server/tests/support/achievement_batch_postgres_cases.rs` (new) — PG cases.
- `content/achievements/counter-thresholds.json` (new) +
  `tools/content-schema/achievement-authoring/counter-thresholds.schema.json` (new).
- `apps/game-server/migrations/<next>_achievement_outcome_receipt.sql` (new) — migration number:
  next free number at allocation time (only one migration PR in the queue). Draft's "0098" is void.

## Accepted design (ACHIEVEMENT0 §§2–5 and CP decisions, as built in the draft)

Coverage report (ACH-COVERAGE-1, §4):

- CLI always adds `coverage`, scoped to supplied supported inputs; lists every catalogue key
  with no granter in admitted content (claim, transition, encounter binding, threshold) as
  `UNEARNABLE_PENDING`. Missing bindings are reportable, not failures.
- `bound_keys` = matching RewardClaim placement reference only (not runtime earnability).
  `retired_keys` stays explicit; retired grants remain no-ops. Existing counts, errors and exit
  semantics unchanged. Invalid keys are reported, not silently dropped.
- No admin, script or migration grant substitutes for content.

Batch grant writer (D1153/D1160):

- Accepts 1..64 requests with one consumed same-TX fenced witness and one Character-root lock;
  rejects duplicate (source kind, event id, key); validates all inputs before storage; writes in
  event/key order; returns outcomes in input order. The single grant delegates to the batch.
- All-retired batch still verifies the witness. Errors require the earning owner to discard its
  transaction; no independent transaction or replacement writer.

Counter thresholds (ACH-COUNTER-1, D1331–D1334, D1340):

- Closed table schema; rejects duplicate `(counter, value, achievement)` triples and unknown
  keys; achievement data comes from `AchievementCatalogue::lookup`, never from the document.
  Different achievements may share a counter/value event.
- Exactly one reviewed row: `level / 50 / oteryn:achievement/seasoned_adventurer / revision 1`.
  The other eight counter keys get no rows (absence not proven historically).
- Trigger is `before < threshold <= after` on a committed counter receipt; no login-time or
  historical automatic grant. Table status stays `PENDING` until earning receipts exist.

Integration obligations (mandatory):

1. Each granter calls the one grant entry point from its own transaction.
2. Encounter outcome receipt prevents double grant after restart.
3. Notice delta emitted only after `Granted` commit.
4. PG cases follow repository convention: without `OTERYN_TEST_POSTGRES_ADMIN_URL` report
   `PRE-ROUTING / NONCANONICAL` and return `Ok(())`; missing PG is never a CI failure in the
   non-PG lane. Mount PG cases once per aggregate (no lib/transitive copies).

## Review dispositions (from the draft)

- CI integration-rest job 117409280578: 8 batch cases failed before routing because the
  admin URL was absent → missing-config branch returns `Ok(())` after the shared diagnostic
  (CP D1366 convention).
- First native counter attempt failed compilation (sccache) — no source defect; attempt 2 PASS.
- Coverage invalid-key repair (2026-10-16): invalid keys reported explicitly.

## Remaining work

Encounter outcome receipt migration + writer (ACH-ENC-1); quest-transition granter
(ACH-QUEST-1); counter earning receipts and XP/build hooks (ACH-COUNTER-1); post-commit
notice wiring; further reviewed threshold rows; content regeneration.

## Acceptance criteria

- [ ] Grant idempotent per (account, achievement).
- [ ] Encounter outcome receipt prevents double grant after restart.
- [ ] Notice delta emitted after `Granted` commit only.
- [ ] Coverage report lists every `UNEARNABLE_PENDING` key; no admin/script grant path.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- migration (outcome receipt table) + writer + durability
- each granter calls the one grant entry point from its own transaction
- threshold table + schema + loader by one author

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability achievement
cargo test --locked -p oteryn-game-server --test account_achievement_postgres achievement_ -- --test-threads=1  # PG 17.6
(cd tools/content-schema/achievement-authoring && python -m unittest test_validate_achievements)
python3 tools/content-migration/regenerate_content.py --check   # content slot only
python3 tools/agents/validate_governance.py
```

## Reference evidence

- ACHIEVEMENT0 §2–§5
- Completion plan §4, §5 (Wave 4), §7; coordination #2131 (D1153, D1160, D1331–D1334,
  D1340, D1363, D1366)
