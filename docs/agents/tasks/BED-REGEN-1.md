# BED-REGEN-1 — bed settlement regeneration and food, native integration (consolidated task card)

```yaml
task_id: BED-REGEN-1
title: "BED-REGEN-1: native settlement integration and producer gates"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "impl (persistence review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row BED-REGEN-1 (row 121); App. 7 house beds; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md   # §7.2 (30 s per HP/mana point, 900 s per Soul point)
  - docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md
base_branch: main
branch: null
pr: null
needs_migration: false   # unless CP allocates the OFFLINE/vitals producer migration here; no number is chosen by this card
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2148, branch skeleton/bed-regen-1
(the draft is a 972-line "bounded checkpoint": a card only, with an inert source/patch appendix, no
`regen.rs`, no caller, no test; regeneration was never implemented or run)

## Goal

Settle vitals regeneration and food for a sleeping Character on wake, per BED0 §7.2, as a
participant of the real OFFLINE settlement transaction after the BED-1 sleeper hook: HP/mana from
durable food, Soul from sleep time, all committed with the OFFLINE marker in one physical
transaction. Until the full durable food/vitals/Soul gate exists BED0 permits neither a partial award
nor a food debit (explicit temporary path: sleeper deletion and OFFLINE training still compose as
accepted). Inseparable bundle: regen settlement + its tests with the BED-1 settlement hook.

## Dependencies (re-verified against origin/main 864beba6d, 2026-10-22)

| Dependency | State |
|---|---|
| BED0 / OFFLINE0 acceptance | ACCEPTED (ACCEPT-BATCH #2209 §1); `PARITY_PENDING` on the coefficients is an evidence label, not permission to invent rates or use online vocation rates |
| BED-1 (sleeper table, USE/sleep, owner wake, settlement hook) | NOT_STARTED re-implementation card (draft #2147 closing): **BLOCKING** (`docs/agents/tasks/BED-1.md`; this task consumes its settlement path, adds no parallel transaction) |
| OFFLINE-1 (logout/offline marker writer, `apply_marker`, chain guards, FND clock/predecessor) | NOT MERGED: **BLOCKING**. D1409 gave the producer to draft #2174, which is closed; it is re-implemented as a child of `docs/agents/tasks/LOOP-TRAIN-1.md`. Migration number unresolved (D1370 reserved 0118, D1381 described 0120, D1409 resolved ownership to #2174, "0120 is not #2174's"); this card chooses none |
| LOOP-VITALS-1 (#2196) durable food/residuals/Soul participant | MERGED (#2196) only as a bounded skeleton checkpoint (stamina arithmetic, Soul policy); the durable residual writer and the same-TX participant are **missing: BLOCKING** (`docs/agents/tasks/LOOP-VITALS-1.md`) |
| CHAR-VITALS-1 (durable HP/mana in the last-position projection, migration 0084) | MERGED (#2045); proves durable HP/mana only, not Soul or food |
| MAP-OVERLAY-1c / active-bundle House placement (head/foot currentness) | **BLOCKING** (NOT_STARTED card; `docs/agents/tasks/MAP-OVERLAY-1c.md`) |
| HOUSE-RUNTIME-1b/1c, HOUSE-1a/1b, STATUE-1, HOUSE-VIEW-1 | BLOCKING as listed in `docs/agents/tasks/BED-1.md`; House window follows the MAIL/HUNT order (D1399b) |
| LOOP-TRAIN-1 (training marker/lease/pool plans) | `docs/agents/tasks/LOOP-TRAIN-1.md`; blocked on OFFLINE-1/STATUE-1/EXERCISE-1 |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be drafted
earlier but must not assume unmerged schemas. Producer allocation: D1355 (durable Soul to #2196, #2167
full active-bundle producer; BED hook inside #2174 `apply_marker` after the plan/fence locks and before
the marker write, only after #2174 and #2196 merge), D1409 (OFFLINE producer to #2174; the bounded
reset-month reader of D1418 is not a BED time source), D1422 (#2196 owns a crate-private vitals
participant borrowing the `apply_marker` transaction, no own BEGIN/COMMIT, no extra CharacterRevision).
D1420: every Cargo command runs in the canonical lease with D1329's PG17.6 wrapper.

## Accepted decisions

- **Settlement arithmetic (BED0 §7.2):** `p = floor(min(f, sleep_s) / 30)` with food `f` in 0..=1200 s;
  `consumed = 30 * p` even when HP/mana are capped (not effective credited points); `f_after = f - consumed`
  (remainder < 30 s retained, never negative); HP and mana each gain `p`, capped independently at the
  current native maxima; Soul gains `floor(sleep_s / 900)` capped at its sourced native maximum, including
  when `f = 0`. Checked bounded arithmetic; p <= 40; no vocation multiplier, online tick phase or food item is an input.
- **Eligible sleep:** an eligible open row gives `sleep_s = d` (OFFLINE0 duration, cap 21 days = 1,814,400 s);
  a freed row gives `min(d, floor((freed_at - slept_at) / 1000))`; first free time/cause wins; stale, freed
  and eligible rows are all deleted in this settlement with a house occupancy revision advance. A freed bed
  limits only BED vitals: OFFLINE training/stamina/pool still use `d`.
- **Previous lease and Logout** are resolved independently (never generation minus one); only the latest
  build-carrying receipt that is that lease's actual logout marker counts; otherwise `d = 0`, no regen/training.
  Same-lease reconnect creates no settlement.
- **Units:** BED `slept_at`/`freed_at` and build receipt `committed_at` are Unix **milliseconds**; terminal
  lifecycle `decided_at` is seconds; the Logout anchor is microseconds. Binding is
  `slept_at == committed_at_us / 1000` (a multiplied equality would refuse a legitimate sub-millisecond
  remainder). Checked exact ms-to-us conversion, widened differences, floor once to seconds. The marker
  `as_of = committed_at` is stamped by the real DB trigger, never a runtime clock argument.
- **Food:** durable remaining-food is not stored on main; online `spell/food_regeneration.rs` and food
  condition ticks are not offline settlement. Eating again, replaying ItemUse or simulating ticks does not qualify.
  CONDITIONS0 §6.3 requires lease-release save, fresh restore and death reset. Preserve the food millisecond
  remainder and online fractional periods (#2196 `DurableVitalsResiduals`: residual periods are not HP/mana pools).
- **HP/mana** come from the ordered last-position projection (`LastPositionRecord.vitals`; NULL means
  admission maxima, not zero) under its `(character_lease_generation, write_sequence)` key; no extra root advance,
  no manufactured vitals receipt; `Superseded`/`PendingObligation` is resolved or the whole transaction fails/retries.
  **Soul** is runtime-local today (not in 0084); the real durable current pool and maximum belong to #2196. Never
  treat full-at-login Soul as a durable before-value.
- **Maxima** come from `load_character_cast_facts` or its transaction-bound native equivalent, never client values,
  a copied vocation table, caller `current` booleans or guessed rates; NoVocation/unavailable is not a default class.
- **Positive-time participant-required refusal** from the draft's proposal is inert review material: it must never
  ship as a blocking runtime call before the real food/vitals/Soul consumer exists. No partial award, no food debit.
- **Sleep authority** (BED-1 scope, recorded here for the consumer): Current Premium (fail-closed), no logout/PZ lock,
  reach/cooldown, active reciprocal head/foot resolver, 60 s reservation; an access revocation does not wake a sleeper,
  and login settlement invents no fresh Premium/access gate to confiscate historical gains.
- Owner-wake and bed-removed are house-runtime-only; a specific-cause free runs **before** the native ownership UPDATE in
  the same transaction; free never heals/debits/trains or advances CharacterRevision.

## Reference behaviour from the draft (re-implement, do not copy)

The draft code never existed; its design requirements are:

1. **Ordered physical transaction.** One native transaction and commit proof with retained operation identity: no nested
   `DurabilityRoot` commit, second food writer, followup receipt or post-commit SQL patch. (1) enter the semantic/recovery
   pass under the recovery fence and `db::lock_admission_relations`; serialize the marker key and return the first historical
   receipt before binding checks or recomputation. (2) fresh marker: session -> scope assignment/node incarnation ->
   admission guards -> Character root `FOR UPDATE` (`assert_session_fence`/`assert_gameplay_fence`), then progression/build rows
   and pending-respawn check; Channel only today, House via HOUSE-RUNTIME fences. (3) before a sleeper lock, determine the
   affected houses, take property locks, then house occupancy revision rows in one canonical World+house order, then the
   sleeper row; re-read after locks (the final cross-domain order is CP-owned; old and new houses on logout cleanup included).
   (4) load the latest marker, predecessor/times, build, durable food and last-position vitals, resolve maxima at the same
   revision, compute OFFLINE and BED after-values once; no write when any owner/rate stage is unresolved. (5) write the
   OFFLINE marker/root successor and receipt, food successor, HP/mana projection, any Soul successor, sleeper deletion and
   house revision together. (6) commit once; only a proven commit or historical reconciliation exposes final state; install
   post-values into the new actor before it is playable and publish via existing vitals deltas; replay carries history values
   and neither re-applies gains nor re-debits food; preserve `training_save` unknown-outcome sequencing.
2. **Logout bundle** (with BED-1/OFFLINE-1): after actor removal and final capture and before terminal release, insert the
   qualified sleeper and activation atomically, delete the Character's older row, advance both house revisions; unique
   conflict or a fenced house means a marker without sleeper and without activation. Integrate with the current
   `release_logout_session` controller (LOGOUT-WIRE), not an obsolete release consumer.
3. **Draft source checkpoint (inert).** A BED helper borrowing the OFFLINE transaction (pin property/revision/sleeper rows,
   bind predecessor and latest Logout, derive bounded eligible sleep, delete the pinned row, require one revision-hook advance),
   and a caller hunk after the plan/fence locks and before the marker write. Both were prepared against #2174 `b5576758e` and
   BED `87674e53c`, never applied or compiled; rebind to the actual merged ports instead.

## Owned paths (ADR-0022 layout)

Old `apps/game-server/src/house_bed/regen.rs` and `regen_tests.rs` (the draft's only code paths, never created). Mapping:

| Old | Current owner |
|---|---|
| `apps/game-server/src/house_bed/regen.rs` | pure arithmetic in `crates/gameplay/housing/src/house_bed_regen.rs` (L3); caller-TX consumer in `crates/gameplay/durability/src/durability/bed_regen.rs` (L4) |
| `apps/game-server/src/training/offline.rs` (#2174) | `crates/gameplay/durability/src/durability/character_build.rs` (D1381 moves the native marker writer here) |
| last-position/vitals projection | `crates/gameplay/durability/src/durability/character_last_position.rs`, `crates/gameplay/housing/src/last_position.rs` |
| `spell/food_regeneration.rs` | `crates/gameplay/spell/src/spell/food_regeneration.rs` (online ticks; read-only reference) |

- `crates/gameplay/housing/src/house_bed_regen.rs` (new, pure: `p`, consumed food, Soul gain, caps; no IO)
- `crates/gameplay/housing/src/house_bed_regen_tests.rs` (new)
- `crates/gameplay/durability/src/durability/bed_regen.rs` (new, participant composing the #2196 vitals participant with the BED-1 sleeper step in the borrowed OFFLINE transaction)
- `crates/gameplay/durability/src/durability/bed_regen_tests.rs` (new)
- `apps/game-server/tests/bed_regen_postgres.rs` (new, PostgreSQL 17.6 harness)
- `docs/agents/tasks/BED-REGEN-1.md`

Slot-only (final commit at the queue turn): module registration in `crates/gameplay/housing/src/lib.rs` and
`crates/gameplay/durability/src/durability/core_modules.rs`/`mod.rs`; the caller hook inside the OFFLINE `apply_marker`
(`character_build.rs`, shared with #2174, after plan/fence locks and before the marker write); any migration only if CP
allocates it; no registry expected. Shared deltas are `cross_domain_finding` items for CP/architect: extend the build
writer/receipt decode (marker-key uniqueness, trigger-stamped clock/as_of, OFFLINE columns, chain guards,
`verify_character_integrity`); CONDITIONS0 durable food ingress/save/restore/death reset; CHAR-VITALS0 projection sequence;
BED schema/provenance guards installed only with the real writer, hooks and canonical locks.

## Acceptance criteria

- [ ] Regen computed from durable timestamps, capped by food and maximum vitals; deterministic for a fixed clock.
- [ ] Food debit is `30 * p` even at capped pools; remainder < 30 s retained; Soul `floor(sleep_s/900)` up to its durable cap, also at `f = 0`.
- [ ] Settlement and free/owner-wake serialize on one property/revision/sleeper order; first free cause/time preserved; no reopen, lost revision or duplicate award.
- [ ] Original-marker replay returns history values and applies no award or food debit again, and consumes no later sleeper.
- [ ] Missing/corrupt participant or revision hook refuses with zero footprint; precommit failure at each write rolls back marker, root, build, food, projection, sleeper and revision together.
- [ ] Final HP/mana/Soul/food correct after commit/reload and crash between commit and actor installation.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer; tests in their own child files; `mod.rs`/`lib.rs` edits wiring-only.

Required native PG 17.6 matrix (none run): food 0/29/30/31 s, Soul 899/900 s, maxima one-below and shrunk maxima, `f` 0/1199/1200,
cap 21 days and max+1, seconds-vs-ms regression, checked ms-to-us overflow, submillisecond Logout boundary, acquisition-vs-later
settlement delay, no eligible sleeper / crash `d = 0` / stale predecessor / already-freed rows (training still over `d`), races
(owner-wake, owner change, disposition, bed-removed, stale connection/lease/scope/node/ACL against insert, death/pending respawn,
projection key supersession, terminal release, same-lease reconnect), food-condition restore, disabled-path no-consumption.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing house_bed_regen
cargo test -p oteryn-gameplay-durability bed_regen
cargo test -p oteryn-game-server --test bed_regen_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md`; `docs/reference/regeneration-food-20261010`
- `docs/agents/tasks/archive/OTV2-20261013-char-vitals-1.md`, `OTV2-20261013-accept-char-vitals-0.md`, `OTV2-20261001-offline0.md`
- CP coordination #2131: D1329, D1342, D1355, D1370, D1381, D1397/D1397a, D1399b, D1403, D1409, D1418, D1420, D1422
- `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4, §5 (Wave 5), §7
