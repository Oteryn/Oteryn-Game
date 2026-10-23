# BED-1 — sleeper table, bed USE/sleep, owner wake and offline settlement (consolidated task card)

```yaml
task_id: BED-1
title: "BED-1: sleeper table, bed USE and sleep, owner wake, offline settlement"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
wave: 5
class: "hard (persistence, session-generation fencing)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4, §5 (Wave 5), §7 BED-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md   # §4-§8
  - docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md
base_branch: main
branch: w5/bed-1
pr: null
needs_migration: true
needs_registry: true   # CP confirmed command 31 under WORLD_INTERACTION_V1
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2304](https://github.com/Oteryn/Oteryn-Game/pull/2304). Command 31 codec and fail-closed dispatch boundary are merged. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/bed_sleep.rs`.

Remaining: Sleeper schema/owner, ACL/reservation composition, fenced sleep/logout, wake and offline settlement remain; a refusal handler is not working sleep. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2147, branch skeleton/bed-1

## Current implementation checkpoint — 2026-10-22 CP allocation continuation

**NOT_FULLY_QUALIFIED — BED-1 is incomplete.** The earlier wire-allocation
question is resolved; no new wire decision is requested. The integrator assigns
only command type **31 BED_SLEEP_INTENT** to BED-1 under existing capability
**11 WORLD_INTERACTION_V1**. APPEARANCE, TUTORIAL and DAILY allocations are
untouched. Migration **0139** remains provisional and has **not** been created.

Local AUTHORING only on `w5/bed-1`, predecessor
`9e5770362a96b2e0e269abf1480f7de3a17163fe`; no candidate freeze, PR, CI or
independent review. No Git mutation, publication or external write was performed.

Implemented wire/transport slice:

- Registry command 31, matching capability ownership table and schema; a strict
  24-byte decoder reuses the existing nonzero ItemTarget handle and admits the
  six explicit skill values. Missing, repeated, unknown and over-bound fields
  refuse; the full u64 handle and 24/25-byte boundaries are tested.
- USE result dispositions 20 BED_CHOOSE_SKILL, 21 BED_OCCUPIED, 22 BED_WOKEN and
  23 OFFLINE_TRAINING are schema/registry documented and gated by capability 11.
  Rune gating remains exactly 18/19; PUSH refuses rune and bed dispositions.
  The synthetic client exhaustive match handles every new variant.
- Command 31 goes through the real transport command sequence/capability gates,
  decoder and session handle lookup. Missing handles return STALE; non-base-map
  targets return NOT_POSSIBLE. The new House-owner callback defaults to refusal:
  the production composition has no successful sleep implementation. Capability
  11 remains unoffered; these changes grant no House or sleep authority.
- Four codec tests and two transport-loop tests cover wire limits, the registry,
  malformed/stale/unselected command refusal and negotiated result filtering.
  These are transport qualification, not successful bed gameplay evidence.

Validation through `/home/mole/oteryn-cache/codex-bin/cargo`:

| Invocation / check | Result |
|---|---|
| `test --locked -p oteryn-protocol-oteryn --lib --quiet` | PASS, 289 tests |
| `test --locked -p oteryn-gameplay-dispatch --lib --quiet` | PASS, 904 tests; 14 existing ignored cases unchanged; existing configured PG cases also executed |
| `test --locked -p oteryn-synthetic-client-harness --quiet` | PASS, 36 tests |
| `test --locked -p oteryn-game-server --test character_authority_postgres house_interior_postgres_stub_privileges_and_fixture_locking -- --nocapture` | PASS, 1 test, 21.03 s; actual PostgreSQL 17.6 migration/locking regression, not a skip |
| `clippy --locked -p oteryn-protocol-oteryn -p oteryn-gameplay-dispatch -p oteryn-synthetic-client-harness --all-targets --quiet -- -D warnings` | PASS |
| `fmt --all -- --check` | PASS |
| New-file `rustfmt --edition 2024 --check`; `git diff --check` | PASS |
| Governance validator / governance unit suite | PASS / 124 tests |

