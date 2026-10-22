# PVP-1b — skull evaluation, World cleanup job, pvp/party rulesets (consolidated task card)

```yaml
task_id: PVP-1b
title: "PVP-1b: skull evaluation in the death transaction, World cleanup job, pvp/party rulesets"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
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
needs_migration: true   # next free number at allocation time (only one migration PR in the queue)
needs_registry: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2203, branch skeleton/pvp-1b

## Goal

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
| D1128 ledger/points adapter (follows #2204) | to be built here or in PVP-DEATH-1; confirm the owner at allocation |
| PVP-DEATH-1 (calls `settle_player_pvp_death`) | BLOCKING — re-implementation pending (draft #2205 closing) |

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
  command binding (empty for PvE so PvE bindings are unchanged). Entry points:
  `commit_character_pvp_death` / `settle_player_pvp_death`.
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

- [ ] Skull points, revenge marks and kill block written atomically with the death (single death transaction, D1443 window semantics).
- [ ] World cleanup job registered in the Channel owner cycle, including expired-skull normalization; lock order matches the death transaction.
- [ ] Ruleset rows pinned against the Rust constants; provenance labels recorded.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Live PvP stays off until PVP-RT-1b, PVP-DEATH-1, PVP-WIRE-1 and PVP-CLIENT-1 land (PVP-0 §8).
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`character_death.rs`, `character_revision_sequencer.rs` take call sites only).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-combat-rules pvp
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test -p oteryn-game-server --test character_authority_postgres pvp_state_postgres_cases::
python3 tools/agents/validate_governance.py
```
