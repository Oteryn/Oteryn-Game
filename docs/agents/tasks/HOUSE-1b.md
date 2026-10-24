# HOUSE-1b — canonical house disposition and move-out (consolidated task card)

```yaml
task_id: HOUSE-1b
title: "HOUSE-1b: canonical house disposition and absolute move-out commands"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
class: "hard (persistence, economy, security)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row HOUSE-1b (row 119); App. 7 HOUSE-*; §5 Wave 5"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md   # §5-§9, §15
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md # §3
  - docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md                    # §0.1, §2.4 (HOUSE-1b)
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_RT_INBOX_PACKETS_2026-10-04.md                # §1.2, §1.6
  - docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md              # §6 disposition freeing
base_branch: main
branch: claude/happy-davinci-ijz3wy-srv-house1b
pr: null
needs_migration: true
needs_registry: true   # protocol 76/77, capability 59; extends existing event 8
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **OPEN**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: none. Inbox delivery exists; no HOUSE-1b move-out/disposition owner is retained. Evidence: `crates/gameplay/durability/src/durability/character_inbox.rs`.

Remaining: Notice/cancel, bounded freeze census, exact reclaim-subject disposition and restart-safe release remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2141, branch skeleton/house-1b

## Goal

Second half of house ownership (SOCIAL-MAP-PACKETS HOUSE-1b): the canonical ordinary house
disposition producer, absolute move-out notice/cancel owner commands, freeze of the house
item census and bounded custody return of every House item to its actual **reclaim subject**
through the native Character Inbox. Direct player resale is deferred by HOUSE-OWN-0, so
"transfer" here means HouseInterior item custody to each item's captured reclaim subject,
never an owner fallback and never a priced sale. The skeleton's "transfer atomic with the bank
ledger" and "owner Inbox" wording is superseded by that ruling (CP correction).

## Dependencies (re-verified on origin/main, 2026-10-22)

| Dependency | State |
|---|---|
| INBOX-1a (native `game_character_inbox_deliver`, migration 0076) | MERGED (#1785) |
| BANK-1 | MERGED (#1766) |
| ACCEPT-SOCIAL-MAP-0 | MERGED (#1771) |
| HOUSE-RUNTIME-1a (house scope handoff) | MERGED (#2163) |
| HOUSE-1a (acquisition, property, Account slot, final owner release) | BLOCKING for final release — re-implementation pending (draft #2137 closing) |
| HOUSE-ACL-1 (`game_house_access` grants) | BLOCKING for the waiting case and owner-command permissions — re-implementation pending (draft #2140 closing) |
| BED-1 (sleeper freeing in the fence transaction) | BLOCKING before production use — re-implementation pending (draft #2147 closing) |
| MAP-OVERLAY-1c (house map/content activation) | BLOCKING for content revision binding — re-implementation pending (draft #2167 closing) |
| GUILD-1 Guild core | MERGED (#2136) as checkpoint, but `domain/guild.rs`/`durability/guild.rs` were dropped by the ADR-0022 move (#2280); must be re-landed before any Guild disposition arm |

Integration order decided by CP (D1162/D1349): HOUSE-1b → HOUSE-1a → MAP-OVERLAY-1c → BED-1.
HOUSE-1b lands first because it owns the canonical disposition schema that HOUSE-1a consumes.

## Owned paths (ADR-0022 layout)

The 2026-10-23 server-first owner instruction supersedes the original standalone PostgreSQL
target planned below. The actual delta uses the existing dispatch `--lib` auto-mount; its full
owned-file manifest appears at the end of this card. No new test target or client path is added.

- `crates/gameplay/housing/src/house_move_out.rs` (new, L3 pure rules: notice deadline bounds, notice/cancel state machine)
- `crates/gameplay/housing/src/house_move_out_tests.rs` (new)
- `crates/gameplay/housing/src/house_transfer.rs` (new, L3 pure rules: census batching, reclaim-subject step planning)
- `crates/gameplay/housing/src/house_transfer_tests.rs` (new)
- `crates/gameplay/durability/src/durability/house_disposition.rs` (new, private PostgreSQL bridge and `apply_house_move_out_command` owner command)
- `crates/gameplay/durability/src/durability/house_disposition_tests.rs` (new)
- `apps/game-server/tests/house_move_out_postgres.rs` (new)
- `apps/game-server/tests/support/house_move_out_postgres_cases.rs` (new)
- `apps/game-server/tests/support/house_move_out_waiting_postgres_cases.rs` (new; unmounted until HOUSE-1a + HOUSE-ACL-1)
- `apps/game-server/tests/support/character_inbox_postgres_cases.rs` (existing; D1413 reader-list hunk only)
- `docs/agents/tasks/HOUSE-1b.md`

Slot-only (final commit at the queue turn): the canonical disposition migration (number:
next free number at allocation time (only one migration PR in the queue)); module
registration in `crates/gameplay/housing/src/lib.rs` and
`crates/gameplay/durability/src/durability/core_modules.rs` / `lib.rs`; Cargo test-target
mount; `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (HouseOperation event).

