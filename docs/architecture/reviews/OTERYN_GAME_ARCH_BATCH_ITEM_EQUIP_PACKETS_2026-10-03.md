# Architect batch: item view and move, equip, drop, speed, bags and exercise packets

```yaml
decision_id: ARCH-BATCH-ITEM-EQUIP-PACKETS-V1
status: CANDIDATE
date: 2026-10-03
owner: Sol Supervising Architect
requested_by: control plane (after #1696: ITEM-MOVE-2a, ITEM-MOVE-2b, EQUIP-RT-1, EXERCISE-1, and what else the accepted EQUIP-0, EXERCISE-0, DEPOT-0, BAGS-0 and IMBUE-FORGE-0 allow)
writes_on_other_prs: none
amended_by: ARCH-ITEM-PACKETS-AMEND-1 (§0.1, §0.2, §0.3, §1.4, §1.7, §1.9-§1.12, §2.0a, §2.0b (#1702 P1s 4175377704 and 4175377707), §2.1, §2.2, §2.2a, §2.3, §2.6, §2.8, §2.9; #1698 round-3 P1 4175197224 and P2 4175197230; #1696 P1 4175041166; control plane: ITEM-SEM-2b-2 narrowed to the patterns model)
```

This bundle packets the item chain that the four requested slices sit on, in order of playable
value. The first playable result is looting a corpse into the backpack; the second is wearing what
was looted, with its effects; the third is dropping and picking up.

The bundle changes no code, no contract and no wire. The rulings in §1 are architecture rulings
under the accepted decisions they cite. Live PR and Issue state governs. The dependency notes record
the state when this was written:

- on `main`: VIS-2, COND-1, CHAR-BUILD-1 and 1b, SKILLS-0, the combat loot MINT
  (`combat/death_reward.rs`), `combat/pickup.rs` (`settle_corpse_pickup`), `item_transfer.rs` with
  `reconcile_item_transfer`, `domain/equipment.rs` (`check_equip`), HOUSE-CUSTODY-1 (0025),
  TIMED-RT-1a (0054), carrier #1675;
- in flight: TIMED-CONTENT-1 (lane 2), TIMED-RT-1b (waits on TIMED-CONTENT-1), FORGE-1a #1691;
- not built and not in flight: ITEM-VIEW-1, ITEM-MOVE-1, ITEM-EQUIP-WIRE-1, ITEM-MOVE-2a/2b,
  ITEM-SEM-2b-2's content, SPEED-1, NPC-VIS-1, VIS-3, MAP-LOAD-1, MAP-OVERLAY-1, MAP-WIRE-1/2, ITEM-USE-WIRE-1,
  WORLDINT-WIRE-1, WORLDINT-USE-1, WO-3, GOLD-FEE-2.

## 0. Leases, order and shared files

### 0.1 Leases

Proposed (the control plane leases; a worker that needs another number stops and asks). Main's
highest migration is 0057; D417 granted 0058-0062, so the proposals start at 0063.

