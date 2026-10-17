# HOUSE-RUNTIME-1c — house position persistence

```yaml
task_id: HOUSE-RUNTIME-1c
title: "HOUSE-RUNTIME-1c: house position persistence"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 3
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 row HOUSE-RUNTIME-1c; §7 'HOUSE-RUNTIME-1a/1b/1c'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md
base_branch: main
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
branch: skeleton/house-runtime-1c
needs_migration: true
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## 2026-10-17 D1366 NON-PG correction — current checkpoint

The predecessor of this correction is the published and fully read-back source checkpoint
`ca7e1c0ffea4a3c3ea27f63bc866770663ac04de`, tree
`70796da031a4d4e2b73c6c17c0bccc9777d93b78`, sole parent
`2a3af11041a8285d4572e935feff04bcb90d8725`.

[CP D1366](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234335395)
directs these eight cases to use the mixed-target repository convention. The common
`run_house_position_case` now returns before runtime or harness construction when
`configured_admin()` reports missing/non-Unicode `OTERYN_TEST_POSTGRES_ADMIN_URL`.
That existing helper emits **PRE-ROUTING / NONCANONICAL**. This route executes no PG
case body and creates no database; it must never be reported as PostgreSQL qualification.
With a configured URL, all eight native case bodies, version enforcement, canonical mount,
assertions, panic/error cleanup and post-cleanup `QUALIFIED PostgreSQL17.6` markers are
unchanged. The complete source suffix beginning at runtime construction is byte-identical
to the predecessor. The same successor removes one extra EOF blank line from Cargo,
leaving `test = false` and exactly one newline.

D1366 explicitly declines the four-file workflow/policy proposal and its Stage A gate-pin
rotation. No workflow, policy, planner, protected audit or canonical PG command changes are
included. The earlier proposal is not an allocated implementation or validation result.
The two owned source edits and this card form one coherent successor. Merge/activation
order remains **#2181 → #2166 → #2187**; this correction does not open any runtime gate.

### Validation identities and limits

The **previous exact head ca7e1c0f** was fetched into a clean native worktree on Molehill-PC
and qualified on 2026-10-17 at 12:59:54 UTC: compile/link 1m55s, eight PG 17.6 cases in
3.22s, **8 passed / 0 failed / 0 ignored**, and eight markers after owned-database removal.
Source hashes and clean Git state were unchanged. The locked fetch and D1329 wrapper
route passed in D1342 slot C. Native log SHA256:
`61db8386db989250d8e2b17dfb69e18242a42ee4d7eb1091349678083b679bf8`.
Before/after nine-file manifest SHA256:
`9446a816408e6d67fbf6b71a044004aae76b3330588e77291478f71aae787a7c`.
This preserves that exact-head receipt; it is not inherited by the D1366 successor.

The predecessor also passed the [canonical GitHub PostgreSQL job 117401720983](https://github.com/Oteryn/Oteryn-Game/actions/runs/39034578891/job/117401720983),
recorded in [the CP receipt](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234339125).
The whole registered targets passed: `durability_postgres` 1274 cases in 131.82s,
`character_authority_postgres` 1580 in 294.84s, `runtime_scope_assignment_postgres` 1151
in 40.82s, and `native_admission_source_postgres` 1143 in 32.09s. All eight House-position
cases executed successfully in the first target. This result belongs to ca7e1c0f;
the contemporaneous NON-PG integration-rest failure is the routing defect D1366 repairs.

| D1366 candidate check | Disposition before publication |
| --- | --- |
| Configured-PG branch preservation | Static byte comparison PASS: complete runtime/harness/case/cleanup suffix unchanged from ca7e1c0f; eight call sites still use the common runner. |
| Cargo EOF | Static PASS: exactly one final newline; no manifest semantics changed. |
| Controlled envless native execution | PENDING: require exactly eight PRE-ROUTING / NONCANONICAL messages, no QUALIFIED markers and no database construction. This run is never PG qualification. |
| Configured PG 17.6 on the new source/head | PENDING: exact native source/head receipt required after formatting/publication. |
| Exact new head and three-file readback | PENDING: root publishes one sole-parent successor and records its independently read-back identity. |
| Full House runtime | NOT_FULLY_QUALIFIED: native placement, dirty capture, final House terminal write and ADMIT/lifecycle integration remain as described below. |

Candidate source SHA256 values before native formatting:
`support/house_runtime_position_postgres_cases.rs` =
`f521f41d01008d6c2e69704cc39df03704d7ae7212ff1841e2a902fa786d042d`;
`apps/game-server/Cargo.toml` =
`559664be4f312bfed1fdae7b6683602af2ab67888c0604e21f32c4568204067b`.
The parent publishes the final source manifest and native receipts after this authoring
checkpoint. Earlier missing-configuration refusals and main-based assembly attempts below
are historical records; D1366 supersedes their NON-PG routing disposition only.


## Historical ca7e1c0f integrated storage checkpoint — source assembly record

CP merged main `f010472bcb6794a2cbc56dfaf755bd122c10225e` into this draft by an ordinary
merge. The verified predecessor is `2a3af11041a8285d4572e935feff04bcb90d8725`; its only
delta from that main is the existing House writer, focused test source and this card.
The checkpoint composes the existing native House storage component with the real
Channel writers and canonical migration/test registration. Full House runtime acceptance
still depends on #2165 and the admitted placement/exit owners described below.

### Concrete implementation in this checkpoint

- `0108_character_position_house.sql` adds `house_world_id UUID NULL` and `house_key TEXT
  NULL` to the existing `game_character_last_positions` row. A single CHECK enforces
  joint nullability, same World and the existing native House-key grammar. Old rows remain
  null. The schema fragment is removed from `character_position_house.rs`.
- The existing native House writer/reader retains its recovery/session/connection/lease,
  exact House assignment/incarnation and canonical guard-history checks. The stored
  discriminator is historical position data; it does not grant House admission or placement.
- `character_last_position::upsert_last_position` clears **both** House columns in the
  same winning INSERT/UPDATE as cell, pins and vitals. Its strict `(lease, sequence)`
  comparison and pending-respawn veto are unchanged. Equal/lower keys, invalid or stale
  fences and transaction rollback preserve the entire previous row.
- Actual native single and batch periodic writers already call that common upsert.
  `consume_pending_respawn_placed` and `place_and_consume_pending_respawn_placed` also
  call it inside the transaction that consumes the pending respawn. Their behavior is
  composed through the common helper; no separate respawn writer or actor-death hook is added.
- `durability/mod.rs` mounts the House component. The focused PG file becomes a thin
  wrapper marked `test=false` in Cargo; its eight native cases are mounted once under
  `tests/durability_postgres.rs` through `support/house_runtime_position_postgres_cases.rs`.
  The old `cfg(any())` disabled target is removed. Missing PG configuration fails explicitly.
- The original four House cases now require SQLx migration 0108's exact installed checksum,
  rather than installing an ad hoc schema string. Four additional cases invoke the actual
  native single, batch, admission-respawn and in-session-respawn APIs. They check same-cell
  clearing, complete changed captures, equal/lower/newer-lease ordering, stale/invalid
  inputs, pending veto, deferred COMMIT rollback and replay without a second mutation.
- All eight cases create their harness outside an unwind catcher, then clean up their own
  database on error or assertion panic before preserving the result/panic. Qualification
  output is emitted only after case success and successful owned-database cleanup.

### Current native validation disposition

The candidate was assembled from exact main plus these owned deltas on Molehill-PC at
`/root/oteryn-house-bed-20261017/position-2166`, under build slot C. `cargo fetch --locked`
completed successfully before the shared-PG wrapper. Native rustfmt output was applied
without semantic changes. Attempt 1 at 12:46:16 UTC stopped because the source checkout
was missing required tracked inputs; it is not a source compiler or test failure. After
restoring the complete tracked checkout, attempt 2 started at 12:49:09 UTC on 2026-10-17;
its locked fetch and compilation passed. The canonical filter executed exactly eight
PostgreSQL 17.6 cases: the four original cases passed and all four new caller cases failed
with SQLSTATE 23514 from the existing monotonic last-position guard. No case was skipped
or ignored, and the wrappers completed owned-database cleanup before returning errors.

The common cause was the new historical-pair fixture changing the row at the same key,
before the intended caller assertions. The focused repair leaves every production guard
unchanged: the fixture now advances `write_sequence` and returns the actual stored key;
equal/lower/winning and respawn/replay test keys are derived from it. The repaired source
was formatted with native Rust 1.94, and attempt 3 started at 12:54:08 UTC in slot C
after another successful locked fetch. Attempt 3 compiled in 34.56 seconds and passed
all eight native PostgreSQL 17.6 cases in 3.13 seconds: zero failed, zero ignored, and
eight qualification markers emitted after successful owned-database removal. Source
hashes before and after the run were unchanged. Attempt 2 remains a test-fixture RED,
not evidence of a production-writer defect.

The exact qualifying command, executed through D1329 `in-pg-netns.sh`, was:

```bash
cargo test --locked -p oteryn-game-server --test durability_postgres house_runtime_position_postgres_cases:: -- --nocapture
```

The staged main-based source assembly has support-file SHA-256
`92c9b5f791cc94243cc9ed1520b135469df04c2ce06d0b686b95638754e8d226`;
the D1342 build environment SHA-256 is
`afc2f04924c412b5965a1c4f6a2b30dfcd9e80ac0a185af737d070ebedbefd82`.
This is a selected-target source-assembly PASS. Publication must still read back the
new exact head and all nine changed files and bind that identity to the unchanged
qualifying source hashes; the publication receipt records that verification.

| Check | Current checkpoint disposition |
|---|---|
| Changed Rust formatting | PASS; focused fixture repair formatted by native Rust 1.94. |
| Canonical compiler/link and selected eight PG cases | Attempt 3 source assembly PASS: 8 passed, 0 failed/ignored, 8 post-cleanup markers; unchanged before/after hashes. Exact published-head readback/requalification remains required. |
| Native current-authority guard fixtures | Explicit synthetic independent current data for the four storage cases; they do not qualify successful sealed House claim rebind or physical placement. |
| Real in-session respawn callback | A placement-acknowledgement fixture, with the actual native durable API and transaction. It does not qualify a runtime controller. |
| House logout/resume/fallback, final House terminal capture, ADMIT, actual placement and runtime lifetime | NOT_RUN / incomplete. |

The source manifest records every staged file hash and exact main/PR predecessors.
Historical selected compiler/PG results later in this card keep their original source
scope and are not substituted for this eight-case integrated candidate.

### CP scope, registry and publication order

[D1347](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418)
reconfirms reserved migration **0108** and grants the common same-row writer, owned House
component, native cases and own module lines. The migrations directory is the only
migration registry. [D1353](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234008098)
grants the support leaf, thin wrapper, own Cargo `[[test]]` entry and line-local canonical
aggregate include. No separate registry row or new migration number is required.

[D1361](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234053855)
permits the **complete compiling checkpoint now**, including that aggregate line. The
order **#2181 → #2166 → #2187** governs merge/activation, not the draft push. After #2181
merges, merge current main ordinarily and resolve only the owned aggregate include.
The root is the single writer; D1345/D1359 require one successor of a freshly read head,
complete explicit file set, fast-forward ref update, immediate ref/file readback and
requalification. Ready and merge remain CP actions.

### Remaining runtime work and next worker boundary

The two-column storage and common Channel clearing do not provide the native House
actor's dirty capture/sequence owner, logout/final House write, login placement, removed
House entrance metadata, access-loss fallback or reverse-handoff destination transaction.
Those actual owner integrations remain required. A final House capture must be written
inside the real terminal/handoff transaction while its authority is held; a write after
release or actor removal cannot replace it. Shared respawn/terminal owner changes retain
their recorded CP order; this checkpoint changes only the common storage helper.

#2165 now owns the missing additive House scope-transition authority variant, sealed
claim predicate, same-TX current predecessor-history validation and persisted successor
under [D1362](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234053855).
Its shared admission/lifecycle hunks and ADMIT installation follow #2187. Native placement
and reservation consumption must precede bind/generation advance. The position component
must consume those genuine current owners and never turn a saved House row, a load snapshot
or the synthetic test guard installer into permission.

D1329 requires shared PostgreSQL 17.6 only, locked fetch before `in-pg-netns.sh`, and owned
database cleanup; no server/container lifecycle is part of these tests. All original
runtime acceptance items remain open until their actual producers and native integration
evidence exist. The older D1122 and code-archive sections below are historical records;
their unallocated-migration/unmounted-writer descriptions are superseded by this checkpoint.


## Historical 2026-10-16 D1122 checkpoint — superseded disposition

The product storage component remains byte-identical to the existing draft head
`2d344f427eccbda584475c01917bad2eded5f271`. Source successor
`a5485e3359d94d2ec850d7f6ec2982300b13ecd4` repairs all four original PG entry points:
missing/non-Unicode database configuration returns an error, never a skipped PASS.
The four repaired cases have not been compiled or executed on this generation.
Historical native/compiler/PG results later in this card retain their historical
source scope; they do not qualify the changed test source or integrated runtime.

| Check | Current-source disposition |
|---|---|
| Independent focused source review | PASS; only four missing-configuration refusal branches changed; existing PG17.6 and isolated-database guards retained. |
| Changed Rust formatting and complete delta whitespace | PASS, official Rust1.94, edition2024, skip_children; exact receipt `/workspace/house-family-handoff/checkpoint-publication-policy/2166-exact-fmt.json`. |
| Current-source native compile/Clippy, LIST and all four PG cases | NOT_RUN. |
| Production single/batch/upsert/respawn/terminal callers, placement/rebind and ADMIT integration | NOT_RUN / incomplete. |

[CP D1168](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6225337534)
assigns #2166 the actual single/batch/upsert callers in
`durability/character_last_position.rs`, respawn in
`foundation/runtime_actor_death.rs` and terminal release after #2187. Both House
columns must clear in the same native upsert as cell and vitals. These callers
are not implemented by this checkpoint. The migration number has been requested
in #2131; the existing owned schema fragment remains uninstalled. Loader and
placement/rebind belong to #2163/#2165. Shared hunks retain the CP merge order;
no merge/rebase is performed under the current owner instruction.

D1122 permits this one coherent checkpoint before full qualification. The draft
remains NOT_FULLY_QUALIFIED; no ready action, review trigger or manual CI restart.
All original acceptance items remain open. Required full checks are still pending.

## Goal

A Character's position inside a house persists through logout/restart via two nullable house columns on the position record (CHAR-POSITION-1).

Children (allocate from the brief's child graph; the plan row is only the family entry gate; no child of a CANDIDATE brief is allocated before the brief is ACCEPTED):

Third of HOUSE-RUNTIME-1a -> 1b -> 1c.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| HOUSE-RUNTIME-1b | BLOCKING (not merged) |
| CHAR-POSITION-1 | MERGED (#2006) |

Rows marked BLOCKING must merge (or be accepted/decided) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/durability/character_position_house.rs`
- `apps/game-server/tests/house_runtime_position_*.rs`
- `docs/agents/evidence/HOUSE-RUNTIME-1c-*.md`
- `docs/agents/tasks/HOUSE-RUNTIME-1c.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/NNNN_character_position_house.sql (two nullable house columns)
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

- CHAR-POSITION-1 position writer (#2006)
- house scope runtime from 1b

## Inseparable bundles (never split across PRs)

- migration + position writer + login fallback tests

## Acceptance criteria

- [ ] Logout inside a house restores the Character inside that house.
- [ ] If the house is gone or access lost, login falls back to the house exit deterministically.
- [ ] Columns are nullable; old rows unaffected.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server house_runtime
cargo test -p oteryn-game-server character_position
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_HOUSE_RUNTIME0_HOUSE_INTERIOR_RUNTIME_DECISION_2026-09-30.md
- docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md §2.3
- PR #2006
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 3), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Bounded native position integration packet — 2026-10-15

### Exact lease and source floor

This pass is **CARD_ONLY / REPORT_ONLY_NATIVE_INTEGRATION_PACKET**. Root's
`house-runtime-1c-allocation.json` permits only this task record on
`skeleton/house-runtime-1c`, unchanged HEAD
`bd691cc1e4cf47e51e336fc22b80cdc21f153961`, existing draft #2166.
It supersedes the historical broader Rust/evidence/module-registration permission
above. Preserve the plan row, inseparable migration/writer/fallback bundle, CP-only
slot rules and every unchecked acceptance criterion. No implementation, registration,
DDL, content, protocol, commit or worker publication is authorized by this packet.

Read accepted HOUSE_RUNTIME0, HOUSE-RT-INBOX §2.3, CHAR_POSITION0 and CHAR_VITALS0
at protected main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.
The historical CHAR_POSITION CANDIDATE references in older packets do not override
its protected acceptance. Prepared HOUSE-RUNTIME-1a and 1b are **not merged runtime
owners or completed acceptance**. The concrete same-row design below is proposed
implementing detail for CP/independent review, not a newly accepted storage/API contract.
Exact floor/main/source hashes and selected current-main consumer extracts are in
`/workspace/oteryn-prep/house-runtime-1c-evidence/`. No merge/rebase was performed.

### Native residue: extend these owners

| Native source / symbol | Existing behavior; required house delta |
|---|---|
| `0080_character_last_position.sql` | One `game_character_last_positions` row per Character, immutable Character/World, monotonic `(character_lease_generation, write_sequence)`, Character-World guard, no deletion/truncate, native runtime grants. Extend this row and preserve all guards; do not create a second house-position store. |
| `0084_character_last_vitals.sql` | Nullable health/mana pair on that same row; both null or both bounded, saved health positive. Cell, House discriminator and vitals must remain one capture/upsert/key, never separate partial updates. |
| `durability/character_last_position.rs::{LastPositionRecord, StoredLastPosition, PeriodicLastPosition, FinalLastPosition}` | Cell, World, frame/map/artifact markers and optional living vitals, no House field. `LastPositionKey` is the native lease/write order, not CharacterRevision. |
| same module::{`validate_fence`, `read_character_last_position`, `write_character_last_position`, `write_character_last_positions`, `upsert_last_position`, `load_last_position`, `decode_row`} | Actual single/batch/read paths and common upsert already exist. Current fence/input paths admit only Channel scope. Common upsert refuses pending respawn and lower/equal key; preserve typed Written/Superseded/PendingObligation. No cause, receipt, replay record or root revision advance belongs to this projection. |
| `character_progression.rs::assert_session_fence` | Current rule-2 session/assignment/incarnation/admission fence without root lock, explicitly Channel-only. Do not globally loosen this shared item/progression function to admit every Instance; house-aware position authority needs the real house admission/session owner. |
| `gameplay_transport/last_position.rs::{LastPositions, Saved, capture_last_position, placed_record}` | One capture under runtime then spell-state locks, monotonic sequence allocated under owner lock, dead actor excluded, dirty tracking currently cell+vitals. Same coordinates/vitals in different House/outside contexts must become dirty; add the House discriminator to the existing signature. |
| same module::{`read_admission_position`, `AdmissionPosition`, `first_entry_input`, `initialize_admitted_position`, `track_restored`, `admitted_vitals`} | New actor: pending respawn before saved row, then current Channel placement/fallback; saved house cells currently go to entry_start. Failed read leaves actor unplayable/untracked. Saved same-World vitals survive even a position fallback, then cap at current maxima; no house fallback heal. |
| `FreshAdmissionStore::with_final_position`, `fresh_admission.rs::write_final_position` | Final capture is written inside terminal release after locked validation and before session_state3. Writer currently skips non-Channel. Extend the real house branch in this transaction, not a write after actor removal/release. |
| current `gameplay_transport/mod.rs` release/death/logout consumers | Main differs from this old skeleton: current `settle_released_death`, terminal-step and `logout`/`release_logout_session` handling must be preserved. Settle real pending death/mutations first, then final capture/release; retain actor/controller while durable result is unresolved. |
| `character_death.rs::{consume_pending_respawn_placed, place_and_consume_pending_respawn_placed}` | Actual consumed-respawn placement writes via the same native upsert, with no saved vitals (maxima). Explicit outside/house-clear state must accompany that placed respawn; a dead old actor cannot overwrite it afterward. |
| `house_scope_handoff.rs::HouseId`, migration0074 | Real House identity is World+canonical key, tagged02 scope key; derived Instance ID is runtime identity. Sessions also retain runtime_scope_house_key and origin_channel_id. Neither a generic Instance nor a saved House key proves current admission, placement/access or property validity. |
| same module / `house_scope::{exit_into_channel, handoff::select_entry_tile}` | Durable entry/assignment/reservation/reconcile primitives exist; ExitNotAdmitted remains real. Pure tile selection is not a placement/access token. Entry currently requires the proven node holding origin Channel and House; remote placement needs its actual owner seam. |
| `world_house_instance.rs::read_current_world_house_admission_in_transaction` | Actual identity observation binds current Active session and control-owned World/Instance/House/content placement. It deliberately is not physical HousePresenceProof; its Active/current-transport checks also do not cover reconnectable terminal releases. Do not relabel it a universal position-write token. |
| `content/project/native_house_tiles.rs`, `spell/house_execution.rs::current_house_presence` | Qualified placement carries House identity/cells and a walkable entry_cell **outside** those cells; omission is Unknown. Channel carrier still returns HouseInteriorOwnerUnavailable. Bundle generic house-tile membership is not a HouseId→interior/entrance catalogue or World-global runtime. |

### Proposed same-row shape and minimal future files

Keep the existing primary key, World/cell, coordinate frame, map/artifact markers,
vitals and native order key. Proposed Rust extension is
`LastPositionRecord.house: Option<HouseId>`: None is an outside Channel position;
Some is that exact House's interior cell in the existing native cell fields.
The two proposed nullable physical fields are `house_world_id UUID` and
`house_key TEXT`; these are private design names, not names mandated by the brief.
They encode the full native HouseId rather than a generic Instance ID. Enforce:

- both null or both non-null, including explicit SQL null checks;
- house_world_id equals the row World and the Character root World;
- existing `game_house_scope_key_valid` / `HouseId::new` canonical key semantics,
  reusing native World decoding and native key validation, not a second grammar;
- all existing cell/digest/vitals/order bounds and immutable Character/World guards.

Existing rows decode with None, without backfill or reinterpretation. Half-null,
malformed key/World or cross-World House data is invalid stored state, never an
outside position silently manufactured by a decoder. Do not FK-cascade the saved
House discriminator away when a house is removed: that would erase the information
needed for the accepted deterministic rejected-house fallback. A current property
existence/access decision belongs at admission, not in a historical-row decoder.

`HouseId` owns a String and is not Copy. Adapting LastPositionRecord and containing
record/capture/Saved types must use actual Clone/ownership changes in their native
callers; no truncated key/hash or fabricated copyable House ID. The position row
stores no new origin Channel: fresh house login records admission's chosen Channel
in the real house session/handoff; a live exit first targets that session's recorded
origin, using ordinary same-World Channel selection only when unavailable/draining.

Future owned `durability/character_position_house.rs` may contain only the native
row-field adapter (`decode_house_fields`, proposed name) and genuine house branch
consumer support once its concrete authority/transaction inputs exist. The actual
read/upsert and capture remain in character_last_position.rs/last_position.rs; do
not implement a parallel loader, writer, order allocator, house session, position
receipt or generic authority wrapper. Reuse the existing native reader/writer/tests.
Future `tests/house_runtime_position_postgres.rs` and focused native owner tests
qualify the composed path after exact allocation. These names are proposals only.

### Capture and write sequence

1. Actual 1b owner captures the current living actor's cell, exact HouseId,
   admitted session/actor, owner/scope generation, active native coordinate context
   and vitals together under its existing owner-lock order. Do not build HouseId
   from input bytes, map membership or Instance alone. No Channel carrier may save
   a supposed World-global house merely because it stands on a house-marked tile.
2. Reuse `LastPositions` sequencing/tracking and periodic cadence/batch limits
   (CHARPOS0-RL-01, native batch64), including due/settled/failed retry and shutdown
   flush. Include House in Saved/dirty identity; a scope transition with equal cell
   and vitals still changes projection. Do not allocate a second sequence per House,
   zero/reset sequence, copied source-lease key or wrapped sequence on overflow.
3. Same native semantic transaction/recovery fence/admission relation locking precedes
   the upsert. The genuine position-specific house branch must independently check
   current session/connection/Character lease, named House and exact assigned Instance,
   current node incarnation/scope generation, admission guards and physical owner.
   Historical record, matching key/digest or retained handoff is expected binding,
   never enough current proof. Arbitrary other Instance scopes retain the skip rule.
4. The existing upsert atomically inserts/replaces all fields, including setting
   both house fields to null on an outside write. The greater native lease/write key
   wins; lower/equal writes alter neither House/cell/pins nor vitals. Preserve no root
   lock/CharacterRevision/cause/receipt for this runtime projection. A new mandatory
   shared root lock or family fence weakening would require owning review, not this card.
5. Final logout/terminal capture uses the same tracker/record/key. Extend the locked
   `write_final_position` house branch in actual terminal release before state3;
   preserve current exact transport/control-loss/death settlement and durable replay
   disposition. Never append an after-release write. A disconnect while logout-blocked
   retains the actor; the eventual genuine terminal release writes its final position.
   A lost response is reconciled by the existing lifecycle owner, not a new receipt.
6. Do not capture dead actors. Pending respawn keeps precedence and its placement
   transaction clears House state while writing the placed outside cell and maxima
   semantics, under the destination/new lease key. Accepted pending arrival similarly
   outranks saved position when its real owner lands; current native position path
   has no complete arrival consumer, so this packet does not simulate one or erase
   an unresolved obligation. Preserve both-obligations integrity failure and no-write
   rules at the actual future composed boundary.

### Load, still-present resume and recovery

Read the existing row only when a **new runtime actor** is required, under the real
admission boundary. Existing same-session resume of a still-present Channel actor
reads/moves nothing. House sessions are different: current native handoff explicitly
requires fresh entry rather than replacing a named house session; the actual owner
must reconcile that transition, not run the Channel `resume_lost` path in an Instance.
A stored position never reinstalls an old controller/session or infers a new lease.

For a new actor, resolve pending respawn (then actual pending arrival when integrated)
before the saved row. None-house rows retain native current same-World/frame,
walkability/door/dynamic-item/occupancy checks and ordinary fallback; a Channel reader
must not suddenly accept ordinary house-marked map tiles. Some-house rows branch to
the real 1b/SCOPE admission owner **before** Channel initialization/playable snapshot.
Do not position a temporary Channel copy inside the house, publish entry_start, then
teleport later; the qualified final owner must decide placement first.

A valid saved HouseId is looked up against current allocated House identity, active
qualified placement/content and current owner assignment. Under the actual admission
commit, `game_house_access` re-reads property state/ACL/guild membership revisions
with its native locks. A stub/no-row result grants no access. Neither prepared 1a
policy roles nor 1b in-memory reservations replace this function's authority.
The house owner selects/reserves the saved tile only if it belongs to that House,
is active-frame walkable, item-unblocked and unoccupied. Revalidate reservation at
commit; if changed, repeat the accepted selection before commit rather than publish
stale placement. Saved map/artifact digests are diagnostic; equality skips no checks.
A changed frame requires the actual qualified mapping/rejection, never a raw cast.

Use the existing native same-floor Chebyshev radius3 fallback in its actual ring
order (north first, clockwise, checked coordinate arithmetic), constrained to this
same House. Reuse its qualified selection semantics, not a new pathfinder or arbitrary
byte ordering. Do not let a generic Channel entry_start escape the same-house search.
No candidate or the accepted200-character limit rejects house placement and enters
the outside fallback path. Only the real owner/handoff reserves and commits placement;
pure select_entry_tile and prepared opaque spatial_position bytes do not prove cells.
1a's opaque HouseInterior endpoint bytes need the owning typed map/cell adapter,
not a new serialization invented by 1c.

On successful house login, the chosen admission Channel becomes origin and the
current destination House session/actor is installed through the actual durable
owner. Only then track periodic/final writes under its own lease/generation. Preserve
saved same-World vitals and clamp at actual new maxima; first login/absent vitals or
respawn uses existing maxima policy. An interior fallback alone does not heal,
advance CharacterRevision or rewrite inventory. A failed storage/fence read is an
unplayable admission with no tracking/write, not a reason to erase House state.

### Every exit and rejected-house fallback must commit the actual outside row

Door exit, leave, kick, access/guild revocation, disposition, shutdown, rejected
saved-house login, and unrecoverable-house fallback use the **real reverse handoff /
fresh admission transaction**, once that owner is qualified. It must atomically:

- resolve the destination Channel from recorded origin first, or normal same-World
  selection when unavailable; fresh login uses the Channel admission selects;
- reserve/revalidate the actual placed outside tile: accepted house entrance if free
  and walkable, otherwise native radius3 fallback from that entrance, then current
  CHAR_POSITION0 step4 (home temple when implemented, else pinned entry_start),
  including applicable Premium relocation through its actual owner;
- clear both House columns and upsert that **chosen tile**, its actual frame/map/
  artifact context and captured vitals under the **destination** Character lease plus
  a newly allocated native write sequence, in the same commit as placement/session
  handoff/fallback admission; release source authority only at the real commit point.

Occupied/blocked entrance never by itself refuses an exit. Do not clear columns in
an independent transaction, retain an old interior cell with null House fields,
write the requested entrance when a different tile was placed, or wait for the next
periodic/final write. A disconnect immediately after commit must load outside.
An older in-flight house periodic/final write cannot resurrect the house row after
the destination's larger order key; test the real generations, not a hardcoded key.

Two concrete owner gaps prevent claiming this fallback currently executable:

1. Native qualified `entry_cell`/`entry()` is outside the house cells, but its mapping
   to the accepted catalogue `entrance`, active bundle coordinates and current
   World/frame must be explicitly qualified by the content/1b owner. Generic bundle
   house-tile flags, default test coordinates or legacy numeric house IDs are not
   an entrance. No guessed conversion/exit tile is supplied here.
2. A removed House still needs the accepted deterministic outside entrance resolution.
   Historical saved HouseId alone has no retained entrance coordinate. The owning
   content/property lifecycle must supply its actual retained qualified mapping or
   reviewed retirement/fallback contract before activation. Unknown metadata is not
   known no-access, and cannot be replaced by an invented tile or a silent successful
   login at entry_start. Keep this integration/acceptance gate explicit.

Before either source or destination resumes, reconcile open handoff by its real ID:
PREPARED/not committed keeps source and releases reservation; COMMITTED keeps destination
and retains reservation until placement/release or proven stale terminal resolution.
Do not issue a new handoff, revive source, mutate the row from an old receipt, or
release reservations because of timeout alone. A current generation must reload;
old owner callbacks cannot capture/write/place after retirement or world reset.

### Exact shared deltas, closed gate and future qualification

CP must allocate and compose native LastPositionRecord/decoders/common upsert,
0080/0084-preserving next-number migration, actual house-specific session fence,
last_position tracker/capture/read/placement/vitals callers, fresh_admission final
write, respawn clear, reverse handoff exit and rejected-house/recovery commit call
sites, and current gameplay terminal/logout/death/read consumers. The historical
new character_position_house.rs path alone cannot implement this inseparable bundle.
Native original Channel behavior and arbitrary non-House Instance skip stay intact.

HOUSE-RUNTIME-1b owner alignment is conceptual/read-only: its current packet proposes
real loader/presence/reservation and actual reverse-handoff callers, not authority
created from local HouseId/generation. Prepared1a has structural item endpoints/audit,
not the complete HouseInterior loader/value writer. Neither is treated as merged.
Current `exit_into_channel`/HouseHandoffError::ExitNotAdmitted remains closed and
`game_house_access` stub returns no row. All entries stay HOUSE_CLOSED until the
independently reviewed ADMIT§3.2 selection/scope-ack/lost-ack/snapshot lifting conditions
are met; acceptance/merge of ADMIT text alone does not lift runtime refusal. No new
wire/capability or 1c-only bypass is authorized. Same-node/current assignment is a
real native limitation; cross-node scope/custody/placement handoff is future owner work.

The matrix below is **planned integrated qualification; none was executed by this card**.
Each negative changes one invariant in a real valid fixture, preserving unrelated
current facts. Independently resolved current facts cannot be copied from the record
or handoff being tested.

| Boundary / invariant | Required actual evidence |
|---|---|
| Native row / nullable shape | Old null-pair row unchanged; exact HouseId roundtrip; each half-null, malformed/cross-World House key/World and wrong Character World refuses. Existing health/mana pair, digest widths and order constraints stay enforced. |
| Single/batch/final upsert / liveness | Independently missing/stale session/connection/lease/scope/assignment/incarnation/admission guard; House/Instance mismatch, retired owner and expired pass deadline write no field. Generic non-House Instance still writes nothing. Cover actual active and reconnectable final release states. |
| Projection / order and dirty tracking | Same cell/vitals but different House discriminator is due; higher sequence replaces all fields; equal/lower and late old lease replace none. Periodic-vs-final and house-vs-destination clear races use actual native keys; new lease orders after old. No CharacterRevision/receipt produced. |
| Capture / vitals/death | Living cell+House+vitals captured together, bounded/current maxima behavior retained; dead capture excluded; pending respawn/arrival outranks House and placement atomically writes actual outside row with null pair. Read/placement failure never tracks a fabricated entry_start. |
| Fresh login / current House admission | Valid access restores saved tile; blocked/occupied tile picks first native same-house radius3 candidate; no candidate/full200 falls outside. Wrong frame/content/House placement/current access/fence does not restore. Old same-session Channel resume reads/moves nothing. |
| Admission / ACL concurrency | Access, property fence, owner or guild membership/revision changes between read/reserve/commit serialize under actual locks: losing login falls back; successful earlier login is visible to subsequent revocation exit. Source remains/reconciles until a committed transition. |
| Exit/fallback / atomicity | Every listed exit reason, rejected saved house and unrecoverable house clears pair with actual destination tile/key in the same commit. Occupied/blocked entrance reaches radius3 then step4; unavailable origin uses ordinary World selection. Immediate disconnect/reload never returns inside. |
| House gone / provenance | Known removed House uses independently retained qualified entrance mapping; unavailable/mismatched metadata cannot be guessed from saved key, old pin, generic house flags or test fixture. Qualify the actual owner hook before opening entry/acceptance. |
| Handoff / unknown/restart | Lost prepare/commit/abort/release replies, restart with PREPARED/COMMITTED and destination not yet bound, successor generation/late callback and source/destination replacement retain exactly one authority/reservation outcome. Historical receipt is not current placement permission. |
| Native compatibility / durability | Actual PG17.6 migration/old-row/reader/single/batch/terminal/respawn and reload tests plus real house-owner/handoff integration. Failures roll back cell+House+vitals together. No fixture-only read test claimed as production logout/relog, cross-node, authority, or E2E qualification. |

This card's actual checks passed: governance22 documents/9 lanes and124 tests,
diff/scope/reverse-patch checks and exact source binding only. No schema, position write, runtime test, House authority, logout/relog,
PostgreSQL or E2E implementation/qualification is claimed. Root alone may publish
this packet to the existing draft; all family acceptance boxes remain unchecked.
Evidence: `/workspace/oteryn-prep/house-runtime-1c-evidence/`.


## 2026-10-15 native implementation supersedes card-only scope

The preceding packet and its checks remain historical design-phase evidence, frozen
under `house-runtime-1c-evidence/design-phase/`. The control plane subsequently
allocated exactly `durability/character_position_house.rs`,
`tests/house_runtime_position_postgres.rs` and this card for a bounded real native
component. This section supersedes the earlier claim that this candidate is card-only;
it does not change the open acceptance criteria or authorize shared integration.

The new durability sibling reads and writes the **same** native
`game_character_last_positions` row, including native cell/digests/vitals and
lease/write-sequence CAS. `HousePositionRecord` and `StoredHousePosition` extend the
native position value with `Option<HouseId>`. The private, uninstalled
`PROPOSED_SCHEMA` adds only nullable `house_world_id`/`house_key` and a joint
null-pair/same-World/native-key check; there is no second position table or saved
origin channel. `None` writes both columns NULL, including a same-coordinate Channel
clear. Native pending respawn wins; equal/lower keys change no fields and no
CharacterRevision or semantic receipt is produced.

The public native Root read/write methods consume the real reconciled recovery
capability and native semantic pass/deadline, lock admission relations, and check
current facts in the transaction. Channel uses the unchanged native Channel fence.
House independently locks the named current session, checks connection/lease/scope
generation and exact House-derived Instance, current House assignment and holder
incarnation, and actual canonical Character/account/runtime admission guards with
the native publication-history checks. Only after that independent resolution is
the captured House compared. The captured row is historical data, not a permission
to place, enter, exit or restore the actor. A typed same-transaction helper exposes
this row operation for a future real owner commit; it neither invents a placement
proof nor performs a physical handoff.

The PostgreSQL fixture uses actual native `assign_house_scope`, prepare/commit House
entry, recovery, node registration and canonical guard fixture persistence. Native entry
currently does not install the controller or rebind these guards: the isolated
fixture separately installs **SYNTHETIC INDEPENDENT guard DATA** through a cfg(test)-only
helper using unchanged native encoding/persistence/mirrors/history and real locks. The
sealed lifecycle/publication predicate is not admitted. Ordinary native Bootstrap
with active holder/presence claims and CAS claim changes correctly return Conflict;
these refusals are preserved as the missing real House guard-rebind integration gate. The existing
`publish_runtime_readiness` API is Channel-only. Native `HouseId::scope_key()`
(World+House key) and the canonical Runtime guard key for its derived Instance are
distinct encodings; the future House publisher must bind them through the real
current assignment/incarnation, not treat either key or synthetic readiness as proof.
Its test-only access function, captured cell/pins and fixture Channel conversion
qualify storage only, **not** production access/admission, entry/exit, a controller,
placement, logout/relog or cross-node handoff.

The auto-discovered tracked PostgreSQL target is deliberately gated with
`#![cfg(any())]` until CP enables it together with native durability registration.
It contributes **zero default Cargo tests**. The qualification snapshot removes
only that crate gate and registers the exact native sibling; the native runtime
bridge copies the exact four owned case bodies with test attributes/visibility and
forwarders only. Native cfg(test), Root and shared fixture implementations remain
unchanged. Original-unit metadata Clippy, enabled original-target metadata Clippy,
tracked disabled-target metadata and actual PostgreSQL execution are reported as
separate results, never as an integrated original-branch test pass.