## Wiring points (verified on origin/main)

- `crates/gameplay/durability/src/durability/character_inbox.rs` (native Inbox writer, migration 0076)
- `crates/gameplay/durability/src/durability/house_interior.rs`, `house_interior_audit.rs` (0025/0095 custody shapes)
- `crates/gameplay/durability/src/durability/house_spell_acl.rs` (0038 ownership anchor and owner revision)
- `crates/gameplay/durability/src/durability/house_scope_handoff.rs`, `crates/gameplay/housing/src/house_scope/handoff.rs` (Entry + reservation, canonical session census)
- `crates/gameplay/durability/src/durability/runtime_scope_assignment.rs` (assigned execution Channel)
- `apps/game-server/tests/support/house_test_fixture.rs`, `house_scope_handoff_postgres_cases.rs`, `character_inbox_postgres_cases.rs`
- HOUSE-WIRE-1 maps the command to the wire only after HOUSE-1b merges.

## Accepted decisions carried from the draft

- **D1382 / D1403 (canonical installer):** one migration installs the three disposition tables
  (operation, census, step) and the ordinary operation, census, step and real Inbox-delivery
  functions before the native removal predicate. The Rust private-schema constants are removed;
  MIGRATOR is the only installer. The existing 0095 player DELETE arms stay unchanged; only the
  new ordinary-disposition DELETE arm is added. No PUBLIC/runtime/control grants except the
  read-only ABI below. HOUSE-1a must not install any private ordinary disposition fragment.
- **Native removal proof:** the new DELETE arm requires the immutable source, the current physical
  transaction, a matching disposition/step, an unchanged item, real Inbox delivery to the captured
  provenance reclaim subject, and absence of the old House location/provenance.
- **D1413:** add `game_house_disposition_delivery_proven` and
  `game_house_disposition_native_removal_proven` to the sorted exact Inbox reader list (and fix
  the adjacent comment). Do **not** add `game_house_disposition_deliver`. Re-check the composed
  sorted union right before push; later MAIL/TRADE entries are retained, never overwritten.
- **D1384 (owner command):** `DurabilityRoot::apply_house_move_out_command` takes recovery
  authority, Node proof, the full current fence, the original CommandRef, HouseId and
  `HouseMoveOutCommand::{Notice { due_at_unix_ms }, Cancel}`.
  - One DB clock sample; inclusive bound `86_400_000 <= due - now <= 2_592_000_000` (1-30 days),
    checked arithmetic, no rounding.
  - Replay under the CommandRef lock returns the original binding and outcome before re-reading
    the clock or ownership. The server mints each operation; Cancel targets the actual pending notice.
  - `read_house_move_out_owner` is a read-only projection.
- **Trigger naming:** `a_house_disposition_{content,provenance,item,entry}_fence`, so the
  `FOR SHARE NOWAIT` 55P03 refusal fires before the generic DUR-03 23514. HOUSE-1a's catalogue
  triggers follow in D1162 order.
