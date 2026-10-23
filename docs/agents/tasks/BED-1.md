# BED-1 — sleeper table, bed USE/sleep, owner wake and offline settlement (consolidated task card)

```yaml
task_id: BED-1
title: "BED-1: sleeper table, bed USE and sleep, owner wake, offline settlement"
mode: IMPLEMENT
status: in_progress   # CP wire allocation implemented; runtime/persistence incomplete
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

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2147, branch skeleton/bed-1

## Latest checkpoint — 2026-10-22 BED-1 run 1d

**NOT_FULLY_QUALIFIED — fenced durable participant and dispatch adapter implemented;
House gameplay activation and OFFLINE integration remain incomplete.**
Local AUTHORING on `w5/bed-1`, HEAD
`6a3e9c833260001b8e7414c5c15182627da5f388`; no frozen SHA, PR, publication,
external write or Git mutation. Run 1c's uncommitted 0139 storage work and its
seven PostgreSQL cases are preserved.

Implemented this slice:

- `durability/bed_sleep/mod.rs::BedSleepOwner::commit` consumes the borrowed
  recovery authority, current node incarnation and existing `HousePresenceProof`.
  Presence must name the same World, House, session and scope generation. The
  bounded semantic transaction independently checks the active native House
  session's transport, connection/lease/scope generations, actual assignment,
  live Character root and latest complete session-use membership. No type-2
  audit event or type-2 transaction opener is added.
- The narrow definer in provisional 0139 locks the session, property and ordered
  old/target House revisions before sleeper mutation. It replaces the Character's
  older row, inserts the sleeper, terminalizes the House session/attempts and
  records the immutable `(GameSessionId, CommandId)` receipt in one transaction.
  Exact replay returns the first result before effects; changed reuse conflicts.
  A stale fence, missing native property, denied ACL or occupied bed writes no
  sleeper, receipt, session change or occupancy revision. Overflow rolls back
  the entire operation. An older row of this Character is cleanup, not another
  Character's occupancy.
- Sleep authorization uses the existing native `game_house_ownership` and
  `game_house_acl` model: owner, Guest or Subowner membership/wildcard; door-only
  grants do not authorize sleep. Neither service role receives direct table
  access. Runtime receives only the bounded sleep function's EXECUTE surface.
- The existing `sleep_in_bed` callback now delegates to the durable owner when
  `prepare_bed_sleep` supplies a House context, binding its session/fence,
  CommandId and skill to the sequenced command. The default supplies none and
  still refuses. Known success answers `OFFLINE_TRAINING` and ends the transport;
  an uncertain durable result sends no refusal and enters control-loss handling.
- Five PostgreSQL cases cover commit/replay/conflict, older-row cleanup,
  immutable receipt/role boundaries, seven fence/scope mutations, native ACL
  refusal plus Guest/Subowner grants, occupancy and rollback on overflow. The
  cases are lib-mounted in durability, dispatch and game-server, as CI permits.
  Eight existing House fixture helpers were moved without body changes to one
  reusable support file; every existing House test and assertion is preserved.
- `bed_sleep_successful_real_dispatch_commits_the_native_owner` exercises the
  actual BED adapter, session handle lookup and default owner delegation through
  the real PostgreSQL commit. It also checks stale handles and mismatched command
  binding. This is adapter-to-owner evidence: House admission uses the accepted
  §10 fixture, and fixed server fixture presence/head/sex/skill do not qualify an
  active bed resolver, reservation, native actor retirement or OFFLINE activation.
  The two existing command-loop capability/refusal tests remain unchanged.

Final-source validation through `/home/mole/oteryn-cache/codex-bin/cargo`, 1.95.0:

| Invocation / check | Result |
|---|---|
| `test --locked -p oteryn-gameplay-durability --lib --quiet bed_sleep -- --test-threads=1` | PASS, 5 actual PG tests, 48.80 s |
| `test --locked -p oteryn-gameplay-dispatch --lib --quiet bed_sleep -- --test-threads=1` | PASS, 8 tests: 6 actual PG and 2 transport, 32.66 s |
| `test --locked -p oteryn-game-server --lib --quiet bed_sleep -- --test-threads=1` | PASS, 5 actual PG tests, 26.33 s |
| `test --locked -p oteryn-game-server --test character_authority_postgres --quiet bed_sleeper_storage -- --test-threads=1` | PASS, 7 actual PG storage regressions, 51.91 s |
| `clippy --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings` | PASS |
| `fmt --all -- --check`; standalone new-file rustfmt; `git diff --check` | PASS |
| Governance validator / governance unit suite | PASS / 124 tests |

Every PG case requires the configured PostgreSQL 17.6 harness; none skipped and
no unfiltered PG suite ran. Initial failures were our fixture setup: additional
admin-seeded Characters must follow recovery qualification, and native ACL rows
require their exact receipt in the same transaction. Both were repaired without
weakening a guard or test. The existing vendored Tokio missing-documentation
warning remains dependency output; all three requested Clippy packages passed.
No unrelated load-contention failure is deferred to CI.

D1462: reread the whole local diff and new files against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; searched all `apps/`, `crates/` and
`tools/` matches, constructors, implementations and consumers of `BedSleepOutcome`,
`BedSleepRequest`, `BedSleepOwner`, fixture presence and the authority callbacks.
All five outcome variants are handled. The new public commit has its production
BED dispatch caller and the named adapter-to-PG test above. SQL lookups are keyed
or bounded to the two ACL lists/two affected Houses; every new SQL function's
call, grant and pinned search path was inspected. Existing fixture helper bodies
and all House test names were mechanically compared with HEAD. No tests were
deleted, skipped or weakened, and no Cargo/dependency/protected CI path changed.

Changed files (including the carried run 1c delta):

- `apps/game-server/migrations/0149_bed_sleeper.sql`
- `apps/game-server/src/lib.rs`
- `apps/game-server/tests/character_authority_postgres.rs`
- `apps/game-server/tests/support/bed_sleeper_postgres_cases.rs` (carried unchanged)
- `apps/game-server/tests/support/bed_sleep_postgres_cases.rs`
- `apps/game-server/tests/support/house_scope_handoff_fixture.rs`
- `apps/game-server/tests/support/house_scope_handoff_postgres_cases.rs`
- `crates/gameplay/durability/src/durability/bed_sleep/mod.rs`
- `crates/gameplay/durability/src/durability/mod.rs`
- `crates/gameplay/durability/src/lib.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bed_sleep.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bed_sleep_dispatch_postgres_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`
- `crates/gameplay/dispatch/src/lib.rs`
- `docs/agents/tasks/BED-1.md`

Unfinished (next slice; no gameplay acceptance checkbox completed):

1. Production House runtime context, active reciprocal bed/head resolution,
   bed handle presentation, reservation/reach/logout-block/Premium checks and the
   native sex source. Existing production authorities keep `prepare_bed_sleep`
   closed. Property lifecycle/content fences and granting-guild locks/currentness
   still require their owners; the native House entry access function remains
   the no-row stub. The basic native bed ACL check is not those prerequisites.
2. Shared `character_build` logout/offline-settlement markers, activation and
   provenance, same-transaction attachment, prior-lease/latest-build-receipt
   checks, admission consumption and the 21-day cap. The new terminal sleep
   receipt is explicitly not an OFFLINE marker or a training backend.
3. Runtime-only owner wake/bed removal, specific-cause disposition hooks before
   ownership UPDATE, occupancy view refresh and settlement/restart/race coverage.
4. Full native connection-loop success with real map-authored bed handles and
   House actor retirement/reconciliation. The passing adapter/PG test does not
   qualify the missing producers or admission/settlement composition.
5. Integrator slot qualification: migration allocation/renumbering, inseparable
   provenance/marker hooks before activation and independent frozen-head review.

## Prior checkpoint — 2026-10-22 BED-1b storage continuation

**NOT_FULLY_QUALIFIED — storage sub-part only; BED-1b remains incomplete.**
Local AUTHORING on `w5/bed-1` at `6a3e9c833260001b8e7414c5c15182627da5f388`;
no freeze, PR, publication or external write.
The owner's one-run continuation permits a coherent tested sub-part when the
whole slice cannot finish. The committed BED-1a transport work is preserved.

Implemented provisional migration `0149_bed_sleeper.sql`:

- Native World/House property FK, Character FK and Character-to-World check;
  full nonzero u64 lease; native signed i32 logical coordinates; closed sex and
  wake causes; millisecond timestamps and paired, ordered free fields.
- One row per Character, including freed rows; one open row per native bed key;
  immutable sleep identity and one-way open-to-freed updates.
- Per-House occupancy revisions initialized for existing/new properties and
  advanced atomically on insert/free/delete. Rejection, rollback and overflow
  retain the preceding state.
- The native ownership UPDATE invokes a SECURITY DEFINER fallback that locks
  the House revision before waking open sleepers with `owner_change`. Same-owner
  revisions do not wake. Prior specific-cause frees retain their first cause/time.
- Neither runtime nor control receives direct sleeper/revision privileges or
  callable mutation functions. All four new functions are trigger-only definers
  with pinned search paths. This is not a logout/activation provenance API.

Seven storage PostgreSQL cases are registered through
`tests/support/bed_sleeper_postgres_cases.rs` in the existing
`character_authority_postgres` CI target. They require the configured 17.6
harness, assert embedded migration 139, and cover storage identity, occupancy,
closed fields, one-way frees, native control-role owner changes, first-cause
preservation, rollback/overflow, role privileges and competing inserts/locks.
Their admin-seeded sleeper rows are test data, not successful gameplay evidence.

Validation through `/home/mole/oteryn-cache/codex-bin/cargo`, toolchain 1.95.0:

| Invocation / check | Result |
|---|---|
| `test --locked -p oteryn-game-server --test character_authority_postgres --quiet bed_sleeper_storage -- --test-threads=1` | PASS, 7 actual PG tests, 60.39 s on the final source; no skips |
| `test --locked -p oteryn-gameplay-dispatch --lib --quiet bed_sleep` | PASS, 2 transport tests |
| `clippy --locked -p oteryn-gameplay-durability -p oteryn-game-server --all-targets --quiet -- -D warnings` | PASS |
| `fmt --all -- --check`; new-file rustfmt; `git diff --check` | PASS |
| Governance validator / governance unit suite | PASS / 124 tests |

The first PG run exposed a test helper calling the UUID generator as the control
role; the helper now binds distinct fixture receipt IDs without expanding any
role's privileges. The Clippy sweep replaced `unwrap_err` with an assertion
helper that requires both rejection and the exact SQLSTATE. No test was weakened.
An existing `vendor/tokio-1.53.1` missing-documentation warning was printed by
dependency compilation; both requested Clippy packages passed. No unfiltered PG
suite ran, and no unrelated contention failure is deferred to CI.

D1462: reread the complete local delta and both new files against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; searched `apps/`, `crates/`, `tools/`
for every new SQL function and verified its trigger registration. No Rust enum
variant, struct field, trait method or public function was added, so there are no
new Rust match/constructor/impl consumers. No Type2 transaction opener was added.
The existing harness parent grows by two wiring lines; tests live in their own
child. Dispatch success/settlement E2E remains unimplemented, not qualified by
the passing storage and refusal tests. No Git mutation was performed by this run.

Unfinished:

1. `bed_sleep*.rs` durable caller-TX owner, using
   `begin_type2_transaction(holder, deadline)` for every Type2 opener; fenced
   sleep plus logout/activation, first-receipt replay and old-row cleanup.
2. Shared `character_build` logout/offline-settlement markers and admission
   settlement, including prior-lease/latest-receipt checks, deletion and the
   21-day sleep cap. Those owners still have no production marker path here.
3. Active reciprocal bundle bed resolution and House reservation/property/guild/
   Premium/reach/current-scope evidence. The existing House access owner remains
   the no-row stub; coordinates alone must never authorize sleep.
4. Production `FreshAdmissionAuthority::sleep_in_bed`, runtime-only owner wake
   and bed removal, specific-cause disposition writer integration, and real
   dispatch/PG coverage of commit, ACL/fence refusal, wake and settlement.
5. Slot qualification: install provenance guards and the real writer/hooks
   inseparably before activation; migration renumbering and independent review
   remain integrator work. No BED gameplay acceptance checkbox is completed.

## Prior checkpoint — 2026-10-22 CP allocation continuation

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
- `crates/gameplay/durability/src/durability/bed_sleep/` (fenced durable sleep/logout owner); future read/free/settlement children remain on this owner
- `crates/gameplay/durability/src/durability/bed_sleeper_tests.rs` (new)
- `apps/game-server/tests/support/bed_sleep_postgres_cases.rs`, lib-mounted in the three touched crates; retained storage cases use `character_authority_postgres` (no new PostgreSQL target)
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

Use `/home/mole/oteryn-cache/codex-bin/cargo` for every Cargo invocation. Owner
2026-10-22 PG scope overrides historical target/full-suite commands above.

```sh
cargo fmt --all -- --check
cargo clippy --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-durability --lib --quiet bed_sleep -- --test-threads=1
cargo test --locked -p oteryn-gameplay-dispatch --lib --quiet bed_sleep -- --test-threads=1
cargo test --locked -p oteryn-game-server --lib --quiet bed_sleep -- --test-threads=1
cargo test --locked -p oteryn-game-server --test character_authority_postgres --quiet bed_sleeper_storage -- --test-threads=1
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md` §4-§8
- `docs/architecture/reviews/OTERYN_GAME_ITEM_SEM_BED_GROUP_PACKET_2026-10-05.md`
- `docs/agents/evidence/OTV2-20261005-bed-facts-v1.json`
- `docs/agents/tasks/archive/OTV2-20261005-item-sem-bed-1-bed-group.md`
- CP coordination: #2131 (D1122, D1162, D1171, D1370, D1381)