Shared CP gates remain: install/review the two-column migration; compose the common
Channel single/batch/final/respawn readers and writers so every outside write clears
the pair **before schema activation**; register this sibling and enable its target;
connect actual House-owner capture/placement/admission/exit/death/terminal callers
and atomic destination projection; qualify retained removed-House entrance metadata
and ADMIT3.2 selection/ack/no-snapshot lifting. In particular the current common
Channel upsert does not clear these new columns and cannot be activated unchanged.
No production House gate is opened by this implementation or fixture.

### Final bounded implementation qualification

Actual PostgreSQL17.6 (`server_version_num=170006`) execution passed **4 cases,
0 failed, 0 ignored**. They exercise full native row reload and equal/lower/newer
ordering, two independently warmed actual max-one Roots with separately opened
native recovery capabilities for database concurrency, independently changed current
session/connection/lease/scope/House/Character/account/runtime/assignment/node facts,
canonical mirror and missing-history refusal, schema null-pair/key constraints,
deferred-commit rollback, same-cell Channel clearing, old null-row decoding, genuine
native pending respawn and actual node-registration revocation. No production holder
limit was raised and no refused result was ignored. Native ordinary Bootstrap/CAS
claim refusals are asserted before synthetic guard fixture installation.

Final native cfg(test) normal-rlib compilation, enabled original PostgreSQL target
Clippy metadata (`-D warnings`), tracked disabled-target metadata, small native driver,
owned Rust formatting and workspace `cargo fmt --all --check` passed. Original native
unit Clippy metadata is reused from round4 only for the byte-identical module and
native root context; the subsequently changed integration target is not read by that
original library-unit context. `original-unit-clippy-reuse.json` binds that reuse;
the changed target/rlib/PG checks used a newly captured coherent dependency graph.
Cargo's intentional nonzero sentinel only captures real native compiler arguments
before root codegen and is **not** a Cargo test/check pass. No full server unit ELF,
full-server/package test pass or integrated default-branch execution is claimed.

