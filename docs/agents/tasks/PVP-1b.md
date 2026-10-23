# PVP-1b — skull evaluation, World cleanup job, pvp/party rulesets (consolidated task card)

```yaml
task_id: PVP-1b
title: "PVP-1b: skull evaluation in the death transaction, World cleanup job, pvp/party rulesets"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "App. 7 PARTY/PVP PVP-1b"
brief: "docs/architecture/reviews/OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md (§6, §6.1, §6.2, §8, §9, RL-14..16)"
class: "hard, persistence and security review"
base_branch: main
branch: null
pr: null
needs_migration: true   # allocated migration 0142
needs_registry: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2203, branch skeleton/pvp-1b

## Goal

### Prerequisite audit (2026-10-22)

Baseline `98b897f848ccfd75c21e48fb53091d5b5f7cd967` has the 0081 state,
receipt guards, deadline writer, admission reader and bounded cleanup, plus the
sequenced DEATH-1 writer. It has no classified PvP death input, points adapter or
PvP death entry points. The runtime still admits creature attacks only; live
classification and ledger snapshots remain outside this card.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

Use the existing death writer and sequencer, not another transaction pipeline.
Add the classified Open-PvP input/binding and entry seams in
`durability/pvp_death_consequences.rs`, with calls from the existing owners.
The empty input preserves PvE bindings and runtime activation remains closed.
Planned modules: `combat-rules/src/pvp/skull.rs` and `skull_tests.rs`;
`durability/pvp_death_consequences.rs`, `pvp_skull_storage.rs`, and
`pvp_expiry.rs`; `dispatch/gameplay_transport/pvp_death.rs` and `source_pvp_expiry.rs`;
child PostgreSQL cases mounted once by existing test owners. Migration: `0142`.

Skull evaluation inside the victim's death transaction (points, revenge marks, kill
block), the World cleanup job that calls the PVP-1a cleanup function (including
expired-skull normalization), and the `rulesets/pvp/` and `rulesets/party/` rows
(`pvp_type` OPTIONAL). Live PvP (attack, killer classification, skull
wire/broadcast) is not this task: PVP-RT-1b, PVP-DEATH-1, PVP-WIRE-1, PVP-CLIENT-1
(PVP-0 §8 go-live gate).

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| PVP-1a (state tables, deadline write-ahead, admission read; migration 0081) | MERGED (#2032) |
| DEATH-1 (death writer) | MERGED (#1278) |
| PVP-RT-1 pure legality/lock; fenced lock-deadline restore | MERGED (#2029, #2204) |
| PARTY-1 | PARTIAL: bounded checkpoint MERGED (#2201); full family still open (card `docs/agents/tasks/PARTY-1.md`) |
| D1128 ledger/points adapter (follows #2204) | Classified input, ledger validation and points adapter implemented in this lane on the existing owners |
| PVP-DEATH-1 (calls `settle_player_pvp_death`) | Required death entry/dispatch seam implemented here; runtime classification, avenging and PvP death-loss policy remain separate follow-ups |

## Accepted decisions

- **D1443 (CP):** frag windows roll from each unjustified kill's committed
  timestamp (`committed_at`, stamped by the 0081 trigger) with an exclusive end: a
  point counts while `now - committed_at < window`. This settles the earlier
  `age < duration` endpoint assumption.
- **D971 / PVP-0 §6.1:** the white skull lasts at least as long as the kill block.
- **§6.2:** stored skulls expire at `until <= now`.
- Value provenance:
  - Official (manual): red 30 d, black 45 d, revenge 7 d, kill block 15 min.
  - `OTS_HYPOTHESIS_ONLY` (Canary): red thresholds 3/5/10 kills per day/week/month
    (3,000/5,000/10,000 milli-points), black 6/10/20 (6,000/10,000/20,000), the 5/N
    damage split.
  - `OTERYN_DECISION` (`PARITY_PENDING`): assist 500 milli-points, 45-day point
    retention, at most 80 contributors, party activity window 2 minutes.
- Only qualified Open-PvP unjustified points use this path; no Optional/Hardcore
  behaviour change and no runtime gate change.

## Reference behaviour from the draft (re-implement, do not copy)

- **Pure skull evaluator.** Totals rolling 24 h / 7 d / 30 d points including one
  positive new increment (1..=1,000 milli, added once to all three totals; history
  passed separately), applies red/black thresholds, preserves an unexpired higher
  skull, resets the red/black deadline to now +30/+45 d but never shortens a later
  unexpired deadline. Refuses zero/oversized increments, malformed state or point
  rows, future points and deadline overflow. Reuses the canonical `PvpSkull` enum
  from `crates/gameplay/durability/src/durability/pvp_state.rs` (a pure L3 rule must
  not depend on durability; move or mirror the enum into the domain/rule layer as
  part of the wiring decision).
- **Death-transaction adapter** `apply_pvp_death_in_transaction`, run in the victim's
  death transaction after the receipt: contributors must equal the victim's ledger
  within 5 minutes (compact kind must match); state rows locked in CharacterId order;
  kill block +15 min for every contributor; for each unjustified contributor a point
  row (1000; 500 assist; floor(5000/n) beyond five damage contributors), a 7-day
  revenge mark, the white skull at least to the kill block, then the skull
  evaluation; then the victim's ledger is deleted. Contributors are part of the death
  command binding (empty for PvE so PvE bindings are unchanged). Entry point:
  `settle_player_pvp_death`, through the revision sequencer; `commit_character_death`
  takes PvP contributors unchanged and stays sequencer-only (CHAR-REV-SEQ-1).
- **Review fixes kept as requirements:** the evaluation samples the database clock
  once, at PostgreSQL microsecond resolution, and bounds history with that same
  instant (no millisecond rounding). The World job locks the state rows of its
  expired-skull candidates and expired-ledger victims in CharacterId order before
  any ledger row — the same order as the death transaction and the 0081 ledger
  guard — so the two cannot deadlock.
- **World cleanup job** `drain_source_pvp_expiry`: 1 s pass in the Channel owner
  cycle together with the party deadlines, calling
  `DurabilityRoot::drain_pvp_expiry` (at most 100 rows per kind per pass) plus
  expired-skull normalization.
- **Migration** (number: next free number at allocation time (only one migration PR in
  the queue)): runtime grants for skull/kill block/points/marks; an UPDATE guard that
  admits a skull or kill-block change only in a death transaction of the row's World
  naming the Character in the victim's ledger, except clearing an expired skull; the
  expiry index.
- **Rulesets:** `rulesets/pvp/skulls/skull-policy.json` (new; pinned by a unit test
  against the Rust constants), `rulesets/party/shared-experience/shared-experience.json`
  (new; no runtime consumer yet). Existing `index.json` files in both directories.
- Draft PG case: `unjustified_kills_raise_a_white_then_red_skull_that_the_world_job_clears`.

Known limits at the draft: no skull field on the creature wire/broadcast
(PVP-WIRE-1/PVP-CLIENT-1, protocol slot); players cannot attack players
(`attackable_kind` creature-only); runtime killer classification and mark avenging
belong to PVP-RT-1b/PVP-DEATH-1. A draft CI run saw four unrelated
`character_authority_postgres` claim/quest-obligation failures
(Ambiguous / Unavailable / CommitOutcomeUnknown) whose cause was never classified —
recheck on the re-implementation, do not assume independence.

## Target layout (ADR-0022)

- Pure evaluator: `crates/gameplay/combat-rules/src/pvp/skull.rs` (new) with tests in
  `crates/gameplay/combat-rules/src/pvp/skull_tests.rs` (new); registration line in
  `crates/gameplay/combat-rules/src/pvp/mod.rs` (wiring-only).
- Death adapter: `crates/gameplay/durability/src/durability/pvp_death_consequences.rs`
  (new), called from `crates/gameplay/durability/src/durability/character_death.rs`
  (2,764 lines: call-site only) and bound through
  `crates/gameplay/durability/src/durability/character_revision_sequencer.rs` (call-site only).
- Expiry reads/cleanup: `crates/gameplay/durability/src/durability/pvp_state.rs` (exists).
- World job composition: `crates/gameplay/dispatch/src/gameplay_transport/source_item_cycle.rs`
  (beside `drain_source_party_deadlines_bounded`).
- PG cases: `apps/game-server/tests/support/pvp_state_postgres_cases.rs` (exists) or a new
  child case file; migration under `apps/game-server/migrations/`.

## Acceptance criteria

- [x] Skull points, revenge marks and kill block written atomically with the death (single death transaction, D1443 window semantics).
- [x] World cleanup job registered in the Channel owner cycle, including expired-skull normalization; lock order matches the death transaction.
- [x] Ruleset rows pinned against the Rust constants; provenance labels recorded.
- [x] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [x] Live PvP stays off until PVP-RT-1b, PVP-DEATH-1, PVP-WIRE-1 and PVP-CLIENT-1 land (PVP-0 §8).
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`character_death.rs`, `character_revision_sequencer.rs` take call sites only).

## Validation (2026-10-22)

All Cargo commands below ran through `/home/mole/oteryn-cache/codex-bin/cargo`,
with `CARGO_NET_OFFLINE=true`, toolchain 1.95.0, and the wrapper-supplied PostgreSQL
admin URL. No dependencies downloaded. No Git mutation or publication performed.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-combat-rules pvp --quiet
cargo test --locked -p oteryn-gameplay-durability --lib character_death::tests:: --quiet
cargo test --locked -p oteryn-gameplay-social-rules party_shared_experience_ruleset --quiet
cargo test --locked -p oteryn-game-server --test character_authority_postgres pvp_state_postgres_cases:: --quiet -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --lib monk_save::postgres_tests::pvp_ --quiet -- --nocapture
cargo run --locked -p oteryn-architecture-check --quiet -- workspace .
python3 tools/content-schema/validate_materialized_game_tree.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

Results: fmt/clippy/architecture PASS (exit 0); rules 15/15, death units 3/3,
party ruleset 1/1; authority PostgreSQL 8/8 (24.38 s), composed dispatch PostgreSQL
2/2 (42.85 s); governance PASS and 124/124 tests; materialized tree 97/97 PASS.
PostgreSQL really ran: the shared harness enforced the then-canonical server version 170006 and runs
GAME_MIGRATOR; the dispatch death emitted `PVP_POSTGRES_EVIDENCE` after its SQL
assertions with migration 0142. No unfiltered PG suite ran. The pre-existing
vendored Tokio missing-docs warning is dependency-capped; workspace Clippy exited 0.

## Implementation and reuse record

Layout: L3 `combat-rules/pvp/skull.rs` owns pure rules and the canonical skull enum;
L4 durability owns classified input/binding, death consequences, paged history and
expiry; L5 dispatch owns forwarding and the one-second cycle call. Child test files
own each new test surface. `pvp_state.rs` re-exports the moved enum and cleanup.

Reused: existing DEATH-1 transaction/receipt/replay/build-loss pipeline, revision
sequencer, session/recovery/scope fences, PVP-1a write-ahead/readmission and cleanup,
0081 point/mark stamping/ledger guards, and the party cycle's existing throttle.
The history adapter uses 256-row keyset pages and one microsecond clock sample;
cleanup locks both bounded state candidate sets in CharacterId order before ledger
rows. The existing EXCLUSIVE admission relation fence serializes competing deaths.

Fixtures reused: Bestiary and sequencer PostgreSQL harnesses; `Dependencies::new()`;
DEATH-1 request and `join_two` via `pvp_death_request_fixture.rs`; the existing
50-level death request via a test-only visibility/re-export at its owner. PVP-1a's
root/ledger fixture bodies were MOVED into `pvp_character_fixture.rs` and
`pvp_ledger_fixture.rs`, with original callers delegated, not copied. Fresh roots
start at revision 1; both concurrent holders reconcile before fixture-only roots
are inserted. `pvp_source_death_fixture.rs` only re-exports those fixtures. Each
fixture/test is mounted once per compiled owner; shared fixtures serve separate
PG target/library crates. No new Dependencies constructor or standalone PG target.

The complete diff was reviewed against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`;
all new fields/types/matches/constructors/callers were searched across apps/crates/tools.
Empty consequences reject a live ledger; changed classification conflicts on replay;
stale/future/compact/identity inputs roll back; the concurrent loser is typed.
Every new public production function reaches the named composed dispatch death or
cleanup PG test. Existing positive direct skull/kill-block writes now assert refusal
under 0142; all previous monotonicity and forfeiture assertions remain.

