# HOUSE-1b — canonical disposition and absolute move-out commands

```yaml
task_id: HOUSE-1b
title: "HOUSE-1b: canonical disposition and absolute move-out commands"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/house-1b
base_sha: f010472bcb6794a2cbc56dfaf755bd122c10225e
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row HOUSE-1b (row 119); App. 7 HOUSE-*; §5 Wave 5"
class: "hard (persistence, economy, security)"
needs_migration: yes
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/house/transfer.rs, move_out.rs (new)"
  - "apps/game-server/src/house/*_tests.rs"
  - "apps/game-server/tests/house_move_out_*.rs"
  - "docs/agents/tasks/HOUSE-1b.md"
  - "apps/game-server/migrations/0121_house_disposition.sql (D1382/D1403 canonical installer)"
  - "apps/game-server/src/durability/house_disposition_native.rs (D1367/D1384 private bridge)"
  - "apps/game-server/tests/support/house_move_out_postgres_cases.rs (D1349 native cases)"
  - "apps/game-server/tests/support/house_move_out_waiting_postgres_cases.rs (unmounted producer gate)"
  - "apps/game-server/tests/support/character_inbox_postgres_cases.rs" # D1413 exact Inbox reader inventory
slot_only_paths:
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## 2026-10-17 completed canonical native receipt — 24/24 PASS

The exact D1413 seven-file source freeze completed the granted first D1409 child
under [D1415](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6238999808).
Molehill-PC slot I ran from 19:06:16 to 19:10:39 UTC. The canonical outer lock was
held in Ubuntu/mole and released at child exit; the existing Ubuntu-24.04/root
worktree remained at `c65dd10af69ce6d2263175a35736fbe531f01a01`. Locked fetch,
offline Cargo, four jobs and the D1329 PG wrapper were used. No PostgreSQL server,
container, alternate heavy lock or new worktree was created.

| Exact native component | Result |
| --- | --- |
| Pure absolute deadline boundary case | LIST1; 1 PASS, 0 failed, 0 ignored |
| `house_move_out_postgres_cases::` | LIST22; 22 PG17.6 PASS, 0 failed, 0 ignored; 41.83 s |
| Canonical Inbox immutable-row/reader inventory case | Exact LIST1; 1 PG PASS, 0 failed, 0 ignored; 1.12 s |

The Inbox case used its existing dedicated target and full MIGRATOR. Its absence
of a QUALIFIED marker is intentional; the successful real-PG route and exact test
summary were verified. House produced exactly 22 post-cleanup qualification
receipts. The normal successful Inbox helper also completed its own database
cleanup. No PRE-ROUTING or NONCANONICAL path supplied these results.

All seven command log hashes were verified after full readback. The result is
`e37c3dc89897519605105b8d43ac4ba43fac4d42694dd0353e7b187f8a2f4cc3`;
the actual raw 14-file manifest before and after is
`d5dab1c221a986d89430ad05e3f7bf04f12a43dba2049fb48fd7546ae53508fd`.
All source bytes, the complete Git patch and checkout inventory remained unchanged.
The final result reports exit0 only after those final integrity gates. Runner:
`43ff089e92b4750582885678bf7818fa4081882fc00f8492cd518041c3d6c034`.

The six SQL/Rust files in this publication are byte-identical to the successful
native source freeze; only this card gains the completed receipt. Original 0095
remains unchanged. This is evidence for the disclosed canonical source assembly,
not a claim that every production House/rent/runtime integration is complete or
that an independently executed future merged head is qualified. The remaining
owner gates below still apply; the PR remains draft / NOT_FULLY_QUALIFIED.

A fresh pre-publication read of main
`1d632319eb2ebfc234609c97fb944523bb1615ad` confirms the complete Inbox baseline
is still identical to c65, blob `958cd155646ae60d2d7690e2138fdfa1abbe43be`.
The published amendment is therefore exactly the granted sorted +2 names/comment
hunk and retains the actual existing reader union. Later MAIL/TRADE entries must
still be retained and requalified when their actual composition lands.

The sections below preserve historical preparation and earlier execution states.
This completed receipt supersedes their pending/NOT_RUN labels for the exact
canonical 24-case source freeze. It does not supersede separately held features.

## 2026-10-17 D1413 composed Inbox inventory and native execution packet

**Historical source-freeze record; execution is completed in the receipt above.**

[D1413](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6238443765)
allocates the exact Inbox reader-list amendment to this 0121 producer in the same
push. It adds `game_house_disposition_delivery_proven` and
`game_house_disposition_native_removal_proven` to the sorted exact reader list,
and corrects the adjacent comment. `game_house_disposition_deliver` is not added.
The query, exact equality and all immutability assertions remain unchanged.

The fresh actual main `6c8d99925fccec46703af2f62698ff30ddf8ba36` and this
packet's parent `c65dd10af69ce6d2263175a35736fbe531f01a01` share Inbox blob
`958cd155646ae60d2d7690e2138fdfa1abbe43be`; neither yet contains D1390's later
MAIL/TRADE entries. The hunk is exactly the approved patch SHA256
`a655e37f93d36f1fe9932d3d0882f51cdda112657da739ea67f4cfb47ba6aa66`.
D1390's actual composed sorted union must be rechecked immediately before push;
a later owner entry must be retained and requalified rather than overwritten.
The shared test path is recorded in `owned_paths` above.

The seven-file publication successor retains all five previously formatted
production/test leaves and adds this one shared test hunk plus the updated card.
The four existing Rust leaves passed native formatting; Inbox native formatting
and the new composed execution remain pending at this source freeze. The earlier
six-file packet and its native formatter receipt are retained separately.

[D1409](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6238179653)
allocates the first House child to this canonical migration. Its actual inventory
is **24 cases: one pure boundary case, 22 House PG cases and one additional Inbox
PG case**. The existing dedicated `character_inbox_postgres` target executes
`character_inbox_postgres_cases::inbox_rows_are_immutable_and_no_other_writer_touches_them`
through the D1329 wrapper and the full `sqlx::migrate!("./migrations")` installer.
No new Cargo or registry mount is needed for that existing target.

The Inbox helper emits no `QUALIFIED` marker. Its successful native receipt must
require the exact LIST1, real wrapper invocation, one PASS with zero ignored,
no PRE-ROUTING/NONCANONICAL route, and unchanged final source. Its normal success
includes its own exact-name database cleanup; its legacy panic/create-failure
cleanup gap remains disclosed in the source review. House's separate expected
post-cleanup `QUALIFIED PostgreSQL17.6` count remains 22. No case has run on this
new composition yet, and this source freeze is **NOT_FULLY_QUALIFIED**.

## 2026-10-17 D1403 canonical producer and D1384 owner command

**Historical six-file source-freeze record, superseded by the D1413 packet above.**
At that freeze, native formatting, compilation, Clippy and PostgreSQL were NOT_RUN.
The four Rust leaves have since passed the retained native formatting receipt;
compiled/PG qualification still awaits the new composed attempt. The publication
parent remains `c65dd10af69ce6d2263175a35736fbe531f01a01`. Earlier private
installers and execution results do not qualify this canonical successor.

[D1403](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237071382),
answering [the complete producer request](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6237062703),
confirms migration `0121_house_disposition.sql` under
[D1382/D1384](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6235281112).
The existing migration directory is the registry. This packet adds no separate
migration registry and no event type; event 8 remains #2137/0119's allocation.

### Canonical SQL and exact extraction

The migration installs the existing three disposition tables and their ordinary
operation, census, step and real Inbox-delivery functions before the accepted
native removal predicate. Both Rust private schema constants and both fixture SQL
installations are removed in this same packet. The existing `MIGRATOR` is the only
installer, so the native tests execute the same guard definitions as production.
The Rust modules remain canonical `house::{move_out,transfer}`, with the existing
private Durability bridge; this packet changes no module root or Cargo registration.

| Retained SQL source | Original extracted SHA256 | Canonical extracted SHA256 |
| --- | --- | --- |
| `move_out.rs` ordinary producer | `b6cb12794737e63c6df05b0f4a9b6c92ee6436b455863b61c42efd7fc72aeeac` | `a6a54944c6accbd5ffe45f50b9f1a727fc9bc4592e9c653628eddcc2a9f34c4f` |
| `transfer.rs` real custody producer | `563c3f2c8acfd148c48adae7797cf5f2147aa1dd34c674301bcfcfd582e82d16` | Identical |

The ordinary-producer change is D1384's replacement of `notice_days` equality with
the checked absolute bound. The migration SHA256 is
`e0a7f4fc7de8e994b661383429ecc3b39dfeea873583d4fbc417371765fece99`.
The separately reviewed removal predicate/guard segment is
`7062dc9e8d02286856239b6e6ae9515e878fbde519ba49d780961918a8008653`.
Native 0095 is not edited. Removing only the new ordinary-disposition DELETE arm
and the `OR REPLACE` keyword reproduces its original guard text exactly (SHA256
`8583585b9a5e10b6058b66d356b3210602817348d986557a659cb7fcb6a321a2`).
Its INSERT and player shape 6/7 DELETE arms, trigger and deferred timing remain.

The new proof requires the actual immutable source, current physical transaction,
matching disposition/step, unchanged native item, real Inbox delivery to the captured
provenance reclaim subject, and absence of old House location/provenance. Its lookup
is bounded by exact World/House and item identity. PUBLIC/runtime/control receive no
direct predicate execution or new table-mutation grant. Existing fail-closed typed
Guild-hook capability checks are retained; no absent relation is replaced with a
fake table, an optional zero, or a fixture-only version of the production guard.

### Native owner command and replay

`DurabilityRoot::apply_house_move_out_command` consumes retained recovery authority,
the real Node proof, complete current fence, original `CommandRef`, `HouseId`, and
`HouseMoveOutCommand::{Notice { due_at_unix_ms }, Cancel}`. The request supplies no
operation UUID, ownership revision or currentness flag. The existing native semantic
transaction, recovery/admission locks and handoff/owner locks enclose the whole write.

The owner samples one DB clock and checks the inclusive interval
`86_400_000 <= due_at_unix_ms - db_now_ms <= 2_592_000_000` with checked arithmetic.
It stores the exact requested absolute date without rounding. Under the original
CommandRef lock, replay returns the original binding and outcome before re-reading
the clock or current ownership. The server resolves and mints each new operation;
cancel targets the actual pending notice under lock. Replaying a cancelled older
notice or cancellation cannot affect a later notice. The separate
`read_house_move_out_owner` projection performs an actual current native fence and
owner check and returns a read-only snapshot, not authority to mutate custody.

### Exact test inventory and previous failure evidence

The actual canonical aggregate retains sixteen prior cases and adds the exact
native-removal regression plus five D1384 cases: absolute-date retention/replay,
refused deadline without a durable effect, cancellation of the actual pending
notice, replay versus current owner projection, and a real two-Root same-command
race. Selector `house_move_out_postgres_cases::` therefore contains **22 PG cases**.
The owner module adds **one pure** inclusive-boundary and integer-overflow case,
`house::move_out::absolute_due_tests::absolute_due_bounds_are_inclusive_checked_and_not_rounded`.
None of these new cases has run yet.

All sixteen domain assertion/command bodies match the previously compiled native
repair. Relative to published c65, fifteen complete bodies are byte-identical; the
remaining body changes only its invalid `durability::house_move_out::DispositionView`
type path to the granted canonical `house::move_out` path. The repair also restores
the two existing real-entry imports, and uses the owned bounded-step constant.
The one waiting committed-access case remains unmounted behind actual #2137/#2140
producers. No `ignore`, disabled whole-file test gate, fake native function, diagnostic
bridge or forced-deferred probe is introduced. D1366 NON-PG routing and cleanup after
ordinary errors/assertion panics remain intact; only actual routed PG and successful
own-database cleanup produce the qualification marker.

The preceding repair assembly compiled, then produced **10 PG PASS / 6 FAIL**, with
zero ignored. Its separate one-case diagnostic exposed native SQLSTATE23514,
`house removal lacks actual source shape`. This is the concrete reason for the
canonical D1382 proof; neither that failed run nor historical private-harness passes
qualifies the new migration. Independent source review passed for the full migration,
preserved player arms/privileges and separately the D1384 command/replay/lock order.
Those reviews precede only the present documentation changes; native execution is
still required on the actual complete candidate and published head.

### Remaining integration gates

DRAINED still retains the owner and content fence. Actual final property/slot release,
registered House events, ongoing rent/grace/eviction, BED release, content revision,
runtime exit/quiescence and production caller permissions remain separate work. WIRE
maps the actual command only after #2141 merges. D1403 does not grant the distinct
Guild removal arm: #2137 retains its honest PROPERTY6 PASS and GUILD9 PASS / 4 FAIL
evidence, with the fourth error awaiting diagnosis. Its own fixture stops reinstalling
ordinary SQL after 0121 reaches main; no Guild proof is silently admitted by this packet.

## Native trigger-order repair — local recovery candidate, NOT_FULLY_QUALIFIED

Actual227 executed only original case06 against PostgreSQL17.6 and observed
SQLSTATE23514 (`DUR-03 item record changes only through a TRANSFER`) before the
required NOWAIT ownership-anchor refusal55P03. PostgreSQL orders same-kind
triggers by name: the generic `game_item_instance_guard` preceded the owned
`house_disposition_item_fence`. This is an observed native RED, not a capacity HOLD.

This authoring successor renames only the four owned trigger objects to
`a_house_disposition_{content,provenance,item,entry}_fence`, so the existing House
NOWAIT fence runs before generic proof guards. Trigger functions, event masks,
ABI1, generic guards and original case06/assertion55P03 remain unchanged. Native
setup now checks the real enabled BEFORE ROW trigger ordering for every relevant
event on all four tables; no new mocked authority or alternate acceptance code.
#2137's exact enabled-hook catalogue and disabled-hook negative consumer follow
these names in D1162 producer→consumer order; mixed generations remain closed.

Focused official Rustfmt1.94 and whitespace checks pass. The repaired native/PG
generation is NOT_RUN and requires independent review plus a new actual executor
allocation; the previous slot allocation has ended. No native PASS is transferred
from the prior source. Independent source review passed after exact hash and
owned-path checks; this successor is preserved as a local commit and verified
bundle outside the checkout. Publication is separate; no ready/merge/rebase.

## 2026-10-17 canonical House source and case routing — native bridge

This packet starts from CP's ordinary main merge
`168c64fa49dccda4af692bae57399adfd699a59a`. Exact preimages for the owned sources,
native module roots and canonical aggregate were read at that head. Earlier
qualification below remains historical. Native formatting passes for this
twelve-file packet; native compilation and PostgreSQL qualification are NOT_RUN.

[D1349](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233992418)
requires `house::{move_out,transfer}`. Their previous private Durability-child
mount directly used Durability-private recovery and transaction helpers. The
private `durability/house_disposition_native.rs` retains the actual existing Root
method, semantic transaction, recovery check, admission/handoff locks and
commit/rollback. Canonical House keeps the SQL/body and real native node/item
fences. A pure forwarder delegates Channel-key encoding to its actual native owner.

[D1367](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234458700)
grants the six-source bridge packet with a private module. The bridge does not
widen `record_for`, `assert_recovery_fence`,
`lock_admission_relations`, `scope_key`, the DB module or node secret access. Its
new path and one private Durability mod line follow that grant. Its pure encoder
is a crate-visible associated function on the existing DurabilityRoot, without a
module visibility change or reexport. The three owned House functions `notice/cancel/freeze`
receive crate visibility only so this native envelope can call them. No callback,
caller-selected currentness flag or replacement authority is introduced.

### Exact original-case prerequisite inventory

Sixteen existing cases have no unmerged bank, Guildhall, BED or access producer.
Their marked native bodies move into
`tests/support/house_move_out_postgres_cases.rs`, with only canonical House import
paths and removal of unused committed-entry imports. The sixteen domain assertions
and commands are retained. Runners additionally follow D1366 below. They use the existing real
`bestiary_postgres_harness`, native migrations and this task's actual private
disposition schema; initial retained ownership/items are disclosed fixture data.

The canonical `tests/durability_postgres.rs` receives only this task's House source
mount and case mount. The old standalone file becomes a thin compilable wrapper; its whole-file
cfg(any()) gate is removed. The own Cargo test=false entry excludes duplicate
default execution, while the actual cases remain mounted once in the aggregate.
The native aggregate selector is `house_move_out_postgres_cases::`.

| Existing case | D1349 route / prerequisite |
|---|---|
| `frozen_native_item_and_provenance_mutations_share_the_content_fence` | Native aggregate now; existing native owners on main |
| `ordinary_drained_fence_remains_without_a_guild_owner_installation` | Native aggregate now; existing native owners on main |
| `notice_is_current_fenced_bounded_and_idempotent` | Native aggregate now; existing native owners on main |
| `unloaded_house_101_items_return_to_real_reclaim_subjects_once` | Native aggregate now; existing native owners on main |
| `reconstructed_roots_resume_frozen_and_partial_custody_without_redelivery` | Native aggregate now; existing native owners on main |
| `native_inbox_failure_rolls_back_source_provenance_cursor_and_evidence` | Native aggregate now; existing native owners on main |
| `native_source_and_frozen_manifest_cannot_be_forged_or_edited` | Native aggregate now; existing native owners on main |
| `current_job_scope_owner_and_native_ownership_revision_are_independent` | Native aggregate now; existing native owners on main |
| `concurrent_same_step_and_new_notice_serialize_without_double_delivery` | Native aggregate now; existing native owners on main |
| `real_prepared_entrant_blocks_custody_and_fence_prevents_late_commit` | Native aggregate now; existing native owners on main |
| `overfull_native_house_is_refused_before_full_census_capture` | Native aggregate now; existing native owners on main |
| `another_current_world_channel_can_process_an_unloaded_house` | Native aggregate now; existing native owners on main |
| `owner_cancel_is_fenced_retained_and_allows_a_fresh_notice` | Native aggregate now; existing native owners on main |
| `overdue_cancel_races_freeze_without_reopening_or_returning_items` | Native aggregate now; existing native owners on main |
| `before_due_cancel_races_not_due_freeze_as_one_retained_successor` | Native aggregate now; existing native owners on main |
| `current_successor_connection_can_cancel_an_older_notice` | Native aggregate now; existing native owners on main |
| `actual_committed_house_session_blocks_after_reservation_is_released` | NOT MOUNTED; #2137 + #2140 actual composed `game_house_access` and current owner/schema fixture |

The waiting case is retained separately in
`tests/support/house_move_out_waiting_postgres_cases.rs`, with no mod declaration,
ignore attribute, successful stub or skip return. Its previous test-only
`game_house_access` replacement is removed. It must not be enabled merely because
an unqualified owner DTO or source snapshot exists.

The Guild name in the ordinary drained-fence negative and ABI capability check
does not make those Guildhall-positive tests: they assert ordinary safety with
the Guild owner absent. None of these seventeen cases performs a bank operation,
Guildhall release or BED write. Future positive bank cases wait for #2136/#2139,
Guildhall cases for #2137 and BED cases for #2147; none is invented or counted as an
authored passing case here.

[D1366](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234335395)
governs NON-PG CI routing: the existing native helper reports the missing routed
database, and these cases return without a PostgreSQL qualification marker.
Both the common runner and reconstructed-Root case keep the actual Harness outside
the panic-catching boundary, then clean its own database after normal errors or
assertion panics. A panic is resumed after cleanup. A QUALIFIED PostgreSQL17.6
marker is printed only when a routed case and its own database cleanup both
succeed; NON-PG CI is not native qualification.

[D1371](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234628102)
allocates the exact four Cargo lines and five canonical aggregate lines for this
coherent twelve-file checkpoint. The private bridge remains under D1367. Root
checks the exact current shared-file preimages before publication.

**PASS:** native formatting of all changed Rust sources in this twelve-file
packet on Molehill, including the final case registration and cleanup runners.
The exact formatter-only changes are incorporated into the publication payload.

**NOT_RUN:** native compilation/Clippy, the sixteen canonical
native PG cases and the separate waiting access-owner case for this source
identity. Root is the executor under D1329/D1342: fetch locked dependencies first,
use the existing PG namespace helper and isolated own fixture database, with the
free Molehill slot/memory gate. Source transform and exact case names are retained
with the publication packet.

This routing does not complete ordinary terminal ownership/slot release, automatic
rent/grace processing, bank burns/refunds, BED freeing, runtime exit/quiescence or
registered House events. DRAINED continues to retain the owner and native content
fence until the actual same-transaction participants are integrated.

## Goal

Second half of house ownership per SOCIAL_MAP_PACKETS HOUSE-1b: move-out and transfer, item return to the inbox, overlay of house tiles.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| HOUSE-1a | NOT MERGED | **BLOCKING** |
| INBOX-1a (MAIL-1) | MERGED (archive OTV2-20261004-inbox-1a) | ok |
| MAP-OVERLAY-1c | NOT MERGED (Wave 3) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/house/transfer.rs, move_out.rs (new)
- `apps/game-server/src/house/*_tests.rs`
- `apps/game-server/tests/house_move_out_*.rs`

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/migrations/00NN_house_transfer.sql
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- inbox writer from INBOX-1a
- apps/game-server/src/durability/house_scope_handoff.rs
- map overlay (MAP-OVERLAY-1c)
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- migration + writer + durability
- mint/burn + ledger (transfer price, item custody to inbox)

## Acceptance criteria

- [ ] move-out returns every house item to the owner inbox exactly once
- [ ] transfer is atomic with the bank ledger
- [ ] restart mid-transfer resumes from durable state
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server house
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (HOUSE-1b)
- `docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md`
- `docs/agents/tasks/archive/OTV2-20261004-house-rt-inbox-packets.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local implementation — bounded native disposition custody (2026-10-15)

Existing draft #2141; exact source `49caf3cacd4afdc14140e4d87e5beef8e5da7e70`,
`skeleton/house-1b`; accepted read-only main `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.
The lead expanded the original card-only lease to the exact allocation's five paths after
user steering. The earlier report and actual evidence are preserved separately in
`/workspace/oteryn-prep/house-1b-evidence/design-phase/`; they are not runtime qualification.
Original plan row, unchecked acceptance criteria and CP rules above remain unchanged.
META 3.1.0, one author, local AUTHORING; worker makes no commit/push, root may batch-publish
the existing draft, and CP alone controls ready/merge/queue. No shared registration or
production migration/permission/content/registry changes were made.

### Accepted scope and prepared component

HOUSE-OWN-0 §5–§9/§15, HOUSE-CUSTODY-0 §3, SOCIAL-MAP §0.1/§2.4 and
HOUSE-RT-INBOX §1.2/§1.6 govern this component. Direct player house resale is deferred by
HOUSE-OWN-0; `TRANSFER` here means HouseInterior item custody to each item's actual
**reclaim subject**, never owner fallback. The skeleton's price/bank-ledger and “owner
Inbox” wording stays unchecked for CP correction. SOCIAL-MAP holds HOUSE-ACL/WIRE,
GUILD-BANK/HALL and every wire child; HIRE is a candidate, not permission.

`src/house/move_out.rs` prepares a real native DurabilityRoot semantic pass and an
**uninstalled delegated private schema**. The physical file is intended to become a
`durability::house_move_out` child at the slot, without widening any native helper visibility.
A new notice uses actual recovery, current Character/session/connection/lease/Channel
fences and native0038 owner/revision; it binds CommandRef, operation, original fences,
house, notice days and owner revision immutably. Notice1..30 days and all persisted times
use DB-clock Unix milliseconds. Same-command/operation changed bindings and another
operation for the same live property return semantic conflict. Accepted §6 cancellation is
implemented before the DB-clock due time with actual current-owner/session fences, retained
CommandRef/original-fence binding and immutable cancelled time/outcome. A single materialized
clock governs both deadline and stored time. CANCELLED history keeps its first cause and
original notice; a partial live-house uniqueness rule permits a fresh notice. Old cancellation
replay cannot affect that fresh operation. At/after due or after freeze cancellation is closed.
NOTICE/CANCEL reuse the native `character_item_fence_is_current` boundary, whose
`scope_of` currently admits Channel sessions only. House-scoped owner commands are not
qualified here: they require the actual admitted House grant/session publication owner,
not a parallel or weaker current-fence predicate. World jobs have no actor-session requirement.

Due freeze and custody steps use independently current native node incarnation and an
assigned **execution Channel in the same World**, not an active destination House assignment
or actor session. Thus unloaded houses and another legitimate World Channel process can
run the job. Root recovery/admission relation locks, handoff relation lock and native property
locks protect the current census. Freeze reads at most2001 sorted IDs, refuses overflow of
the accepted2000 PARITY_PENDING bound, and retains exact native source location/provenance/
item snapshots plus count and SHA256 of sorted ItemInstanceIds. Native HouseInterior and
entry fences prevent subsequent source mutation or a new PREPARED/COMMITTED entrant.
First cause and notice binding are immutable; this subset admits MOVE_OUT only.

`src/house/transfer.rs` locks real recipient Character roots before property, items by
ItemInstanceId and native Inbox counters by CharacterId. Each step selects at most100
unmoved census items. Its private SECURITY DEFINER verifies current native owner/source
and canonical sessions plus unreleased PREPARED/COMMITTED entrant reservations, then
removes the real0025 source/provenance and calls **the actual0076
`game_character_inbox_deliver`** in the same transaction. Full native custody, cause,
Inbox exclusivity and deferred proof guards stay active. Private operation/step/item proof
also commits atomically and binds real source/reclaim/returned ordinal; it is not a
registered HouseOperation event. Native OTI errors roll the entire step back. Exact
historical step replay returns original item/recipient/ordinal without redelivery or a new
authority grant. DRAINED requires full retained census delivery and no source item; it
**retains ownership and the content fence** pending actual final reconciliation.

### Qualification and authority model

`tests/house_move_out_postgres.rs` uses the native private-root path-loaded pattern already
used by character_authority_postgres. Its explicit crate gate defers autodiscovery until CP
registers the two owned durability children. Disabled-target metadata is only build hygiene;
zero disabled tests are not a behavioral pass. An outside-tree exact-source snapshot removes
only that gate and adds literal module registrations for separate enabled standalone metadata
qualification. Native-root case execution retains the exact marked test body; normal rlib
validation removes only #[test] attributes and appends a dispatcher, avoiding the known
full unit-test codegen OOM. Every transform and source hash is recorded; no native visibility,
authority implementation, feature, guard or production source is patched for validation.

The fourteen PostgreSQL cases cover: notice stale lease/changed binding/conflict/due/first cause;
unloaded101 mixed-reclaim items split100+1 and stable replay; missing native cause full
rollback/retry; frozen source/manifest tamper/deferred proof and runtime privilege refusal;
stale Channel and changed native owner; same-step race and same-house conflict; real
PREPARED entrant census/abort/post-freeze entry refusal; bounded2001-item refusal; real
COMMITTED canonical house session independent of released reservation; and a different
current World Channel worker with wrong-holder/foreign-World negatives. Four cancellation cases add stale
lease and cross-kind CommandRef reuse, exact retained replay after a fresh notice, immutable
first cause/time, both before-due and overdue cancel/freeze races, and an independently current
successor connection cancelling an older notice. The last uses a DB-owner persisted generation
operator with native guards active; it qualifies fence consumption, not reconnect E2E.

Fixture history is explicit: only preexisting ownership/item history is authored under
setup-only replication mode; all candidate operations run with all native guards enabled.
The second reclaim subject is created through actual native source observation and
bootstrap_character, including immutable creation/audit/name facts. This local source fixture
does not qualify Platform TLS/producer delivery. A retained earlier-day NOTICE fixture avoids
a day-long wait but is never current authority. The native game_house_access no-row stub
means COMMIT refusal alone cannot qualify the new fence. The actual pre-freeze PREPARE
is the positive control; a fresh post-freeze PREPARE has typed database refusal. A
separate INSERT through the actual native handoff trigger checks SQL23514 and the fence
message; production error mapping remains private. A separate owner-derived test-only
access function enables the actual canonical handoff/session positive control; it does not
qualify production grants/access policy or exits.

AuthorityInvariant × ConsumerBoundary × MutationOperator: identity/binding (CommandRef,
house World/key, operation, owner revision, source/reclaim snapshots); current liveness
(actual recovery, Character lease/session for notice, node and independent assigned World
Channel for jobs, canonical occupants/reservations before custody); temporal/provenance
(DB due time, immutable first cause/census, native same-transaction source→Inbox proof; strict cancellation deadline and retained terminal history).
Missing cause, stale lease/Channel/session, mismatched World/holder/owner, changed immutable data,
forged moving proof and concurrent/replayed step have actual focused negatives. Current
authority is never reconstructed from the retained disposition or supplied as a boolean.
Cancel/freeze races use two actual ready native Roots whose real reconciled authorities are
opened before racing mutation passes, avoiding a fixture race for one idle pool holder.
A session is NOT_APPLICABLE to accepted World-job execution; historical replay grants no
current controller. Native admission EXCLUSIVE relation locks serialize Root passes; a
reviewed hypothetical actor/property deadlock was rejected on that actual boundary.

Final exact-source qualification: **14/14 PostgreSQL cases PASS, zero ignored**, including
real native source→Inbox commits,100/101 batching, rollback, replay, races and cancellation.
Whole production library metadata, original gated target metadata, **enabled original
standalone private-root target metadata**, exact native original-case Clippy with warnings
denied, normal cfg(test) native rlib and real-case driver compilation all PASS. Scoped
rustfmt, governance validator,124 governance tests, diff/scope/preserved-prefix checks PASS.
This is bounded native/private PostgreSQL qualification, not full gameplay/runtime E2E or
Cargo all-target/full unit-test execution. The explicit original target gate yields no
behavioral tests and is never counted as a test pass.

Evidence: `/workspace/oteryn-prep/house-1b-evidence/index.md`,
`entrant-green-qualification.json`, `entrant-green-pg-tests.log`, final source hashes and
whole-owned patch. Earlier initial setup/compile0/10, intermediate8/10, test-only lint and
13/14 entrant fixture-order failures remain real raw records, separately classified, not
product RED. The final repair captures the actual native PREPARED row before COMMIT, whose
access refusal can delete it; native COMMIT/abort semantics and all guards stay unchanged.
Root owns final independent closure and freeze/publication choice; READY_TO_FREEZE applies
to this bounded component only, with every production integration gate below retained.

### Exact production integration gates; no completed release claim

1. CP must compose/install the private schema and literal durability child registrations,
   production privilege boundary and registered **HouseOperation event**. Private atomic
   custody evidence does not replace the shared registry/audit contract. No current runtime
   caller or production permission is installed; no production mutation is claimed.
2. Actual HOUSE1a acquisition/property/Account-slot reconciliation and final owner release
   must consume the complete drained census, retain immutable outcomes and clear the fence
   atomically. Native0038 has an active same-World owner and strict successor guard, not the
   complete accepted tenancy/slot writer; prepared HOUSE1a is arithmetic only, not merged API.
   Owner stays present here; restart/retry reads retained private rows, not a claimed final release.
3. Independent current house map/content activation and house content projection revision,
   actual grants/guild policy and runtime exit/quiesce must be bound by their owners. Native
   handoff Entry+reservation and canonical session census exist; ExitNotAdmitted remains.
   No closure/open/quiescent/current-guild boolean or forged HousePresenceProof is accepted.
4. BED-0 §6 disposition freeing belongs in the same fence transaction, before any owner
   change; preserve first freed cause/time in native Unix milliseconds and retain sleepers for
   actual admission settlement. This subset does not install that hook, regenerate food/vitals
   or grant XP. It must not run as production until composed with the BED owner.
5. Rent/eviction eligibility, reset per-item retirement and retained final outcomes
   remain additional accepted lifecycle work. Bank-only house burns use the shared registered
   HouseOperation cause; current FeeBurnCause::CharmUnassign is not reused or invented.
   No direct resale, guildhall, ACL/wire hold bypass, reset/overlay writer or Inbox callback.

## Root reconstruction regression candidate (2026-10-16, not executed)

The owner confirmed exclusive authorship for the House/Bed family; a separate lease ID is
not required. This batch changes only `tests/house_move_out_postgres.rs` and this card on
the existing `skeleton/house-1b` candidate based on
`37c1879f3e431e5b23a64700bd3285d2bc01c1cd`. No production implementation, shared harness,
registration, schema, permission, migration, event registry or protocol source is changed.

`reconstructed_roots_resume_frozen_and_partial_custody_without_redelivery` adds a native
Root reconstruction scenario at three durable cuts: after freeze, after the first100 of101
mixed-reclaim deliveries, and after DRAINED. Each reconstruction connects a distinct native
Root and restores the real assignment-writer slot, replaces both fixture holders, and asserts
the old Root's actual liveness weak reference has expired before any resumed disposition
command. Readiness restoration can overlap the old Root; no old Root survives resumed work.
Every subsequent pass opens a fresh reconciled Character authority from the retained native
recovery seal. The PostgreSQL server, local recovery store and Node incarnation remain live;
this does not qualify an OS/process crash, PostgreSQL restart or Node-incarnation replacement.

The scenario retains the earlier-day NOTICE fixture's original operation, cause, command
binding, owner/revision and DB times, rather than fabricating a current grant from them. It
checks unchanged freeze replay, rejects a stale execution generation before a new step,
replays the committed100-item step after reconstruction, delivers the last item once, and
replays both completed steps after another reconstruction. It verifies the frozen count/digest,
all101 native Inbox delivery records and exact reclaim-recipient/ordinal matches, the50/51
recipient split and committed counters, and the original owner/revision plus typed native
content-fence refusal after DRAINED. A new step beyond the terminal cursor is rejected.

Validation status: official Rust1.94 rustfmt parsing/formatting and `git diff --check` passed.
The new case has **not been compiled or run**; no Cargo/heavy build was authorized for this
batch. Existing14-case results above remain historical exact-source evidence and do not
cover this added case. The original target still has `#![cfg(any())]`, so its disabled Cargo
target cannot qualify the new scenario. Required next validation is an enabled, source-bound
native private-root test registration and execution on PostgreSQL17.6, including this case
and the prior14 cases, with independent exact-candidate review before freeze/publication.

This remains a regression candidate over current0038 ownership and uninstalled private
custody. It cannot satisfy restart against the unmerged HOUSE-1a acquisition/property and
Account-slot writer or final owner release. Actual tenancy/rent/eviction, canonical ownership
reconciliation, registered HouseOperation event, BED same-transaction freeing, runtime exit
and content revision, installation/grants and the CP slot remain the production gates above.

### Explicit real Guild release integration authoring successor

Root authorized this material source generation from frozen `7765540c3e50dea77029b940827de49799e3c7d9` in the isolated `/workspace/house-1b-release-authoring` checkout. The frozen candidate and PC source snapshots remain unchanged. This batch owns only `src/house/move_out.rs`, its focused `tests/house_move_out_postgres.rs` case and this card; the separately allocated `transfer.rs` path requires no edit in the final design. No registration, migration or shared/D path is changed.

The final agreed boundary preserves the ordinary MOVE_OUT schema, states, Character/session/notice contract, census, views and replay behavior. HOUSE-1a owns the distinct typed Guild operation/census/steps, actual native Inbox delivery, protected reservation and terminal release journal. Only that real Guild operation has terminal state5. Ordinary DRAINED remains content-fenced.

The existing `game_house_disposition_fence_check` now also consults the actual Guild owner through `game_house_guild_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) RETURNS VOID`. The actual native item mutation and House entry triggers already call this common function, including native custody DELETEs. A present `game_house_guild_dispositions` relation or the actual legacy `owner_guild_id` column requires both the typed relation and exact VOID-returning owner function; partial or incompatible installation refuses with23514. Schema-qualified lookup and execution use the function's existing pinned search_path. With neither Guild relation nor legacy Guild owner column installed, the ordinary native fixture continues to use its existing guard; no fake Guild table or test hook is introduced. Native provenance INSERT/UPDATE/DELETE also uses the same boundary, and native ItemInstance UPDATE/DELETE consults it for its actual House location. The common ownership lock uses `FOR SHARE NOWAIT`: Root writers prelock the actual ownership anchor, while a late native row trigger refuses55P03 instead of waiting in an item/provenance-to-property cycle. The error propagates and aborts its transaction; this layer never catches it to proceed or retries automatically. A successfully acquired shared lock remains transaction-held and excludes a concurrent freeze. The ABI is a protected read-only constant: PUBLIC execution stays revoked, and only its EXECUTE is granted to `oteryn_game_runtime` for the genuine participant's direct read; trigger, table and release authority is not granted. The strict LO-01 class-order contract still needs its explicit narrow native-trigger NOWAIT-property-pin exception before production integration; no Root/Guild lock is acquired in that fallback, and no shared contract file is edited in this owned batch. The protected `game_house_disposition_guild_fence_abi()` version1 is installed with these real guards; the typed Guild owner must require it before freezing or applying effects, so an older ordinary-only fence is not a valid integration.

One focused new native case drains an empty ordinary house through the real Root and step machinery, confirms the absent Guild component, requires23514 for both entry and unauthorized-delete fence checks after DRAINED, and verifies retained step replay leaves the native counts unchanged. A second case verifies ABI1 is readable under the actual runtime role without direct common-fence execution authority, using SET LOCAL ROLE and rollback. It first holds the actual ownership anchor from a competing transaction and requires55P03 from real ItemInstance/provenance mutations, then rolls back that holder. It requires the actual content fence to reject native ItemInstance UPDATE/DELETE and provenance DELETE while frozen, then admits the real same-transaction custody return. Actual Guild freeze/delete/entry/release and missing-owner integration cases belong to the HOUSE-1a author using its real typed schema. Native PostgreSQL qualification of this new generation remains required; source formatting/static checks are not a behavioral pass. No Cargo, PC execution, push or publication is part of this authoring batch. After the read-only ABI runtime grant repair passed SQL review, Root explicitly authorized a local Git freeze commit and incremental bundle for the D handoff.

## Complete recoverable native evidence for the canonical 24-case run

The following gzip/base64 archive preserves the exact pre-run publication packet,
runner, formatter receipt, source manifests, independent source review, final native
result, all seven command logs and root readback verification. Decoding recovers
304652 UTF-8 JSON bytes with SHA-256 `814710e6bee6b19369150adf642068ed75e29fca93036c900a84740f298a0b49`.
Each entry records its own path, byte length, SHA-256 and complete content. The
archive is data; recovering it does not authorize executing an embedded script or
applying an older source freeze over a later checkout.

Recover the text between the standalone BEGIN/END marker lines, remove its
`text` fence and join the base64 lines, then decode base64 and gzip. Verify the
JSON byte hash above and each entry hash before using it. The original native
source freeze remains preserved even though this card now includes the receipt.

<!-- HOUSE-1b-native-d1413-evidence BEGIN -->
```text
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
egi7DKYEAA==
```
<!-- HOUSE-1b-native-d1413-evidence END -->
