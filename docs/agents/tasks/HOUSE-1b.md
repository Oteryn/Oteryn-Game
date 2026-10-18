# HOUSE-1b — house ownership second half (skeleton task record)

```yaml
task_id: HOUSE-1b
title: "HOUSE-1b: house ownership second half (skeleton)"
mode: IMPLEMENT
status: skeleton
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

## 2026-10-17 D1413 composed Inbox inventory and native execution packet

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