## Changed files and sizes

Only two files grew beyond 200 lines: `pvp_death_postgres_cases.rs` (233) owns the
skull/replay/rollback transaction cases; `pvp_death_validation_postgres_cases.rs` (226)
owns input/fence/split/concurrent cases. Both are child PG evidence beside the
existing durability test owner, reusing its harness. No existing file grew over
150 lines. Shared slot changes are registrations, re-exports, input fields, or
forwarding call sites; the hot monk-save file has only two child registrations.

| File | Before → after lines |
|---|---:|
| `Cargo.lock` | 4782 → 4783 |
| `apps/game-server/migrations/0142_pvp_death_consequences.sql` | 0 → 48 |
| `apps/game-server/tests/character_authority_postgres.rs` | 3451 → 3452 |
| `apps/game-server/tests/support/character_death_writer_postgres_cases.rs` | 1421 → 1426 |
| `apps/game-server/tests/support/character_last_position_postgres_cases.rs` | 706 → 707 |
| `apps/game-server/tests/support/character_progression_admission_postgres_cases.rs` | 508 → 509 |
| `apps/game-server/tests/support/character_progression_postgres_cases.rs` | 1278 → 1284 |
| `apps/game-server/tests/support/character_revision_sequencer_postgres_cases.rs` | 1240 → 1249 |
| `apps/game-server/tests/support/item_transfer_postgres_cases.rs` | 3395 → 3396 |
| `apps/game-server/tests/support/pvp_character_fixture.rs` | 0 → 27 |
| `apps/game-server/tests/support/pvp_death_postgres_cases.rs` | 0 → 233 |
| `apps/game-server/tests/support/pvp_death_request_fixture.rs` | 0 → 11 |
| `apps/game-server/tests/support/pvp_death_validation_postgres_cases.rs` | 0 → 226 |
| `apps/game-server/tests/support/pvp_ledger_fixture.rs` | 0 → 25 |
| `apps/game-server/tests/support/pvp_source_death_fixture.rs` | 0 → 9 |
| `apps/game-server/tests/support/pvp_state_postgres_cases.rs` | 682 → 672 |
| `crates/gameplay/combat-rules/src/pvp/mod.rs` | 10 → 13 |
| `crates/gameplay/combat-rules/src/pvp/skull.rs` | 0 → 156 |
| `crates/gameplay/combat-rules/src/pvp/skull_tests.rs` | 0 → 187 |
| `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` | 8198 → 8194 |
| `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs` | 1399 → 1405 |
| `crates/gameplay/dispatch/src/gameplay_transport/pvp_death.rs` | 0 → 27 |
| `crates/gameplay/dispatch/src/gameplay_transport/pvp_death_postgres_tests.rs` | 0 → 60 |
| `crates/gameplay/dispatch/src/gameplay_transport/pvp_expiry_postgres_tests.rs` | 0 → 55 |
| `crates/gameplay/dispatch/src/gameplay_transport/source_item_cycle.rs` | 233 → 234 |
| `crates/gameplay/dispatch/src/gameplay_transport/source_pvp_expiry.rs` | 0 → 23 |
| `crates/gameplay/durability/Cargo.toml` | 49 → 50 |
| `crates/gameplay/durability/src/durability/character_death.rs` | 2764 → 2779 |
| `crates/gameplay/durability/src/durability/character_revision_sequencer.rs` | 1319 → 1319 |
| `crates/gameplay/durability/src/durability/core_modules.rs` | 3457 → 3461 |
| `crates/gameplay/durability/src/durability/pvp_death_consequences.rs` | 0 → 197 |
| `crates/gameplay/durability/src/durability/pvp_death_request.rs` | 0 → 45 |
| `crates/gameplay/durability/src/durability/pvp_expiry.rs` | 0 → 73 |
| `crates/gameplay/durability/src/durability/pvp_skull_storage.rs` | 0 → 75 |
| `crates/gameplay/durability/src/durability/pvp_state.rs` | 456 → 394 |
| `crates/gameplay/social-rules/src/lib.rs` | 10 → 13 |
| `crates/gameplay/social-rules/src/party_ruleset_tests.rs` | 0 → 29 |
| `docs/agents/tasks/PVP-1b.md` | 146 → 282 |
| `rulesets/party/shared-experience/index.json` | 10 → 10 |
| `rulesets/party/shared-experience/shared-experience.json` | 0 → 21 |
| `rulesets/pvp/skulls/index.json` | 10 → 10 |
| `rulesets/pvp/skulls/skull-policy.json` | 0 → 25 |
| `workspace-boundaries.toml` | 114 → 114 |

