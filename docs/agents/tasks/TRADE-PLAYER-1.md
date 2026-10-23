# TRADE-PLAYER-1 — player-to-player trade (consolidated task card)

```yaml
task_id: TRADE-PLAYER-1
title: "TRADE-PLAYER-1: player-to-player trade"
mode: IMPLEMENT
status: waiting   # TRADE-PLAYER-1 slice merged (#2332); remaining slices per card
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row TRADE-PLAYER-1 (row 82); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (TRADE-WIRE-1, TRADE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
base_branch: main
branch: claude/happy-davinci-ijz3wy-trade-player-1
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@43914f183`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: #2332: pure offer/accept/invalidation rules and fenced atomic two-Character leaf exchange, receipts, audit and replay/reconciliation backend. Evidence: `crates/gameplay/economy/src/player_trade.rs`, `crates/gameplay/durability/src/durability/player_trade.rs`, `apps/game-server/migrations/0147_player_trade.sql`, `crates/gameplay/economy/src/player_trade/player_trade_tests.rs`, `apps/game-server/src/lifted_tests/auto/player_trade_postgres_cases.rs`.

Remaining: Live Channel trade book and session-ingress/FND routing, current visibility/LOS/busy/reservation checks, lifecycle invalidation routing, wire/capability/egress and real two-admitted-session/restart dispatch qualification remain. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2132, branch skeleton/trade-player-1

Status note: the draft reached a qualified writer-only stage (D1369) but used the
pre-ADR-0022 `apps/game-server/src/...` layout and the now-immutable migration number
0100. It is closing; the mechanic is re-implemented fresh on the current layout.

## Goal

Two players of one channel exchange items safely: capability, trade command and domain
(TRADE-WIRE-1), trade session with TRANSFERRING state and the two-Character swap
transaction with every persistence delta (TRADE-1).

Children (from the PLAYER_TRADE0 child graph; the plan row is only the family entry gate):
TRADE-WIRE-1 (impl, protocol review, Reg) -> TRADE-1 (hard, persistence and security
review). BAGS-TRADE-1 is an integration child released by TRADE-1 and outside this set.
CP D1369 staged the work: WRITER-ONLY first; common GameSession ingress for all admitted
command families belongs to SESSION-INGRESS-1; wire IDs/capability/domain/consumer
activation follow in a post-ingress slot.

## Dependencies (re-verified on origin/main 59785d76, 2026-10-22)