- **D1349:** canonical modules are move-out and transfer (now `housing::house_move_out`,
  `housing::house_transfer`).
- **D1367:** the durability bridge is private and does not widen `record_for`,
  `assert_recovery_fence`, `lock_admission_relations`, `scope_key` or node-secret access.
- **D1366:** NON-PG routing for the pure case; **D1371:** Cargo and aggregate mount lines are slot-only.
- **D1409:** qualification inventory is 24 cases (1 pure, 22 House PG, 1 Inbox PG).

## Reference behaviour built by the draft (re-implement, do not copy)

Notice and cancel:
- A notice uses actual recovery, current Character/session/connection/lease/Channel fences and
  the 0038 owner/revision; it binds CommandRef, operation, original fences, house, notice days
  and owner revision immutably. All persisted times are DB-clock Unix milliseconds.
- Same command/operation with a changed binding, or another operation for the same live
  property, is a semantic conflict. A partial live-house uniqueness rule allows a fresh notice
  after cancel; an old cancel replay never affects the fresh operation.
- Cancel is allowed only before the DB-clock due time, with current owner/session fences; it
  retains CommandRef/original-fence binding and an immutable cancelled time/outcome. At/after due
  or after freeze, cancel is closed. CANCELLED history keeps first cause and original notice.
- Notice/cancel reuse `character_item_fence_is_current`, whose `scope_of` admits Channel sessions
  only. House-scoped owner commands need the admitted House grant/session owner (HOUSE-ACL-1),
  never a parallel weaker predicate. World jobs have no actor-session requirement.

Freeze and custody steps (World jobs):
- Run under the current node incarnation and an assigned **execution Channel in the same World**,
  not a destination House assignment or actor session — unloaded houses and another legitimate
  Channel of the World can run the job; wrong holder and foreign World are refused.
- Root recovery/admission relation locks, the handoff relation lock and property locks protect
  the census. Freeze reads at most 2001 sorted IDs and refuses more than the accepted 2000
  PARITY_PENDING bound; it retains exact source location/provenance/item snapshots plus count
  and SHA-256 of sorted ItemInstanceIds. Content and entry fences then block source mutation and
  any new PREPARED/COMMITTED entrant. First cause and notice binding are immutable; only
  MOVE_OUT is admitted.
- A step locks recipient Character roots, then property, then items by ItemInstanceId, then Inbox
  counters by CharacterId. It moves at most 100 unmoved census items (101 items → 100 + 1).
- The SECURITY DEFINER step verifies current owner/source, canonical sessions and unreleased
  PREPARED/COMMITTED entrant reservations, removes the 0025 source/provenance and calls the real
  0076 `game_character_inbox_deliver` in the same transaction. Native errors roll back the whole
  step. Exact historical step replay returns the original item/recipient/ordinal without
  redelivery or a new authority grant.
- DRAINED requires full census delivery and no source item; it **keeps ownership and the content
  fence** pending final reconciliation by HOUSE-1a.

Guild fence ABI (agreed boundary with HOUSE-1a):
- `game_house_disposition_fence_check` also consults
  `game_house_guild_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) RETURNS VOID` when a
  `game_house_guild_dispositions` relation or the legacy `owner_guild_id` column exists; partial
  or incompatible installation refuses with 23514. With neither present, the ordinary guard
  applies; no fake Guild table or test hook.
- Item mutation, provenance INSERT/UPDATE/DELETE and House entry triggers all call the common fence.
  The ownership lock uses `FOR SHARE NOWAIT`: Root writers prelock the ownership anchor, a late
  row trigger refuses with 55P03 instead of waiting (no item/provenance-to-property cycle); the
  error aborts the transaction and is never caught or auto-retried.
