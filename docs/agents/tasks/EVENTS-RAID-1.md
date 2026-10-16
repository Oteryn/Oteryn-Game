# EVENTS-RAID-1 — raids (skeleton task record)

```yaml
task_id: EVENTS-RAID-1
title: "EVENTS-RAID-1: raids (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/events-raid-1
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 4 row EVENTS-RAID-1 (row 102); App. 7 MONSTER-META-1, EVENTS-RAID-1, EVENTS-WORLD-1, ARENA-1; §5 Wave 5"
class: "hard (child graph)"
needs_migration: maybe
needs_registry: maybe
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/raid/ (new) + tests"
  - "content/raids/ source inputs"
  - "docs/agents/tasks/EVENTS-RAID-1.md"
slot_only_paths:
  - "regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/world/pins/*, Rust pins"
  - "migration if raid state is durable"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

Raids: RAID-CONTENT-1 and RAID-1 (scheduled raid runtime, announcements) per BOSS_RAID0.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| BOSS_RAID0 acceptance (CANDIDATE) | NOT ACCEPTED | **BLOCKING** |
| LOOP-MONSTER-1 | NOT MERGED (Wave 2) | **BLOCKING** |
| CHAT-1; CHAT-1b-2b (RAID-1 announcements) | CHAT-1 MERGED; CHAT-1b-2b unconfirmed | **BLOCKING** |
| encounter admission (E4), ENCOUNTER_RT0 | ACCEPTED 2026-10-14; ENC-RT-1 not merged | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/raid/ (new) + tests
- `content/raids/ source inputs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/world/pins/*, Rust pins
- migration if raid state is durable
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (broadcast)
- apps/game-server/src/monster_summon.rs, creature spawn
- world clock (world_clock.rs)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author
- migration + writer + durability (if durable)

## Acceptance criteria

- [ ] raid schedule deterministic per world epoch
- [ ] At most one running firing per (WorldId, raid key), with persistent raids firing at most once per reset epoch; restart-safe (BOSSRAID0-RL-02).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server raid
python3 tools/content-migration/regenerate_content.py && git diff --exit-code
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md`
- `docs/architecture/reviews/OTERYN_GAME_ENCOUNTER_RT0_ENCOUNTER_RUNTIME_AND_BOSS_LEVERS_DECISION_2026-10-01.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.

## Local private component design (before source)

Allocated draft #2177, base `3b3e9a94160bd6df7c48f091ddf0b072d6ebd243`.
The controlling BOSS_RAID0 §4/12/14/17 is ACCEPTED on main
`ac9f172836f00226b68088aa04db95ccf1ab994e`; historical CANDIDATE and aggregate
integration dependencies above do not revoke separately authorized owned-path preparation.
Only `raid/mod.rs`, `raid/store.rs`, `raid/tests.rs`, `raid/postgres_tests.rs` and this
card are allocated. Content, shared registration, production migrations/grants and wiring stay CP.

Source trace: HOUSE_OWN0 §9 and BOSS_RAID0 §12 require the real recovery fence and
admission-relation locks for World jobs, without a Character session fence. Actual
`durability/character_authority::assert_recovery_fence` and
`durability/db::lock_admission_relations` are durability-private. Their visibility is preserved.
The native assignment writer is Channel/House-scoped, not a WorldReset or raid-content producer.
`world_clock.rs` supplies Tibian light/time, not a durable current WorldReset epoch. Existing
carrier `SpawnSourceId(u16)` is content-spawn identity, not the accepted raid occurrence
`(firing_id,wave,entry,index)`. No available source establishes admitted raid content or the
assigned-and-admitting channel set at fire time. These are concrete mandatory integration seams.

Prepare a coherent **data-only** component:

- A pure immutable transition proposal over one `(WorldId, raid ProductionKey)` schedule,
  its most recent firing and exact target-channel/run rows. Native WorldId, ChannelId,
  ScopeOwnershipGeneration, ProductionKey and simulation-determinism are reused. Inputs are
  proposed facts; they are never sealed/current authority or permission to act.
- Random checks use the accepted 60-second interval and one native deterministic RNG draw
  bound to World, raid key and check ordinal. Due checks advance the ordinal even when blocked;
  next check is from the recorded now, never a downtime catch-up loop. Fixed calendar/reset
  resolution remains with its real owner; the core receives an explicit proposed absolute slot
  and retains its once-only high-water mark, without inventing calendar/reset rules.
- At most one RUNNING firing **per raid**, as accepted §4.2 (the skeleton's broader one-per-World
  checkbox is not implemented as a new restriction). Persistent firings block another firing
  in the same epoch even after DONE. Target channels are frozen; late channels cannot join.
  Claims are metadata: same-generation retry never creates another run; a newer generation
  marks an older claimed run LOST and never re-runs it. No actor or announcement action is emitted.
- Close requires terminal rows for all nonempty targets or the accepted deadline; missing rows
  become LOST at the deadline. Reset cancels a RUNNING firing. Claimed/DONE metadata never proves
  physical spawn, announcement delivery or actor removal.
- A private three-table design retains schedule state, immutable firing identity/targets and
  per-target run state. A named transaction advisory lock serializes one key; complete expected
  snapshots and revision form CAS, exact after-state retry is replay, stale/different snapshots
  compare-fail. Historical firings remain retained; the core reads only the latest firing needed
  for guards. Private helpers do not commit or expose a production authority entry.

The schema is installed only in an isolated PostgreSQL 17.6 test database. Its private payload
encoding and constraints are implementation storage, not a public protocol/content schema.
Qualification covers actual SQL CAS/reload/rollback/concurrent losing writer and pure transitions;
it does not qualify native recovery/admission fences, live content/reset/channel truth or runtime.
Future CP must wrap the core in the actual semantic transaction/deadline, take recovery and
admission locks before occurrence/World rows, independently resolve those current facts, and
integrate actual actor/announcement occurrence deduplication. Failed/uncertain metadata CAS can
never authorize action; claim/commit reconciliation must inspect exact retained identity first.

### Prepared private result and remaining CP work

The four allocated files now contain the pure transition aggregate, private SQL core and tests.
The schedule uses a private revision (not CharacterRevision), preserves check/firing ordinals
across reset, preserves durable min-gap through restart, and retains retired rows. Fixed slots
older than the current 60-second check interval are missed rather than caught up; converting
weekday/day-of-month/reset offsets into the proposed absolute slot remains unimplemented until
the actual calendar/current-reset owner supplies it. Native deterministic RNG is bound to the
framed `(WorldId, raid key, check ordinal)`; no second generator or clock is introduced.

Private physical design: the three accepted table names use `(WorldId, raid key)` keys;
firings add the ordinal and globally unique UUIDv7 bytes; runs add the targeted ChannelId.
Schedule/firing/run payloads are strict, bounded private JSON encodings in BYTEA, validated on
decode against native identities/generations, relational revision/firing identity, exact frozen
targets and cross-row invariants. These are not public content/protocol or authority envelopes.
The schedule payload carries the accepted last-fired/next-check/check/firing counters plus
explicit reset/bundle/timing/retirement/fixed-slot guards. The firing payload retains fired time,
epoch, bundle, close deadline, state and immutable targets. Target membership is separate from
run-row presence, so absent rows cannot falsely finish a firing. Historical firing/run rows
remain retained when a new firing is created; this core loads only the latest firing and its
targeted runs, not unbounded history. Exact payload encoding, constraints, immutable-column
protection, production schema/grants and migration installation need the CP slot.

The private CAS takes the per-key named transaction advisory lock, compares the **whole** expected
snapshot and writes schedule/firing/run changes in the caller's transaction. It never commits.
Exact after-state retry is `Replayed`; an obsolete/different snapshot is `CompareFailed`, including
after later transitions. No outcome is a realization, announcement, current owner or commit token.
There is no exported production writer or guessed authority callback.

Qualification on the final exact source: **14 tests PASS,0failed/ignored** (13 pure and one actual
PostgreSQL17.6 test with its own uninstalled private schema). The PG body covers transactional
rollback, exact replay, distinct concurrent proposal loser, fresh-connection reload, old snapshot
refusal, World isolation, retained history and single-invariant stored revision/target/firing-ID/
zero-generation corruption. Native WorldId/ChannelId/ScopeOwnershipGeneration/ProductionKey and
the actual determinism crate are used unchanged through a small source-bound harness; no authority
types or validators are replicated. A controlled outside-tree mutant that omits the expected-
snapshot comparison fails the concurrent-loser assertion; unchanged original then passes all14
tests and Clippy metadata. The fixture drops its exact database even on an assertion panic.
Initial14/0 test evidence and two initial Clippy style diagnostics are retained separately; those
diagnostics were corrected before final qualification. Full native unit/server/E2E is not claimed.

Root review identified a real numeric-order defect in the initial loader: selecting
`ordinal::text` and then unqualified `ORDER BY ordinal` sorted the output as text, so9 could
outrank10. Accepted and fixed: a new real-PG regression closes/creates/reloads firings through
ordinals10 and11. It fails on the original exact source at numeric MAX10 (13pure PASS,1PG FAIL),
then passes with the single query repair `ORDER BY game_world_raid_firings.ordinal DESC`.
The initial14-pass evidence was insufficient coverage and is explicitly superseded. Actual
RED sources/logs and repaired GREEN hashes are retained; the sibling sweep found this as the
only ORDER BY site in the owned component. This is distinct from the controlled CAS mutant.

Mandatory concrete CP seams before runtime activation:

- A native semantic transaction/deadline wrapper must call the actual sealed recovery check
  and admission-relation locks before occurrence/World keys. Current helper visibility stays
  unchanged here. Candidate enumeration must be bounded to the accepted100 rows/pass and keys
  locked in the accepted deterministic order. There is no new World lease/grant in this component.
- Resolve actual current WorldReset epoch, active/admitted raid definition/bundle, recorded time
  and fixed calendar slot independently. Validate all admitted waves/creatures/anchors and accepted
  200-creature/32-wave/16-announcement bounds through their real content/encounter owners. Neither
  a raw epoch/digest/Timing value nor a successful data CAS supplies this authority.
- Resolve target channels from current assigned-and-admitting scopes under admission locks at
  firing commit; do not copy a saved set into an alleged current fact. For claims, use actual
  scope-assignment high-water/history, current incarnation possession and current owner generation
  (the real assignment/history/prove_current_incarnation seams), not the proposed generation alone.
- Install actual raid actor sources `(firing_id,wave,entry,index)` and announcement occurrences
  under the live owner. Run-row claim does not prove installation or delivery. Same-generation
  retry must reconcile exact occurrences without repeating them; new-generation LOST never
  re-runs them. Ambiguous commit must reconcile metadata and live-owner occurrence state before
  acting. Completion proposals need actual owner terminal facts; reset actor removal remains the
  real overlay/reset owner's operation. CAS metadata cannot authorize any of these actions.
- Shared module/writer registration, physical migration/grants/immutability guards, accepted row
  registration, content population/regeneration/pins, CHAT/BOSS-WIRE and encounter/runtime binding
  remain CP-only. No original acceptance checkbox is closed by this private preparation.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Fixed-calendar conversion preparation — 2026-10-16

Allocation: sole calendar-source writer for existing draft #2177, branch
`skeleton/events-raid-1`, authoring base `7d2a417c48704fab683413851bdf7e4c87dcf70a`.
Only `raid/calendar.rs`, `raid/mod.rs`, `raid/tests.rs` and this record are allocated.
The lead resolved META and confirmed the other boss/raid worker holds no #2177 writes.

Protected local main `24fc332f256638e3b784bff1b3c85b18e2665a76` carries the
ACCEPTED BOSS_RAID0 decision. Its §4.1 defines fixed weekday/month-day rules with
an offset after reset; §4.2 defines once-only firing and missed-check behavior.
ADR-0021 §3 leaves the reset schedule operational. Calendar timezone, current reset
and content authority therefore remain independently supplied by their real owners.

`calendar.rs` accepts a typed selector, nonnegative offset in internal milliseconds,
and explicitly caller-resolved reset timestamp/weekday/month-day facts. It matches
the selector and checked-adds the offset; a nonmatching date returns an explicit
absent slot. Bounds are intrinsic weekday/month-day bounds and representability;
no offset cap, midnight truncation, UTC/DST mapping or content encoding is invented.
Supplied facts remain data rather than a current-authority or content-admission proof.

`Snapshot::check_fixed_calendar` is a concrete consumer: it rejects Random policy
and a simultaneous raw `fixed_slot_ms`, converts the calendar facts, and enters the
same existing fixed-check transition. A calendar mismatch consumes a due check without
creating a synthetic slot. The raw `Snapshot::check` still rejects a missing fixed
slot. Shared ordinal/deadline advancement, running/persistent guards, fixed-slot
high-water, missed-slot handling and restart state are preserved. Stored Policy,
payloads, CAS, schema and DDL are unchanged; no second clock or scheduler is added.

Seven new tests exercise all weekdays, supplied month-day boundaries, mismatches
with/without a prior firing, zero and multi-day offsets, raw/calendar equivalence,
exact replay, running/persistent/high-water guards, restart skipping, conflicting
inputs and invalid/overflow data. Rustfmt and whitespace checks pass. Behavioral
tests and native compilation have not yet run; they await the lead's shared Rust
validation slot. No outside-tree harness, shim, production module registration,
DDL installation or successful native/runtime validation is claimed here.
Governance validation and all 124 agent unit tests pass for the task-record change.

Remaining production work is unchanged: real calendar/reset/content/channel fact
producers, sealed recovery/admission transaction boundary, bounded World-job query,
physical raid actors and announcements, migration/grants and composition wiring.
Those steps remain separate from this feasible owned-source preparation; no full
raid acceptance checkbox is closed by the calendar conversion alone.