| Dependency | State |
|---|---|
| ITEM-LOOP-1 (family entry gate) | MERGED (#2190) |
| ITEM-MOVE-2a (TRADE-1) | MERGED (equip slice EQUIP-WIRE-1 #2009; remainder in ITEM-LOOP-1 #2190) |
| ITEM-VIEW-1b, VIS-2 | MERGED (archive records) |
| ADMIT-CAP-1 (precondition of SESSION-INGRESS-1) | MERGED (#2187) |
| MAIL-1 (shared Inbox read-only function inventory) | MERGED (#2133) |
| PLAYER_TRADE0 acceptance | ACCEPTED (PLAYER-TRADE-0 #1365; effective text on accepted main) |
| SESSION-INGRESS-1 (common session-owned ingress) | BLOCKING for wire/activation (not on main) |
| D1377 shared `SpellLanePermit` guard + first equipment consumer | BLOCKING for physical exclusion (after SESSION-INGRESS-1) |

## Code locations (ADR-0022; verified on origin/main 59785d76)

Existing owners to reuse (do not fork):

- `crates/gameplay/durability/src/durability/item_transfer.rs` — `character_item_fence_is_current`
  (line ~885); the draft's D1315 split into fence-only + ordered root lock is NOT on main
  and must be re-done as a wiring-sized change in this file.
- `crates/gameplay/durability/src/durability/item_mint_audit.rs` — native type-2 operation
  mount and outbox insert-site inventory.
- `crates/gameplay/durability/src/durability/runtime_scope_assignment.rs` — `load_assignment`.
- `crates/gameplay/durability/src/durability/character_equipment.rs` —
  `assert_current_equipment_in_transaction` (recovery/session/lease/scope/Node/Content checks).
- `crates/gameplay/entities/src/carrier.rs` — `ChannelRuntimeAssignmentBinding` (line ~259).
- `crates/gameplay/foundation/src/foundation/mod.rs` — `CommandIngress` (FND reserve/terminalize).
- `crates/gameplay/dispatch/src/gameplay_transport/item_view.rs` — `SessionItemView::resolve`.
- `crates/gameplay/dispatch/src/gameplay_transport/item_move.rs` — `observe_inventory`.
- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`, `capabilities.rs` — wiring only.
- `docs/contracts/game-events/v1/native_one_item_transaction.proto` — hand-authored type-2 op.
- `apps/game-server/tests/character_authority_postgres.rs` + `apps/game-server/tests/support/`
  — canonical PG case mount.

Proposed new files (one mechanic per file, layer per ADR-0022):

- `crates/gameplay/domain/src/domain/player_trade.rs` (new) — L0 trade values/ids.
- `crates/gameplay/economy/src/player_trade.rs` (new) + `player_trade_tests.rs` (new) — L3 pure
  lifecycle policy (OFFERED/READY/TRANSFERRING, 120 s idle, tradeable predicate).
- `crates/gameplay/durability/src/durability/player_trade.rs` (new) + child test file — L4
  native writer/reconciler, inventory reads, audit codec.
- `crates/gameplay/social/src/player_trade/` (new) — L4 runtime TradeOwner book/issuance
  (or the L4 owner CP assigns); `crates/gameplay/dispatch/src/gameplay_transport/player_trade.rs`
  (new) — L5 command handler only.
- `crates/protocol-oteryn/src/player_trade.rs` (new) + tests — TRADE-WIRE-1.
- `apps/game-server/tests/support/player_trade_postgres_cases.rs` (new).
- Migration: next free number at allocation time (only one migration PR in the queue).

## Accepted design (PLAYER_TRADE0 §§4–6 and CP decisions, as built in the draft)

Lifecycle policy:

- `OFFERED` holds the first reference; a revalidated counter-offer enters `READY`. Both
  accepts bind the same immutable pair. The internal pair key was the native counter-offer
  `CommandRef`; TRADE-WIRE must allocate/map the server-issued `trade_id` (no protocol id
  chosen in advance).
- Self, same-Account and different-Channel pairs are refused. The first terminal accept is
  immutable; the second side's accept is the trigger. A stale pair or foreign command does
  not cancel a different current READY pair.
- `OFFERED`/`READY` expire after exactly 120 s without a new offer/accept (owner's monotonic
  clock; backward time refused). Pre-transfer invalidation or reconnect cancels.
- `TRANSFERRING` retains its immutable binding on unknown outcome, timeout, cancel and
  same-GameSession reconnect; a different session cannot adopt it. Only a verified writer
  outcome closes it; a supplied terminal enum never does.
- Trade flag predicate: `tradeable: UNKNOWN` and Known true allowed; Known false/ambiguous
  refused; `marketable` is never the trade flag.

Runtime owner (TradeOwner):

- Owner-local, capacity-bounded book; per-actor index (one open trade per actor). Each
  operation borrows the current Channel runtime; no exclusive borrow is held between
  operations. Exactly one book must be installed per live Channel owner (CP composition).
- At start, counter-offer and **every** accept: owner continuity, player control, bound
  Character lease, Channel/generation, native interest queries both directions (including
  the 256-result cutoff), same floor, Chebyshev range ≤ 2, qualified map/LOS. Unknown
  dynamic map facts refuse. No passed boolean/callback grants admission.
- Binding key must use the full `ChannelRuntimeAssignmentBinding` (including NodeId and node
  registration revision) plus the Content digest — the draft's first key omitted the Node
  fields (fixed under D1311).
- Current DB reads: both roots locked in CharacterId order; assignment under SHARE lock;
  direct-backpack membership, definition, quantity, lifecycle, key, state revision, placement
  TransactionId and timed state. Refused: main backpack itself, ground/corpse/equipped
  sources, items with contents, untradeable items. Room is computed as two new direct
  entries before removal; no merge and no freed-entry credit.
- Offered-item observation port (D1158) `item_reserved_by_trade`: reports both offered leaves
  in Offered/Ready/Transferring, excludes Closed; checks the carrier fence before any
  empty-book answer. Its `false` is local observation, not global admission (MAIL caller
  must hold the book's serialization through its own TX).
- Issuance: first accept installed only inside FND terminalization after its preflight
  succeeds; a matching second pending command (first of the FND pending suffix) issues one
  occurrence with the captured `ReservationIntent`; retries observe the same record; replay
  returns `ReplayedAccepted` without revalidating current authority. Trade ids and
  occurrences use distinct command-derived identity tags (16-byte convention).

Atomic writer (TRADE-1):

- Takes the type-2 activation gate, recovery/admission locks and occurrence lock, then looks
  up an existing outcome: matching committed → `AlreadyCommitted` (no live fences needed);
  conflicting identity/CommandRef → refuse.
- New swap: both fence-only checks before either root; first accept may advance only its
  same-session connection generation; roots in CharacterId order, items in ItemInstanceId
  order, backpack slots in CharacterId order; re-read every captured before-value; require
  a free destination entry before removing either source.
- One transaction: one immutable cause, two participants, two receipts, delete both source
  entries, stamp both items' `last_transaction_id`, insert both reciprocal entries, one
  type-2 audit event. Ids, definitions, quantities, lifecycle, keys and timed rows unchanged;
  never merge; no MINT, no CharacterRevision change, no pre-swap durable reservation rows.
- Migration (new number): immutable cause/participant/receipt tables; unique full-u64
  CommandRefs; deferred validation of the exact 2-item/4-location/2-participant shape,
  unchanged state, capacity, final custody, no third item change, one fresh outbox event;
  one TRADE arm added to each latest item-change/removal/placement guard function with all
  earlier arms (including HOUSE) preserved verbatim; SECURITY DEFINER
  `game_player_trade_lock_slots` for ordered slot locks; runtime gets SELECT/INSERT on the
  outcome tables only. Inbox read-only function inventory must list
  `game_player_trade_complete` (D1378) alongside MAIL's entries.
- Audit: `PlayerTrade` at the next free `OneItemOperationV1` tag (draft used 12 after HOUSE11;
  re-check on main). Measured maximal event 5,634 B payload / 5,914 B envelope, inside the
  7,936/9,216 B ceilings; fixed 6 effect work units.
- Continuation: freeze the request before the first storage await; on unresolved state run a
  recovery-fenced, occurrence-locked lookup first; retry the identical request only after
  proven absence; cancellation, unknown COMMIT, pool/recovery errors keep TRANSFERRING and
  actor indexes; cache the committed receipt or definitive refusal before FND terminal
  output so an output retry performs no second storage call.

Review/finding dispositions carried forward:

- Whole-writer move into gameplay crossed durability's private transaction/recovery boundary
  (15 compile errors). Keep the transaction in durability behind a borrowed owner adapter;
  do not make DB/recovery internals public or add synthetic modules to standalone roots.
- A PG fixture that early-returns without a configured admin URL is not DB qualification.
- `runtime_actor_source_reservation` reserves combat slots, not items/entries — not a
  substitute for PLAYER_TRADE0 reservations.

## Remaining integration (post-ingress slot)

- Install one session-owned ingress (SESSION-INGRESS-1) and the sole live Channel TradeOwner.
- Route every issued/committed physical mutation (move/use/drop, fee/NPC sale, equipment,
  reward mint, death), range loss, logout, kick, reconnect and channel transfer to the
  open-trade index: cancel before TRANSFERRING; during it refuse conflicts, delay channel
  transfer/logout/kick, death waits. Physical exclusion via D1377 `SpellLanePermit` guard.
- Decoded normalizer, item-handle/partner projection, both-session egress; PLAYER_TRADE
  registry/capability (`TRADE_V1`)/command/domain rows; resource rows.

## Acceptance criteria

- [ ] A trade between two Characters on one channel either commits both item deltas or none (crash/restart test).
- [ ] TRANSFERRING state survives restart and resolves deterministically; concurrent offers lose with a typed refusal.
- [ ] Cross-channel trade is refused.
- [ ] Wire gated by capability; older clients receive no trade domain.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Two-admitted-session and native restart qualification on the integrated candidate; canonical PG cases show real database-start markers.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files (not appended to existing files).
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Inseparable bundles

- wire + registry + capability + handler + tests (TRADE-WIRE-1)
- migration + swap writer + durability tests (TRADE-1)
- both inventory deltas + audit in one transaction (DUR-03 trade swap amendment)

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p oteryn-gameplay-economy -p oteryn-gameplay-durability player_trade
cargo test --locked -p oteryn-game-server --test character_authority_postgres player_trade -- --test-threads=1  # PG 17.6 admin URL
```

## Reference evidence

- PLAYER_TRADE0 brief §3–§6; DUR-03 §34 trade swap paragraph (amended by PLAYER_TRADE0)
- Completion plan §4, §5 (Wave 4), §7; coordination #1622 / #2131 (D1158, D1311, D1315, D1369, D1377, D1378)
- Archive: OTV2-20260930-item-move-wire1


## Current pass — local AUTHORING, 2026-10-23

Task `TRADE-PLAYER-1`; base/head `ac3f7fbc6` (`origin/main`), local changes only.
No freeze, commit, publication, PR transition or external review was performed.
Bound META policy 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`
was refreshed read-only before implementation. The owner's current local-only scope
supersedes the historical card publication/validation commands.

Owned paths: economy `player_trade.rs` and `player_trade/`; durability
`player_trade.rs` and `player_trade/`; wiring in the two crate module lists;
fence-only split in `item_transfer.rs`; reuse visibility in `item_loop_move.rs`;
additive audit tag/inventory in `item_mint_audit.rs`; migration `0147_player_trade.sql`;
sole PG auto mount `src/lifted_tests/auto/player_trade_postgres_cases.rs`;
Inbox read-only inventory; native type-2 schema/registry and seven trade resource
fragments `0598`–`0604`; this task card. No workflow or repository-policy changes.

### Step 0 — actual dependency and existing-owner inventory

Every dependency row was checked with `git log --oneline origin/main --grep <ID>`
and a bounded source/archive search. No player-trade code, table, handler or wire
operation exists on this main; NPC trade is a different existing mechanic.

| Dependency | Actual evidence on `ac3f7fbc6` |
|---|---|
| ITEM-LOOP-1 | Merged `a43c37f12` (#2190); live `item_loop_move.rs` and dispatch item-move handler. Extended/reused, no parallel inventory owner. |
| ITEM-MOVE-2a | Included by #2190; current direct/nested move and equipment owners exist. |
| ITEM-VIEW-1b, VIS-2 | Existing `SessionItemView`/`observe_inventory`; archived item-view and MOVE-RL11 visibility decisions. Later BAGS-WIRE-1 `bf03807cf` (#1720) consumes these. |
| ADMIT-CAP-1 | Merged `94288ef41` (#2187); fresh/V2 capability admission checks remain in place. |
| MAIL-1 | Merged `5b814baa1` (#2133); reuse existing Inbox exclusion/read-only proof inventory. |
| PLAYER_TRADE0 acceptance | Decision `1f8c18e82` (#1365), accepted text on main. |
| SESSION-INGRESS-1 | No matching main commit or common session-owned ingress implementation. Still blocking wire/activation. |
| D1377 | Existing `SpellLanePermit` is reusable for staged DB calls; it is not the missing shared physical-mutation/trade exclusion integration. Still blocking activation. |

Existing durable owners reused: `character_item_fence_is_current` (split to allow
both live fences before ordered roots), equipment authority/active Content check,
`carried_weight`, derived B3-3 capacity, direct-slot/entry reads, type-2 activation
transaction, recovery pass/deadline and outbox. `protocol-oteryn` gets no trade wire
ID. Tag 14 is next free after native audit tag 13 (tag 12 remains another lane's
proposal). Highest main migration is 0144; 0147 follows the owner's lower bound.
`MECHANICS_STATUS.md` is absent on main, so none was created. Test-support has no
`property` module on main: framework property tests skipped; deterministic boundary
cross-products cover arithmetic overflow and capacity bounds.

### Delivered in this pass

- L3 leaf, identity/range/visibility, trade-flag, incoming capacity/free-entry rules;
  immutable counter-offer/accept lifecycle, monotonic 120-second idle cancellation,
  typed invalidation and TRANSFERRING retention.
- L4 fenced observation, immutable request, one atomic two-leaf exchange with
  ordered roots/items/slots, complete before-state recheck, no merge/freed-entry
  credit, reciprocal receipts and one type-2 audit event. Items retain definition,
  quantity, keys, timed/text state and item/Character revisions.
- Immutable READY pair key (native counter-offer CommandRef), unique across committed
  occurrences as well as both full-u64 accept CommandRefs.
- Occurrence-locked recovery lookup and replay of the exact original binding before
  live fences. A failed/unknown storage operation leaves resolution to this lookup;
  no pre-swap durable reservations or Inbox writes.
- Additive immutable cause/participant/receipt schema with full-u64 CommandRefs,
  runtime SELECT/INSERT only on outcomes, exact same-transaction shape/state/custody
  checks and preserved earlier item proof arms (including HOUSE).

### Unfinished

- SESSION-INGRESS-1 / D1377: the sole Channel book, current visibility/LOS/map
  admission, per-actor busy index, FND issuance/terminalization and durable-outcome
  cache, movement/logout/death/kick/reconnect physical routing and reservation
  exclusion. L3 exposes the invalidation rules; no live handler invokes them yet.
- Wire/capability/domain/command registration, normalization, both-session egress
  and real-dispatch/two-admitted-session E2E. Storage ports in this writer-only
  pass consequently have no active dispatch caller; do not mark runtime acceptance
  or the production-caller/E2E checklist complete.
- BAGS-TRADE-1 container trees, equipped/ground/corpse sources and house-tile trade
  remain outside the accepted leaf slice. They are refused, not silently supported.
- Independent exact-head persistence/security review and CI remain integration work
  for the control plane; this worker did not publish or spend on review.

## Reference parity

Read-only source, pinned 2026-10-23:
[Canary game.cpp](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/game/game.cpp)
and [Canary player.cpp](https://github.com/opentibiabr/canary/blob/52550d5e3c0ffe565f4bd1014f4e5575abf1059b/src/creatures/players/player.cpp).
This is emulator evidence; it is not a claim of access to CipSoft internals.
The existing accepted decision identifies official-manual evidence separately.
Each row is a named row of economy `player_trade_reference_parity_table`.

| Rule / source function | Reference value and accepted slice | L3 test row |
|---|---|---|
| Partner distance / `playerRequestTrade` | Chebyshev distance ≤2 | `range_2` |
| Partner floor / `playerRequestTrade` | Same floor | `same_floor` |
| Sight line / request and accept | Qualified line of sight required | `sight_line` |
| Self trade / `playerRequestTrade` | Refused | `no_self` |
| Item / `playerRequestTrade` | Pickupable required | `pickupable` |
| Offer item count / `playerRequestTrade` | One root, at most 100 items in Reference; this slice one whole leaf, nonempty containers deferred | `one_root_100_items` |
| Handshake / `internalStartTrade`, `playerAcceptTrade` | Counter-offer and both accepts before transfer | `counter_offer_both_accept` |
| Placement / `playerAcceptTrade` | Room tested before removal; IGNOREAUTOSTACK, no merge | `room_before_removal_no_merge` |
| Capacity / `playerAcceptTrade` | Incoming weight must fit before removal | `incoming_capacity` |
| Change / `checkTradeState` | Item changes cancel before transfer | `item_change_cancels` |
| Movement / `onCreatureMove` | Losing range cancels before transfer | `distance_cancels` |
| Logout / `onRemoveCreature` | Partner trade closed on removal | `logout_cancels` |


### Validation for this local delta

All final Cargo commands used the queued wrapper (`login=false` preserves its PATH),
with the toolchain from `rust-toolchain.toml` and locked dependencies. Earlier direct
Cargo's route-less PG result was discarded and rerun through the wrapper. Actual
PG marker: `it_player_trade_atomic_1792772936179567736`, `server_version_num=180006`.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS; new included durability/auto-test modules also formatted explicitly |
| `cargo clippy --locked --quiet -p oteryn-gameplay-economy -p oteryn-gameplay-durability -p oteryn-game-server --all-targets -- -D warnings` | PASS |
| `cargo test --locked --quiet -p oteryn-gameplay-economy --lib` | 15 passed; includes the 12-row Reference table and arithmetic boundary cross-product |
| `cargo test --locked --quiet -p oteryn-gameplay-durability --lib player_trade -- --nocapture` | 1 passed; both type-2 tuples, canonical roundtrip, malformed bindings, field-bound payload/envelope 6880/7163 B (under 7936/9216) |
| `cargo test --locked --quiet -p oteryn-gameplay-durability --lib item_mint_audit -- --nocapture` | 24 passed; additive tag preserves earlier encodings and the complete type-2 writer/fence inventory |
| `cargo test --locked --quiet -p oteryn-game-server --lib player_trade -- --test-threads=1 --nocapture` | 1 real PG case passed; setup, per-side before-value/fence refusals, no-room/weight/flags, rollback after the first reciprocal write, incomplete SQL shape, role grants, two-root serialization, newer first connection, closed-pair reuse refusal, cold-root replay |
| `python3 tools/repository/validate_repository_policy.py` | PASS |
| `git diff --check` | PASS |

Re-read the complete local delta against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`;
reviewed ordered locks (including Content-before-root), fresh same-transaction proofs,
immutable outcomes, complete runtime write grants and earlier proof preservation.
Workspace constructor/impl/match sweep: 98 new-mechanic sites in 13 Rust files;
also examined all 16 files referencing `OneItemOperationV1` across `apps/`, `crates/`
and `tools/`. No external exhaustive consumer needed a new arm; all existing codecs
select their closed variant and the native oneof tag list now includes 14.
The production-caller and real-dispatch E2E requirements remain explicitly unfinished
under the ingress/physical-exclusion gate above. No full PG suite was run; the changed
MAIL whitelist remains for CI's existing MAIL filter. No unrelated contention failures.

### Changed files (complete local manifest)

- `apps/game-server/migrations/0147_player_trade.sql`
- `apps/game-server/src/lifted_tests/auto/player_trade_postgres_cases.rs`
- `apps/game-server/tests/support/character_inbox_postgres_cases.rs`
- `crates/gameplay/durability/src/durability/core_modules.rs`
- `crates/gameplay/durability/src/durability/item_loop_move.rs`
- `crates/gameplay/durability/src/durability/item_mint_audit.rs`
- `crates/gameplay/durability/src/durability/item_transfer.rs`
- `crates/gameplay/durability/src/durability/player_trade.rs`
- `crates/gameplay/durability/src/durability/player_trade/audit.rs`
- `crates/gameplay/durability/src/durability/player_trade/player_trade_audit_tests.rs`
- `crates/gameplay/durability/src/durability/player_trade/snapshot.rs`
- `crates/gameplay/durability/src/durability/player_trade/storage.rs`
- `crates/gameplay/economy/src/lib.rs`
- `crates/gameplay/economy/src/player_trade.rs`
- `crates/gameplay/economy/src/player_trade/lifecycle.rs`
- `crates/gameplay/economy/src/player_trade/player_trade_tests.rs`
- `docs/agents/tasks/TRADE-PLAYER-1.md`
- `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json`
- `docs/contracts/game-events/v1/native_one_item_transaction.proto`
- `docs/contracts/resource-limits/entries/0625-DUR03-RL-01-TRADE.json`
- `docs/contracts/resource-limits/entries/0626-DUR03-RL-02-TRADE.json`
- `docs/contracts/resource-limits/entries/0627-DUR03-RL-06-TRADE-PARTICIPANTS.json`
- `docs/contracts/resource-limits/entries/0628-DUR03-RL-06-TRADE-EFFECT-WORK-UNITS.json`
- `docs/contracts/resource-limits/entries/0629-DUR03-RL-07-TRADE-PAYLOAD-BYTES.json`
- `docs/contracts/resource-limits/entries/0630-DUR03-RL-07-TRADE-ENVELOPE-BYTES.json`
- `docs/contracts/resource-limits/entries/0631-TRADE0-RL-02.json`
