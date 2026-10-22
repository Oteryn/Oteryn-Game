# TRADE-PLAYER-1 — player-to-player trade (consolidated task card)

```yaml
task_id: TRADE-PLAYER-1
title: "TRADE-PLAYER-1: player-to-player trade"
mode: IMPLEMENT
status: NOT_STARTED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 row TRADE-PLAYER-1 (row 82); §7 'TRADE-PLAYER-1, MAIL-1, MARKET-1' (TRADE-WIRE-1, TRADE-1)"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md
base_branch: main
branch: null
pr: null
needs_migration: true
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

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