The earlier round3 actual0/4 guard-bootstrap failure and round4 actual2/2 result are
preserved. The former exposes the real missing sealed House-claim rebind. The latter
revealed competing requests on one native max-one holder (open-vs-write checkout was
not logged) and an Account fixture update whose security publication wrapper stayed
at revision1. The repair uses a second actual max-one Root and the native-supported
publication-wrapper rebind while preserving security source time/revision/decision.
The production reader/writer/current-fence/schema bytes are unchanged from the prior
source; only the explicit cfg(test) synthetic-data helper was added to that module.

| Class / owned consumer | Actual mutation evidence |
|---|---|
| Identity / fenced write | Named session, connection, lease, scope generation, exact House instance and captured House mismatch independently refuse without a position row. |
| Current authority / fenced write | Independently installed canonical holder/presence/readiness negatives, actual assignment revocation with unchanged expected DTOs, wrong incarnation secret and actual registration revocation refuse; current mirrors/history are independently checked. |
| Temporal/provenance / row mutation | Equal/lower keys, concurrent actual-root writes, old lease, missing publication history, mirror substitution, pending respawn and deferred SQL commit failure preserve the required row/order outcome. No input clock/TTL is introduced; native recovery/deadline behavior is delegated unchanged, not independently exhaustively retested here. |
| Historical read / projection | Old null pair and complete House row reload preserve native cell/pins/vitals/key; reads grant no placement/admission/controller authority. Physical placement, exit and runtime admission remain unexecuted CP integration. |