Focused filters covering the new cases (their bodies ran in the full suites
above): `test --locked -p oteryn-protocol-oteryn --lib bed::tests` and
`test --locked -p oteryn-gameplay-dispatch --lib bed_sleep`.
The PG regression explicitly asserts `server_version_num = 170006` and ran
migration plus locking checks. No BED-specific PostgreSQL suite exists yet.

D1462 review: reread the complete tracked delta and new source files against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; searched all `apps/`, `crates/` and
`tools/` consumers/constructors/matches of UseDisposition, BedSleepIntent and
FreshAdmissionAuthority. The decoder has its production connection caller and
transport-loop coverage. Existing authority implementations retain the new
callback's refusal default; this does not count as a native House owner.
Shared parent growth is wiring only (connection +34 lines); new tests live in
separate files. No test was removed, ignored or weakened. Existing unknown-code
assertions now use 24 because 20 is a newly registered disposition.

Remaining implementation, still required within the accepted BED scope:

1. Native reciprocal active-bundle House bed resolution, reservation and current
   ACL/property/guild/premium/scope checks; real House runtime and view consumers.
2. The existing build owner's offline/logout/settlement marker and activation
   prerequisites, with provenance, lease/revision fences and first-receipt replay.
3. Migration 0139, sleeper storage and occupancy revision guards; insert with the
   logout marker, specific-cause frees before ownership UPDATE, owner wake and
   bed removal, and deletion/settlement in the real admission transaction.
4. Pure housing rules, runtime sleep/logout composition, occupied presentation,
   concurrency/restart/replay coverage and `bed_sleeper_postgres` qualification.

No accepted gameplay criterion is marked complete. BED-REGEN remains excluded;
no food/vitals behavior was introduced. The prior audit below is historical:
its allocation blocker is superseded by this CP decision, while its identified
source gaps remain unfinished implementation work.

## Prerequisite audit — 2026-10-22 continuation

Inspected clean worktree `w5/bed-1` at
`9e5770362a96b2e0e269abf1480f7de3a17163fe`. Local-only authoring;
no Git mutation or remote write is authorized by this packet. Migration allocation
is **0139**; historical migration numbers below are not allocations.

Planned module layout, before coding:

- L3 `housing/src/house_bed.rs` with `house_bed_tests.rs`: native bed key,
  closed wake causes and capped sleep duration.
- L4 `durability/src/durability/bed_sleeper.rs` and mechanic children for
  logout insertion, freeing and admission consumption; separate child tests.
- Missing offline markers/settlement extend the existing `character_build`
  owner through child modules; no second training store or receipt chain.
- Active-bundle reciprocal pairing belongs in a `world/src/map/` child;
  reservation, USE and sleep dispatch belong in separate `dispatch` children.
  Existing shared parents receive wiring only. House property/admission/view
  prerequisites extend their existing owners, with no fixture authority.
- `apps/game-server/tests/bed_sleeper_postgres.rs` reuses the existing
  `tests/support/postgres.rs` isolation harness, with strict configuration and
  PostgreSQL 17.6 checks; no copied harness or duplicate module mounts.

PROVEN source gaps at this checkout:

1. `durability/character_build.rs::BuildCause` has only Training,
   VocationChoice and Promotion. There is no logout/offline-settlement marker
   or shared offline activation production path to call.
2. Migration 0074's `game_house_access` remains a closed stub. The native
   ownership/ACL reader in `durability/house_spell_acl.rs` does not supply
   BED0's property lifecycle/content fence and granting-guild locks.
3. `world/map/facts.rs` has placement facts but no reciprocal same-house
   bed resolver. The current `world/house_scope/mod.rs` still refuses exit;
   production house runtime/admission/view composition needs prerequisites.