- `game_house_disposition_guild_fence_abi()` version 1 is a protected read-only constant: PUBLIC
  execution revoked, EXECUTE granted only to `oteryn_game_runtime`. The typed Guild owner must
  require it before freezing. Guild operation/census/steps, terminal state 5 and the release
  journal belong to HOUSE-1a; ordinary DRAINED stays content-fenced.

## Findings and open questions

- **LO-01 lock-order exception (open):** the strict class-order contract needs an explicit narrow
  native-trigger NOWAIT property-pin exception before production integration. Not edited here.
- **Production integration gates (all remain open):**
  1. CP installs the schema, durability child registrations, the production privilege boundary and
     the registered **HouseOperation event**; private atomic custody proof does not replace it.
  2. HOUSE-1a acquisition/property/Account-slot reconciliation and final owner release consume the
     drained census, retain immutable outcomes and clear the fence atomically.
  3. House map/content activation and content revision (MAP-OVERLAY-1c), grants/guild policy
     (HOUSE-ACL-1) and runtime exit/quiesce (`ExitNotAdmitted` remains) bound by their owners. No
     closure/open/quiescent/current-guild boolean or forged HousePresenceProof is accepted.
  4. BED-0 §6 disposition freeing in the same fence transaction, before any owner change, keeping
     first freed cause/time; no food/vitals regeneration or XP here.
  5. Rent/grace/eviction eligibility, per-item reset retirement and retained final outcomes.
     Bank-only house burns use the registered HouseOperation cause; `FeeBurnCause::CharmUnassign`
     is not reused.
- No direct resale, guildhall, ACL/wire bypass, reset/overlay writer or Inbox callback is in scope.
- **Test fixtures:** only pre-existing ownership/item history is authored in setup-only replication
  mode; every candidate operation runs with all guards enabled. A second reclaim subject is created
  through `bootstrap_character`. An earlier-day NOTICE fixture avoids a day-long wait but is never
  current authority. The native `game_house_access` stub returns no rows, so COMMIT refusal alone
  cannot qualify the entry fence — pre-freeze PREPARE is the positive control and a fresh
  post-freeze PREPARE must give the typed refusal.
- Cancel/freeze races need two real ready Roots opened before racing, not one idle pool holder.
- Draft result: 24/24 PASS on PostgreSQL 17.6 for its exact source (an earlier assembly was
  10 PASS / 6 FAIL on 23514 before the trigger rename). This is not runtime/E2E qualification.

## Required test inventory (re-implement)

- Pure: absolute deadline boundary (1 day / 30 days inclusive, overflow).
- House PG (22), including: frozen item/provenance share content fence; drained fence without guild
  owner; notice fenced/bounded/idempotent; unloaded house with 101 mixed-reclaim items returned once
  (100 + 1, 50/51 recipient split); reconstructed Roots resume after freeze, after the first 100 and
  after DRAINED without redelivery; Inbox failure rolls back; source/manifest unforgeable and runtime
  privilege refused; job scope vs ownership revision independent; concurrent same step; prepared
  entrant blocks custody; 2001-item house refused; another World Channel processes an unloaded house;
  owner cancel; overdue and before-due cancel/freeze races; successor connection cancels an older
  notice; the five D1384 owner-command cases; one native-removal regression; Guild ABI readable by
  runtime role without common-fence EXECUTE, and 55P03 under a held ownership anchor.
- Inbox PG (1): `inbox_rows_are_immutable_and_no_other_writer_touches_them` with the D1413 list.
- Waiting (unmounted): `actual_committed_house_session_blocks_after_reservation_is_released`.

## Acceptance criteria

- [x] Move-out returns every House item exactly once to its captured reclaim subject's Inbox (no owner fallback).
- [x] Notice/cancel bounded by one DB clock sample; replay returns the original outcome.
- [x] Freeze census bounded (2000) and immutable; steps bounded (100) and replay-safe.
- [x] Restart mid-disposition resumes from durable state without redelivery.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [x] DRAINED keeps ownership and content fence; final release is HOUSE-1a's.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.
- [x] One mechanic per module file in its ADR-0022 layer.
- [x] New tests in their own child files.
- [x] Edits to mod.rs/lib.rs/connection.rs/capabilities.rs are wiring-only.
- [x] No file over ~1000 lines grows by more than ~150 lines.