Final governance/scope/checksum results are recorded in `source-manifest.json` under
`/workspace/oteryn-prep/house-runtime-1c-evidence/`, together with final source copies,
unchanged registration roots, qualification root copies, exact overlay/body transforms,
source hashes, failed rounds, actual logs and restoration scripts. The historical
card-only packet remains immutable under `design-phase/`. All original acceptance
boxes remain open; the module and explicit PostgreSQL target gate are not registered
or enabled on this branch. No schema is installed, runtime gate opened, commit or
worker push performed. Root alone owns any existing-draft publication after review.

## 2026-10-16 owned-path re-evaluation and PG routing repair

The owner directly confirmed exclusive authorship of the existing draft; no
additional CP lease number is required. This pass uses
`skeleton/house-runtime-1c` at `2d344f427eccbda584475c01917bad2eded5f271` and owns only
`durability/character_position_house.rs`,
`tests/house_runtime_position_postgres.rs` and this record. It does not allocate
common outside writers, runtime callers, shared registration or a schema slot.

The bounded source sweep found no new actual native port that can complete House
fallback or guard rebind within these paths. The existing storage sibling already
checks actual named session/connection/lease/scope, current House assignment,
incarnation and canonical Character/account/runtime guard mirrors/history, and
its own Channel capture clears both nullable House fields. The separate
`world_house_instance` observation is not a physical presence/current assignment
proof and cannot replace those checks. No synthetic fixture was promoted into a
production authority source and no second loader/fallback/guard writer was added.