4. `PROTOCOL_OTERYN_V1_REGISTRY.json` and the protocol schema/code contain
   no `BED_SLEEP_INTENT`, `BED_CHOOSE_SKILL`, `BED_OCCUPIED`, `BED_WOKEN` or
   `OFFLINE_TRAINING` allocation. BED0 §9 explicitly requires a CP-reserved
   command number; this card permits registry entries only on CP confirmation.
   The WORLDINT-WIRE-1 archive also says no BED command number was used.

The integrator's option B authorizes implementing missing prerequisites on
these existing owners. It does not supply the outstanding wire allocation.
Stopped before implementation at that authority decision, as the continuation
packet requires; no prerequisite is claimed implemented. Draft #2147's current
PR metadata confirms a closed private-storage checkpoint without migration,
registry, runtime caller or completed BED scope; its code was not copied.

Historical CP/owner question (resolved by the checkpoint above): provide the reserved `BED_SLEEP_INTENT` command ID
and result disposition codes, and confirm their registry/schema registration
under existing `WORLD_INTERACTION_V1`. No new capability is proposed. After
that decision, implement prerequisites and BED scope in this layout with real
production callers and unit/dispatch/PostgreSQL coverage.

Continuation validation/file-size record:

- Documentation-only checkpoint: governance validator PASS; governance unit
  tests **124 passed**. No Rust, SQL, Cargo or registry changes.
- BED test suites are not implemented; Rust compilation, Clippy, formatting,
  architecture and PostgreSQL qualification are **NOT_RUN**. No zero-match
  test invocation is counted as BED evidence; PostgreSQL has not run.
- Required future wrapper invocations: `cargo test --locked -p
  oteryn-gameplay-housing house_bed`, `cargo test --locked -p
  oteryn-gameplay-durability bed_sleeper`, and `cargo test --locked -p
  oteryn-game-server --test bed_sleeper_postgres`, each through
  `/home/mole/oteryn-cache/codex-bin/cargo`.
- Only this card changes (175 to 244 lines, +69); no file grows by more than
  200 lines. Reuse identified above is planned; no helper, fixture, harness,
  public API, enum variant, struct field or trait method was added.

## Goal

