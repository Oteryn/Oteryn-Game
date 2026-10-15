# RANK-1 — highscores and analysers (skeleton task record)

```yaml
task_id: RANK-1
title: "RANK-1: highscores and analysers"
mode: IMPLEMENT
status: local_preparation_pending_qualification
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row RANK-1 (rows 48, 128); §7 'HUNT-1, RANK-1' HS-* and ANALYSER-* rows"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md
  - docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/rank-1
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Highscore snapshot tables, per-World snapshot job, categories, ranks and filters (HS-1) with `HIGHSCORES_V1` query (HS-WIRE-1); analyser fact emission at commit points (ANALYSER-EMIT-1). Exports are normalized Game data Atlas consumes.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

HS-1, HS-WIRE-1 (Reg), ANALYSER-EMIT-1. ANALYSER-WIRE-1 MERGED; ANALYSER-CLIENT-1 is client lane.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING |
| CHAR-BUILD-1b | MERGED |
| CHARM-1 | MERGED |
| account achievements 0021 | MERGED |
| ANALYSER-WIRE-1 | MERGED |
| ATTACK-1b | MERGED |
| D3 corpse mint | MERGED (#1213) |
| acceptance of HIGHSCORES0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |
| acceptance of ANALYSERS0 (brief CANDIDATE; plan §7 acceptance gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/highscores/`
- `apps/game-server/src/durability/highscore_snapshot.rs`
- `apps/game-server/src/durability/highscore_snapshot_tests.rs`
- `apps/game-server/src/gameplay_transport/highscores.rs`
- `crates/protocol-oteryn/src/highscores.rs`
- `crates/protocol-oteryn/src/highscores_tests.rs`
- `apps/game-server/src/analyser_emit.rs`
- `apps/game-server/tests/highscores_postgres.rs`
- `docs/agents/tasks/RANK-1.md` (this record: status and notes only)

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

- crates/protocol-oteryn/src/analyser.rs (capability 10, domain 15; existing)
- commit points in combat/item lanes for ANALYSER-EMIT-1
- node job list for snapshot job
- capabilities.rs `GATED` (capability 9 `HIGHSCORES_V1`, command type 14)

## Inseparable bundles (never split across PRs)

- migration + snapshot job + durability
- wire + registry + capability + handler + tests (HS-WIRE-1)

## Acceptance criteria

- [ ] Snapshot deterministic per World and epoch; ranks stable on ties.
- [ ] Paged query bounded.
- [ ] Analyser facts emitted only at commit, deterministic replay.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server rank
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md §3-§6
- docs/architecture/reviews/OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md §4-§6
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — HS-1 ranking rules (2026-10-15, unpublished)

- Existing draft #2168, branch `skeleton/rank-1`, allocated head
  `e2fb944ced21bcc9905889cb2f4427c95290ad11`. Only
  `highscores/ranking.rs`, `ranking_tests.rs` and this record are allocated.
  No commit, publication, ready flag, schema, registry, content or shared wiring.
- The lead confirmed effective HIGHSCORES0 acceptance through merged #2209.
  Owning text was read at accepted main
  `e2da9805d296a27b06b32335bf67394509d4910b`. Its §5 and D309 clarification
  govern this preparation; historical CANDIDATE dependency rows above no longer
  describe that acceptance. LOOP-PROGRESS-1 still gates family integration, not
  these separately accepted pure ranking rules.
- Prepared callable ordering over a complete, already eligible committed cut for
  one World/category: descending full value, standard competition ranks
  (1, 2, 2, 4), positions tied by raw Unicode code point name then native
  `CharacterId`. There is no case folding, Unicode normalization, locale or SQL
  collation. The caller's scalar or `(level, progress)` tuple is compared as-is;
  no category, points formula, achievement aggregation or Charm score is created.
- Each base-vocation list preserves its rows' global ranks and applies its own
  1,000-row cap **after** filtering the uncapped eligible cut. Promotions reuse
  the existing `character_wheel::base_vocation` mapping. An invalid/non-base
  filter or duplicate CharacterId produces a local typed preparation error,
  not a new wire/error-registry code. Source rows are borrowed and unchanged.
- Native CharacterId is the real protocol/foundation type. Durable vocation keys
  remain strings, as in the existing build state; no parallel vocation enum is
  introduced. Snapshot name text is borrowed rather than minted as a Character
  name: the current ASCII-only creation policy remains separate from the owning
  highscore contract's explicit Unicode ordering rule.

### Remaining indivisible integration work (REPORT_ONLY)

No production module or consumer is registered. The future snapshot owner must
select the complete eligible live-root cut for one World/category, exclude `none`
for categories 1–9 and 11, choose each account category's representative and
source-native values, and supply all rows before capping. These are source
selection/eligibility responsibilities, not facts that this pure comparator can
authenticate. Snapshot persistence, repeatable-read source transaction, job
lease/restart/retention, wire paging and snapshot tokens, registries, protocol,
Atlas exports and analyser emission remain outside this local scope. Ranking
rows grant no session, World, Channel or gameplay authority.

### Validation and self-review

- Source-bound isolated harness: **11 passed, 0 failed**. Cases cover competition
  gaps, raw Unicode/non-BMP and normalization/case boundaries, CharacterId tie
  order independent of input, full level/progress tuple values, global ranks
  across vocation ties, all five promotion families, a filtered list whose first
  member lies below the global cap, truncation inside a tie, empty/`none` inputs,
  invalid filters/duplicate sources, and u64 extremes without score arithmetic.
- Isolated all-target Clippy with the native root lint table: passed. Owned-file
  rustfmt and `cargo fmt --all --check`: passed. The harness imports the real
  protocol crate and the literal existing vocation helper; no native type is
  recreated. Full-server/package, persistence, wire and live-authority
  qualification are not claimed; all remain pending their actual consumers.
- Actual logs, harness and source/hash binding are retained outside the repository
  in `/workspace/oteryn-prep/rank-1-evidence`. Rust commands used the shared flock,
  target and two-job limit. No test binary was copied.
- Governance validator passed (22 policy documents, 9 lanes); governance unit
  tests passed (124, 0 failures). Logs are retained in that evidence directory.
- Whole-diff self-review: rank equality depends on the full value alone; tie
  position ordering cannot change rank; filtering cannot renumber or inherit the
  all-list cap; native types and promotion rules are reused. No authority-bearing
  mutation exists here, so current-session/replay-fence qualification is not
  applicable to this callable subset. Those checks remain required at future
  snapshot/wire consumer boundaries.
