# ARCH-NPC-PACKETS-1 NPC track: content model and wire packets

- Decision: `ARCH-NPC-PACKETS-V1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  review on the frozen head and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane (#162, 2026-10-05): packet the NPC track's first children that can
  start now. Executes NPC-0 (`OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md`) with
  the amendments of NPC-BEHAVIOUR-0, QUEST-GATE-0 and TIMED-ITEM-0, all on `main`. No contract
  changes; this document only binds scope, paths, order and acceptance.
- Runtime, migration, production and protected-World authority: NONE. Each packet needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **NPC-CONTENT-1 (impl).** A pure runtime content model over the merged `NpcDataCatalogue`:
   the offer and route rules of NPC-0 §3.4 (with the TIMED-ITEM-0 §6.2 count rule and the
   QUEST-GATE-0 route amendment), item resolution through a narrow item-facts trait, and the
   generated minimal replies of §3.3. No wire, no runtime state, no persistence (§2.2).
2. **NPC-WIRE-1 (impl, protocol review).** The registry and proto rows of NPC-0 §4
   (capability 3, commands 7 and 8, domains 7 and 8, `offered: false`), the fail-closed codecs
   in `protocol-oteryn`, the server capability rows and the limit rows `NPC0-RL-01..07`. No
   send path (§2.3).

Order: NPC-CONTENT-1 now -> NPC-WIRE-1 after it and after #1824 (§2.1). NPC-PLACE-1, NPC-TALK-1
and the later children are not packeted here (§1.4).

Owner questions: none.

## 1. Rulings

### 1.1 Facts on `main` (dfdb4e21b)

- `apps/game-server/src/content/npc_catalogue.rs` loads the data-only catalogue from the World
  Project (`content/world`) and fails closed on a tree digest mismatch
  (`load_data_only_npc_catalogue`). `tests/content_npc_catalogue.rs` pins revision
  `g4-npc-provisional-enrichment-r28`, digest `5ae5b900…baa05cbaf2`, 1,282 NPCs, 836 Dialogues
  and 380 Services.
- Services: 316 with offers (12,234 offers: 7,032 sell to player, 5,202 buy from player; at most
  757 in one Service, which is `NPC0-RL-03`'s floor), 56 with routes (195 routes, 12 at price 0,
  all in the `global-target-2026-09-27` frame), 8 with neither. Routes carry no gate field.
- Offers NPC-0 §3.4 classifies today: 31 with a non-gold currency (two token items), 299 selling
  above 1,009,999, 430 with `count` above 100 (counts 500, 1,800 and 14,400 on 27 item keys, by
  value charges or durations the item facts must classify), 2 with `count` 2..100, none with `sub_type`,
  none `parity_pending`.
- 446 NPCs have no Dialogue (generated replies, §3.3); 377 NPCs name at least one Service.
- The protocol registry leaves capability 3, commands 7 and 8 and domains 7 and 8 free.

### 1.2 NPC-CONTENT-1 is a model, not a loader

The catalogue loader and its digest pin exist; NPC-CONTENT-1 does not reload or re-pin. It adds,
in a child module of `npc_catalogue.rs`, one entry point on `NpcDataCatalogue` that builds an
immutable `NpcServiceModel` from the catalogue and an item-facts source:

- **Admitted or held, with a reason.** Every offer and route is either admitted or held with a
  closed reason enum: `NonGoldCurrency`, `UnknownItem`, `CountOutOfRange`,
  `TimedCountMismatch`, `SellPriceAboveCoinCapacity`, `ArbitragePaying`, `ParityPending`. Each
  rule is NPC-0 §3.4 verbatim, applied in that order; arbitrage runs last, over the offers still
  admitted, per item definition and sub-type, per unit (`unit_price / count`, compared exactly as
  a rational, never in floating point). `parity_pending: true` holds the offer (the source
  marks it unresolved).
- **Count.** On a stackable item `count` defaults to 1 and must be 1..=100. On a charged or
  fluid item `count` is the charges or sub-type; on an admitted timed definition it must equal
  the definition's charges (TIMED-ITEM-0 §6.2, refused as `TimedCountMismatch`). Anything else
  is `CountOutOfRange`.
- **Routes.** With QUEST-GATE-0 on `main`, a quest-gated route loads; the source has no gate
  field, so every route loads with its level and Premium gate; price 0 is free. The destination
  stays in the source frame; NPC-PLACE-1 maps it.
- **Held NPC.** An NPC whose Dialogue or Service reference does not resolve in the catalogue is
  held whole. A Service with no admitted offer and no admitted route is kept with its held list;
  NPC-TALK-1 decides what an empty window shows.
- **Minimal replies (§3.3).** An NPC without an admitted Dialogue gets generated lines: greeting,
  farewell and one line per admitted service, from a fixed Oteryn template filled with the NPC
  name and, for travel, the destination key and price. The template is code, the output is data
  (strings and template ids), never wiki text. An admitted Dialogue replaces them.
- **Determinism.** Iteration in ascending key order; the model is equal for equal inputs, and a
  test builds it twice and compares.

Item facts come through a trait in the new module (`NpcItemFacts`: known, stackable, charged,
fluid, timed charges), so the model has no dependency on files other open writers hold. The
production adapter reads the existing public item API read-only. If that API cannot answer one
of the five facts without editing `reference_playable.rs` or `item_admission.rs`, the worker
returns `QUESTION` and does not edit them.

### 1.3 NPC-WIRE-1 registers the wire, it does not serve it

- Capability 3 `NPC_SERVICE_V1` is registered `offered: false`, no requirement; commands 7 and 8
  and domains 7 and 8 exactly as NPC-0 §4. The proto file defines the messages; the codecs
  decode fail-closed (unknown field, oversize text, control characters, out-of-range enum) with
  max and max+1 tests.
- Limits `NPC0-RL-01..07` are registered with measured values: `-03` at least 757 (measured
  757 today), `-07` = 4 (Chebyshev, same floor), the others measured or set by the worker from
  the encoded worst case and cited in the row. A domain-8 snapshot of `-03` offers must fit the
  domain byte bound registered with it.
- The server rows in `capabilities.rs` make capability 3 known and unoffered, so a client that
  asks for it is not granted it, and commands 7 and 8 are refused as unsupported for everyone.
  The send path, the domain owner and the session revision stream belong to NPC-TALK-1.
- The decoded view types in `protocol-oteryn` are the client views of NPC-0's brief; the client
  UI is a later client packet.

### 1.4 Children not packeted now

| Child | Waits for |
|---|---|
| NPC-PLACE-1 | NPC-CONTENT-1; MAP-BUNDLE-1 and the map track's bundle World (ARCH-MAP-TRACK-PACKETS-V1) |
| NPC-TALK-1 | NPC-WIRE-1, NPC-PLACE-1, the bundle World serving sessions (MAP-CUTOVER-1b) |
| NPC-TRADE-1, NPC-TRAVEL-1 | NPC-TALK-1 and GOLD-FEE-1b / GOLD-FEE-ACT-1 |
| NPC-ACTOR-1, NPC-VOICE-1, NPC-VIS-1, NPC-TALK-2, NPC-CONTENT-2 | NPC-BEHAVIOUR-0 order: NPC-PLACE-1, CREATURE-MOVE-1, SPEED-1, NPC-WIRE-1, CHAT-1, VIS-2, NPC-TALK-1, NPC-CONTENT-1 |
| TRAVEL-CONTENT-1, NPC-QUEST-CONTENT-1, BANK-NPC-1 | NPC-CONTENT-1 plus their own decisions' dependencies |

NPC-TALK-1 also takes the NPC-0 §3.1 "compiled content identity" binding (the catalogue digest
in the boot content identity) and the `content/mod.rs` re-export of the model, because both
touch files open writers hold today.

### 1.5 Checklist before freeze

1. No contract text changes; NPC-0 §3.4, §4 and its amendments are executed as written.
2. Concurrency: the model is immutable and built once per catalogue; no shared mutable state.
3. Restart-sufficient: the model is a function of the pinned catalogue and the item facts.
4. Typed references: Oteryn keys and typed ids; held reasons are an enum, not free text.
5. Older peers: capability 3 was never offered and stays unoffered.
6. Split work: each packet is observable alone by its tests and reverts cleanly.

## 2. Packets

### 2.1 Order and shared files

- NPC-CONTENT-1 owns only new files plus one `mod` line in `npc_catalogue.rs`. It must not edit
  `content/mod.rs` or `lib.rs` (#1807), `serve.rs` (#1804), `reference_playable.rs`,
  `item_admission.rs` (#1830) or any `content/**/index.json` (#1807).
- NPC-WIRE-1 shares `PROTOCOL_OTERYN_V1_REGISTRY.json` and `crates/protocol-oteryn/src/lib.rs`
  with #1824 and `RESOURCE_LIMITS_REGISTRY.json` with #1839 and KRC. The control plane
  allocates it after #1824 merges and serialises the limit rows behind the open writers of that
  file. It does not touch `connection.rs`.

### 2.2 NPC-CONTENT-1 (impl worker)

```yaml
task_id: OTV2-20261005-npc-content-1
decision: ARCH-NPC-PACKETS-V1 §1.2; NPC-0 §3.1, §3.3, §3.4, §6.2
depends_on: []
worker: oteryn-impl-worker
review: Codex, on the frozen head
branch: agent/npc-content-1-20261005
base: main
owned_paths:
  - apps/game-server/src/content/npc_catalogue.rs            # the child `mod` line and the entry point only
  - apps/game-server/src/content/npc_catalogue/service.rs     # new
  - apps/game-server/src/content/npc_catalogue/service_tests.rs  # new
  - apps/game-server/src/content/npc_catalogue/replies.rs     # new
  - apps/game-server/tests/content_npc_service_model.rs      # new
  - docs/agents/tasks/archive/OTV2-20261005-npc-content-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server npc_catalogue
  - cargo test --locked -p oteryn-game-server --test content_npc_catalogue
  - cargo test --locked -p oteryn-game-server --test content_npc_service_model
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope:** §1.2: `NpcItemFacts`, `NpcServiceModel`, the held-reason enum, the rules in order,
  exact-rational arbitrage, the generated replies, and the production item-facts adapter over
  the existing public item API.