Sleeper table, bed USE and sleep, owner wake and offline settlement per BED0 §4-§8. The
sleep USE ends the session through the fenced logout path; settlement runs inside the real
OFFLINE admission transaction.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| BED0 acceptance | ACCEPTED (ACCEPT_BATCH §1, owner item 5, #2209) |
| BED-CONTENT-1 (bed group facts, `item_bed_promotion.rs`) | MERGED (#1861) |
| WORLDINT-WIRE-1 (USE map_item arm) | MERGED (#2094) |
| HOUSE-RUNTIME-1a | MERGED (#2163) |
| LOOP-VITALS-1 (food/vitals/soul, #2196) | MERGED (#2196) as skeleton; the durable food/HP/mana residual writer was taken out of it — the same-TX food participant BED needs is still missing (`docs/agents/tasks/LOOP-VITALS-1.md`) |
| OFFLINE-1 (logout/offline_settlement build markers, activation, receipt chain; #2174) | NOT MERGED — **BLOCKING** |
| STATUE-1 (shared offline activation path) | NOT MERGED — **BLOCKING** |
| HOUSE-RUNTIME-1b | BLOCKING — re-implementation pending (draft #2165 closing) |
| HOUSE-RUNTIME-1c | BLOCKING — re-implementation pending (draft #2166 closing) |
| MAP-OVERLAY-1c (active-bundle House placement/currentness, head/foot pair) | BLOCKING — re-implementation pending (draft #2167 closing) |
| HOUSE-1b (property/disposition writer) | BLOCKING — re-implementation pending (draft #2141 closing) |
| HOUSE-1a (property/auction owner) | BLOCKING — re-implementation pending (draft #2137 closing) |
| HOUSE-VIEW-1 | NOT MERGED — **BLOCKING** |

Integration order from D1162: HOUSE-RUNTIME-1a (#2163) → HOUSE-1b → HOUSE-1a → BED-1.
Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas.

## Owned paths (ADR-0022 layout)

- `crates/gameplay/housing/src/house_bed.rs` (new, L3 pure rules: stored bed key, freed causes, sleep-duration cap)
- `crates/gameplay/housing/src/house_bed_tests.rs` (new)
- `crates/gameplay/durability/src/durability/bed_sleeper.rs` (new, sleeper storage, caller-TX read/lock, free/delete steps)
- `crates/gameplay/durability/src/durability/bed_sleeper_tests.rs` (new)
- `apps/game-server/tests/bed_sleeper_postgres.rs` (new)
- `docs/agents/tasks/BED-1.md`

Slot-only (final commit at the queue turn): the migration (number: next free number at
allocation time (only one migration PR in the queue)); module registration in
`crates/gameplay/housing/src/lib.rs`, `crates/gameplay/durability/src/durability/core_modules.rs`
/ `mod.rs`; the bed USE arm in `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs` /
`connection.rs`; `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`,
`GAME_EVENT_FOUNDATION_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json`,
`OTERYN_GAME_ERROR_CODE_REGISTRY.json` only if CP confirms an entry; `capabilities.rs`
wiring (no new capability offered until CP decides).

## Wiring points (verified on origin/main)

- `crates/gameplay/content/src/content/item_bed_promotion.rs` — BED-CONTENT-1 authoring facts (`ReferenceItemBed`); not proof of an active head/foot pair
- `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs` (619 lines) — USE map_item arm (WORLDINT-WIRE-1)
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` command chain `else if command.command_type == COMMAND_TYPE_...`; `capabilities.rs` offered sets
- `crates/gameplay/durability/src/durability/character_build.rs`, `crates/gameplay/domain/src/domain/character_build.rs` — `BuildCause` admits only `Training`, `VocationChoice`, `Promotion`; OFFLINE-1 adds logout/offline markers (D1381 moves the native marker writer here)
- `crates/gameplay/durability/src/durability/fresh_admission.rs` (`FreshAdmissionStore::release`) — terminal release, final position and lifecycle receipt; no sleeper hook yet
- `crates/gameplay/durability/src/durability/house_spell_acl.rs`, `crates/gameplay/housing/src/house_spell_acl.rs` — native ownership/ACL reads (`game_house_ownership`, `house_access_for_character`)
- Native `HouseId` (World + revision-free house content key) and `LogicalCell` — the bed identity; never a generated bed UUID or a Channel-local identity

## Accepted decisions carried from the draft

- **D1171:** OFFLINE/logout marker/activation belong to #2174 (OFFLINE-1); durable food/soul to
  #2196 (LOOP-VITALS-1); the caller-TX sleeper step and the BED-CONTENT-1 head/foot leaf to
  BED-1. A `map/facts.rs` accessor follows #2156.
- **D1370 / D1381:** the real OFFLINE receipt/build extension, chain guards and clock/predecessor
  source belong to #2174; their migration numbers are void — this card makes no number choice.
- **D1162:** registration order HOUSE-RUNTIME-1a → HOUSE-1b → HOUSE-1a → BED-1.
- **D1122:** the draft checkpoint was private storage only (NOT_FULLY_QUALIFIED); no runtime authority.
- Mandatory CP ordering: a disposition or other specific-cause free runs **before** the native
  ownership UPDATE in the same transaction; otherwise the generic owner-change fallback records
  `owner_change` first and the one-way guard refuses the real cause.
- No caller boolean, raw admin SQL, stored coordinate, separate bed training backend or new
  authority trait replaces a missing owner seam.

## Reference behaviour built by the draft (re-implement, do not copy)

- Sleeper row: native property/Character FKs, Character-to-World binding, one row per Character
  (including freed rows), one open sleeper per native bed key, nonzero u64 lease and revision,
  native `appearance::Sex` variant, four closed freed causes, paired timestamp-ordered free fields,
  immutable sleep identity and a one-way occupied→freed transition (no overwrite/reopen).
- Units: `slept_at`/`freed_at` are Unix **milliseconds** (as native `character_build`
  `committed_at`); the Logout anchor binds `slept_at == committed_at_us / 1000`. Second
  conversion belongs to the OFFLINE owner.
- Owner-change fallback: `AFTER UPDATE OF owner_character_id` on the native ownership table frees
  only open rows with `owner_change` at `clock_timestamp()` after taking the house revision row
  lock; same-owner revisions do not wake; first time/cause is preserved. Access revocation is not
  a wake cause.
- Insert/free/delete advance the house occupancy revision in the same transaction; rollback and
  rejected writes keep the prior state; revision overflow fails the mutation.
- Read: one snapshot including the house revision; the caller-TX variant takes `FOR UPDATE OF s`
  and neither commits nor frees/deletes/awards. Missing revision, bad lease/identity/variant/time
  or free pairing fail closed.
- PG test harness errors (never skips) on a missing or non-Unicode
  `OTERYN_TEST_POSTGRES_ADMIN_URL` and checks `server_version_num=170006`.
- Draft evidence (historical): 13 authored cases (1 pure, 12 PG) including competing free
  (`55P03` loser, zero-row retry) and competing inserts (`23505`, rollback isolation); current
  generation compilation, Clippy and PG execution NOT_RUN.

## Findings and open questions

Integration still owned elsewhere and required before activation:
1. Active-map head/foot resolver: USE, choice, reservation, insert, view and bed-removed checks
   share the real current bundle's reciprocal same-house resolver (MAP-OVERLAY-1c); stored
   coordinates are not proof.
2. Property/ACL currentness: the property row lacks BED0's OWNED / MOVE_OUT_PENDING / content
   fence; the writer takes property `FOR SHARE` plus granting guild/membership locks and
   revalidates session/lease/scope/Premium/block/reach. Owner-wake/bed-removed are
   house-runtime-only.
3. Logout bundle (with OFFLINE-1/STATUE-1): first-receipt replay before mutations; marker
   timestamp/lease/activation and native sex at insert; cleanup of the Character's older row;
   qualified head insert. A bed unique conflict commits the marker without sleeper or activation.
   Multi-house revision locking includes the old row's house.
4. Admission bundle: the OFFLINE settlement transaction locks the row; only the matching previous
   lease plus its latest logout receipt count; any row (including stale/freed) is deleted with a
   revision advance. `sleep_s = min(d, freed_duration_s if freed else d)`, cap 21 days; training
   still runs over OFFLINE0's `d` after wake. No food consumed and no regeneration coefficients
   promoted until DUR-02 durable vitals (BED-REGEN stays separate).
5. The slot installs schema + permissions + marker provenance guards + the real writer/hooks
   inseparably.

## Acceptance criteria

- [ ] Sleep ends the session through the fenced logout path.
- [ ] Owner wake and settlement are idempotent across restart.
- [ ] Only the house ACL may use a bed.
- [ ] Specific-cause free precedes the ownership UPDATE in the same transaction.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer (pure rules in `housing`, storage in `durability`, USE arm in `dispatch`).
- [ ] Structure: new tests in their own child files (`<module>_tests.rs` or `apps/game-server/tests/` child), not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines (`connection.rs` is already over).

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing house_bed
cargo test -p oteryn-gameplay-durability bed_sleeper
cargo test -p oteryn-game-server --test bed_sleeper_postgres   # PostgreSQL 17.6 harness
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md` §4-§8
- `docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`
- `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json`
- `docs/agents/tasks/archive/OTV2-20261005-item-sem-bed-1-bed-group.md`
- CP coordination: #2131 (D1122, D1162, D1171, D1370, D1381)
