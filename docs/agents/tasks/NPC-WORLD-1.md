# NPC-WORLD-1 — NPC world: travel, walking actors and voices (skeleton task record)

```yaml
task_id: NPC-WORLD-1
title: "NPC-WORLD-1: NPC world: travel, walking actors and voices"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row NPC-WORLD-1; §7 'NPC-WORLD-1' (TRAVEL0 and NPC_BEHAVIOUR0 graphs)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_TRAVEL0_NPC_TRAVEL_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/npc-world-1
needs_migration: true
needs_registry: true
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2157](https://github.com/Oteryn/Oteryn-Game/pull/2157), [#2129](https://github.com/Oteryn/Oteryn-Game/pull/2129), [#2308](https://github.com/Oteryn/Oteryn-Game/pull/2308). Travel/voice/walking preparation and admitted NPC facts are merged; Newhaven free Anna/Seagull routes now have real command/arrival composition. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/newhaven_travel.rs`.

Remaining: General paid/Premium travel fee/placement composition, walking owner handoffs and budgeted voice chat publication remain; free tutorial routes do not complete the family. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Goal

NPCs become world actors: travel NPCs move Characters between destinations for a fee (premium-gated where the brief says), NPCs walk within their movement facts and speak admitted voice lines.

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

TRAVEL-CONTENT-1 -> NPC-TRAVEL-2 (NPC-TRAVEL-1, PREM-3); NPC-ACTOR-1 (CREATURE-MOVE-1, NPC-CONTENT-2); NPC-VOICE-1 (CHAT-1).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| #2040 NPC trade writer | MERGED |
| CHAR-POSITION-1 | MERGED (#2006) |
| PREMIUM-1 (PREM-WIRE-1) | MERGED (#2110) |
| CHAT-1 / CHAT-1b-2b (NPC-VOICE-1) | MERGED (#2093, archive chat-1b-2b) |
| acceptance of TRAVEL0 (TRAVEL-* children) | BLOCKING |
| NPC-CONTENT-2 (movement facts, voices) | BLOCKING (open PR #2129, Wave 2) |
| CREATURE-MOVE-1 residue (NPC-ACTOR-1) | BLOCKING (Wave 2, not merged) |
| NPC-PLACE-1b, NPC-WIRE-1 (NPC-ACTOR-1) | MERGED (archive npc-place-1b, npc-wire-1) |
| NPC-TRAVEL-1, PREM-3 (NPC-TRAVEL-2) | BLOCKING (no record on main) |
| PREM-1, GOLD-FEE-1b, QUEST-PRED-1, SPEED-1, VIS-2 | MERGED (archive records) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `content/npc/travel/** (sources only)`
- `crates/gameplay/npc/src/npc_travel.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_travel_tests.rs`
- `crates/gameplay/npc/src/npc_actor.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_actor_tests.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/npc_voice.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs` (D1344a: only the synchronous with_npc_customer_state read)
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` (D1160: one ordinary npc_actor module registration line)
- `crates/gameplay/entities/src/carrier.rs` (D1340b: read_movement_position and commit_movement_position only)
- `crates/gameplay/entities/src/carrier/npc_actor.rs` (D1340b: six native Movement regression cases only)
- `crates/gameplay/durability/src/durability/npc_travel.rs`
- `apps/game-server/tests/npc_world_postgres.rs`
- `docs/agents/evidence/NPC-WORLD-1-*.md`
- `docs/agents/tasks/NPC-WORLD-1.md` (this record: status and notes only)

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
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- crates/gameplay/durability/src/durability/core_modules.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- connection.rs NPC_TRADE_INTENT l.2766 / talk path (travel keyword routing)
- world_runtime.rs actor tick registration
- CHAR-POSITION-1 position writer (travel destination)
- crates/protocol-oteryn/src/lib.rs `pub mod` l.2545-2568

## Inseparable bundles (never split across PRs)

- NPC-TRAVEL-2: fee debit + position write + audit in one transaction
- NPC-ACTOR-1: movement facts + actor tick + visibility tests

## Acceptance criteria

- [ ] Travel debits the fee once and moves the Character atomically; premium refusal typed.
- [ ] Walking NPCs stay within movement facts and never block mandatory tiles.
- [ ] Voice lines come only from admitted content and respect chat rate limits.
- [ ] Restart resumes NPC positions from content, not stale runtime.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server npc_travel
cargo test -p oteryn-game-server npc_actor
cargo test -p oteryn-game-server npc
cargo test -p oteryn-protocol-oteryn && cargo test -p oteryn-game-server capabilities
python3 tools/content-migration/regenerate_content.py --check   # slot commit only
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_TRAVEL0_NPC_TRAVEL_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md
- PRs #2040, #2006, #2110, #2093, #2123; open #2129
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local preparation — NPC-TRAVEL pending arrival and hold (2026-10-15)

Allocated exclusively by CP from `945bedb22bef5b4402748215aaf874e729914474` on
`skeleton/npc-world-1`, draft #2157. TRAVEL0 is **ACCEPTED** at protected main
`3ef900c160c9713092a86bfcd1a8377c7587c898` via the protected ACCEPT-BATCH integration;
the skeleton's historical dependency table above is not a new acceptance gate. This packet
prepares only the private pending-arrival reader/consume and the per-actor hold/replay component.
No NPC actor/voice/content child, fee producer or production relocation is included.

### Delivered component and private schema

- `TravelHoldSlot` retains exactly one unresolved invocation. It freezes the confirming native
  CommandRef, NPC/route/fare/destination/occurrence, native session/item fence, native Content
  generation, captured position revision and native owner work stamp. Start requires an exact
  Channel/World and independently supplied current scope-bound native owner; none of these facts
  admits a route, substitutes for eligibility or grants executable relocation authority.
- Pending fee outcomes refuse walking and defer logout/transfer through the caller's slot.
  At 2,000 ms the component requests an alert only. Ambiguous fee/consume keeps the hold;
  initial fee reconciliation MUST use retained command/cause outcome, never row absence.
  Known commit enters placement-pending. Placement compares independent current native owner,
  Character/session/lease/Channel/scope generation, Content generation and position revision;
  a mismatch leaves the obligation for admission. Combat continues. Death cancels immediately,
  even during ambiguity, and every late completion becomes inert. The owner must actually drop
  queued steps/refuse new walk intents and gate terminal release/transfer at the shared call site.
- The consume is the accepted §7.4 projection, outside CharacterRevision: native reconciled
  recovery authority and semantic pass/deadline, admission relation locks, retained native
  assignment-history validator, exact current session/lease/scope/node proof, pending-row lock,
  exact `(character_id, occurrence_id)` deletion, and the existing native last-position upsert
  in ONE transaction. No matching row writes no position. A different occurrence is never
  deleted. Any failed/superseded/pending-obligation position write rolls the whole transaction
  back; no committed delete without its placed-position projection is accepted.
- Private storage is `game_character_pending_arrivals(character_id UUID PRIMARY KEY REFERENCES
  game_character_roots, occurrence_id UUID NOT NULL, world_id UUID NOT NULL, tile_x INTEGER NOT
  NULL, tile_y INTEGER NOT NULL, tile_floor SMALLINT NOT NULL)`. There is no Channel column:
  a later admission chooses its Channel normally. Native World/Character/UUIDv7 occurrence
  identities are decoded on reads. Both pending arrival and respawn, or a row in another World,
  fail closed as integrity errors. No shape-only insert API can manufacture a paid obligation.
  Own PostgreSQL fixture DDL is isolated/test-only; no numeric migration/production grants exist.

### Mandatory integration hooks and remaining criteria

1. **One item-only travel source transaction, not the current gold-fee entry point.**
   `durability/item_fee_burn.rs::FeeBurnCause` and `item_fee_burn_audit.rs` currently admit only
   CharmUnassign; `burn_fee_in_transaction` requires a CharacterRevision successor and emits its
   separate fee event. It cannot be called as travel's item-only atomic fee. The shared CP change
   must add the accepted NpcTravel cause/event/guards, bind its one occurrence 1:1 to the confirming
   CommandRef, bind NPC/route/content/base price/applied discount keys/final fare, and commit fee
   lines + its one cause + this pending row together, with no CharacterRevision advance. A zero
   fare produces no fee lines. Bank activation/coin-first rules must remain native.
2. **Actual current admission/eligibility.** Native talk currently exposes ordinary conversation
   and offers, not admitted travel confirmations/routes. `content/npc_catalogue/service.rs` has raw
   route data, not the new TRAVEL-CONTENT gate/discount admission. The shared source must consume
   admitted same-World/base-scope routes and re-read current level, quest discounts/fare, Premium
   activation/currentness, PZ/kill blocks, durable obligations and funds in the accepted order
   under the appropriate native locks. No guessed routes, fake Premium/current-reset facts or
   boolean authorization callbacks are introduced here.
3. **Physical owner placement/capture.** `FinalLastPosition` is DATA, never proof of placement.
   The caller must apply actual owner-fenced §7.3 relocation of a living, uncancelled actor using
   native collision/actor authority, then obtain the real `Shared::placed_record` capture (with
   actual vitals retained as required), before consume. The native SpellRelocationProof is sealed
   for its audited world-owner producer and is not repurposed or newly implemented here. Same
   World/Channel, native scope ownership/session/Content/position fences, fallback, summon removal,
   attack validity, logout-block retention and spatial deltas remain mandatory. A bookkeeping
   `PlacementPending` result authorizes none of those effects. Uncertain consume retains the hold
   until a native-fenced reread: exact row present -> retry consume only; row absent/different
   occurrence -> complete old consume, never repeat relocation. Lost session leaves admission work.
4. **Shared obligation owners.** `character_last_position::upsert_last_position` currently guards
   only pending respawns. Periodic/final position writers MUST also refuse while this pending row
   exists; terminal release must consume an actually arrived row before its final position write.
   The native death transaction must delete the arrival and receipt its deleted occurrence; the
   admission reader must prefer respawn, then arrival, consume the actually placed tile in the same
   transaction and fail closed if both rows exist. An arrival's current reader lock/shape does not
   satisfy those missing death/admission/periodic integrations. Clock/WorldReset is not inferred.
5. **CP slot:** production schema/roles and transaction/audit constraints, module registration,
   shared runtime/wire routing, content/registry/resource rows and independent exact-head review.
   All original acceptance boxes remain open. No full NPC-TRAVEL atomic fee writer, playable path,
   p99 hold/throughput measurement, actor relocation, readiness or merge claim is made.

### Qualification

Source-bound native overlay logs and manifests are retained outside the draft in
`/workspace/oteryn-prep/npc-world-1-evidence`; only literal module registration changes occur in
that disposable snapshot. Final validation counts, exact source hashes, setup/mutant attempts
and explicitly unqualified surfaces are recorded there after completion.

Production schema/cause constraint: the final pending row must be bound to the immutable travel
cause's `(occurrence, Character, World, destination)` in the same source transaction. The current
isolated schema intentionally has no invented travel-cause table/event/grant; its test seed rows
are fixture data only, never evidence of a paid trip. This cross-owner origin constraint remains
part of the shared fee/cause/migration CP hook above, not an omitted production guarantee.

Final local evidence: **5 pure + 1 genuine native PostgreSQL17.6 PASS**, including actual native
Root reader/consume semantic wrappers, exact occurrence/concurrency, provenance/fence negatives,
rollback and reload. Original untransformed native unit-context Clippy metadata, workspace fmt,
governance 124 tests (22 documents/9 lanes) and diff/scope checks pass. The initial global native
unit harness codegen was SIGKILL (zero tests ran); CP-approved exact native cfg(test) rlib + six
unchanged-body forwarding wrappers completed instead. Snapshot-only registration/test-attribute/
visibility/runner deltas are fully recorded; native Root/fixtures/private API bodies are unchanged.
The alternative no-global-harness build's 2,304 baseline unused-helper warnings are retained, not
hidden as clean full-test lint evidence. No behavioral RED/all-target/server/E2E/physical arrival
or full atomic fee-writer qualification is claimed. Exact final source/hash/patch binding and
remaining seams: `/workspace/oteryn-prep/npc-world-1-evidence/index.md` and `scope.json`.

## Local authoring — native voice preparation (2026-10-16)

The owner confirmed this family's exclusive authoring assignment in the active chat. This
preparation changes only `gameplay_transport/npc_voice.rs` and this record; it does not grant a
shared-path slot or mark a candidate frozen/qualified.

- Added a candidate voice book from the existing `NpcDataCatalogue` and `NpcServiceModel`,
  requiring matching source digests/revisions and exact NPC Dialogue references. Held/missing
  NPCs contribute no profile; invalid native `ChatText` and unresolved placeholder lines are
  excluded. This consumes existing typed source fields and does not replace NPC-CONTENT-2's
  missing voice admission/active-bundle producer.
- Added ephemeral per-exact-actor cadence using the native current owner fence, actor lookup,
  pinned movement context and player perception geometry. Idle thinks add no time; retries
  neither advance cadence nor produce another line; stale sequences and retired owners refuse.
  Native think occurrences and `NPC_VOICE` draws select chance and a uniform source line.
- The output prepares native `ChatLine::Local` say/yell values with original NPC text, as
  NPC-BEHAVIOUR0 §6 requires. It writes no egress and applies no player-only yell/spam rules.
  CP must assign CHAT_V1 listener selection/egress and budgeted think wiring. The native
  source-catalogue-to-active-runtime-pin binding producer is absent: same-source catalogue/model
  comparisons and runtime context fencing do not establish that binding or admit raw voices.
- Seven tests were authored for a real native catalogue/model path with exact referenced
  Dialogue and source-mismatch rejection, invalid/placeholder text, cadence/retries, idle/resume,
  absent profiles and actor/owner rejection, deterministic/zero-chance selection, and original-case
  say/yell projection. At the initial preparation freeze they had **not run**; subsequent actual
  native evidence is recorded below. No production registration, admitted-content qualification,
  egress or playable voice claim is made. The lead-provided
  isolated official `rustfmt` 1.94 formatted the owned module and its focused `--check` passed;
  `git diff --check` also passed. Native compilation/test validation was pending at that
  preparation freeze; syntax formatting alone is not type checking or behavioral evidence.

All original acceptance boxes remain open. Travel fee/cause, eligibility, actual relocation,
death/admission/position integration, actor movement/content and CP slot blockers above remain.

## Actual native voice qualification and lint repair (2026-10-16)

The root's actual PC `npc-world-attempt-2` tested frozen candidate
`16ac7974e877aec3072c511cb629f36ed1b1973f` with the reviewed private mount
`a44be4c081997c6bca4c869df6bcc56cb66a4381b99e2e926890f2801de5c48f` (normal
`mod npc_voice;` only). Evidence is retained in
`/workspace/oteryn-family-b/actual-npc-native-2/summary.json` and adjacent native logs.
That exact old head passed ordinary library type checking, full native library unit-test
compilation, exact seven-case listing, **7 passed / 0 failed / 0 ignored**, and workspace fmt.
The run verified its mounted source unchanged; it did not execute a replacement harness.

Strict library and all-target Clippy both failed with the actual
`clippy::if_same_then_else` finding in `npc_voice.rs:246-249`; overall disposition is
`NOT_QUALIFIED_GATE_FAILURE`. The retained `clippy-lib.log` and `clippy-all-targets.log` are
the failing evidence, not lint passes. No full-package test run or production activation
was claimed.

The lead reopened authoring for this owned repair. The duplicate quiet branches now share
one condition using short-circuit `||`: when the cadence has not elapsed, the chance draw
is still skipped; once due, the same native chance draw decides whether the line is quiet.
No source admission, actor ownership, randomness, cadence state or chat policy is changed.
The seven earlier passes belong only to `16ac7974`. At the repair-author handoff the successor
was uncommitted and required its own freeze and fresh real Cargo/lint evidence; the subsequent
frozen `547557b8` run is recorded below. The repair author ran no PC/Cargo job and made no
commit, push, registration or ref change.

Repair-local checks passed: focused official rustfmt 1.94 and diff checks, governance validation
(22 documents / 9 lanes), and all 124 governance unit tests. These are not successor Cargo or
strict Clippy evidence.

Full-package tests, active content binding/admission, budgeted think/chat integration and
the other actor/travel/shared CP gates above remain outstanding. All original acceptance
boxes remain open.

## Frozen 547 native continuation — actual full-package failure (2026-10-16)

The lead froze the repaired source at `547557b88c25129d7e140c1a879b8b63244f9233`.
For that exact source, ordinary focused **7 passed**, full-package `--no-run`, workspace fmt,
and strict Clippy for both `--lib` and `--all-targets` passed. These results supersede the
old-head lint failure only for this new source; the original failing/interrupted evidence
remains retained.

The earlier full-package runtime interruption remains an incomplete run. SAME547 attempt2
completed its fmt/strict lint gates and stopped before PostgreSQL admission; it is not a
completed full-package runtime. Attempt3 reuses the verified same-source build/proofs and
retains those green gates without repeating them. The actual ordinary full Cargo invocation
under canonical **B + PG** finished with **exit 101**. Its library stage had **3,157 passed /
29 ignored**, but `character_authority_postgres` finished **1,490 passed / 1 failed**:
`postgres_target_aggregation::every_postgres_target_runs_its_cases_in_a_ci_run_target`
reported `npc_world_postgres: includes no support/*_cases.rs file`. The raw evidence is
`/workspace/oteryn-family-b/actual-npc-successor-pg-3/full-package-tests.log`.
**Overall full-package result: FAIL**; the library subtotal and earlier green gates do not
waive this independent aggregation failure or qualify the entire package.

The previously planned
`547557b88c25129d7e140c1a879b8b63244f9233 -> future docs-only head` bridge was superseded by
the material owned fixture repair below. Preserve the original 547 manifest, mount, source
proofs and all interrupted/failed runtime evidence; do not relabel them as execution of a
future successor.

## Owned PostgreSQL fixture placement repair — successor pending (2026-10-16)

The standalone `tests/npc_world_postgres.rs` contained only the private `TEST_SCHEMA`
constant, so Cargo discovered an empty PostgreSQL target that violated the existing
aggregation guard. The unchanged schema string now lives in the existing local
`#[cfg(test)] mod npc_travel_fixture_data` in `durability/npc_travel.rs`; its native unit
consumer keeps the same `use` and `sqlx::query(TEST_SCHEMA)` call. The empty standalone
target is removed. No support/shared file, artificial include, production schema, new
registration or aggregation-guard exception is introduced.

This material successor is **PENDING a new freeze and real native qualification**. No tests,
PC/Cargo execution, commit or push were performed by the repair author. The failed 547
full-package run remains failed; no successor or full-package pass is claimed.

CP admission/current-content binding producer, budgeted NPC scheduler and chat-owner egress
are still missing. Actor/travel/shared integration gates and all original acceptance boxes
remain open; no production activation or full NPC-WORLD delivery is claimed.


## Frozen 9f private mount profiles and local harness repair (2026-10-16)

The lead froze the fixture-placement repair at
`9f2009dd9469733a85194ea6e8a9e31fd0f05179`. The first private profile registered only
`gameplay_transport::npc_voice`: its travel listing contained **0 tests**, because
`durability::npc_travel` was not registered. The zero-inventory guard correctly stopped
qualification; that profile supplies no travel execution evidence.

The second private profile added the ordinary `durability::npc_travel` module registration
alongside voice. Its actual native run completed in 465.27 seconds: workspace fmt, ordinary
library check, library `--no-run` and strict library Clippy passed. The exact seven native
voice cases and the real PostgreSQL travel case passed: **8 passed / 0 failed / 0 ignored**,
with PostgreSQL reporting **170006**. Cleanup completed cleanly.

Strict all-target Clippy and full-package `--no-run` both failed with **exit 101 / E0432**:
`npc_travel.rs` imported `crate::bestiary_postgres_harness`, which is absent from the
`native_admission_source_postgres`, `runtime_scope_assignment_postgres` and
`wp5_s3b_composition` integration crate roots. **Full-package runtime: BLOCKED_COMPILE**;
the focused eight passes do not qualify the package or supersede that failure.

The attempted local-harness successor was frozen at
`1ac9e881799e2c6d77cb6c552bab3dae946dcab7`. It loaded the same canonical support file
inside `npc_travel.rs` and imported it through `super`. Actual strict all-target Clippy
failed with **exit 101 / `clippy::duplicate_mod`**: the same file was also registered in
`src/lib.rs`, `tests/character_authority_postgres.rs` and `tests/durability_postgres.rs`.
The lead stopped its own running native job with SIGTERM through pidfd after a fresh
process-identity proof. Unfinished stages remain incomplete; no full-package runtime PASS
is claimed. The local duplicate-module approach will not be published.

The lead reopened owned authoring and removed only that local module/import change.
`durability/npc_travel.rs` is restored byte-for-byte to tested 9f source SHA256
`fabdc7c83889a533f3f6048fe5204d2d1b52857b5663fba78a64bf3b713100d5`, using
`crate::bestiary_postgres_harness`. The canonical support file and native test body were
never changed. Relative to tested 9f, the resulting net delta is **this task record only**;
the successor commit relative to its 1ac parent contains a Rust reversal plus documentation
and must not be described as a docs-only commit. The earlier native8 passes remain bound
to their exact 9f source/private mount, not relabeled as execution of a future head.

**Required CP shared test-root slot:** add one ordinary crate-root registration of the
existing `tests/support/bestiary_postgres_harness.rs` to each of
`tests/native_admission_source_postgres.rs`,
`tests/runtime_scope_assignment_postgres.rs` and `tests/wp5_s3b_composition.rs`, matching
the existing shared subset-helper allowance. Reuse the existing single registrations in
the library, Character Authority and Durability roots. Both independent targeted reviews
confirmed there is no owned-only single-instance solution across these crate roots; the
lead's refreshed CP state grants no shared slot. No shared root, suppression, skipped test
or alternative harness is introduced here. The tested 9f all-target/full-package compile
blocker therefore remains open pending that slot and fresh qualification.

This owned reversal awaits the lead's freeze. The repair author ran no Cargo/tests/PC job
and made no commit or push. Prior 9f profile results, 1ac diagnostics/controlled stop, 547
failures and older interruptions remain retained under their original source/mount profiles.
CP admission/current-content binding producer, budgeted NPC scheduler, chat-owner egress
and the other actor/travel/shared gates remain missing; all original acceptance boxes remain
open. No full-package, production activation or full NPC-WORLD delivery PASS is claimed.

## D1114 — granted shared test-root registration authoring (2026-10-16)

CP [comment 6222415946](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6222415946)
granted #2157 exclusive authoring of the three missing test-root registration hunks and
the two branch module registrations. The prior no-slot state above is historical. The
lead reopened authoring from `70c85937925fa9d16eb0e5fe26b57e044150b896`.

Applied exactly the accepted proposal: one ordinary canonical
`#[path = "support/bestiary_postgres_harness.rs"] mod bestiary_postgres_harness;` in each
of `tests/native_admission_source_postgres.rs`,
`tests/runtime_scope_assignment_postgres.rs` and `tests/wp5_s3b_composition.rs`. Each new
registration uses only the existing scoped `dead_code` subset-helper allowance/reason.
Existing cases, canonical harness/helper source and existing root registrations are
unchanged. The local duplicate module from 1ac remains absent. The travel global consumer
remains byte-identical to tested 9f SHA256
`fabdc7c83889a533f3f6048fe5204d2d1b52857b5663fba78a64bf3b713100d5`.

Added only `pub(crate) mod npc_travel;` in `durability/mod.rs` and `mod npc_voice;` in
`gameplay_transport/mod.rs`, as CP now permits on this branch. These are ordinary source
registrations of the unchanged native modules, not an overlay, replacement harness or
test exclusion. Final shared module registration must be reapplied in the CP slot after
merging main. This grant does not provide the remaining activation/producer contracts.

**New candidate qualification: PENDING freeze and execution.** Strict all-target Clippy
and full-package `--no-run` must pass first; then repeat the exact native voice7 + genuine
PostgreSQL travel1 on that new head, with nonzero exact inventory and no ignored cases.
Full-package runtime may start only after green compilation and is not passed until actual
completion. Old 9f native8 passes, E0432 failures, 1ac duplicate-module failure/controlled
stop and earlier interrupted/failed runs retain their original source/profile bindings.
The author performed no Cargo/tests/PC execution, commit or push for D1114.

NPC content/current admission (#2129/#2184), NPC-TRAVEL-1, PREM-3, fee/cause writer, physical
relocation/obligation owners, budgeted scheduler, chat-owner egress and all other actor/travel
activation gates remain open. All original acceptance boxes remain open; no full-package,
production activation or full NPC-WORLD delivery PASS is claimed.

### CP D1160/D1122: route travel PG entry to canonical target (2026-10-16)

Status NOT_FULLY_QUALIFIED. CP comment6224912485 identifies the ordinary `--lib` CI
failure: the travel PG entry requires PostgreSQL17.6 in a job without PG. Move its
`#[test]` entry to support/npc_travel_postgres_cases.rs and register it in the existing
character_authority_postgres root using the existing D1114 native harness. Preserve the
complete native exercise in a test-only child fixture of npc_travel (moved physically
to tests/support); the canonical entry calls that exact exercise. It is no longer a
lib test entry. All production functions/privacy and negative fence/rollback assertions
remain unchanged; test-only visibility reaches only the canonical test entry. No cfg(any),
optional early return, skipped body, alternative harness or test weakening is added.

Prior8196b1e7 final native result is retained: all15 stages pass, focused8/8, 55executables
9761passed/0failed/37ignored, doctests37. This routing successor needs fresh exact-head CI
and its new PG entry remains NOT_RUN before the checkpoint. Predecessor GitHub CI is RED
at the old lib PG entry. Format/diff checks and independent review precede the D1122 single
existing-draft push. No old job rerun/restart and no new runner. Remaining runtime admission,
scheduler/chat/travel/fee/position/death callers still require their actual owners/CP slots.

## Travel hold module discovery repair — AUTHORING (2026-10-17)

The current owner reopened AUTHORING from draft
`0faec1fea29f16fe457db58536fbe468cbfdb51a`. CP
[D1160](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224912485)
permits the minimal ordinary module registration for this owned component. This repair
adds only `mod npc_travel;` beside the existing NPC registrations in
`gameplay_transport/mod.rs`. The travel hold implementation and its five existing test
bodies remain byte-identical to that source head.

The missing registration left `gameplay_transport/npc_travel.rs`, including
`npc_travel_tests.rs`, outside the native crate's module graph. The previous focused
voice7 + PostgreSQL travel1 inventory therefore did not exercise the five hold cases.
Ordinary registration makes those existing ambiguity/replay, invocation identity,
current-owner/session/content/position, death-cancellation and invalid-input cases
discoverable by Cargo; discovery and execution still require the successor run.

### Recovered predecessor evidence and successor requirements

The raw log of [Merge gate job 117205955228](https://github.com/Oteryn/Oteryn-Game/actions/runs/38969920168/job/117205955228)
verifies checkout `0faec1fea29f16fe457db58536fbe468cbfdb51a`, successful strict
changed-crate Clippy, native library **3,157 passed / 0 failed / 29 ignored**, and
operations-binary **6 passed**. All seven native voice test names report `ok`.
These results belong to that exact source and do not execute the missing hold module
or the routed `character_authority_postgres` travel entry. No exact-head PostgreSQL
execution for that moved entry was recovered by this review.

This is a material module-graph change. **Status remains NOT_FULLY_QUALIFIED; the new
candidate is not frozen and successor Cargo/Clippy/runtime checks are NOT_RUN.**
After the lead freezes it, verify ordinary library compilation and strict all-target
Clippy, then require a nonzero exact inventory of **5 hold + 7 voice** native library
cases, all executed without ignored cases. Recover any existing same-head PG job
before scheduling the canonical **1 travel PostgreSQL** case. The 12 library cases
and one PG case are separate inventories; no lib job requires PostgreSQL for this
travel entry. Format/diff checks and independent review precede the coherent draft
checkpoint under [D1122](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6224930201).

The remaining fee/cause producer, current content and eligibility, physical placement,
admission/death/position owners, actor movement, budgeted scheduler and chat egress
gates remain open. All original acceptance boxes remain open.

## D1314 — retain verified NPC-LOAD-1 catalogue binding — AUTHORING (2026-10-17)

The owner reopens AUTHORING from draft
`937c260716f0557dd2c978a61152902e8c12af07` for the bounded shared hunk granted by CP
[D1314](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6232733238).
The grant covers `node/serve/bundle_npcs.rs` only in the NPC-LOAD-1 lineage: retain the
already-verified bundle catalogue digest and expose it read-only to the native boot path.

`bind` continues to compare the loaded catalogue's `catalogue_sha256` with the digest
read from the verified World Bundle. Only the successful `NpcBoot::Place` branch now
moves that exact bundle digest into the retained boot state. The read-only accessor
borrows it from that state; absent or skipped boot state returns no digest. Native NPC
boot uses the accessor to record the bound catalogue digest in its diagnostic event.
No new filesystem read, digest calculation, revision substitute or alternate hash domain
is introduced. Production refusal, non-production skipping, NPC admission and placement
behavior are unchanged.

This is the compiler's NPC catalogue-files digest under NPC-LOAD-1. It is not the
canonical World Project tree digest and establishes no membership in `ActiveGeneration`.
D1314 explicitly reserves that membership for a separate CP slot, so
`content/npc_runtime_pin.rs` remains blocked and is not introduced by this change.
The activation caller remains ordered after #2173 under D1159. Scheduler, chat egress,
travel/admission callers and the fee producer still require their respective allocations.

The four existing ordinary `node::serve::bundle_npcs::tests` cases retain their native
bundle writer/reader, binding and Channel placement paths. Added assertions verify the
exact retained digest before and after successful placement; a bound catalogue that
fails NPC admission still carries only catalogue-binding evidence. Missing or mismatched
catalogues, an absent boot and the empty-frame/no-catalogue skip expose no digest. The
existing damaged-bundle refusal and all existing placement assertions remain intact.

**Status remains NOT_FULLY_QUALIFIED.** This material source-retention successor needs
its own frozen head and fresh native validation. At author handoff Cargo compilation,
strict all-target Clippy and execution of these four boot cases are **NOT_RUN**, as are
the successor's hold5 + voice7 and canonical PostgreSQL travel1 checks. Prior CI and
native results retain their original exact source bindings. The lead coordinates the
new explicitly reviewed native/PG recipe required by CP; no historical runner, PG mapping
or old-head test result qualifies this successor. All original acceptance boxes remain
open; no production NPC runtime activation or completed NPC-WORLD delivery is claimed.

## D1340b — native NPC Movement position owner — AUTHORING (2026-10-17)

Return to AUTHORING from `6db40946466ec56379261419177ca010ecba9373` under
[CP D1340b](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233358050).
The two shared files and exact hunks are now included in owned_paths above. The
independently reviewed source patch is SHA256
`3891447cc0529f9e3ee7351dd46ef37a30bb27ec4de60e355a236979d972dc87`.

The existing ChannelActorCarrier Movement read and cardinal-commit methods now
accept an NpcOccupied slot. They use the actual current namespace continuity,
complete ActorRef and native position snapshot, check context and revision, accept
one cardinal step on the same floor, and update position and facing in that same
slot. NPC visibility and occupancy observe the existing native slot immediately.
General relocation, spell, combat, removal, player/creature Movement, placement
and ActiveGeneration remain byte-identical. No second position store or authority
proof is added. NPC-BEHAVIOUR-0 §4.4 requires this Movement owner port; scheduler,
terrain/occupancy eligibility, SPEED timing and the walking caller still need their
concrete consumers before NPC walking is complete.

Six new native regressions cover all cardinal directions and visibility/occupancy,
invalid steps, changed and replayed snapshots, stale/foreign/retired owners,
revision exhaustion and the still-closed combat/removal/general-relocation paths.
They use real native NPC placement; only the exhaustion case injects an otherwise
unreachable slot revision. The seven existing NPC cases are unchanged. After the
new freeze, the exact focused module inventory is thirteen ordinary cases:

```bash
cargo +1.95.0 test --locked -p oteryn-game-server --lib foundation::runtime_actor_carrier::npc_actor::tests:: -- --list
cargo +1.95.0 test --locked -p oteryn-game-server --lib foundation::runtime_actor_carrier::npc_actor::tests::
```

Also qualify the existing channel_actor_position_tests group, strict all-target
Clippy and the original boot4 + hold5 + voice7 library groups. The canonical travel
PG case remains separate. Source review, actual rustfmt and unchanged-scope/apply/
whitespace checks PASS; new-head compile/Clippy/native/PG execution are NOT_RUN at
author handoff. Status remains NOT_FULLY_QUALIFIED and all acceptance boxes stay open.

Predecessor6db ordinary CI passed (3162 library cases,0failed,29ignored;ops6), with
all boot4/hold5/voice7 cases explicitly successful; evidence is
[#2157 comment6233193289](https://github.com/Oteryn/Oteryn-Game/pull/2157#issuecomment-6233193289).
Its D1335 PC attempt stopped before Cargo or PG because the held peer lock had no
JSON state; [attempt evidence6233336481](https://github.com/Oteryn/Oteryn-Game/pull/2157#issuecomment-6233336481).
D1340a replaces that peer-JSON observation route. D1329 requires the existing shared
PG17.6 through in-pg-netns.sh and cargo fetch --locked before the namespace; no PG
server/container lifecycle is part of this work. Any successor run binds its actual
new head and reviewed recipe hash. Predecessor evidence does not qualify this delta.

## Exact20 CI checkpoint and PostgreSQL routing repair — AUTHORING (2026-10-17)

Reopen AUTHORING from `20befdf90f5fa7d127a908101397e96b28d4c967` for this bounded
test-entry and task-card repair. The unchanged native travel exercise passed in
[canonical PostgreSQL job117386230841](https://github.com/Oteryn/Oteryn-Game/actions/runs/39029255995/job/117386230841)
on that exact head with Rust1.95.0 and PostgreSQL17.6: its containing target reports
**1498 passed / 0 failed / 0 ignored**, including the exact
`npc_travel_native_postgres_replay_fences_rollback_and_reload` case. This is real
PostgreSQL evidence for20, not a result from the proposed repair or a PC rerun.

Two remaining CI failures have specific causes: integration-rest job117386230673
reports1497 passed /1 failed because this entry returns an error when the PostgreSQL
admin variable is absent; routing-contract job117386246129 identifies an extra blank
line at the task card's EOF. Remove that blank line and use the existing canonical
`configured_admin()` check in the routed entry. Its absent-configuration branch
prints PRE-ROUTING / NONCANONICAL and returns without entering the PostgreSQL exercise.
That ordinary routing outcome supplies no PostgreSQL qualification. A configured
entry still calls the exact unchanged exercise, including native fixtures, version
and URL checks, all replay/fence/rollback/reload assertions and cleanup. The native
completion marker remains exclusively in that exercise after its assertions; the
routing branch cannot emit it. No test is ignored and no native body is weakened.

Status remains NOT_FULLY_QUALIFIED. This repair needs its own frozen successor and
fresh routing/compile checks; successor native/PG execution is NOT_RUN at author
handoff. The exact20 PostgreSQL result is retained under20 and does not qualify new
source. No PC test was launched to duplicate the completed current-head CI job.
The runtime and unpublished walking work are outside this two-file repair; the
remaining source-pin, terrain/trigger, scheduler, chat and travel activation gates
and the original acceptance boxes remain open.

## NPC-CONTENT-2 voice consumer binding repair — AUTHORING (2026-10-17)

Reopen AUTHORING from `75dc4edafbd7c4402cb1232641b5d47e3b4a5604` for the owned
`gameplay_transport/npc_voice.rs` consumer and its native tests. The actual merged
[NPC-CONTENT-2 producer #2129](https://github.com/Oteryn/Oteryn-Game/pull/2129)
writes voice fields on each NPC definition: Integer `voice_interval_ms`, Integer
`voice_chance_percent`, and Text `voice_lines` containing ordered JSON `{text,yell}`
rows, all under `oteryn:source.npc.`. The old consumer read `Dialogue.voices`
instead, so it did not consume that producer's data and could use dialogue data
when source-held or unsourced voice fields were absent.

The book now resolves each exact admitted NPC reference in the same native
catalogue/model and consumes those three typed fields. Percent is converted
exactly to ppm; line order, text and say/yell mode are retained. Missing,
duplicate or mistyped fields, invalid cadence/chance, malformed JSON and missing
or unknown line members refuse the profile. The existing native text/placeholder
filter still rejects unusable lines. There is no Dialogue fallback, so absence
retains the source hold. This adds no default voice, source regeneration, runtime
activation, scheduler or egress path. The canonical project digest comparison
remains distinct from the compiler NPC catalogue-files digest.

The existing current-owner/actor/context checks, perceived-only interval, retry
and deterministic NPC_VOICE draws, native chat value and player-yell exemption
are byte-identical. Six prior native test bodies remain byte-identical; the
catalogue fixture/test now exercises the real NPC-CONTENT-2 shape and deliberately
different linked Dialogue text. Three added cases cover exact percent/cadence,
Unicode/escaped text and line order/mode, malformed/absent/duplicate/held input,
and absence refusing a linked or unrelated Dialogue fallback. Focused inventory
is now ten ordinary `gameplay_transport::npc_voice::tests::` cases.

The source producer is merged on main at `b3e4fa0dccb37856fbb6f6cd8ce52e2b80de0d16`;
this older task branch still needs normal main composition to acquire its actual
presence fields. A matching data catalogue/model still does not establish active
runtime membership. The real source→runtime binding producer, budgeted NPC think
caller and CHAT_V1 recipient/egress owners remain open; this repair completes only
the demonstrated producer/consumer projection mismatch.

Status remains NOT_FULLY_QUALIFIED. Source/hash, rustfmt, preserved-scope and
apply/whitespace checks precede the draft checkpoint. Successor compilation,
strict Clippy, voice10/hold5/boot4/Movement13 and canonical travel PG execution are
NOT_RUN at author handoff and require their own frozen head. Prior20 PostgreSQL
and earlier voice evidence retain their original source bindings. No build, PC
job or PostgreSQL action is performed by this authoring repair.

## D1344a walking preparation and voice repair — source checkpoint (2026-10-17)

This six-file source packet composes the preceding voice repair with the owned
walking consumer and [CP D1344a](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233730553).
The current-main and exact75 `chat_intent.rs` preimages were independently read;
both are blob `74c8cf66b606eafd8dc2a95f64a9c83748bf281d`. The granted 15-line
`with_npc_customer_state` hunk is patch
`90d94f1cd0e10cc86fbb9fdb093cce31f6012e3b7b173100e5defa254dbd8e8f`.
It holds the actual sessions mutex through a synchronous observation, returns
None on poison, preserves runtime -> spell -> chat ordering and changes no queue
state. The remainder of that shared file is byte-identical. D1160 supplies only
the ordinary `mod npc_actor;` line; the rest of the module root is unchanged.

The walking book resolves each admitted NPC's exact native Behavior reference,
then reads NPC-CONTENT-2 Integer base_speed and Boolean floor_change fields.
Missing, duplicated, held, mistyped, zero-speed or unsupported floor-change data
does not produce a walking profile. The behavior owns the interval and radius.
Each state retains the actual initial Movement position at revision1 as its fixed
radius anchor. Later positions always come from the real native Movement owner.
One perceived think adds1000ms; unperceived thinks pause elapsed ticks and an
actual conversation resets them. Repeating a native occurrence cannot advance
the clock or prepare another proposal. A poisoned conversation owner consumes
neither the occurrence nor its interval.

The due occurrence uses the real deterministic decision root with NPC_WANDER and
bounded Fisher-Yates sampling. Its one cardinal ordering filters coordinate/radius
exits and real native occupancy. A committed intervening Movement change makes
the old proposal stale. Optional terrain preparation checks the full runtime
artifact/frame/map identity, qualified collision and spell-tile observations,
protection-zone boundary, floor changes, teleports and known raised tiles, and
uses the genuine SPEED-1 duration. Absent terrain metadata refuses preparation;
the current bundle's unknown height cannot be treated as a walkable flat tile.

This is **non-committing preparation** as D1344a requires. It provides no movement
permission or replacement position/terrain authority. The active NPC-source
binding, complete current damaging-field and ON_ENTER/ON_LEAVE trigger reads,
budgeted caller, pacing and publication still have to be installed under their
real owners. D1159 allocates the pin leaf to this PR, activation after2173, and
scheduler/egress/travel callers in A2 after2187. Merging NPC-CONTENT-2 supplies
source fields but does not itself establish active runtime membership.

The twelve new ordinary gameplay_transport::npc_actor::tests cases use the native
catalogue/model, actual player/NPC slots, actual conversation queue and genuine
Movement commits. They cover source/hold refusal, cadence/replay, perception and
conversation changes, poison, immutable radius, coordinate edges, retired owner,
deterministic ordering and absent/mismatched qualified terrain. Together with
the voice repair this checkpoint adds15 ordinary cases (walking12 plus voice3);
the six unchanged older voice bodies and all existing Movement/travel cases are
retained. Source-only reviews by mail_market and root, exact hashes, repository
Rust1.94 formatting and patch/whitespace checks pass. No compile or execution is
claimed for this successor; strict Clippy, native test discovery/execution and
PostgreSQL qualification remain NOT_RUN at source freeze. Any required PC run
uses D1342 (the prescribed env, slot target and lock, >=12GiB before Cargo) and
D1329 shared PostgreSQL after cargo fetch --locked. No private PG is started.

The underlying reviewed source patches remain voice
`b3f7d85a6fbc8677a7cc82f0aa7c904e8d4808fa14de37d504624341921276d8`
and walking
`ab062d125c3eeeda75aa8168508cd4a5d01f04f680b22a301829ef04315bb477`;
this combined packet changes only their task-card record. The original acceptance
boxes remain open and the PR stays NOT_FULLY_QUALIFIED.

## Native coordinate type repair after main composition (2026-10-17)

CP merged main `f010472bcb6794a2cbc56dfaf755bd122c10225e` into the presence
checkpoint, producing `c806f4bca0b2e4409d01c377731990cfe95a15e1` with parents
`f8ca704a3a7f54394875f6e165ca9ff8288bb612` and that main head.
Its [strict Rust job](https://github.com/Oteryn/Oteryn-Game/actions/runs/39032203119/job/117395051384)
and [PostgreSQL job](https://github.com/Oteryn/Oteryn-Game/actions/runs/39032203170/job/117394780056)
stopped at E0308 in the new walking consumer: Movement floor is i16, while the
native map LogicalCell z coordinate is i32. The same compile error appeared in
integration-rest and S3-B. No new walking, voice or PostgreSQL case executed on
that head; earlier successful evidence retains its original source identity.

This successor widens that coordinate with `i32::from(position.floor)`. It is an
exact, lossless conversion of every possible native floor and leaves the existing
floor bounds, radius, Movement authority and terrain checks intact. This is the
only Rust change; existing test bodies and registrations remain byte-identical.
Source inspection, repository rustfmt and patch/whitespace checks pass. Successor
Cargo, strict Clippy and native execution are NOT_RUN at source freeze and must
be read from the successor's own CI. The active NPC-source membership, complete
trigger/height producers and actual budgeted caller remain open under D1314 and
D1344a. The PR remains draft and NOT_FULLY_QUALIFIED.

## Walking enum size repair after exact-head strict Clippy (2026-10-17)

On `efbdae23e94cbb94113912eed4e7c6ceeef0a60c`, the
[strict Rust job](https://github.com/Oteryn/Oteryn-Game/actions/runs/39032843180/job/117396866444)
failed on `clippy::large_enum_variant`: `WalkThink::Propose` stores an inline
`NpcWalkProposal` of at least 248 bytes while the other variants carry no data.
This was the only reported strict error at repair intake. S3-B had passed;
integration-rest and PostgreSQL were still running. Those observations belong
to that exact predecessor and do not qualify this successor.

This isolated repair changes that variant to `Box<NpcWalkProposal>`, wraps the
one constructor with `Box::new` and unwraps ownership with `*proposal` in the
existing test helper. The proposal data, authority and terrain checks, interval
and replay behavior, and all twelve walking test bodies remain unchanged. No
lint suppression or new test is added. The earlier lossless floor conversion
remains intact.

The three full preimages are checked against the published head's exact Git
blobs. Repository Rust1.94 formatting and apply/whitespace checks pass at source
freeze. Successor Cargo, strict Clippy and native PostgreSQL execution are
NOT_RUN and require results from its own head; no predecessor result is carried
forward. The active NPC-source membership, complete terrain/trigger producers
and actual budgeted caller still require their accepted integration under
D1314/D1344a. The PR remains draft and NOT_FULLY_QUALIFIED.

## Native entry terrain fixture repair after exact-head CI (2026-10-17)

On `b0cfab87cf7a1e12b83916839c318f81e8b1afaa`, the
[strict Rust and unit job](https://github.com/Oteryn/Oteryn-Game/actions/runs/39034984138/job/117403907839)
compiled and ran the library cases: 3272 passed, one failed and 29 were ignored.
The sole failure was
`npc_actor::tests::a_qualified_collision_cell_without_terrain_metadata_is_never_a_step_approval`.
The canonical PostgreSQL and integration jobs for that head succeeded; their
results remain evidence for that predecessor only.

The accepted native entry source explicitly contains spell tiles, including the
walkable east cell used by this test. Its qualified tile has no source step facts:
`QualifiedSpellTiles::qualify` preserves `source_step: None`. The lookup therefore
correctly succeeds, and the walking consumer correctly omits the unknown-step
candidate. This repair asserts that actual contract: the tile exists, its step
facts are absent, the prepared terrain candidate list is empty, and the original
Movement position remains unchanged. It changes no production terrain admission,
source metadata, fixture content, test registration or other test body.

Exact Git preimages, repository Rust1.94 formatting and isolated patch/whitespace
checks pass at source freeze. Successor Cargo, strict Clippy, unit and native
PostgreSQL execution are NOT_RUN and require results for the published successor.
The D1357 native NPC source producer is a separate work item; D1358 complete
height/trigger admission remains deferred. The PR stays draft and
NOT_FULLY_QUALIFIED.

## Main composition after APP-2 merge (D1385, 2026-10-18)

Normal merge of main `afa97ba0` (APP-2 #2173) into the branch; the merge was
clean with no conflicts. Validation on the merged tree: `cargo fmt --all --check`
pass; strict `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
pass; `cargo test -p oteryn-game-server --lib` 3427 passed, 0 failed, 29 ignored
(npc_travel 5, npc_voice 10, npc_actor 12, runtime_actor_carrier npc_actor 13,
bundle_npcs 4, all passing); the native PostgreSQL travel case passed through
`in-pg-netns.sh` against shared PostgreSQL 17.6. The D1357/D1385 13-file native NPC
source packet is not yet integrated, and D1358 remains deferred. The PR stays
draft and NOT_FULLY_QUALIFIED.

## Completion + wiring (post-checkpoint)

Imported from closed draft #2232 and rebased onto the ADR-0022 layout (`docs/architecture/ADR-0022-gameplay-crate-layering.md`); every existing path below was verified on origin/main `59785d76`. The checkpoint merged unfinished; nothing in the running game calls it. The completion PR must not merge until the mechanic is playable end to end through the real dispatch path, CI is green and Codex review is clean. Dispatch chain: `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (command `else if` chain) and the `GATED` table in `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` (highest capability id on main: 30). Wire modules live in `crates/protocol-oteryn/src/`. Migration numbers: next free number at allocation time (only one migration PR in the queue); 0130 MAP-PICKUP-1, 0131 BAGS-USE-1 and 0132 (provisional TIMED-RT-1c) are reserved.

Structure rules (binding acceptance criteria):
- [ ] One mechanic per module file in its ADR-0022 layer (pure rules L3, PostgreSQL in `durability`, runtime L4, dispatch L5).
- [ ] New tests in their own child files, not appended to existing files.
- [ ] Edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] No file over ~1000 lines grows by more than ~150 lines (`connection.rs` is ~8,260 lines).

### Present on main (#2157)
- Travel: `crates/gameplay/npc/src/npc_travel.rs` (`TravelHoldSlot`, fee/placement/consume state machine; dispatch side `crates/gameplay/dispatch/src/gameplay_transport/npc_travel.rs`) and `crates/gameplay/durability/src/durability/npc_travel.rs` (`read_npc_pending_arrival`, `consume_npc_pending_arrival_placed`). Walking: `crates/gameplay/npc/src/npc_actor.rs` (`NpcWalkBook`, `NpcWalkState`, proposals), placements in `crates/gameplay/entities/src/carrier/npc_actor.rs`. Voices: `crates/gameplay/dispatch/src/gameplay_transport/npc_voice.rs` (`NpcVoiceBook`, `NpcVoiceState`, `chat_line`) and `crates/gameplay/npc/src/npc_voice.rs`. `apps/game-server/src/node/serve/bundle_npcs.rs` only retains and logs the verified catalogue digest.
- `NpcWalkBook`, `NpcVoiceBook`, `NpcVoiceState`, `NpcWalkState` and `TravelHoldSlot` have no callers outside their own files and tests. No migration for the pending-arrival table; the PG fixture `apps/game-server/src/lifted_tests/durability_npc_travel_postgres_fixture.rs` (cases `apps/game-server/tests/support/npc_travel_postgres_cases.rs`) installs it privately.

### Checklist
- [ ] **Migration**: NPC pending-arrival / travel occurrence tables with privileges — next free number at allocation time (only one migration PR in the queue).
- [ ] **Travel (NPC-TRAVEL-2)**: dispatch from the NPC trade/talk path (`crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs`, `npc_trade_command.rs`, `npc_talk.rs`) to `TravelHoldSlot::start`; fee debit through the Bank/gold path; destination placement via the Character position writer (`crates/gameplay/housing/src/last_position.rs`, dispatch `last_position.rs`); hold cancellation on death/logout/disconnect (`gameplay_transport/logout.rs`, `resume.rs`); arrival consume after restart; premium gate; destination list/confirm messages (extend `crates/protocol-oteryn/src/npc_service.rs` or add a capability, then `GATED` in `capabilities.rs`).
- [ ] **Walking actors (NPC-ACTOR-1)**: build `NpcWalkBook::from_catalogue` at content activation (`activate_content` in `apps/game-server/src/node/serve.rs` and `bundle_npcs.rs`), own `NpcWalkState` per placed NPC in the actor carrier (`crates/gameplay/entities/src/carrier.rs`), tick it from a scheduler like `crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs`, apply steps through `gameplay_transport/actor_movement.rs` and publish positions via `crates/protocol-oteryn/src/world_spatial_entities.rs`. D1357 native NPC source packet and D1358 height/trigger admission are not integrated.
- [ ] **Voices (NPC-VOICE-1)**: tick `NpcVoiceState::think` and emit `ChatLine` through the chat relay (`gameplay_transport/chat_intent.rs`, `chat_relay.rs`) to listeners in range.
- [ ] **Talk book install**: confirm the production path calls `install_npc_talk` (`gameplay_transport/chat_intent.rs`); on main the visible call sites are tests.
- [ ] **Content**: TRAVEL-CONTENT-1 destinations/fees, NPC-CONTENT-2 voice and movement facts (merged #2129); regenerate content and pins.
- [ ] **Tests**: unit tests exist (travel 5, voice 10, actor 12); add PG test for pending arrival after restart in the real harness; **end-to-end through `connection.rs` dispatch**: talk to travel NPC, confirm destination, fee debited, character arrives, restart mid-hold resumes once; a walking NPC changes position in a spatial snapshot; a voice line reaches a nearby client.

### Blockers and dependencies (re-verified on origin/main `59785d76`)
- CREATURE-MOVE-1 (chase/step composition): re-implementation pending (draft #2184 closing; card `docs/agents/tasks/archive/CREATURE-MOVE-1.md`).
- TRAVEL-CONTENT-1: BLOCKING (content draft #2235 open). D1357/D1358 native NPC source work: open.
- MERGED: ADMIT-CAP-1 (#2187), NPC-CONTENT-2 (#2129), NPC-ECON-1 (#2197); NPC-TRAVEL-1, CHAT-1 and PREMIUM-1 merged earlier.
