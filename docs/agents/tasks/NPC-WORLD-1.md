# NPC-WORLD-1 — NPC world: travel, walking actors and voices (skeleton task record)

```yaml
task_id: NPC-WORLD-1
title: "NPC-WORLD-1: NPC world: travel, walking actors and voices"
mode: IMPLEMENT
status: skeleton
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
- `apps/game-server/src/gameplay_transport/npc_travel.rs`
- `apps/game-server/src/gameplay_transport/npc_travel_tests.rs`
- `apps/game-server/src/gameplay_transport/npc_actor.rs`
- `apps/game-server/src/gameplay_transport/npc_actor_tests.rs`
- `apps/game-server/src/gameplay_transport/npc_voice.rs`
- `apps/game-server/src/durability/npc_travel.rs`
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
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

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
The seven earlier passes belong only to `16ac7974`; this uncommitted successor requires its
own freeze and fresh real Cargo/lint evidence. The repair author ran no PC/Cargo job and
made no commit, push, registration or ref change.

Repair-local checks passed: focused official rustfmt 1.94 and diff checks, governance validation
(22 documents / 9 lanes), and all 124 governance unit tests. These are not successor Cargo or
strict Clippy evidence.

Full-package tests, active content binding/admission, budgeted think/chat integration and
the other actor/travel/shared CP gates above remain outstanding. All original acceptance
boxes remain open.