## Validation commands

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-housing house_move_out
cargo test -p oteryn-gameplay-housing house_transfer
cargo test -p oteryn-gameplay-durability house_disposition
cargo test -p oteryn-game-server --test house_move_out_postgres
cargo test -p oteryn-game-server --test character_inbox_postgres
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- docs/architecture/reviews/OTERYN_GAME_SOCIAL_MAP_PACKETS_2026-10-04.md (HOUSE-1b)
- docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md
- docs/agents/tasks/archive/OTV2-20261010-house-1-contract.md
- docs/reference/tibia-manual/houses.md
- Draft #2141 (decisions D1349, D1366, D1367, D1371, D1382, D1384, D1403, D1409, D1413, D1415 on CP PR #2131)

## Server wiring gap list

Audit on `cb5b3de92d2e19df53e05a7e0f5b9d98fa11d299` (2026-10-23), before implementation. The E2E audit row
`MECHANICS_STATUS.md: houses` says BACKEND; source inspection confirms this residue.

| Layer | Pre-change HEAD | Implemented server delta |
|---|---|---|
| L3 | House custody, handoff, auction rules | Absolute move-out notice/cancel and bounded transfer rules |
| L4 / migrations | 0038 owner anchor, 0025/0095 custody, 0076 Inbox; 0143 acquisition | Canonical disposition operation/census/step and fenced owner writer (0178) |
| Dispatch / runtime | Admitted Channel runtime, current item fence, house handoff | Mechanic handler and due-job caller |
| protocol-oteryn | No move-out command or capability | Bounded request/result codec; reserved commands 76–77, capability 59 |
| Capability | No house disposition offer | Offer in all three production sets, including bundle/default Worlds; no loaded-House prerequisite |
| Qualification | Foundation tests only | Real admitted dispatch notice/cancel, custody/replay/restart PostgreSQL E2E |

AUTHORING only; local worker edits, no publication or frozen candidate in this session.

## Server basic-form result (2026-10-23, local AUTHORING)

Implemented locally; no commit, publication, frozen SHA or PR was created. The basic server
path is `ClientCommand(76/77) -> serve_admitted -> house_move_out -> DurabilityRoot ->
0178 native port -> CommandResult`. The existing source Channel owner cycle discovers due
operations, commits freeze, then drains one bounded batch per house per pass. It needs a
current execution Channel in the owning World, never a loaded House assignment or an actor
session for the job. Pending work and the scan cursor survive reconstructed Roots.

A logged-in owner can schedule an absolute 1–30 day notice, cancel the actual pending notice
before its due instant, and query its durable state/counts. No client identity, owner revision,
reclaim subject, operation identity or authority boolean is accepted from the wire. There is
no price, direct resale or client UI change. Capability 59 has no spatial-content dependency;
existing native ownership must authorize every command, including in the default/bundle World.

Canonical operation, command receipts, census, steps and Foundation event 8 are installed only
by MIGRATOR (0178). Canonical tables and producer functions remain private. A narrow runtime
SECURITY DEFINER ingress independently verifies Node possession, execution scope and full
session/admission fences; the Rust bridge also retains `character_item_fence_is_current`.
Historical receipt replay precedes clock/owner/current-fence reads. New commands, checkpoints
and every native item removal require a same-physical-transaction HouseOperation event.
The ordinary source removal predicate retains both 0095 player arms. The exact Inbox reader
inventory gains only the two read-only disposition proofs.