## Unfinished / integrator handoff

Implementation and scoped validation complete; status remains `in_progress` for
CP-owned independent persistence/security review, candidate publication, repository
CI and Merge Queue. Worktree is detached at baseline
`98b897f848ccfd75c21e48fb53091d5b5f7cd967`; no frozen SHA or PR was created here.
Full PG suites and the historical unrelated claim/quest failures remain CI scope;
no unrelated failure was reclassified or suppressed. Live player attacks, killer
classification, ledger snapshots, avenging, PvP death-loss policy and skull wire/client
activation remain their named follow-ups, with the PVP-0 gate unchanged.

## Port onto DISPATCH-WIRING-1 (2026-10-23)

Cherry-picked onto `de3fd2190` (DISPATCH-WIRING-1 with REGISTRY-SPLIT-1, DAILY-1a and
QUEST-RT-1). No behaviour change against the lane commit; adaptation only:

- Test registries follow the DW convention: the `pvp_death`/`pvp_expiry` mounts in
  `monk_save_postgres_tests.rs` and the `pvp_source_death_fixture` mount in
  `character_revision_sequencer_postgres_cases.rs` sit in alphabetical position.
- `mod pvp_death;` sits in alphabetical position in `gameplay_transport/mod.rs`;
  `pvp_death.rs` and `source_pvp_expiry.rs` stay per-mechanic impl blocks. The
  dispatch tests use the DW `Dependencies::owner` / `ComposedTestOwner` path unchanged.
- Durability keeps both new layering edges (social-rules from main, combat-rules from
  this lane); `workspace-boundaries.toml` lists both in sorted order.
- Migration 0142 kept (no collision with 0130-0132, 0136, 0140, 0144).
- World pin `inputs_digest` recomputed for the merged tree; the dispatch evidence
  line names the canonical PostgreSQL 18.6.