The concrete independent repair is in all four existing PostgreSQL cases:
missing or non-Unicode `OTERYN_TEST_POSTGRES_ADMIN_URL` now returns an error rather
than reporting a successful test with no database execution. The existing
PostgreSQL17.6 version and safe isolated test-database checks stay unchanged.
The target's `cfg(any())` remains explicit: registration of this private durability
sibling and consolidation into the native aggregate are still a CP slot. This
repair adds no default CI cases and is not new runtime/storage qualification.

Still required from their real owning paths: common single/batch/final/respawn
outside-column clearing; real actor capture and placement plus same-transaction
destination projection; native House guard rebind/publication; qualified retained
removed-House entrance metadata; and the reviewed ADMIT0 §3.2 reverse-transfer
contract. HOUSE-RUNTIME-1a/1b are dependencies, not installed authority on this
source floor. All five original acceptance boxes remain open.

No Cargo/rustc/heavy build or PostgreSQL execution is authorized here under D1011.
Only formatter, governance and static scope/whitespace checks qualify this repair;
the current source-bound handoff is `/workspace/house-family-handoff/runtime.json`.
There is no commit, worker push, schema installation, runtime activation or merge.

## 2026-10-17 source proposal recovery archive — NOT_APPLIED

At the owner's request, the previously external source proposal is preserved
below inside this task record, rather than only in a local handoff or PR comment.
The `patch_lines` array joins to the original UTF-8 patch bytes and SHA256.
The original base and historical source-only review are retained for recovery;
they do not qualify the current branch or authorize applying shared hunks.
Recheck the actual successor source and obtain the required CP shared slot,
migration allocation and independent review before implementation.
**Shared proposal NOT_APPLIED; native execution NOT_RUN; runtime acceptance
and all existing integration gates remain unchanged.** This appendix changes
only this owned task record and introduces no executable implementation.