- **Acceptance:** unit tests per rule at its boundary (count 0, 1, 100, 101; sell price
  1,009,999 and 1,010,000; a timed count equal and unequal to the charges; arbitrage equal,
  below and above the lowest sell per unit; an unknown item; a non-gold currency); the model is
  identical when built twice; the repository test builds the model over the pinned `main`
  catalogue and pins the admitted and held counts per reason (expected near §1.1: 31
  `NonGoldCurrency`, 299 `SellPriceAboveCoinCapacity`; the rest as measured and explained in the
  archive record), the 446 generated-reply NPCs and 195 admitted routes.
- **Not in scope:** placement, the wire, runtime conversation state, any edit to the files of
  §2.1, a new catalogue digest pin, content data changes.

### 2.3 NPC-WIRE-1 (impl worker, protocol review)

```yaml
task_id: OTV2-20261005-npc-wire-1
decision: ARCH-NPC-PACKETS-V1 §1.3; NPC-0 §4, §4.1; NPC-BEHAVIOUR-0 §7 (NPC0-RL-07)
depends_on: [OTV2-20261005-npc-content-1, "#1824 merged"]
worker: oteryn-impl-worker
review: Codex, on the frozen head; protocol review
branch: agent/npc-wire-1-20261005
base: main
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json          # capability 3, commands 7 and 8, domains 7 and 8
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json             # own NPC0-RL-01..07 rows only
  - docs/contracts/protocol-oteryn/v1/npc_service_v1.proto   # new
  - crates/protocol-oteryn/src/lib.rs                        # mod line and re-exports only
  - crates/protocol-oteryn/src/npc_service.rs                # new
  - crates/protocol-oteryn/src/npc_service_tests.rs          # new
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - docs/agents/tasks/archive/OTV2-20261005-npc-wire-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --locked --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-game-server capabilities
  - python tools/repository/validate_repository_policy.py
  - python tools/agents/validate_governance.py
  - git diff --check
```