| Packet | Migration | Capability / command / state domain |
|---|---|---|
| CAP-NEG-1 | none | none (selects registered capabilities; allocates none) |
| NPC-VIS-1 | none | none (entity kind 5 `Npc` in capability 6's schema) |
| VIS-3 | none | none (offers the registered capability 6 `WORLD_SPATIAL_ENTITIES`) |
| ITEM-VIEW-1a | none | capability 4 `ITEM_VIEW_MOVE_V1`, state domains 9 `CHARACTER_INVENTORY` and 11 `OPEN_CONTAINER`, command type 9 `ITEM_MOVE_INTENT` (all assigned by D212; nothing new) |
| ITEM-VIEW-1b | none | none |
| ITEM-MOVE-1 | none (the existing `0014` corpse-entry TRANSFER) | none |
| ITEM-CLIENT-1 | none | none |
| ITEM-SEM-2b-2 | none | none (#1672 packet, rebased and narrowed by §1.4) |
| ITEM-SEM-2b-3 | none | none (a `ReferenceBaseVocation` wire value and a requirements group in the typed artifact; §1.12) |
| ITEM-EQUIP-WIRE-1 | none | capability 12 `ITEM_EQUIP_DROP_V1` (proposed) |
| ITEM-MOVE-2a | 0063 (proposed) | none |
| EQUIP-CONTENT-1 | none | none |
| SPEED-1 | none | capability 13 `PACED_MOVEMENT_V1` (proposed); no command, no domain |
| EQUIP-RT-1 | none | none |
| ITEM-MOVE-2b | 0065 (proposed) | none |
| BAGS-WIRE-1 | none | capability 14 `CONTAINER_TREE_V1`, state domain 14 `CONTAINER_VIEWS`, command type 21 `CONTAINER_VIEW_INTENT` (proposed) |
| BAGS-1 | 0064 (proposed) | none |
| EXERCISE-1 | 0066 (proposed) | none |
| ITEM-CLIENT-2, ITEM-CLIENT-3, ITEM-CLIENT-4 | none | none |

Migration numbers follow the expected merge order, and the migration history stays monotonic
whatever order the packets actually merge in. A migration packet may merge only when its
number is higher than every migration on `main`. If a higher-numbered packet of this bundle
merges first (for example BAGS-1 or EXERCISE-1 while ITEM-MOVE-2b waits on MAP-OVERLAY-1),
the waiting packet stops and asks the control plane for the next free number. It then returns
to AUTHORING, renames its migration file and freezes again. It never merges below a number
already applied. Each migration packet's acceptance repeats this as a merge condition.

### 0.2 Order (by playable value)

| # | Packet | Worker | Starts when |
|---|---|---|---|
| 0 | CAP-NEG-1 | hard, protocol and session review | now (§1.9) |
| 0a | NPC-VIS-1 | impl, protocol review | NPC-BEHAVIOUR-0 is accepted and ITEM-VIEW-1a has merged (§1.10) |
| 0b | VIS-3 | hard, protocol and session review | NPC-VIS-1, ITEM-VIEW-1a and CAP-NEG-1 have merged (§1.10) |
| 1 | ITEM-VIEW-1a | impl, protocol review | now |
| 2 | ITEM-SEM-2b-2 | impl, content review | now (§1.4) |
| 2a | ITEM-SEM-2b-3 | hard, contract review | ITEM-SEM-2b-2 has merged; not open together with EQUIP-CONTENT-1 (§1.12) |
| 3 | SPEED-1 | impl, movement review | CAP-NEG-1 has merged |
| 4 | ITEM-VIEW-1b | hard, protocol and session review | ITEM-VIEW-1a and CAP-NEG-1 have merged |
| 5 | ITEM-EQUIP-WIRE-1 | impl, protocol review | ITEM-VIEW-1a has merged |
| 6 | ITEM-MOVE-1 | hard, persistence review | ITEM-VIEW-1b and VIS-3 have merged (§1.10) |
| 7 | ITEM-CLIENT-1 | impl | ITEM-MOVE-1 has merged |
| 8 | ITEM-MOVE-2a | hard, persistence review | ITEM-MOVE-1, ITEM-EQUIP-WIRE-1 and ITEM-SEM-2b-2 have merged |
| 9 | EQUIP-CONTENT-1 | content lane | ITEM-SEM-2b-2 and TIMED-CONTENT-1 have merged (§1.5) |
| 10 | EQUIP-RT-1 | hard (combat), combat and determinism review | ITEM-MOVE-2a, SPEED-1 and EQUIP-CONTENT-1 have merged |
| 11 | BAGS-WIRE-1 | impl, protocol review | ITEM-VIEW-1b and ITEM-EQUIP-WIRE-1 have merged |
| 12 | BAGS-1 | hard, persistence and performance review | ITEM-MOVE-2a and BAGS-WIRE-1 have merged |
| 13 | ITEM-MOVE-2b | hard, persistence review | ITEM-MOVE-2a and MAP-OVERLAY-1 have merged |
| 14 | EXERCISE-1 | hard (persistence), persistence and determinism review | TIMED-RT-1b, EXERCISE-CONTENT-1 and WORLDINT-USE-1 have merged |
| 15 | ITEM-CLIENT-2 | impl, client review | ITEM-CLIENT-1 and ITEM-MOVE-2a have merged |
| 16 | ITEM-CLIENT-3 | impl, client review | ITEM-CLIENT-2 and BAGS-1 have merged |
| 17 | ITEM-CLIENT-4 | impl, client review | ITEM-CLIENT-3 and ITEM-MOVE-2b have merged |

Items 0-2 can run in parallel now; SPEED-1, ITEM-VIEW-1b and VIS-3 follow CAP-NEG-1. Items 4 and 5, and later 9 and 11, can run in parallel. The
client packets 7, 15, 16 and 17 own the same client files and therefore run one at a time, in
that order. If ITEM-MOVE-2b merges before BAGS-1, ITEM-CLIENT-4 may go before ITEM-CLIENT-3;
the control plane swaps their bases and records the swap.

### 0.3 Shared files

The packets own disjoint code paths except these append-only registers (each packet's own lines
only; the second of two open packets merges `main` as a union):

- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (wire packets 1, 5, 3 and 11 only);
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`;
- `apps/game-server/src/durability/mod.rs` and `apps/game-server/tests/durability_postgres.rs`
  (one `mod` line each);
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` (own paragraphs);
- `crates/protocol-oteryn/src/lib.rs` (one `mod` line and one `REGISTERED_CAPABILITY_IDS_V1` entry
  each);
- `apps/game-server/src/gameplay_transport/connection.rs` (after CAP-NEG-1: one command dispatch
  arm each; the negotiation itself stays CAP-NEG-1's, and the domain 1 composition VIS-3's);
- `apps/game-server/src/gameplay_transport/mod.rs` (one `mod` line and one dispatch arm each;
  EQUIP-RT-1 also adds the recompute calls at fresh admission, reconnect and release);
- the world spatial wire files `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto`,
  `crates/protocol-oteryn/src/world_spatial.rs` and `world_spatial_entities.rs`, and
  `apps/game-server/src/gameplay_transport/world_spatial.rs` (NPC-VIS-1, VIS-3, ITEM-VIEW-1a,
  SPEED-1 and ITEM-MOVE-2b: each only the field, kind, disposition or function its packet names).
  `world_spatial_entities.rs` is edited serially: ITEM-VIEW-1a, then NPC-VIS-1, then VIS-3.

`gameplay_transport/item_move.rs` is created by ITEM-MOVE-1 and then owned in turn by ITEM-MOVE-2a,
BAGS-1 and ITEM-MOVE-2b; the order in §0.2 never has two of them open at once except BAGS-1 and
ITEM-MOVE-2b, which merge `main` as a union and each add only their own arms.

## 1. Rulings

### 1.1 ITEM-VIEW-1 is split into a wire slice and a server slice

ITEM-MOVE-WIRE-0 gives ITEM-VIEW-1 the wire, the session handle table, the corpse open, resume
continuity and the client views. That is three review kinds (protocol, session, client) in one PR.
Following the ANALYSER-WIRE-1 precedent:

- **ITEM-VIEW-1a** registers the whole ITEM-MOVE-WIRE-0 wire: capability 4 (`offered: false`),
  domains 9 and 11, the handle field on the D85 item entry, USE field 2 `ItemTargetV1 {handle}`,
  and command type 9 with its one `MAIN_BACKPACK` destination and its results table (§5 there). It
  has no server behaviour. Registering command 9 here keeps one protocol review for the whole
  WIRE-0 wire; ITEM-MOVE-1 still owns its handling.
- **ITEM-VIEW-1b** builds the server side of §4: the handle table, the domain 9 and 11 views and
  revisions, the corpse open and close, and resume continuity.
- **ITEM-MOVE-1** offers capability 4. A session that can see items but not take them has no
  playable value, so the capability is offered only when the loot loop is complete.
- **ITEM-CLIENT-1** builds the client windows that WIRE-0 assigned to ITEM-VIEW-1.

### 1.2 ITEM-MOVE-1 needs no migration

The corpse-entry TRANSFER is the existing `0014` shape through `item_transfer.rs`, reached by
`combat/pickup.rs` `settle_corpse_pickup`. Replay first (WIRE-0 §5) uses the writer's existing
committed-outcome lookup by CommandRef (`reconcile_item_transfer`). If the worker finds that the
lookup cannot return a committed outcome after the handle became `STALE`, it stops with a BLOCKER
and asks for a migration lease; it does not add a result table on its own.

### 1.3 ITEM-MOVE-2a, 2b, BAGS-1 and EXERCISE-1 each need one migration

- 2a: the nine slot rows, the per-hand uniqueness, deletion of a backpack entry by TRANSFER, the
  widened TRANSFER guard and the swap receipt (WIRE-1 §6.2). `0011` defines only the container
  slot.
- 2b: Ground insertion by TRANSFER, removal evidence keyed by (item, transaction), nullable
  `corpse_ref`, the per-tile lock and the per-channel counter shards (WIRE-1 §6.2).
- BAGS-1: entries keyed by parent item and the tree guards (BAGS-0 §3-§4).
- EXERCISE-1: the composed checkpoint and composed expiry burn shapes, which carry a build receipt
  in the same transaction (EXERCISE-0 §5.1, §5.3; TIMED-ITEM-0B §12). Earlier guards are amended by
  replacement in the new migration, never edited.

### 1.4 ITEM-SEM-2b-2 starts now on `main`

The ITEM-SEM-2b-2 packet (#1672) is based on `main after #1599 merges`. #1599 was closed as
superseded when carrier #1675 merged (D434), and D318 is released. The packet starts now with
`base: main` and `depends_on: []`. ITEM-MOVE-2a's legality check needs its level, vocation, hands
and slot facts in content (WIRE-1 brief).

**Narrowed to `main`'s model (ARCH-ITEM-PACKETS-AMEND-1).** On `main` the typed item semantics
carry equipment only as `equipment.patterns` (`ReferenceEquipmentPattern`: `primary_slot`,
`additional_reserved_slots`, `mutually_exclusive_groups`, `vocations`, `level`), and #1675 already
lowers slot, hands, level and vocations into it. The compact form of 2b-2 §2.2 exists only in the
authoring schema. 2b-2 therefore writes one pattern per Item, not the compact form, and keeps
§2.1 and §2.3 as occupancy facts of that pattern (`additional_reserved_slots: [left_hand]` for a
two-handed weapon, `mutually_exclusive_groups: [non_quiver_left_hand]` for the rest). What the
patterns model cannot hold moves to ITEM-SEM-2b-3 (§1.12), and 2b-2 reports those rows and writes
no field for them:
- the `none` vocation (2b-2 §2.5). An Item whose vocations include `without` gets no
  `equipment` block until 2b-3, because omitting the vocation would make it unrestricted;
- `requirements` with `on_use`, and `min_magic_level` (2b-2 §2.4: runes and ammunition).
2b-2 §2.7 (the D303 guard) no longer applies, since 2b-2 writes the patterns form the guard admits.

### 1.12 ITEM-SEM-2b-3 adds what the patterns model lacks

ITEM-SEM-2b-3 extends the typed item semantics, which is a durable content contract (DUR-04 and
the typed artifact profile), so it is a hard slice with contract review:
- `None` becomes a `ReferenceBaseVocation` value (the A13 key `none`), with the next free `u8`
  wire value in `reference_artifact.rs`; existing values are unchanged.
- A use-requirements group joins `ReferenceItemSemantics`: `min_level`, `min_magic_level`,
  `vocations` and `enforcement_mode` (`on_use` only in v1), for runes and ammunition. Enforcement
  stays with RUNE-USE-0 and RANGED-0.
- It then lowers the rows that 2b-2 reported: the `without` vocations into patterns, and the
  `requirements` rows of runes and ammunition.

If the artifact encoding of either addition changes bytes of an existing artifact, it is a new
typed artifact profile revision and existing artifacts still decode (test). 2b-3 and EQUIP-CONTENT-1
own the same lowering paths and content tree, so they are never open together: the first allocated
goes first, and the other starts from `main` after it merges. ITEM-MOVE-2a does not wait for 2b-3;
it admits `none` from the moment the value exists (§2.8).

### 1.5 EQUIP-CONTENT-1 waits for the two content slices before it

EQUIP-CONTENT-1 regenerates the same content tree and lowering paths as ITEM-SEM-2b-2 and
TIMED-CONTENT-1, and its `timed` flag is derived from TIMED-CONTENT-1's `charges` and
`temporal.duration_ms` facts (EQUIP-0 §3.2). It starts after both have merged, on a fresh `main`.

### 1.6 Promoted vocations are matched by ITEM-MOVE-2a

ITEM-SEM-2b-2 §6 lowers a base vocation key and leaves promotion matching to the equip rule. 2a's
requirement check admits a promoted vocation for its base key (an Elite Knight for `knight`), with
a test per vocation pair.

### 1.7 Slot call sites of the timed host

The ARCH-SLOT-WIRING-1 conditions (#1696; WIRE-1 §4 and §5 amendments; TIMED batch §2.3) bind
ITEM-MOVE-2a and ITEM-MOVE-2b as written there. If TIMED-RT-1b has merged when either is
allocated, that packet wires the `timed_item_host` call sites, with the stop, empty-lane and rehost
tests, as a merge condition. Otherwise TIMED-RT-1b does, and the worker of the later PR names the
call-site files at allocation.

For ITEM-MOVE-2b this covers the slot-to-Ground drop, success and rejection (#1696 P1
4175041166): before the drop transaction the item is stopped, its checkpoint is on the lane and
the lane is empty; a rejected drop makes it live again from the row. The tests spend time or
charges since the last checkpoint and check them after a rejected drop and on the Ground row.
TIMED-RT-1c is not the owner: it starts only after ITEM-MOVE-2b, so ordering 2b after RT-1c would
be circular, and 2b and a live host may coexist only with these call sites wired.

### 1.9 Capability negotiation is built once, first

On `main` the server answers every fresh admission and resume with `selected_capabilities: &[]`
(`gameplay_transport/connection.rs`). Offering a capability in the registry is therefore not
enough to make it usable. CHAT-1b-2 and the registry `offer_gate` notes expect that seam from
CHARM-5-COMP, which is not in flight. CAP-NEG-1 builds it now, alone, as the root of this bundle.
CHARM-5-COMP and CHAT-1b-2 then reuse it and keep only their routing and offering. A packet that
offers a capability (VIS-3, ITEM-MOVE-1, SPEED-1, ITEM-MOVE-2a, BAGS-1) proves the capability is
selected in a production-path admission test.

### 1.10 Capability 4 needs capability 6 offered first (VIS-3)

ITEM-MOVE-WIRE-0 makes capability 4 require capability 6 `WORLD_SPATIAL_ENTITIES`, and CAP-NEG-1
selects a capability only with its `requires` closure (#1698 P1 4175197224). Capability 6 is
`offered: false` until VIS-3, and VIS-3 may not offer it before NPC-VIS-1 adds kind 5 `Npc`
(MOVE-RL-11 §4.3 and NPC-BEHAVIOUR-0 §3.2 amendments), so no released client meets an unknown kind.
This bundle therefore packets both (§2.0a, §2.0b) and orders ITEM-MOVE-1, which offers capability
4, after VIS-3. ITEM-VIEW-1a and 1b do not offer capability 4 and need not wait.

NPC-VIS-1 is gated by acceptance of NPC-BEHAVIOUR-0, which is CANDIDATE; that acceptance is now on
the critical path of the loot chain. Offering capability 6 without kind 5 and giving NPCs their own
capability later is rejected: it adds a capability for one enum value.

### 1.11 Ground speed in SPEED-1 is a seam; MAP-LOAD-1 supplies the source

On `main` the production map is the engineering static cell index, which carries only a
`CollisionClass` and no ground item; WO-0's `ground_speed` reaches the runtime only with
MAP-LOAD-1 (ADR-0021), which is not built (#1698 P2 4175197230). SPEED-1 therefore takes the
ground speed from a lookup seam keyed by tile, whose engineering implementation returns 150 for
every tile, and tests pacing with an injected non-default source. Pacing with 150 on the
engineering map is exact, because that map has no ground item with another speed.

The client must pace with the same ground speed as the server (#1702 P1 4175377704), and today
`MAP_TILES` carries no ground speed and MAP-CLIENT-1 only decodes and draws. So:
- MAP-LOAD-1 builds the map source of the seam from the bundle's ground items (WO-0
  `ground_speed`), with a test, but production keeps the 150 source (ADR-0021 amendment).
- MAP-WIRE-2 carries each described tile's ground speed in `MAP_TILES`, omitted when 150, with
  max and absent codec tests (MAP-WIRE-1 amendment).
- MAP-CLIENT-1 paces client steps from that value, and switches the server seam to the map source
  in the same PR, as a merge condition. Its production-path test steps onto a tile whose ground
  speed is not 150 and shows the server step duration and the client pacing equal.
So server and client change source together, and neither paces from map data before the other
has it.

### 1.8 Not packeted now

| Slice | Why not now | Starts with |
|---|---|---|
| DEPOT-WIRE-1, DEPOT-CONTENT-1 | need MAP-LOAD-1 and MAP-WIRE-1 (lockers are map items) | the map packets |
| DEPOT-1 | its prerequisites (HOUSE-CUSTODY-1, ITEM-MOVE-2a) are or will be here, but without DEPOT-WIRE-1 it stores items no player can reach (playable-first) | DEPOT-WIRE-1 |
| IMBUE-1, FORGE-1 (1b already packeted) | GOLD-FEE-2 is not built and not packeted | a GOLD-FEE-2 packet |
| IMBUE-WIRE-1, IMBUE-CONTENT-1, FORGE-CONTENT-1 | ITEM-SEM-USE and USE-WIRE-V1 item use are not built | ITEM-USE-WIRE-1 |
| EXERCISE-CONTENT-1 | needs TIMED-CONTENT-1 (in flight) and WO-2 | after TIMED-CONTENT-1 |
| EQUIP-PARITY-1, EXERCISE-PARITY-1 | fixtures on top of their runtime slices | after EQUIP-RT-1, EXERCISE-1 |
| BAGS-USE-1, BAGS-GROUND-1, BAGS-DEPOT-1, BAGS-TRADE-1 | wait on ITEM-USE-1, ITEM-MOVE-2b, DEPOT-1, INBOX-1, TRADE-1 | their prerequisites |

The missing roots that block the most here are MAP-LOAD-1 (and through it MAP-OVERLAY-1 and
MAP-WIRE-1), ITEM-USE-WIRE-1 and GOLD-FEE-2. The architect packets them in a separate batch when
the control plane asks.

## 2. Packets

Every packet's validation includes `python3 tools/agents/validate_governance.py` and
`git diff --check`; Rust packets also run `cargo fmt --all --check` and `cargo clippy --locked
--all-targets --quiet -- -D warnings` on the crates they touch. Each task record is archived in the
PR's final authoring commit.

### 2.0 CAP-NEG-1

```yaml
task_id: OTV2-20261003-cap-neg-1
decision: foundation.proto `supported_capability_id` / `selected_capability_id` (fresh and resume), PROTOCOL_OTERYN_V1_REGISTRY offer gates; this bundle §1.9
worker: oteryn-hard-worker   # session admission and resume state
review: independent protocol and session review (Codex, final frozen head)
branch: claude/cap-neg-1-20261003
base: main
migration_lease: none
depends_on: []
owned_paths:
  - apps/game-server/src/gameplay_transport/capabilities.rs        # new: the server's offered set and the selection
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs  # new
  - apps/game-server/src/gameplay_transport/connection.rs          # selection at fresh admission and resume; the selected set handed to dispatch
  - apps/game-server/src/gameplay_transport/resume.rs              # the selected set in resume state only
  - apps/game-server/src/gameplay_transport/mod.rs                 # shared register
  - docs/agents/tasks/archive/OTV2-20261003-cap-neg-1.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
```

Acceptance:

- The server's offered set is derived from the registry's `offered: true` entries, checked against
  `REGISTERED_CAPABILITY_IDS_V1` by a test. On `main` it is empty, so production behaviour is
  unchanged until a packet offers a capability.
- Fresh admission selects the client's supported capabilities that the server offers and whose
  `requires` are all selected; unknown ids are ignored, never selected. The result is echoed in
  `selected_capabilities` and kept per `GameSessionId`.
- Resume and channel transfer keep the session's original selected set. A resume never widens it.
  A resume whose supported set lacks a selected capability fails closed as resume-unavailable,
  and the client falls back to fresh admission (downgrade test).
- Dispatch and domain emission read the session's selected set. A command or domain whose
  capability is not selected stays refused or unsent, as today (test per path).
- Tests run with an injected offered set: selection, the `requires` closure, resume equality, and
  the empty production set.
- Not in scope: offering any capability, chat or charm routing.

### 2.0a NPC-VIS-1

```yaml
task_id: OTV2-20261003-npc-vis-1
decision: NPC-BEHAVIOUR-0 §3.2; MOVE-RL-11 §4.2 and §4.3 amendments; this bundle §1.10
worker: oteryn-impl-worker   # one wire enum value and codecs; no server behaviour
review: independent protocol review (Codex, final frozen head)
branch: claude/npc-vis-1-20261003
base: main after NPC-BEHAVIOUR-0 is accepted and ITEM-VIEW-1a merges
migration_lease: none
depends_on: [NPC-BEHAVIOUR-0 accepted, ITEM-VIEW-1a]   # both edit world_spatial_entities.rs
owned_paths:
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto     # EntityKind 5 ENTITY_KIND_NPC only
  - crates/protocol-oteryn/src/world_spatial_entities.rs         # kind 5 as an actor entry, with its tests
  - apps/client/src/scene.rs                                     # draw kind 5 as an actor
  - docs/agents/tasks/archive/OTV2-20261003-npc-vis-1.md
validation:
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-client --quiet
```

Acceptance:

- Kind 5 `Npc` is an actor entry (direction, appearance, health percentage 100) and ranks with
  actors (D222); round-trip and max-size codec tests; the client decodes and draws it.
- An unknown kind still fails decode (test).
- Not in scope: offering capability 6, NPC actors in the runtime (NPC-ACTOR-1).

### 2.0b VIS-3

```yaml
task_id: OTV2-20261003-vis-3
decision: MOVE-RL-11 §4 (VIS-1 interest set, D84-D87, D222); registry offer_gate of capability 6; this bundle §1.9, §1.10
worker: oteryn-hard-worker   # session emission of domain 1 and resume state
review: independent protocol and session review (Codex, final frozen head)
branch: claude/vis-3-20261003
base: main after NPC-VIS-1, ITEM-VIEW-1a and CAP-NEG-1 merge
migration_lease: none
depends_on: [NPC-VIS-1, ITEM-VIEW-1a, CAP-NEG-1]   # ITEM-VIEW-1a and NPC-VIS-1 also edit world_spatial_entities.rs
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs    # the domain 1 snapshot and delta composition from the interest set
  - apps/game-server/src/gameplay_transport/world_spatial.rs # the server encode helpers' caller; the dead-code allowance removed
  - apps/game-server/src/movement/interest.rs                # only what the production caller needs
  - apps/game-server/src/gameplay_transport/mod.rs           # shared register
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json          # capability 6 offered: true only
  - crates/protocol-oteryn/src/world_spatial_entities.rs     # the registry test's `offered` assertion only (#1702 P1 4175377707)
  - docs/agents/tasks/archive/OTV2-20261003-vis-3.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-protocol-oteryn
```

Acceptance:

- With capability 6 selected, the initial domain 1 snapshot and every delta are composed from the
  VIS-1 interest set of the session's channel: players, creatures and corpses on `main`; NPCs and
  Ground items join through the index when NPC-ACTOR-1 and ITEM-MOVE-2b add them. Without
  capability 6, domain 1 stays the v1 own-actor type (test both ways).
- The 256 ceiling, the degrade and resync dispositions and the canonical order of `main` are kept
  on the production path (ITEM-MOVE-2b adds the D222 order) (tests at 256 and 257 entities, and a delta above 256 changes becoming a
  snapshot).
- Resume keeps the selection (CAP-NEG-1) and resends a snapshot from the interest set.
- Capability 6 becomes `offered: true`, and a production-path admission test shows it selected
  (§1.9).
- Not in scope: line of sight, invisibility, spectators, NPC actors.

### 2.1 ITEM-VIEW-1a

```yaml
task_id: OTV2-20261003-item-view-1a
decision: ITEM-MOVE-WIRE-0 §4 and §5 (wire only), D212; this bundle §1.1
worker: oteryn-impl-worker   # wire registration and codecs; no server behaviour
review: independent protocol review (Codex, final frozen head)
branch: claude/item-view-1a-20261003
base: main
migration_lease: none
depends_on: []
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json      # shared register (§0.3)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json         # ITEMV0-RL-01/-02/-03; own rows only
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto  # new: domains 9 and 11, ItemTargetV1, command 9
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto   # USE field 2 only
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto  # the handle field on the D85 item entry only
  - crates/protocol-oteryn/src/{lib,item_view,item_view_tests,world_object,world_spatial_entities}.rs   # the D85 entry codec is in world_spatial_entities.rs
  - docs/agents/tasks/archive/OTV2-20261003-item-view-1a.md
validation:
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo check --locked --workspace --all-targets
```

Acceptance:

- Capability 4 `ITEM_VIEW_MOVE_V1`, `offered: false`, requires capability 6. Domains 9 and 11 with
  their snapshot and delta types; command type 9 with `{source: handle, destination}` and the
  `MAIN_BACKPACK` destination only; the WIRE-0 §5 results, at most 4 bytes.
- USE field 2 becomes `ItemTargetV1 {handle}`; fields 3 and 4 stay reserved. A session without
  capability 4 that sends field 2 fails closed.
- The handle is a `uint64`, non-zero; the D85 item entry carries it only under capability 4, and a
  session without it decodes the entry unchanged (round-trip test both ways).
- Rows: `ITEMV0-RL-01` 30 entries (WIRE-1 §6.3 supersedes WIRE-0's 21), `ITEMV0-RL-02` the corpse
  container capacity, `ITEMV0-RL-03` live handles per session, each measured, with max and max+1
  codec tests; snapshot and delta bytes within FND-02.
- Not in scope: any server or client code, offering capability 4, equipment destinations.

### 2.2 ITEM-SEM-2b-2 (rebased)

The #1672 packet with `base: main` and `depends_on: []`, narrowed to the patterns model (§1.4):
it writes `equipment.patterns`, reports the `without` and `on_use` rows, and drops §2.7. Worker
oteryn-impl-worker, content review.

### 2.2a ITEM-SEM-2b-3

```yaml
task_id: OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements
decision: ITEM-SEM-2b-2 §2.4 and §2.5; A13 §4.3; DUR-04; this bundle §1.4, §1.12
worker: oteryn-hard-worker   # durable typed content contract and artifact encoding
review: independent contract review (Codex, final frozen head)
branch: claude/item-sem-2b3-20261003
base: main after ITEM-SEM-2b-2 merges (and after EQUIP-CONTENT-1 if that is open first)
migration_lease: none
depends_on: [ITEM-SEM-2b-2]
owned_paths:
  - apps/game-server/src/content/reference_playable.rs     # ReferenceBaseVocation::None, the use-requirements group
  - apps/game-server/src/content/reference_artifact.rs     # their wire encoding; a profile revision only if bytes change
  - apps/game-server/src/content/project/v2.rs             # authoring to typed lowering of the two additions
  - apps/game-server/tests/content_reference_artifact.rs
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md  # own paragraph
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py,README.md}
  - apps/game-server/src/content/item_stats_promotion.rs
  - content/world/** and the content tree (regenerated)
  - docs/agents/tasks/archive/OTV2-20261003-item-sem-2b3-vocation-none-and-use-requirements.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-game-server --test content_reference_artifact --quiet
  - python3 -m unittest tools/content-schema/item-authoring/test_lower_wiki_stats_packet.py
```

Acceptance:

- `None` decodes and encodes with its new value, and an unknown value still fails closed (test).
  Existing artifacts decode unchanged, or a new profile revision is added with a cross-revision
  test.
- The use-requirements group round-trips with each field present and absent, and
  `enforcement_mode` other than `on_use` is rejected.
- The 2b-2 reported rows are lowered: the `without` Items (2b-2 §2.5) get their patterns with
  `none`, and the 53 `mlrequired` runes and the ammunition get `requirements`; the record lists
  the counts.
- Not in scope: enforcing use requirements (RUNE-USE-0, RANGED-0), Premium.

### 2.3 SPEED-1

```yaml
task_id: OTV2-20261003-speed-1
decision: CONDITIONS-0 §4 (as amended by CREATURE-AI-0 §5.1)
worker: oteryn-impl-worker
review: movement review (Codex, final frozen head)
branch: claude/speed-1-20261003
base: main after CAP-NEG-1 merges
migration_lease: none
depends_on: [CAP-NEG-1]
owned_paths:
  - tools/content-schema/step-speed/{generate_step_speed_table.py,test_generate_step_speed_table.py,README.md}   # new, offline generator
  - content/movement/step_speed_v1.json                  # new: generated table with its digest
  - apps/game-server/src/movement.rs
  - apps/game-server/src/movement/{speed,pacing}.rs      # new
  - crates/protocol-oteryn/src/{lib,world_spatial}.rs    # TOO_EARLY disposition and capability 13 only
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto  # TOO_EARLY only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json      # capability 13; shared register
  - apps/client/src/input.rs                             # step pacing on the client
  - apps/game-server/src/gameplay_transport/connection.rs   # shared register (§0.3): the step arm passes the selected set and the session clock
  - apps/game-server/src/gameplay_transport/mod.rs          # shared register: FreshAdmissionAuthority::step buffers, paces and returns TOO_EARLY or Rejected
  - apps/game-server/src/gameplay_transport/world_spatial.rs  # the step outcome encoding of TOO_EARLY only
  - docs/agents/tasks/archive/OTV2-20261003-speed-1.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-client --quiet
  - python3 -m unittest tools/content-schema/step-speed/test_generate_step_speed_table.py
```

Acceptance:

- Effective speed: player base 110 + level − 1, plus the `SPEED` condition delta, plus an
  equipment term that is 0 until EQUIP-RT-1 supplies it (one function argument), clamped to
  [10, 65,535].
- The step-speed table is generated offline once, checked in with its digest, and read by the
  runtime and the client; nothing evaluates `ln` at runtime. A test checks the digest and sample
  values against Canary.
- Step duration per §4.2, rounded up to 50 ms, with ground speed from the §1.11 lookup seam
  (the engineering map returns 150 for every tile). A test with an injected source paces a step
  onto a tile whose ground speed is not 150. Diagonal × 3 only where diagonal steps exist.
- Players: one buffered step; a second early request is `TOO_EARLY` under capability 13
  `PACED_MOVEMENT_V1` and `Rejected` without it. The existing movement tests are re-measured
  against pacing.
- The production path is wired: the step arm of `connection.rs` and
  `FreshAdmissionAuthority::step` pace every player step. Tests through that path: a buffered
  step executes when due; an early second step is `TOO_EARLY` with capability 13 selected and
  `Rejected` without it; the buffer is dropped at disconnect and resume.
- Capability 13 becomes `offered: true`, and a production-path admission test shows it selected
  (§1.9).
- Not in scope: creature step timing (CREATURE-MOVE-1), chase steps (RANGED-0).

### 2.4 ITEM-VIEW-1b

```yaml
task_id: OTV2-20261003-item-view-1b
decision: ITEM-MOVE-WIRE-0 §4 (server side); this bundle §1.1
worker: oteryn-hard-worker   # session resume state and session-generation scoped handle table
review: independent protocol and session review (Codex, final frozen head)
branch: claude/item-view-1b-20261003
base: main after ITEM-VIEW-1a and CAP-NEG-1 merge
migration_lease: none
depends_on: [ITEM-VIEW-1a, CAP-NEG-1]
owned_paths:
  - apps/game-server/src/gameplay_transport/item_view.rs        # new: handle table, domains 9 and 11
  - apps/game-server/src/gameplay_transport/item_view_tests.rs  # new
  - apps/game-server/src/gameplay_transport/resume.rs           # handle counter and the two high-water revisions only
  - apps/game-server/src/gameplay_transport/world_spatial.rs    # the handle field on item entries only
  - apps/game-server/src/gameplay_transport/mod.rs              # shared register
  - apps/game-server/src/gameplay_transport/connection.rs       # shared register (§0.3): the USE item-target decode and domain 9 and 11 emission only
  - apps/game-server/src/interaction/dispatch.rs                # the USE item-target arm only
  - apps/game-server/src/interaction/corpse_open.rs             # new: open and close (§4.3)
  - docs/agents/tasks/archive/OTV2-20261003-item-view-1b.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
```

Acceptance:

- Handles are monotonic per `GameSessionId`, never reused, live only while the item is in one of
  the session's views, bounded by `ITEMV0-RL-03` (max and max+1). A gone or invisible item is
  `STALE`. A handle never exposes an ItemInstanceId, placement key or row.
- Domain 9 shows the main backpack slot and its direct entries from `read_character_backpack`;
  domain 11 shows the one open corpse. Deltas only after the durable commit that changed the view;
  revisions monotonic per `GameSessionId` (FND-02 §15).
- Resume and channel transfer carry the handle counter and the two high-water revisions; every
  snapshot after them reissues fresh handles, and every older handle is `STALE` (test).
- USE with an item target opens a corpse within Chebyshev 1 on the same floor; dispositions
  `COMMITTED`, `TOO_FAR`, `STALE_STATE`, `NOTHING_TO_USE`. Opening writes nothing. Every closing
  trigger of §4.3 has a test. The D133 disclosure is recorded as `PARITY_PENDING`.
- Capability 4 stays `offered: false`; the tests negotiate it directly.
- Not in scope: command 9 handling, the client.

### 2.5 ITEM-EQUIP-WIRE-1

```yaml
task_id: OTV2-20261003-item-equip-wire-1
decision: ITEM-MOVE-WIRE-1 §3
worker: oteryn-impl-worker
review: independent protocol review (Codex, final frozen head)
branch: claude/item-equip-wire-1-20261003
base: main after ITEM-VIEW-1a merges
migration_lease: none
depends_on: [ITEM-VIEW-1a]
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json      # capability 12; shared register
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto  # EquipmentSlotV1, the two destinations, the slots in domain 9, the new results
  - crates/protocol-oteryn/src/{lib,item_view,item_view_tests}.rs
  - docs/agents/tasks/archive/OTV2-20261003-item-equip-wire-1.md
validation:
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo check --locked --workspace --all-targets
```

Acceptance:

- Capability 12 `ITEM_EQUIP_DROP_V1`, `offered: false`, requires capability 4. Without it command 9
  and domain 9 decode exactly as ITEM-VIEW-1a (test).
- Destinations `EQUIPMENT {slot}` and `GROUND {WorldTilePosition}`; `EquipmentSlotV1` with
  `UNSPECIFIED = 0` and the nine non-container slots; no count field. `UNSPECIFIED` fails closed.
- Domain 9 gains the nine slots (empty, or handle, definition, count, sub-type), within
  `ITEMV0-RL-01` 30.
- Results `SLOT_MISMATCH`, `REQUIREMENT_NOT_MET`, `BLOCKED`; still at most 4 bytes.
- The proto enum values are never durable slot keys (a doc comment and a test that the mapping to
  GAME-ITEM-01 §6.1 keys is a separate table).

### 2.6 ITEM-MOVE-1

```yaml
task_id: OTV2-20261003-item-move-1
decision: ITEM-MOVE-WIRE-0 §5; this bundle §1.1-§1.2
worker: oteryn-hard-worker   # durable value through the TRANSFER writer, replay
review: independent persistence review (Codex, final frozen head)
branch: claude/item-move-1-20261003
base: main after ITEM-VIEW-1b and VIS-3 merge
migration_lease: none
depends_on: [ITEM-VIEW-1b, VIS-3]   # capability 4 requires capability 6 (§1.10)
owned_paths:
  - apps/game-server/src/gameplay_transport/item_move.rs         # new: command 9 handling
  - apps/game-server/src/gameplay_transport/item_move_tests.rs   # new
  - apps/game-server/src/gameplay_transport/mod.rs               # shared register
  - apps/game-server/src/gameplay_transport/connection.rs        # shared register (§0.3): the command 9 arm only
  - apps/game-server/src/combat/pickup.rs                        # only if settle_corpse_pickup needs the CommandRef cause plumbed
  - apps/game-server/tests/corpse_transfer_postgres.rs
  - apps/game-server/tests/support/corpse_transfer_postgres_cases.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json              # capability 4 offered: true only
  - docs/agents/tasks/archive/OTV2-20261003-item-move-1.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test corpse_transfer_postgres --quiet
```

Acceptance:

- Source: an entry of the open corpse only; anything else `NOT_SUPPORTED`. Reach checked by the
  runtime before the TRANSFER. Destination by the B3 rule (container slot or main backpack).
- Replay first: the CommandRef's committed outcome is looked up before the handle; a committed
  command returns `MOVED` even when its handle is `STALE` after a reconnect (PG test). The frozen
  intent binds the ItemInstanceId and destination, never the handle.
- Every `ItemTransferRefusal` maps to the WIRE-0 §5 table (one test per row); D133 and D134 on the
  database clock.
- The open corpse's domain 11 delta and the domain 9 delta follow the commit.
- Capability 4 becomes `offered: true` (§1.1). A production-path test admits a session through
  `connection.rs`, shows capabilities 6 and 4 selected, opens a corpse and loots an entry by command 9
  (§1.9).
- Not in scope: equipment, Ground, partial counts.

### 2.7 ITEM-CLIENT-1

```yaml
task_id: OTV2-20261003-item-client-1
decision: ITEM-MOVE-WIRE-0 §4 (client views); this bundle §1.1
worker: oteryn-impl-worker
review: client review (Codex, final frozen head)
branch: claude/item-client-1-20261003
base: main after ITEM-MOVE-1 merges
migration_lease: none
depends_on: [ITEM-MOVE-1]
owned_paths:
  - apps/client/src/inventory.rs    # new: backpack and open-container windows
  - apps/client/src/{lib,scene,windows_shell,input}.rs   # registration, corpse USE, drag to backpack only
  - docs/agents/tasks/archive/OTV2-20261003-item-client-1.md
validation:
  - cargo test --locked -p oteryn-client --quiet
```

Acceptance:

- The client negotiates capability 4, shows domain 9 as the backpack window and domain 11 as the
  corpse window, opens a corpse by USE on it, and loots an entry by dragging it to the backpack
  (command 9). Results are shown as status text.
- A `STALE` result refreshes nothing locally; the next snapshot or delta does.
- Test: a scripted session that opens a corpse and loots one entry against a test server.
- Not in scope: equipment slots, nested bags and Ground (ITEM-CLIENT-2, -3 and -4).

### 2.7a ITEM-CLIENT-2, ITEM-CLIENT-3 and ITEM-CLIENT-4

```yaml
task_id: OTV2-20261003-item-client-2   # -3, -4 as below
decision: ITEM-MOVE-WIRE-1 §4 and §5 (client side); BAGS-0 §5 (client side); this bundle §0.2
worker: oteryn-impl-worker
review: client review (Codex, final frozen head)
branch: claude/item-client-2-20261003   # -3, -4 likewise
base: main after the packet's dependencies merge
migration_lease: none
depends_on:
  ITEM-CLIENT-2: [ITEM-CLIENT-1, ITEM-MOVE-2a]
  ITEM-CLIENT-3: [ITEM-CLIENT-2, BAGS-1]
  ITEM-CLIENT-4: [ITEM-CLIENT-3, ITEM-MOVE-2b]
owned_paths:
  - apps/client/src/inventory.rs
  - apps/client/src/{lib,scene,windows_shell,input}.rs   # the packet's own windows and drags only
  - docs/agents/tasks/archive/OTV2-20261003-item-client-<n>.md
validation:
  - cargo test --locked -p oteryn-client --quiet
```

Acceptance:

- **ITEM-CLIENT-2:** the client negotiates capability 12 and shows the nine equipment slots
  from domain 9. It equips by dragging a backpack entry to a slot (the `EQUIPMENT` destination),
  unequips by dragging a slot item to the backpack, and swaps on an occupied slot. Refusals,
  `SLOT_MISMATCH` among them, show as status text. Test: a scripted session that equips, swaps
  and unequips against a test server.
- **ITEM-CLIENT-3:** the client negotiates capability 14 and opens a nested bag as its own
  window (domain 14). It closes the bag's subtree when the bag moves, and moves an entry into or
  out of a nested bag by drag (command 21 and command 9). Test: a scripted session that nests a
  bag, opens it, moves an entry in and out, and closes it by moving the parent.
- **ITEM-CLIENT-4:** the client drops a backpack entry or a slot item onto a visible tile in
  range (the `GROUND` destination) and picks up a Ground item from an adjacent tile. Refusals
  (`BLOCKED`, out of range, no line of sight) show as status text. Test: a scripted drop and
  pickup against a test server.
- In all three, a `STALE` result refreshes nothing locally; the next snapshot or delta does.

### 2.8 ITEM-MOVE-2a

```yaml
task_id: OTV2-20261003-item-move-2a
decision: ITEM-MOVE-WIRE-1 §4 and §6 (as amended by ARCH-SLOT-WIRING-1); this bundle §1.3, §1.6, §1.7
worker: oteryn-hard-worker   # persistence, DUR-03 shapes, guards
review: independent persistence review (Codex, final frozen head)
branch: claude/item-move-2a-20261003
base: main after ITEM-MOVE-1, ITEM-EQUIP-WIRE-1 and ITEM-SEM-2b-2 merge
migration_lease: 0063 (proposed)
depends_on: [ITEM-MOVE-1, ITEM-EQUIP-WIRE-1, ITEM-SEM-2b-2]
owned_paths:
  - apps/game-server/migrations/0063_item_equipment_slots.sql
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/item_transfer_audit.rs
  - apps/game-server/src/domain/equipment.rs
  - apps/game-server/src/gameplay_transport/item_move.rs
  - apps/game-server/src/gameplay_transport/item_move_tests.rs
  - apps/game-server/src/gameplay_transport/item_view.rs        # the nine slots in domain 9 only
  - apps/game-server/src/durability/mod.rs                      # shared register
  - apps/game-server/tests/item_transfer_postgres.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                # DUR03-RL-01/-02/-06-EQUIP-SWAP, GAMEITEM01-REACHABLE-ITEMS; own rows only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json             # capability 12 offered: true only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §39 supersessions of WIRE-1 §6.1; own paragraphs
  - docs/agents/tasks/archive/OTV2-20261003-item-move-2a.md
call_sites: the timed_item_host slot call sites when TIMED-RT-1b has merged first (§1.7); the worker names each file at allocation and adds only the calls
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_transfer_postgres --quiet
```

Acceptance:

- Equip a whole backpack entry into one of the nine slots; unequip to the main backpack by the B3
  rule; swap when the slot is occupied (two items, entry count unchanged).
- Legality by `check_equip` with the Item's `equipment.patterns` (§1.4: `primary_slot`,
  `additional_reserved_slots`, `mutually_exclusive_groups`): unknown semantics, a hands conflict
  and any `container` item are `SLOT_MISMATCH`. Level and vocations from the same pattern, inside
  the transaction under `character_root`; a promoted vocation matches its base key (§1.6), and a
  character without a vocation matches only a pattern that lists `none` (once ITEM-SEM-2b-3 adds
  it; before that no pattern lists it). Premium from
  PROD-ENTITLEMENTS-01 §6 evidence at commit, fail closed when stale. An item stays equipped when
  the level drops or Premium ends.
- Migration 0063 makes the WIRE-1 §6.2 2a deltas, amending the 0011 and 0014 guards by replacement:
  slot rows keyed by semantic slot key in the single-location guard; per-hand uniqueness for a
  two-handed claim; backpack entry deletion by TRANSFER; the widened source and receiver kinds;
  the swap receipt with two sources and two receivers.
- Rows of WIRE-1 §6.3 for 2a with max and max+1 tests; `DUR03-RL-07` worst-case swap proven within
  the envelope.
- Equipping never advances `CharacterRevision` (test).
- The §1.7 slot call sites when TIMED-RT-1b is already on `main`, with the stop, empty-lane, reload
  and rejection-rehost tests.
- Capability 12 becomes `offered: true` for the `EQUIPMENT` destination; `GROUND` stays
  `NOT_SUPPORTED` until 2b. A production-path admission test shows it selected (§1.9).
- Merge condition: 0063 is higher than every migration on `main` (§0.1).
- Not in scope: Ground, quivers (QUIVER-1), partial counts (STACK-0), equipment effects.

### 2.9 EQUIP-CONTENT-1

```yaml
task_id: OTV2-20261003-equip-content-1
decision: EQUIP-0 §3.1-§3.2; this bundle §1.5
worker: oteryn-impl-worker   # content lane
review: content review (Codex, final frozen head)
branch: claude/equip-content-1-20261003
base: main after ITEM-SEM-2b-2 and TIMED-CONTENT-1 merge
migration_lease: none
depends_on: [ITEM-SEM-2b-2, TIMED-CONTENT-1]
owned_paths:
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py,README.md}
  - apps/game-server/src/content/{item_abilities.rs,mod.rs}     # new typed ability rows
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/** and the content tree (regenerated)
  - docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json
  - docs/agents/tasks/archive/OTV2-20261003-equip-content-1.md
validation:
  - python3 -m unittest tools/content-schema/item-authoring/test_lower_wiki_stats_packet.py
  - cargo test --locked -p oteryn-game-server --test content_world_project_repository --quiet
```

Acceptance:

- The six typed abilities of §3.1 on item definitions, TibiaWiki first, Canary `items.xml` (the
  D384 pin) as fallback, with Canary's doubled speed unit converted.
- `timed` is derived from `charges.count` or `temporal.duration_ms`; the validator rejects a
  definition whose flag and fields disagree.
- Extra-slot items are those whose pattern's `primary_slot` is `Extra` (2b-2 writes the patterns
  form, §1.4); EQUIP-CONTENT-1 reads that and adds no flag of its own.
- It is never open together with ITEM-SEM-2b-3 (§1.12).
- Evidence file lists every source per item; a definition with no source has no abilities.

### 2.10 EQUIP-RT-1

```yaml
task_id: OTV2-20261003-equip-rt-1
decision: EQUIP-0 §3-§4
worker: oteryn-hard-worker   # combat, derived reads on the ability pipeline
review: independent combat and determinism review (Codex, final frozen head)
branch: claude/equip-rt-1-20261003
base: main after ITEM-MOVE-2a, SPEED-1 and EQUIP-CONTENT-1 merge
migration_lease: none
depends_on: [ITEM-MOVE-2a, SPEED-1, EQUIP-CONTENT-1]
owned_paths:
  - apps/game-server/src/domain/equipment_effects.rs        # new: the active set and the evaluation plan
  - apps/game-server/src/domain/equipment_effects_tests.rs  # new
  - apps/game-server/src/domain/mod.rs                      # one mod line
  - apps/game-server/src/ability/{effects,condition}.rs     # the PROTECTION stage and SUPPRESS admission only
  - apps/game-server/src/movement/speed.rs                  # the equipment term only
  - apps/game-server/src/gameplay_transport/item_move.rs    # recompute calls on equip and unequip only
  - apps/game-server/src/gameplay_transport/mod.rs          # shared register (§0.3): recompute at fresh admission and reconnect, drop at release
  - apps/game-server/src/premium/refresh.rs                 # recompute when an account's Premium view changes (pull, expiry, not current) only
  - apps/game-server/src/premium/tests.rs                   # the Premium-change recompute tests only
  - docs/agents/tasks/archive/OTV2-20261003-equip-rt-1.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
```

Acceptance:

- The equipment owner keeps each actor's active set (§3.2) and recomputes it on equip and unequip,
  and on death, transfer and respawn where those paths exist on `main`. It writes nothing durable.
- Lifecycle triggers, each with a test:
  - a character admitted already equipped, after a fresh login or a server restart, gets its
    active set derived from its slot rows before its first gameplay tick;
  - a reconnect re-derives it;
  - a Premium change re-derives it: a successful pull that changes the view, an expiry and a
    view that stops being current (fail closed);
  - Premium-only effects never outlive the entitlement, while the item itself stays equipped
    (ITEM-MOVE-2a);
  - release drops the in-memory set.
- A `timed` item grants nothing (fail closed); the Extra slot grants all abilities only to its own
  items, else only `LIGHT`.
- The §3.3 plan: slot order, then definition key, then ability order; flats sum; percent stats on
  the base, then flats; i64 with one truncation; protections summed, clamped to [-100, 100],
  applied once at the GAME-ABILITY-01 §10 incoming stage.
- Skills, stats, speed, protections and light are derived reads; nothing is written to A13 or the
  build state. `SUPPRESS` refuses admission and removes an existing instance. Lowering a maximum
  clamps the current value.
- Determinism test: the same equipment in any equip order yields the same derived values.
- Not in scope: imbuement protections (IMBUE-RT-1), parity fixtures (EQUIP-PARITY-1).

### 2.11 BAGS-WIRE-1

```yaml
task_id: OTV2-20261003-bags-wire-1
decision: BAGS-0 §5
worker: oteryn-impl-worker
review: independent protocol review (Codex, final frozen head)
branch: claude/bags-wire-1-20261003
base: main after ITEM-VIEW-1b and ITEM-EQUIP-WIRE-1 merge
migration_lease: none
depends_on: [ITEM-VIEW-1b, ITEM-EQUIP-WIRE-1]
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 14, domain 14, command 21; shared register
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # BAGS-0 §10 wire rows; own rows only
  - docs/contracts/protocol-oteryn/v1/container_tree_v1.proto   # new
  - docs/contracts/protocol-oteryn/v1/item_view_v1.proto   # the CONTAINER destination only
  - crates/protocol-oteryn/src/{lib,container_tree,container_tree_tests,item_view}.rs
  - apps/game-server/src/gameplay_transport/container_view.rs   # new: the up-to-16 views, inner handles, the view command
  - apps/game-server/src/gameplay_transport/container_view_tests.rs  # new
  - apps/game-server/src/gameplay_transport/mod.rs         # shared register
  - apps/game-server/src/gameplay_transport/connection.rs  # shared register (§0.3): the command 21 arm only
  - docs/agents/tasks/archive/OTV2-20261003-bags-wire-1.md
validation:
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-game-server --quiet
```

Acceptance:

- Capability 14 `CONTAINER_TREE_V1`, `offered: false` until BAGS-1, requires capabilities 4 and 12.
  Domain 14 holds up to 16 container views; command 21 opens, closes and goes up; inner entries get
  handles from the ITEM-VIEW-1b table; command 9 gains `CONTAINER {handle}`.
- Domains 9 and 11 keep their meaning; corpses stay at depth 1.
- The BAGS-0 §10 wire rows with max and max+1 tests.
- Not in scope: any durable tree move (BAGS-1).

### 2.12 BAGS-1

```yaml
task_id: OTV2-20261003-bags-1
decision: BAGS-0 §3, §4, §6, §10; this bundle §1.3
worker: oteryn-hard-worker   # persistence, tree guards, locks
review: independent persistence and performance review (Codex, final frozen head)
branch: claude/bags-1-20261003
base: main after ITEM-MOVE-2a and BAGS-WIRE-1 merge
migration_lease: 0064 (proposed)
depends_on: [ITEM-MOVE-2a, BAGS-WIRE-1]
owned_paths:
  - apps/game-server/migrations/0064_item_container_trees.sql
  - apps/game-server/src/durability/item_tree.rs           # new
  - apps/game-server/src/durability/item_tree_audit.rs     # new
  - apps/game-server/src/durability/mod.rs                 # shared register
  - apps/game-server/src/gameplay_transport/item_move.rs   # the CONTAINER destination arm only
  - apps/game-server/src/gameplay_transport/container_view.rs        # post-commit domain 14 updates and closes only
  - apps/game-server/src/gameplay_transport/container_view_tests.rs  # the post-commit view tests only
  - apps/game-server/tests/item_tree_postgres.rs           # new
  - apps/game-server/tests/support/item_tree_postgres_cases.rs  # new
  - apps/game-server/tests/durability_postgres.rs          # shared register
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # BAGS-0 §10 persistence rows; own rows only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 14 offered: true only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # own paragraphs
  - docs/agents/tasks/archive/OTV2-20261003-bags-1.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_tree_postgres --quiet
```

Acceptance:

- Entries keyed by parent item, the tree guards (depth, item count, no cycle), the BAGS-0 §4.2 lock
  order, the tree move, moves into a nested container, and the container slot with contents.
- The `GAMEITEM01-REACHABLE-ITEMS` row is re-registered for trees as BAGS-0 §10 says.
- Every row with max and max+1 tests, and a performance test of the worst-case tree move.
- After each committed tree move, and only after the commit, every affected domain 14 view
  updates (BAGS-0 §5). An entry moved into an open bag appears in that bag's view, and one moved
  out leaves it. A view whose container leaves the character's own trees closes with its
  subtree. Every departed handle is `STALE`. One transport test per case, plus one showing that
  a refused move changes no view.
- Capability 14 becomes `offered: true`, and a production-path admission test shows it selected
  (§1.9).
- Merge condition: 0064 is higher than every migration on `main`; otherwise re-lease (§0.1). It
  amends the guards as left by every lower-numbered migration on `main`.
- Not in scope: Ground trees (BAGS-GROUND-1), depot and trade trees, nested use.

### 2.13 ITEM-MOVE-2b

```yaml
task_id: OTV2-20261003-item-move-2b
decision: ITEM-MOVE-WIRE-1 §5 and §6 (as amended by ARCH-SLOT-WIRING-1); D191, D222; this bundle §1.3, §1.7
worker: oteryn-hard-worker   # persistence, Ground writes, scope fence
review: independent persistence review (Codex, final frozen head)
branch: claude/item-move-2b-20261003
base: main after ITEM-MOVE-2a and MAP-OVERLAY-1 merge
migration_lease: 0065 (proposed)
depends_on: [ITEM-MOVE-2a, MAP-OVERLAY-1]
owned_paths:
  - apps/game-server/migrations/0065_item_ground_drop.sql
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/item_transfer_audit.rs
  - apps/game-server/src/gameplay_transport/item_move.rs      # the GROUND destination and Ground source arms only
  - apps/game-server/src/gameplay_transport/world_spatial.rs  # the D222 order: actors before items
  - apps/game-server/tests/item_transfer_postgres.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json              # ITEMMOVE1-RL-01/-02; own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # own paragraphs
  - docs/agents/tasks/archive/OTV2-20261003-item-move-2b.md
call_sites: the timed_item_host call sites of a slot-to-Ground drop when TIMED-RT-1b has merged first (§1.7)
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_transfer_postgres --quiet
```

Acceptance:

- Drop a whole backpack entry or slot item within 15 tiles, same floor, visible, in line of sight,
  on a tile that accepts items; house tiles `BLOCKED`. Pick up a live pickupable non-corpse Ground
  item from Chebyshev 1 by the `0011` shape.
- Migration 0065 makes the WIRE-1 §6.2 2b deltas by replacement; the §32 scope fence on every
  Ground write. It amends the guards as left by every lower-numbered migration, BAGS-1's 0064
  included.
- Merge condition: 0065 is higher than every migration on `main`; otherwise re-lease (§0.1).
- `ITEMMOVE1-RL-01` 10 per tile under a real tile row lock; `ITEMMOVE1-RL-02` 20,000 per channel,
  alarm at 16,000, with shard rows; the counter changes in the same transaction as each drop,
  pickup and `WorldReset` retirement (tests for each, and that the counter equals the live rows).
- D222: actors rank before items in D87's order; dropped items never push an actor out of a
  snapshot (test at the 256 ceiling).
- The §1.7 drop call sites when TIMED-RT-1b is on `main`, with success and rejection tests.
- Not in scope: Ground to Ground (GROUND-MOVE-1), Ground to slot, trees (BAGS-GROUND-1).

### 2.14 EXERCISE-1

```yaml
task_id: OTV2-20261003-exercise-1
decision: EXERCISE-0 §4-§5; TIMED-ITEM-0B §12; TIMED batch §2.3 (the exercise binding and the two composed writers moved here); this bundle §1.3
worker: oteryn-hard-worker   # persistence, composed DUR-03 writers, determinism
review: independent persistence and determinism review (Codex, final frozen head)
branch: claude/exercise-1-20261003
base: main after TIMED-RT-1b, EXERCISE-CONTENT-1 and WORLDINT-USE-1 merge
migration_lease: 0066 (proposed)
depends_on: [TIMED-RT-1b, EXERCISE-CONTENT-1, WORLDINT-USE-1]
owned_paths:
  - apps/game-server/migrations/0066_exercise_training_checkpoint.sql
  - apps/game-server/src/domain/exercise.rs              # new: start, tick, stop, idle
  - apps/game-server/src/domain/exercise_tests.rs        # new
  - apps/game-server/src/domain/mod.rs                   # one mod line
  - apps/game-server/src/domain/timed_item_host.rs       # the exercise binding (place 3) only
  - apps/game-server/src/durability/item_timed_state.rs  # the composed checkpoint and composed expiry burn writers only
  - apps/game-server/src/durability/item_timed_state_audit.rs
  - apps/game-server/src/durability/character_build.rs   # the `training` receipt cause only
  - apps/game-server/tests/item_timed_state_postgres.rs
  - apps/game-server/tests/support/item_timed_state_postgres_cases.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json         # EXERCISE0-RL-01/-02/-03; boundary_tests of the two composed rows; own rows only
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # own paragraphs
  - docs/agents/tasks/archive/OTV2-20261003-exercise-1.md
call_sites: the USE-WITH arm of WORLDINT-USE-1's gate and the stop hooks of §4.3; the worker names each file at allocation and adds only the calls
validation:
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test item_timed_state_postgres --quiet
```

Acceptance:

- Start by `USE-WITH` weapon on dummy with the six checks of §4.1 in order, each refusal changing
  nothing; the `exercise_dummy` cooldown starts only at an admitted start.
- 2 s ticks on the channel owner's clock; live charges and tries only; no random draw; every stop
  of §4.3 has a test; the idle timer pauses and resumes.
- The composed checkpoint: build receipt (cause `training`) and the timed-row `STATE_MUTATION` in
  one transaction under `character_root`, at the cadence, at every level advance, at the stop,
  before death, at logout and handoff; never split; a mismatch writes nothing and stops the
  session. The §5.2 value guard by the receipt's `exercise` provenance fields.
- The last charge retires the weapon by the composed expiry burn in the same transaction; 0 charges
  are never stored.
- Writer-backed max and max+1 tests on the composed checkpoint and composed expiry burn rows, added
  to their `boundary_tests` (TIMED batch §2.3 first item; merge condition).
- `TIMEDITEM0B-RL-04` repeated with the binding as the 11th live item.
- Crash between checkpoints loses at most one interval of charges and tries together (PG test).
- Merge condition: 0066 is higher than every migration on `main`; otherwise re-lease (§0.1).
- Not in scope: house dummies' single-user lane beyond check 6, parity fixtures.

## 3. Decision test

- Every requested slice (ITEM-MOVE-2a, ITEM-MOVE-2b, EQUIP-RT-1, EXERCISE-1) has a packet with
  owned paths, dependencies, a migration need and a worker kind.
- Every other slice of EQUIP-0, EXERCISE-0, DEPOT-0, BAGS-0 and IMBUE-FORGE-0 is either packeted or
  listed in §1.8 with what it waits on.
- No packet starts before its prerequisites; no two open packets own the same path except the
  §0.3 registers.
- Every offered capability is selectable on the production path (§1.9, CAP-NEG-1), with its
  `requires` offered first (§1.10).
- Every value a packet reads has a production source or an allocated seam with the packet that
  supplies it (§1.11).
- Migration history stays monotonic in any merge order (§0.1 merge condition and re-lease).
- Every server destination these packets add has a client packet (ITEM-CLIENT-1 to -4), and
  every equipment lifecycle trigger (admission, restart, reconnect, Premium change, release) has
  an owned path in EQUIP-RT-1.
- No ruling widens an accepted decision; §1.1 and §1.5 only order or split work, and §1.6 applies
  ITEM-SEM-2b-2 §6 as written.