Freeze snapshots at most 2000 exact item/location/provenance records and their sorted-ID
SHA-256. Item, provenance, location and entry triggers retain the content fence through
DRAINED. The freeze transaction frees open bed sleepers with the existing `disposition`
terminal cause. Custody waits for canonical live House sessions and unreleased PREPARED or
COMMITTED entrants; it never invents quiescence. Steps retain (operation, step, item, subject,
ordinal) and call the real Inbox writer. Errors abort the entire house transaction. A job
pass has a retained cursor and at most 100 houses, so an occupied/oversized house cannot
starve all later houses. Each house commits separately; there is no native same-pass retry.

### Reference parity

| Visible behavior | Reference evidence | Oteryn now |
|---|---|---|
| Schedule moving out 1–30 days ahead | `docs/reference/tibia-manual/houses.md`, Getting Houses / Moving Out; accepted HOUSE-OWN-0 §6 | Command 76 Notice uses one DB-clock millisecond sample, inclusive bounds and checked L3 arithmetic |
| Cancel an outstanding owner notice | D1384 and this card's accepted owner-command contract | Command 76 Cancel targets the actual pending operation, closes at due/freeze, retains original replay outcome |
| Automatic return of remaining belongings | Manual Moving Out; HOUSE-CUSTODY-0 §3.2 | Due World job returns each item to its captured reclaim subject, with no owner fallback (accepted Oteryn difference) |
| Bounded, restart-safe custody work | HOUSE-OWN-0 §§7,9 and HOUSE_RT_INBOX_PACKETS §1.2 | Exact immutable census <=2000; steps <=100; native Inbox, immutable step receipts, fresh-Root reload |
| House cannot acquire new contents/entrants while draining | Accepted disposition/fence and handoff contract in this card | Native content/item/provenance/entry fences; prepared entrants prevent delivery |
| House becomes vacant and may be acquired again | Manual Moving Out; HOUSE-OWN-0 §7 release | Intentionally remains DRAINED with owner/fence retained; final reconciliation is HOUSE-1a's owning step |

### Validation evidence

Evidence is for the local worktree delta, not a published/frozen candidate. Commands use the
absolute queued wrapper `/home/mole/oteryn-cache/codex-bin/cargo`, toolchain from
`rust-toolchain.toml`, `--locked` and offline dependency resolution. PostgreSQL tests execute
against the wrapper-provided PostgreSQL 18.6 route; no route-less early return is qualification.

- `cargo test --locked -p oteryn-protocol-oteryn --lib` (includes filter `house_move_out`, registry/capability mapping).
- `cargo test --locked -p oteryn-gameplay-housing --lib` (includes filters `house_move_out` and `house_transfer`).
- `cargo test --locked -p oteryn-gameplay-dispatch --lib capabilities::tests`.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib house_move_out_postgres`:
  `house_move_out_postgres_real_dispatch` and `house_move_out_postgres_oversized_census`.
  Real dispatch covers query/notice/cancel, original-result replay and changed binding,
  cancel replay isolation from a newer notice, malformed/wrong-kind/no-capability/non-owner/
  deadline rejection, and the mechanic's stale lease check. The connection's independent
  stale-lease Bank prelude disconnects before command dispatch; that direct-handler negative
  case is explicitly separate from the wire scenario. A second Character is created by the
  actual authenticated bootstrap. The historical 101-item house drains 100 + 1 with a 50/51
  subject split, reconstructing Root/owner at freeze, partial drain and DRAINED; the command and restart phases
  use `oteryn_game_runtime` (the test also verifies the connection role). Native guards prove entry/provenance/item/DRAINED fencing,
  prepared-entry waiting, injected Inbox failure rollback, no redelivery and retained ownership.
  The 2001-item case refuses without any census row or lost item.
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres inbox_rows_are_immutable_and_no_other_writer_touches_them` (D1413 exact reader inventory).
- `cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-housing -p oteryn-gameplay-durability -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets -- -D warnings`.
- `cargo fmt --all -- --check`; explicit rustfmt check of include-mounted durability and dispatch test children.
- `python3 tools/agents/validate_governance.py`; `python3 -m unittest discover -s tools/agents/tests`;
  `python3 tools/repository/validate_repository_policy.py`; `git diff --check`.