Previous reviewable handoff: https://github.com/Oteryn/Oteryn-Game/pull/2166#issuecomment-6230752861.

```json
{
  "archive_version": 1,
  "owning_pr": 2166,
  "status": "SOURCE_PROPOSAL_NOT_APPLIED_NOT_RUNTIME_QUALIFIED",
  "original_source_sha": "1ea22dd82bcee468dc06b82011a42191483c2237",
  "patch_sha256": "9e4ea3c3ff0fb00b7d62c226c9cc6067b6c866aa7881afafef558488628b0b0e",
  "patch_lines": [
    "--- a/apps/game-server/src/durability/character_last_position.rs\n",
    "+++ b/apps/game-server/src/durability/character_last_position.rs\n",
    "@@ -345,9 +345,9 @@\n",
    "         \"INSERT INTO game_character_last_positions AS p \\\n",
    "             (character_id, world_id, tile_x, tile_y, tile_floor, coordinate_frame_digest, \\\n",
    "              map_revision_digest, server_artifact_digest, character_lease_generation, \\\n",
    "-             write_sequence, health, mana) \\\n",
    "+             write_sequence, health, mana, house_world_id, house_key) \\\n",
    "          VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, $7, $8, \\\n",
    "-                 $9::text::numeric(20,0), $10::text::numeric(20,0), $11, $12) \\\n",
    "+                 $9::text::numeric(20,0), $10::text::numeric(20,0), $11, $12, NULL, NULL) \\\n",
    "          ON CONFLICT (character_id) DO UPDATE SET \\\n",
    "              world_id = EXCLUDED.world_id, tile_x = EXCLUDED.tile_x, \\\n",
    "              tile_y = EXCLUDED.tile_y, tile_floor = EXCLUDED.tile_floor, \\\n",
    "@@ -356,7 +356,8 @@\n",
    "              server_artifact_digest = EXCLUDED.server_artifact_digest, \\\n",
    "              character_lease_generation = EXCLUDED.character_lease_generation, \\\n",
    "              write_sequence = EXCLUDED.write_sequence, \\\n",
    "-             health = EXCLUDED.health, mana = EXCLUDED.mana \\\n",
    "+             health = EXCLUDED.health, mana = EXCLUDED.mana, \\\n",
    "+             house_world_id = NULL, house_key = NULL \\\n",
    "          WHERE (p.character_lease_generation, p.write_sequence) \\\n",
    "              < (EXCLUDED.character_lease_generation, EXCLUDED.write_sequence)\",\n",
    "     )\n"
  ],
  "historical_source_manifest": {
    "status": "DEFERRED_EXACT_HUNKS_PROPOSAL_NOT_APPLIED",
    "pr": 2166,
    "source_sha": "1ea22dd82bcee468dc06b82011a42191483c2237",
    "cp": "D1168/6225337534",
    "shared_order": "after2163->2197->A2; do not merge/rebase/apply shared paths now",
    "source_preimage_sha256": {
      "apps/game-server/src/durability/character_last_position.rs": "5fdf2ef1bb0c88b028a6292b6733547911e813119efcb03e38737b4b33439056",
      "apps/game-server/src/foundation/runtime_actor_death.rs": "cb9d6566a783694741cf28f99fa2cc9fba0dbdbee3f2a496d5cd44d15f7f0937",
      "apps/game-server/src/durability/character_death.rs": "ec04c1cf62f3c756f295e4ccc9ba7c29c926c1c2b478334dd5e1fb20f02a0ae3",
      "apps/game-server/src/durability/fresh_admission.rs": "7c23a211c825016a79c7d48bee5233a4bdbe257c956491a268836bcfe8c08a7d",
      "apps/game-server/src/gameplay_transport/mod.rs": "062f34bbb6dee8a0db205333f9a1e8f83f9d9f1279f671410df6723a3629fc4e",
      "apps/game-server/src/durability/character_position_house.rs": "62c8555b411796a67f40be3e25b74f6ecdbd4900f40bbbf1b5811932d9bd6fce"
    },
    "proposed_after_sha256": "b8e4ad7b2c3842cd8acbeff33747d61e850a7ce1b913845387fe0d8a7d177882",
    "patch_sha256": "9e4ea3c3ff0fb00b7d62c226c9cc6067b6c866aa7881afafef558488628b0b0e",
    "scope": "Only original shared upsert column/value/SET list: both House columns NULL in same INSERT or strictly-key-winning conflict UPDATE as original cell/vitals, no other SQL statement/parameter/fence/API changes",
    "equal_or_lower_order": "Original conflict WHERE unchanged; therefore no cell/vitals/House binding changes",
    "single_and_batch": "Original write_character_last_position and write_periodic used by batch already validate true Channel+sameWorld then call this upsert; no extra caller edits needed",
    "terminal": "Existing write_final_position only forwards sameWorld Channel writes to same upsert inside releaseTX; actual terminal caller hunk remains after2187 perD1168",
    "respawn": "No foundation/runtime_actor_death.rs hunk: it mutates only in-memory Channel carrier and owns no durable row/TX/orderkey. Existing consume_pending_respawn_placed and place_and_consume_pending_respawn_placed validate Channel/sameWorld and call this exact upsert in the pendingrespawn-consumption TX. They inherit atomic House clear; no added UPDATE or fake foundation DB authority.",
    "native_respawn_evidence": [
      "foundation/runtime_actor_death.rs:31 and62 local placement only",
      "durability/character_death.rs:433,485 validate_placed; upsert at577,667 after pending deletion",
      "durability/character_death.rs:803 validate_placed restricts Channel matchingWorld",
      "gameplay_transport/mod.rs:1458,1654 existing true caller paths"
    ],
    "prerequisites": [
      "Actual numberedHousecolumns migration/pairedcheck from native character_position_house DDL must be installed first; number requestedCP6225859967, no new number allocated here",
      "Runtime migration0080 table INSERT/UPDATE grant already covers columns; preserve actual successor permissions checks",
      "Recheck exact shared preimage after prescribed owners; no blind patch application"
    ],
    "qualification": "SOURCE_PROPOSAL_ONLY_NOT_NATIVE_PASS",
    "required_actual_pg_cases": [
      "winning higherSequence clears bothHousecolumns with cell/vitals",
      "equalSequence andlowerSequence change nofield",
      "newhigherLease wins despite lowerSequence andclearsHouse",
      "staleSession/World/assignment refuses entirewrite",
      "pendingRespawn preserves entireoldrow",
      "single/batch andbothrespawnconsumption paths nativeTX",
      "rollback preserves oldHouse/cell/vitals; nosecondUPDATE race"
    ],
    "checkout_changes": false,
    "pc_calls": false,
    "new_cp_request": false,
    "local_checks": {
      "git_apply_check_exact_preimage": "PASS, no application",
      "official1_94_rustfmt_edition2024_skip_children_proposed_source": "PASS",
      "native_pg_execution": "NOT_RUN"
    }
  },
  "historical_source_review": {
    "reviewer": "Root independent Medium",
    "verdict": "PASS_SOURCE_PROPOSAL_ONLY_DEFERRED",
    "patch_sha256": "9e4ea3c3ff0fb00b7d62c226c9cc6067b6c866aa7881afafef558488628b0b0e",
    "checks": "Only shared upsert INSERT column/value + conflictSET add bothHouseNULL; all binds/session/World/lease/sequence predicates unchanged; keystrictgreater means equal/lower untouched; oldHouse writer ownupsert unaffected; originalpendingrespawnconsumptionTX routes here; no foundationDBauthority invented",
    "required_before_apply": "Actualmigrationnumber/columninstallation+sharedCPorder2163->2197->A2 and exactnewpreimage/review; nativePG NOT_RUN",
    "shared_files_applied": false
  }
}
```