- **Scope:** §1.3: registry rows, proto, codecs, limits, capability 3 known and unoffered,
  commands 7 and 8 refused as unsupported.
- **Acceptance:** encode and decode round trips; max and max+1 tests for every bounded field;
  a 757-offer domain-8 snapshot encodes within its registered byte bound; a client asking for
  capability 3 is not granted it; commands 7 and 8 return the unsupported refusal; registry
  validators pass.
- **Not in scope:** `connection.rs`, the domain owner, the revision stream, any send path, the
  client UI, offering capability 3.

## 3. Rejected options

- **One NPC-CONTENT-1 that also re-exports from `content/mod.rs` and pins the content identity.**
  Rejected: both files are held by open writers (#1807, #1804); the re-export and the pin have
  no consumer before NPC-TALK-1.
- **Editing the item modules to expose charges and stackability.** Rejected for this packet:
  #1804 and #1830 hold them. The trait keeps the model testable; a missing fact is a `QUESTION`.
- **Offering capability 3 in NPC-WIRE-1.** Rejected: there is no server send path until
  NPC-TALK-1; an offered capability with no domain owner would lie to the client.
- **Packeting NPC-PLACE-1 now.** Rejected: it maps destinations into the bundle frame and checks
  walkability against the bundle, which needs the map track's bundle World.

## 4. Decision test

The decision holds if NPC-CONTENT-1 lands a model whose held counts are explained per reason
and NPC-WIRE-1 lands codecs and rows with capability 3 unoffered, both without touching a file
§2.1 forbids, and NPC-TALK-1 can consume both without reopening this document.