Whole-diff D1462 checklist and workspace constructor/match/impl sweep cover the new L3 command,
durable error, wire request/reply, private dispatch variant/authority method and event fields.
Each production public function has a server caller and is reached by the dispatch E2E.
Protocol ids are exclusively 76/77 and 59; new resource entries are exclusively 0890–0894.

### Unfinished / integration boundary

- HOUSE-1a owns final owner/Account-slot/ACL reconciliation, fence release and reacquisition.
  Acquisition/catalogue activation and House-scoped admission remain their owners' gates;
  this change neither manufactures ownership nor bypasses House grants.
- House-scoped owner commands and actual committed-House-session exit/quiescence qualification
  need HOUSE-ACL-1 / HOUSE-RUNTIME-1b/1c. This implementation accepts current Channel sessions;
  existing House sessions/reservations keep custody closed.
- LO-01's accepted lock-order exception, full original 24-case concurrency/foreign-Channel/
  successor-connection/positive-sleeper qualification, and independent exact-head review remain
  required before production integration. The original historical draft's 24/24 is not reused
  as evidence for this implementation. No unrelated PG suites were run on the shared PC.
- Rent/eviction, Guild disposition and direct resale remain outside the ordinary move-out path.
  The real Guild fence ABI stays fail-closed for partial/incompatible installation.
- Client UI is outside this task. No client paths, workflows, policy enforcement, credentials,
  dependency files or Git publication state were changed. The task record remains active for
  CP integration, rather than being archived by an uncommitted worker.

Verified local results: protocol **393/393**, housing **24/24**, capability negotiation
**61/61**, mechanic PostgreSQL **2/2**, D1413 Inbox PostgreSQL **1/1**, governance Python
unit tests **129/129**. The PostgreSQL E2E also checks that native removal and checkpoint
writes without the physical HouseOperation event fail, before the production writer runs.

### Changed-file manifest

```text
apps/game-server/migrations/0178_house_disposition.sql
apps/game-server/tests/support/character_inbox_postgres_cases.rs
crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs
crates/gameplay/dispatch/src/gameplay_transport/capability_quest_log_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/connection.rs
crates/gameplay/dispatch/src/gameplay_transport/house_move_out.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_move_out.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/house_move_out/fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/source_item_cycle.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/house_disposition.rs
crates/gameplay/durability/src/durability/house_disposition/audit.rs
crates/gameplay/durability/src/durability/house_disposition/jobs.rs
crates/gameplay/housing/src/house_move_out.rs
crates/gameplay/housing/src/house_move_out_tests.rs
crates/gameplay/housing/src/house_transfer.rs
crates/gameplay/housing/src/house_transfer_tests.rs
crates/gameplay/housing/src/lib.rs
crates/protocol-oteryn/src/capability_tables.rs
crates/protocol-oteryn/src/house_move_out.rs
crates/protocol-oteryn/src/house_move_out_tests.rs
crates/protocol-oteryn/src/lib.rs
docs/agents/tasks/HOUSE-1b.md
docs/contracts/game-events/v2/house_operation.proto
docs/contracts/protocol-oteryn/v1/house_move_out_v1.proto
docs/contracts/protocol-oteryn/v1/registry/capabilities/0059-house-move-out-v1.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0076-house-move-out.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0077-house-move-out-query.json
docs/contracts/resource-limits/entries/0890-HOUSE1B-RL-01.json
docs/contracts/resource-limits/entries/0891-HOUSE1B-RL-02.json
docs/contracts/resource-limits/entries/0892-HOUSE1B-RL-03.json
docs/contracts/resource-limits/entries/0893-HOUSE1B-RL-04.json
docs/contracts/resource-limits/entries/0894-HOUSE1B-RL-05.json
```

Final local checks: clippy `-D warnings` on all five touched crates, rustfmt (including
auto-mounted children), both governance validators and `git diff --check`: **PASS**.
No CI-only contention failures or pending local check remain.
