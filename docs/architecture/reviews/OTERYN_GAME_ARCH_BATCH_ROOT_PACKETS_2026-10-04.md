# Architect batch: root packets for the map, item use and bank fee chains

```yaml
decision_id: ARCH-BATCH-ROOT-PACKETS-V1
status: CANDIDATE
date: 2026-10-04
owner: Sol Supervising Architect
requested_by: control plane (ARCH-ROOT-PACKETS-1; the missing roots named by ARCH-BATCH-ITEM-EQUIP-PACKETS-V1 §3) and the owner (2026-10-04, #162; answers recorded in §1.1)
writes_on_other_prs: none
```

This batch packets the three roots that block the most packeted and unpacketed work:
MAP-LOAD-1 (the map chain: MAP-OVERLAY-1, MAP-WIRE-1, DEPOT-WIRE-1, NPC-ACTOR-1 and the ground speed
source of SPEED-1), ITEM-USE-WIRE-1 (food, potions, IMBUE-WIRE-1, FORGE-CONTENT-1) and GOLD-FEE-2
(stage 2 of D174, which IMBUE-1, FORGE-1, CHARM-6, NPC-TRADE-1 and NPC-TRAVEL-1 consume). It also
packets the two bank packets GOLD-FEE-2 needs (BANK-RET-0, BANK-1), records the acceptance review of
NPC-BEHAVIOUR-0, and disposes of Issue #513's resource limits.

The batch changes no code, no contract and no wire. Its rulings in §1 are architecture rulings
under the decisions they cite. Live PR and Issue state governs. When this was written:

- on `main`: MAP-BUNDLE-1 (the format document, the compiler and its reader in
  `tools/world-bundle-compiler`), SPEED-1 (#1715, capability 13 and the `GroundSpeedSource` seam),
  CAP-NEG-1 (#1705), CAP-NEG-RESUME-FALLBACK-1 (#1708), ITEM-VIEW-1a (#1703), ITEM-VIEW-1b
  (#1713), BAGS-WIRE-1b (#1730), GOLD-FEE-1b (`0031`, the three coins and the change guards),
  the gold fee writer `durability/item_fee_burn.rs` with its one source `CharmUnassign`;
- not built: any runtime map reader, any bank table or code, `ITEM_USE_V1`, fields 4 and 5 of
  `USE_INTENT`.

## 0. Leases, order and shared files

### 0.1 Leases

Leased by the control plane (#1733, 2026-10-04). A worker that needs another number stops and asks.

| Packet | Migration | Capability / command / event / profile |
|---|---|---|
| MAP-LOAD-1 | none | none |
| ITEM-USE-WIRE-1 | none | capability 15 `ITEM_USE_V1` (leased; `offered: false`, `requires: [4]`); no new command type (USE stays command type 2) |
| BANK-RET-0 | none | retention profiles `ECONOMY_LEDGER_RETENTION_V1` (purpose `ECONOMY_LEDGER`, event type 3) and `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2` (successor for event type 2, future admission only; §1.7) |
| BANK-1 | 0071 | game event type 3 `BANK_OPERATION` |
| GOLD-FEE-2 | 0072 | none (the fee event stays event type 2) |

Migrations 0061-0067 are leased to FORGE-1b, TIMED-RT-1c, ITEM-MOVE-2a, BAGS-1, ITEM-MOVE-2b,
EXERCISE-1 and (conditionally) CAP-NEG-RESUME-FALLBACK-1; 0068 is leased to #1534 (D451), `main`
already carries 0069, and 0070 is leased to WHEEL-W1 (D483). BANK-1 therefore takes 0071 and
GOLD-FEE-2 0072; the next free number is 0073. A migration merged with a number lower than one
already applied is refused by the ordered runner, so each migration packet's acceptance carries the
merge condition: **it merges only when every lower leased migration has merged or been released by
the control plane.** If BANK-1 or GOLD-FEE-2 would wait on a stalled lower lease (0061-0068 or
0070), the control plane re-leases it the next free number above the highest applied one; the
worker renames the file only on that instruction.

Capability numbers registered on `main` are 1, 4, 6, 7, 8, 10, 12, 13 and 14. Numbers 2, 3, 5, 9 and
11 are reserved by accepted decisions (WEAPON_PROFICIENCY, NPC_SERVICE, DEPOT, HIGHSCORES,
TIMED_ITEMS). The control plane leased 15 to `ITEM_USE_V1`; QUEST-LOG-WIRE-1 moves to 16.

### 0.2 Order

| Step | Packet | Worker / review | Starts when |
|---|---|---|---|
| 1 | MAP-LOAD-1 | hard, security review of the reader | this batch merges |
| 1 | ITEM-USE-WIRE-1 | impl, protocol review | this batch merges |
| 1 | BANK-RET-0 | control plane routes; privacy review | this batch merges |
| 2 | BANK-1 | hard, persistence review | BANK-RET-0 has merged |
| 3 | GOLD-FEE-2 | hard, persistence review | BANK-1 has merged |

The three step-1 packets touch disjoint files except `RESOURCE_LIMITS_REGISTRY.json` (MAP-LOAD-1
changes MAP01 rows, ITEM-USE-WIRE-1 adds ITEMUSE0-RL-03). They are separate rows; the second to
merge takes `main` in with a merge commit and keeps both.

### 0.3 Shared files

| File | Packets | Rule |
|---|---|---|
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | MAP-LOAD-1, ITEM-USE-WIRE-1, BANK-1, GOLD-FEE-2 | each edits only its own rows |
| `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` | BANK-RET-0, BANK-1, GOLD-FEE-2 | BANK-RET-0 the profiles, BANK-1 event type 3, GOLD-FEE-2 nothing unless §2.5 says so |
| `apps/game-server/src/durability/mod.rs` | BANK-1, GOLD-FEE-2 | module lines and re-exports only |
| `Cargo.toml`, `Cargo.lock` | MAP-LOAD-1 | one new workspace member (§1.3) |

## 1. Rulings

### 1.1 Owner answers recorded (2026-10-04)

- **BANK-0 Q1 = b.** A junior (starter-island) character cannot use the bank until it has left the
  island; it pays with coins only. BANK-0 §4.4, §4.3 and §11 are updated by this batch: the
  assumption becomes the owner's decision. GOLD-FEE-2 keeps stage 1 for a junior payer
  (BANK-FEE-0 §3).
- **Batch scope 2a.** BANK-RET-0 and BANK-1 are packeted here, in the order BANK-RET-0, BANK-1,
  GOLD-FEE-2, and #513's limits are reviewed here (§1.6).
- **Bundle staging** was left to the architect; §1.2 rules it.
- **Next wave.** MAP-OVERLAY-1, ITEM-USE-1 and NPC-ACTOR-1 are packeted in the next architect
  batch, after this one merges, so this batch stays one review round.

### 1.2 Bundle staging: a CI-built artifact pinned by digest

The production World Bundle is not checked in (about 23.7 MB) and is not built on the node. It is
an artifact built by repository CI with the compiler from the pinned World Project source, and
its digest is pinned in the World's configuration. The server loads a bundle only if its digest,
checksums, schema versions, content revision and `build_class` match the pins (ADR-0021 §4.2); any
other bundle stops the World before admission.

- One reproducible source: compilation is deterministic (MAP-BUNDLE-1), so CI can rebuild the
  artifact and compare digests. The other agent that prepares map sources feeds the World Project
  source; it never hands the server a bundle.
- Rejected: building at node start (couples boot to the compiler and its inputs, and makes boot
  time depend on compilation); a checked-in bundle (a 23.7 MB binary in Git, reviewed by nobody);
  a bundle fetched from an unpinned location.
- MAP-LOAD-1 implements the check against a pinned digest passed in by its caller and tests it.
  Where the pin lives in the World configuration, the CI job and the artifact store bind
  MAP-CUTOVER-1, which is packeted with the next wave; they are not MAP-LOAD-1's.
- Coordination with the agent preparing maps goes through the control plane.

### 1.3 One bundle byte layout, shared by the compiler and the server

`tools/world-bundle-compiler/src/bundle.rs` and `sector.rs` already hold a reader (`read_with`,
`ReadCaps`, `sector::decode`) used by the compiler's own tests. The server must not depend on a
tool crate, and a second reader would let the two drift. MAP-LOAD-1 therefore moves the byte-layout
types, the reader and its caps into one new library crate, `crates/world-bundle`
(`oteryn-world-bundle`), with no dependency on the compiler or the server. The compiler keeps its
writer and depends on the new crate for the layout; `apps/game-server` depends on it for reading.
The move changes no byte and no rule of the format document, and the compiler's tests stay green.

### 1.4 The ground item of a tile, and the ground-speed source

- A tile's ground item is the first top-level entry whose resolved Terrain record has kind
  `ground`. 2,244 Terrain records are kind `ground`, and each has a KNOWN `ground_speed`; border,
  wall, roof and field records have none.
- A tile with no ground item is not walkable and has no ground speed. The map source returns 0 for
  it, and for a ground item whose `walkable` is KNOWN `false` (the 200 records with speed 0;
  ARCH-ITEM-PACKETS-AMEND-2 §1.11). `player_step_duration` already refuses 0, so nothing paces on
  it.
- MAP-LOAD-1 adds a map-backed `GroundSpeedSource` next to `EngineeringGroundSpeed`. Production
  keeps `EngineeringGroundSpeed` (150) until MAP-CLIENT-1 switches server and client together
  (ADR-0021 amendment). The map source is built and tested, not wired into the live path.
- **Where the semantics come from (#1733 P1 4176957737).** The bundle carries only a palette key,
  family and compact id (format §3, §5), and the server's `TerrainDefinition` holds only a key, so
  neither gives kind, walkable or ground speed. The bundle format stays v1: the loader takes a
  second input, the Terrain catalogue of the World Project (`content/world/terrain/terrain-*.json`)
  at the bundle's `identity.content_revision`. The loader refuses the pair unless the catalogue's
  `project_revision` and lock digest (`content/world/content.lock.json`) equal the bundle's
  `content_revision` and `content_lock_digest`. A palette entry of family `terrain` resolves to the
  record with that key; one of family `item` resolves to the one Terrain record whose
  `item_pointer` names that key, or to none (not ground, not walkable). The loader reads `kind`,
  `walkable` and `ground_speed` from the resolved record and refuses the bundle when a palette
  key of family `terrain` has no record, two records point at one Item key, or a `ground` record
  has an UNKNOWN `walkable` or `ground_speed`. Versioning the format would move the same data into
  every bundle and needs a format v2 and a compiler change; the catalogue input needs neither.

### 1.5 NPC-BEHAVIOUR-0 acceptance review

NPC-BEHAVIOUR-0 was reviewed against its sources:

- §3 (one runtime actor per placement per channel; `NPCBEH0-RL-01` 2,048) matches NPC-0 §3.2 and the
  multichannel model. Kind 5 `Npc` in capability 6's schema is safe because capability 6 is not
  offered (`PRODUCTION_OFFERED_CAPABILITIES` holds only 13) and kind 5 is absent from
  `world_spatial_entities.rs`; NPC-VIS-1 adds it before VIS-3 offers capability 6.
- §4 (a 1,000 ms think only while perceived; `NPCBEH0-RL-02` 256; RNG streams `NPC_WANDER` and
  `NPC_VOICE`) matches SIM-DETERMINISM-01 §10 and §12 and CREATURE-AI-0 §5.1.
- §5 (walking), §6 (voices) and §7 (focus queue in GAME-NPC-SERVICE; `NPC0-RL-07` 4 tiles) stay
  within the boundaries they cite. Its R1-R3 are ruled a, and it has no owner question.

One defect: the decision amends four documents without an `Amends` line. This batch adds:
"MOVE-RL-11 §4.2, §4.3, §4.5; CREATURE-AI-0 §4.1, §7; CHAT-0 §3; NPC-0 §4.1". One clarification:
the VSL-MOVE-01 occurrences `NPC_STEP`, `NPC_RELOCATE` and `NPC_TURN` are defined by NPC-ACTOR-1
under VSL-MOVE-01 §10, not by this decision.

Result: **ACCEPTED WHEN THIS DECISION MERGES.** This batch sets NPC-BEHAVIOUR-0's status line.
NPC-VIS-1's gate (ARCH-BATCH-ITEM-EQUIP-PACKETS-V1 §0.2 step 0a) is then met when this batch
merges, because ITEM-VIEW-1a has merged.

### 1.6 Issue #513 (DUR03-RL limits)

The DUR-03 resource maxima decision (D50-D52, merged in `0f80b8c`) set the limits, and B4
registered them in `RESOURCE_LIMITS_REGISTRY.json` with evidence
`docs/agents/evidence/OTV2-20260928-dur03-b4-binding-513.md`:
`DUR03-RL-01` 1, `RL-02` 2, `RL-03` 0, `RL-04` 0, `RL-05` 0, `RL-06` 1/3, `RL-07` (1 event,
9,216 B envelope, 7,936 B payload, 512 B key, 128 B technical), `RL-08` 3, and the P90D audit
retention.

Review: they are still correct for every shape on `main`. Later shapes extend them only through
per-shape rows, never by raising the base rows: `DUR03-RL-03-BANK` and `DUR03-RL-01-BANK` (BANK-1),
`DUR03-RL-03-FEE` (GOLD-FEE-2), and the shapes' own rows. Each such packet re-measures the
`DUR03-RL-07` envelope with its added lines and registers the measured value, with a max and
max+1 test.

Disposition: nothing in #513 blocks work. `DUR03-RL-08` (3) is the one value the decision says
stage C must decide again. Recommendation to the control plane: close #513 and record the stage C
re-decision of `DUR03-RL-08` on the stage C task, and move its active task record
(`docs/agents/tasks/active/OTV2-20260917-dur03-reference-one-item-resource-evidence-513.md`,
status `blocked`) to the archive in that close. The record is outside this batch's owned paths.

### 1.7 The bank event's retention

`DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1` excludes an economy purpose, so the bank event
cannot reuse it. BANK-RET-0 adds `ECONOMY_LEDGER_RETENTION_V1` for event type 3: purpose
`ECONOMY_LEDGER` (prove and reconcile bank balances, ledger entries and coin lines; no market
analytics, public history, detector or AI use), privacy class `RESTRICTED_PLAYER_LINKED`, a
finite ceiling no longer than P90D unless the privacy review records why, and the same legal-hold
and deletion rules as the DUR-03 profile.

A fee event that carries a `FEE_DEBIT` value line (GOLD-FEE-2) stays event type 2, one fee event
per fee (BANK-FEE-0 §4.3). The registry makes a profile's identity immutable once an event is
admitted under it (`retention_policy`: `in_place_policy_change_after_admission: FORBIDDEN`,
`successor_policy_requirement: NEW_IMMUTABLE_PROFILE_ID_WITH_POSITIVE_POLICY_REVISION`), so
`DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1` is never revised (#1733 P1 4176849286). BANK-RET-0
instead registers a successor profile, `DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2`, with a positive
`policy_revision` and a purpose that adds the bank part of a fee (the `FEE_DEBIT` value line) to
the V1 purpose, with no other change unless the privacy review records why. BANK-RET-0 only
registers V2; event type 2 stays bound to V1 until the activation boundary below.

**One activation boundary for every type-2 producer (#1733 P1 4176877703).** The registry binds an
event type to one `retention_profile_id`. So V2 cannot apply to fee events only. Because V2's
purpose is V1's plus the bank part, it covers every type-2 shape, and all type-2 producers move to
V2 together. Every producer (mint, transfer, reward claim, decay retire, timed expiry and fee
burn) already writes the one shared constant `item_mint_audit::RETENTION_PROFILE_ID`. The boundary
is GOLD-FEE-2's merge and deploy. In that one change:

- the event type 2 `retention_profile_id` in the registry becomes V2;
- the shared constant becomes V2, so every type-2 producer writes V2 from the same build;
- migration `0072` replaces the `0010` outbox's two single-value CHECKs, `schema_revision = 1`
  and `retention_profile_id = V1`, with one tuple CHECK: `(schema_revision,
  retention_profile_id)` is `(1, V1)` or `(2, V2)` (#1733 P1 4176934053). Stored rows are
  `(1, V1)` and stay valid. New rows are `(2, V2)`. `(1, V2)` and `(2, V1)` are refused;
- the audit verifier admits a stored event under V1 or V2, and judges each by the profile in its
  own envelope.

Every event already admitted keeps V1 (`existing_envelope_binding: ORIGINAL_RETENTION_PROFILE_ID`),
including events a not-yet-upgraded node writes during the rollout. No existing event is
migrated, and V1 is never revised (`successor_rollout_scope:
FUTURE_ADMISSION_ONLY_AFTER_REVIEWED_ACTIVATION_BOUNDARY`). A separate event type for fee events
is rejected: it would need a new payload contract, outbox type and verifier for one shape, where
the shared constant moves every producer at once.

The ledger, balance, operation and coin-line tables are authoritative game state, not event
retention. They are never deleted by retention (BANK-0 §3 grants no DELETE).

### 1.8 BANK-1 scope

BANK-1 builds the four tables, guards, grants, the writer and event of BANK-0 §3-§5 for the four
kinds `DEPOSIT`, `WITHDRAW`, `TRANSFER_OUT` and `TRANSFER_IN`, and a Rust entry point per
operation. It has no wire and no NPC: BANK-NPC-1 calls the entry points. The ledger kind and the
entry's operation reference are CHECK-constrained columns that a later migration widens, as
GOLD-FEE-2 does for `FEE_DEBIT` and the fee record; the house, guild and Market kinds of the
pending amendments are their own children's work, not BANK-1's. A junior character is refused
with `JUNIOR_ACCOUNT` (sender) or `RECIPIENT_CANNOT_RECEIVE_TRANSFERS` (recipient). Until the
starter island and its departure fact exist (D119, DAWNPORT-1), no character is junior, so the
check is a function over that fact that returns "not junior", with a test that it refuses once
the fact says "on the island".

## 2. Packets

### 2.1 MAP-LOAD-1

```yaml
task_id: MAP-LOAD-1
decision: ADR-0021 §4.1, §4.2, §4.8 and the §1.11 amendment; OTERYN_WORLD_BUNDLE_FORMAT_V1 §9; this batch §1.2-§1.4
worker: oteryn-hard-worker
review: security review of the bundle reader (ADR-0021 §4.8)
branch: allocated by the control plane
base: main (MAP-BUNDLE-1 and SPEED-1 merged)
migration_lease: none
depends_on: [MAP-BUNDLE-1, SPEED-1]
owned_paths:
  - crates/world-bundle/**                         # new crate: layout, reader, caps (§1.3)
  - tools/world-bundle-compiler/**                 # moves the reader out; depends on the new crate
  - Cargo.toml                                     # one workspace member
  - Cargo.lock
  - apps/game-server/Cargo.toml                    # the new dependency
  - apps/game-server/src/map/**                    # new: base model, loader, pin check, Terrain catalogue reader (§1.4)
  - apps/game-server/src/movement/speed.rs         # the map-backed GroundSpeedSource
  - apps/game-server/src/world_runtime.rs          # the Arc<WorldBase> handle only; no live wiring
  - apps/game-server/tests/map_load_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # MAP01-BASE-LOAD-MS, -BASE-RSS-BYTES, -VIEWPORT-US: measured value and evidence
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md # the "Runtime reader" line only
  - docs/agents/evidence/MAP-LOAD-1-*.md            # the measurement evidence
validation:
  - cargo test --locked -p oteryn-world-bundle
  - cargo test --locked -p oteryn-world-bundle-compiler
  - cargo test --locked -p oteryn-game-server map_load
  - cargo check --locked --workspace --all-targets
  - cargo run --locked -p oteryn-architecture-check
```

Builds:

- the new crate (§1.3), with the format document's §9 rules: reject the whole bundle on the first
  failure, every MAP01-BUNDLE-* and MAP01-TILE-* cap checked before allocation;
- `WorldBase`: a compact, read-only model of every tile (positions, palette-resolved item compact
  ids, the ground item and its ground speed, the walkable flag), decoded eagerly and shared by
  `Arc` by every channel of the World (ADR-0021 §4.1);
- the load function, which takes the bundle bytes, the revision-matched Terrain catalogue (§1.4)
  and the expected pins (digest, schema versions, content revision, production flag) and returns a
  `WorldBase` or a typed error; a production World
  refuses a bundle whose `build_class` is not `production` (missing counts as `non-production`);
- the map-backed `GroundSpeedSource` (§1.4); production keeps `EngineeringGroundSpeed`.

Not in scope: the overlay, Ground rebuild, MINT of map items and reset (MAP-OVERLAY-1), booting
from the bundle and the CI artifact (MAP-CUTOVER-1), the wire (MAP-WIRE-1/2).

Tests (in the PR, against a small fixture bundle that the test compiles with the compiler, not a
checked-in binary):

- load and tile-by-tile equivalence: compiler input, bundle and `WorldBase` agree on every tile;
- negative: wrong digest, wrong content revision, wrong schema version, non-production bundle in
  a production World, a corrupt sector checksum, an unknown top-level key; each refuses the whole
  bundle;
- each MAP01-BUNDLE-* and MAP01-TILE-* cap at its maximum (accepted) and maximum + 1 (refused),
  among them 64 top-level entries and 4,096 entries per tile;
- the Terrain catalogue (§1.4): a catalogue whose revision or lock digest differs from the
  bundle's is refused; a `terrain` palette key without a record, two records pointing at one Item
  key, and a `ground` record with an UNKNOWN `walkable` or `ground_speed` each refuse the bundle;
  an Item palette key with no Terrain record loads as not ground;
- ground speed 0 non-walkable accepted, 0 walkable refused, 1,000 accepted, 1,001 refused (#1707
  P2 4175486632);
- the map source returns the tile's ground speed for a non-150 tile and 0 for a tile without a
  ground item; `player_step_duration` refuses both 0 cases;
- a fuzz target (or a bounded property test in CI) over the reader that never panics and never
  allocates past the caps;
- the budgets: a measurement over the real bundle on the reference node, run manually and recorded
  as evidence, confirms or revises `MAP01-BASE-LOAD-MS` (5,000 ms), `MAP01-BASE-RSS-BYTES`
  (1 GiB) and `MAP01-VIEWPORT-US` (100 µs p99 for 18x14 over the visible floors). A revision above
  the ADR value stops and asks the architect. `MAP01-CHANNEL-OVERLAY-BYTES` is MAP-OVERLAY-1's.

Acceptance: the tests above pass; the compiler's existing tests pass unchanged; the security
review of the reader is recorded on the PR; the format document's "Runtime reader" line names
the new crate.

### 2.2 ITEM-USE-WIRE-1

```yaml
task_id: ITEM-USE-WIRE-1
decision: ITEM-USE-0 §3 with the BAGS-0 §8 amendment; this batch §0.1
worker: oteryn-impl-worker
review: protocol review
branch: allocated by the control plane
base: main (ITEM-VIEW-1a, ITEM-VIEW-1b, CAP-NEG-1 and CAP-NEG-RESUME-FALLBACK-1 merged)
migration_lease: none
depends_on: [ITEM-VIEW-1a, ITEM-VIEW-1b, CAP-NEG-1, CAP-NEG-RESUME-FALLBACK-1]
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json   # capability 15 ITEM_USE_V1, offered false, requires [4]; the four dispositions
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto  # USE_INTENT fields 4 and 5, field 3 reserved
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json      # ITEMUSE0-RL-03
  - crates/protocol-oteryn/src/lib.rs                 # capability constant and registered set
  - crates/protocol-oteryn/src/world_object.rs
  - crates/protocol-oteryn/tests/** (new codec tests for USE_INTENT, if the crate keeps them out of src)
  - apps/game-server/src/gameplay_transport/capabilities.rs  # 15 in the gated, not the offered set
  - apps/game-server/src/gameplay_transport/connection.rs    # USE dispatch: refuse fields 4 and 5 without capability 15
  - apps/game-server/tests/item_use_wire_*.rs
validation:
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-game-server item_use_wire
  - cargo check --locked --workspace --all-targets
```

Builds the wire only:

- capability 15 `ITEM_USE_V1`, `offered: false`, `requires: [4]`; CAP-NEG-1 selects it only with
  its closure;
- `USE_INTENT` field 5 `ItemByDefinitionV1 {definition_index: uint32}` in the `target` oneof,
  field 4 `use_with` (`creature {actor_id, generation}`) outside it, field 3 still reserved;
- dispositions `REQUIREMENT_NOT_MET`, `EXHAUSTED`, `FULL`, `NO_TARGET`, sent only under
  capability 15;
- the server: without capability 15, a non-corpse use stays `NOTHING_TO_USE` and a command with
  field 4 or 5 is `REJECTED`. The capability is never offered here, so every live session takes
  that path. What a use does under capability 15 is ITEM-USE-1's, which offers it.

Tests:

- codec round trip of fields 2, 4 and 5; field 4 with field 1, with a corpse handle, or alone is
  refused by the decoder or the dispatcher as ITEM-USE-0 §3 says; field 3 present is refused;
- the payload at 529 bytes accepted and 530 refused (`ITEMUSE0-RL-03`), and the result at most
  4 bytes;
- a session without capability 15 that sends field 4 or 5 gets `REJECTED`; a non-corpse field 2
  still gets `NOTHING_TO_USE`; a corpse field 2 still opens the corpse;
- capability 15 is registered, not offered, requires 4, and a selection of 15 without 4 is
  refused by CAP-NEG-1; a resume that would change the selected set takes the
  CAP-NEG-RESUME-FALLBACK-1 path (§1.9 of the item batch).

Acceptance: the tests above; the protocol review on the PR; the PR does not change
`PRODUCTION_OFFERED_CAPABILITIES`.

### 2.3 BANK-RET-0

```yaml
task_id: BANK-RET-0
decision: BANK-0 §5; BANK-FEE-0 §4.3; this batch §1.7
worker: control plane routes (contract-only change)
review: privacy review
branch: allocated by the control plane
base: main
migration_lease: none
depends_on: [BANK-0]
owned_paths:
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json   # ECONOMY_LEDGER_RETENTION_V1; successor DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V2 for event type 2 (§1.7); V1 unchanged
  - tools/agents/tests/** (only if a registry test must name the new profile)
validation:
  - python3 tools/agents/validate_governance.py
  - python3 tools/repository/validate_repository_policy.py
  - the registry's own tests (tools/agents/tests)
```

Builds the two profiles of §1.7 with every `required_profile_fields` entry, and records the
activation boundary of §1.7 (GOLD-FEE-2's merge and deploy). It does not change event type 2's
binding, which stays V1 until GOLD-FEE-2. Acceptance: the privacy
review on the PR; the registry validates; V1 and every admitted event are unchanged (a test or
validator check that V1's fields are byte-identical to `main`); no event type is added (BANK-1
adds type 3).

### 2.4 BANK-1

```yaml
task_id: BANK-1
decision: BANK-0 §3-§5 and §8 (Q1 = b); this batch §1.7, §1.8
worker: oteryn-hard-worker
review: persistence review
branch: allocated by the control plane
base: main (GOLD-FEE-1b and BANK-RET-0 merged)
migration_lease: 0071 (merge condition of §0.1)
depends_on: [GOLD-FEE-1b, BANK-RET-0]
owned_paths:
  - apps/game-server/migrations/0071_account_bank.sql
  - apps/game-server/src/durability/bank.rs            # writer: deposit, withdraw, transfer
  - apps/game-server/src/durability/bank_audit.rs      # event type 3, value lines
  - apps/game-server/src/durability/mod.rs             # module lines only
  - apps/game-server/tests/bank_*.rs
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json # event type 3 BANK_OPERATION
  - docs/contracts/game-events/v2/bank_operation.proto # the bank event and the closed value-line message
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # the BANK-0 §8 rows and the measured DUR03-RL-07 envelope of the bank event
validation:
  - cargo test --locked -p oteryn-game-server bank_
  - cargo test --locked -p oteryn-game-server item_fee_burn
  - cargo check --locked --workspace --all-targets
```

Builds BANK-0 §3-§5 within §1.8: the balance, operation, ledger and coin-line tables with their
deferred guards and grants; the branches of the `0010`, `0011`, `0012` and `0023` proofs that accept
a coin line of a committed bank operation; the writer with the lock order of BANK-0 §4.1, replay
by occurrence and binding, and the typed refusals of BANK-0 §4.3 (junior included); the bank event
and its outbox. It registers the §8 rows before the code that relies on them.

Tests:

- each operation commits one transaction with the entries and lines its kind needs; the balance
  equals its latest entry; a transfer's two entries commit together;
- limits at max and max + 1: balance 999,999,999,999 accepted and one more refused before any write
  (typed, never a CHECK abort); deposit 20,000,000 and 20 input stacks, withdraw 1,009,999 and 3
  output stacks, transfer 999,999,999,999; each + 1 refused;
- replay: the same occurrence and binding returns the first outcome; a changed binding conflicts;
- crash: an abort at each step leaves no partial row; an ambiguous commit resolves by replay;
- guards: a hand-written row that breaks each deferred guard is refused at commit (as the `0023`
  tests do), and the runtime role cannot DELETE;
- junior: a junior sender is `JUNIOR_ACCOUNT`, a junior recipient
  `RECIPIENT_CANNOT_RECEIVE_TRANSFERS` (§1.8);
- the existing gold fee tests pass unchanged, with the rewritten guard functions.

Acceptance: the tests above; the persistence review on the PR; the migration merge condition.

### 2.5 GOLD-FEE-2

```yaml
task_id: GOLD-FEE-2
decision: BANK-FEE-0 §3-§5; BANK-0 Q1 = b; this batch §1.7
worker: oteryn-hard-worker
review: persistence review
branch: allocated by the control plane
base: main (BANK-1 merged)
migration_lease: 0072 (merge condition of §0.1)
depends_on: [BANK-1, BANK-RET-0, GOLD-FEE-1b]
owned_paths:
  - apps/game-server/migrations/0072_character_gold_fee_bank_debit.sql  # 0023 and 0010 widening; FEE_DEBIT kind and fee reference on the ledger
  - apps/game-server/src/durability/item_fee_burn.rs
  - apps/game-server/src/durability/item_fee_burn_audit.rs
  - apps/game-server/src/durability/item_mint_audit.rs  # RETENTION_PROFILE_ID to V2; verifier admits V1 or V2; the fee bank-debit codec
  - apps/game-server/examples/dur03_native_one_item_audit.rs  # the same constant and codec
  - apps/game-server/src/durability/bank.rs        # the FEE_DEBIT entry writer only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto  # OneItemFeeBankDebitV1, OneItemFeeBurnV1 field 13; header text
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json  # event type 2: retention_profile_id V2, current_schema_revision 2 (§1.7)
  - apps/game-server/tests/item_fee_burn_*.rs
  - apps/game-server/tests/gold_fee_bank_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # DUR03-RL-03-FEE; the re-measured DUR03-RL-07 envelope with the value line
validation:
  - cargo test --locked -p oteryn-game-server item_fee_burn
  - cargo test --locked -p oteryn-game-server gold_fee_bank
  - cargo test --locked -p oteryn-game-server bank_
  - cargo test --locked -p oteryn-game-server item_mint_audit
  - cargo check --locked --workspace --all-targets
  - python3 tools/agents/validate_governance.py
```

Builds BANK-FEE-0 §3 and §4 for the one fee source on `main`, `CharmUnassign`: coins first; when
`T < F`, every eligible input burned whole and `F - T` debited from the payer's balance by one
`FEE_DEBIT` entry in the same transaction; a junior payer keeps stage 1. The `0023` record gains
`bank_debit_gold_units`, its conservation CHECK and the coins-first guard; the `0010` outbox item
may be NULL only for a fee wholly paid from the bank; the Rust audit accepts the value line and
drops the 20,000,000 cap. NPC BUY and travel take the bank part in NPC-TRADE-1 and NPC-TRAVEL-1,
whichever lands later (BANK-FEE-0 §5).

The payload contract (#1733 P1 4176877701). The registered proto excludes value lines, so
GOLD-FEE-2 changes it in the same PR:

- a new message `OneItemFeeBankDebitV1`, BANK-0 §5's closed value line for this one shape:
  - the 16-byte ledger entry id;
  - asset `gold`;
  - the payer's historical `AccountId`, 16 bytes (#1733 P1 4176934049), which is the Account
    whose balance is debited, as resolved under the fee transaction's lock. It is never re-read
    later, so the event proves the debited Account even after the character moves;
  - the `WorldId`, equal to the fee record's World;
  - kind `FEE_DEBIT` and class `BURN`;
  - `debit_gold_units` (1..999,999,999,999);
  - `balance_before_gold_units` and `balance_after_gold_units` (after = before - debit).
  The `AccountId` and `WorldId` equal the `FEE_DEBIT` ledger entry's own, which a commit guard
  checks;
- `OneItemFeeBurnV1` gains `optional OneItemFeeBankDebitV1 bank_debit = 13`, present exactly when
  `bank_debit_gold_units > 0`;
- the header's exclusion text names this one admitted value line; every other value line stays
  excluded;
- `fee_gold_units` widens from the 20,000,000 cap to `T` plus 999,999,999,999, and
  `burned - change + bank_debit = fee` holds;
- the registry's `current_schema_revision` for event type 2 becomes 2. `interpretation_revision`
  stays 1, since revision-1 bytes mean the same thing.

Compatibility and codec qualification:

- every revision-1 event of every type-2 shape decodes and verifies unchanged under the new codec
  (golden bytes from `main`);
- a fee event with field 13 round-trips canonically;
- field 13 on a non-fee operation, a bank debit with `bank_debit_gold_units = 0`, and a balance that
  does not satisfy after = before - debit are each rejected;
- a bank debit with a missing, zero or non-16-byte `AccountId` is rejected by the codec. So is
  an `AccountId` or `WorldId` that differs from the `FEE_DEBIT` entry's. The canonical encoding of
  the `AccountId` round-trips byte for byte;
- the in-repo codec and the example are the only readers, and both are upgraded in this PR.

Tests:

- `T >= F` unchanged (the existing tests pass as they are); `T < F` with enough balance burns every
  eligible input whole and debits `F - T`; with too little balance it is `InsufficientFunds` and
  nothing is written; `T = 0` with no backpack pays wholly from the bank;
- the conservation CHECK and the coins-first guard refuse a hand-written record with change and a
  bank part, or with an untouched eligible input;
- the same-transaction guard both ways: a fee with a bank part and no `FEE_DEBIT`, or a `FEE_DEBIT`
  with no fee or a different amount, is refused at commit;
- limits: a fee equal to `T` plus 999,999,999,999 accepted, one more refused; `DUR03-RL-03-FEE` 1
  with a bank part, 0 without; the event at the re-measured envelope maximum accepted and + 1
  refused;
- replay and crash: the bank part is an outcome, recalculated after a known abort and returned by
  the occurrence replay after an ambiguous commit;
- a junior payer with `T < F` is refused as in stage 1;
- the activation boundary (§1.7): the registry's type-2 binding equals the shared constant, both
  V2; every type-2 producer (mint, transfer, reward claim, decay retire, timed expiry, fee burn)
  writes V2. A stored `(1, V1)` row of each shape still passes the CHECK after `0072` and
  verifies as V1. A new `(2, V2)` row passes and verifies as V2. `(1, V2)`, `(2, V1)`, revision 3
  and a profile id that is neither V1 nor V2 are each refused by the CHECK and the verifier.

Acceptance: the tests above; the persistence review on the PR; the migration merge condition.

## 3. What this unblocks

| Work | Was blocked by | After this batch |
|---|---|---|
| MAP-OVERLAY-1, MAP-CUTOVER-1, MAP-WIRE-1/2, DEPOT-WIRE-1, DEPOT-CONTENT-1 | no map reader | packetable after MAP-LOAD-1 (next wave) |
| ITEM-USE-1, FOOD-REGEN-1, IMBUE-WIRE-1, IMBUE-CONTENT-1, FORGE-CONTENT-1 | no `ITEM_USE_V1` wire | packetable after ITEM-USE-WIRE-1 (ITEM-USE-1 in the next wave) |
| IMBUE-1, FORGE-1, CHARM-6, NPC-TRADE-1, NPC-TRAVEL-1 bank part | GOLD-FEE-2 not packeted | packeted (§2.5) |
| BANK-NPC-1, STASH-1, HOUSE-OWN-1, MAIL-1 economy retention | no bank tables, no economy profile | after BANK-1 and BANK-RET-0 |
| NPC-VIS-1, NPC-ACTOR-1 | NPC-BEHAVIOUR-0 not accepted | accepted when this batch merges (§1.5) |

## 4. Rejected options

- **Packet the next wave now.** It would double this batch's review and needs MAP-LOAD-1's
  `WorldBase` shape for MAP-OVERLAY-1; it comes in the next batch.
- **Keep the reader in the compiler crate.** A server depending on a tool crate, or a second
  reader, is rejected (§1.3).
- **One migration for BANK-1 and GOLD-FEE-2.** The persistence reviews are separate and BANK-1 is
  useful on its own (BANK-NPC-1, STASH-1).
- **Reuse the DUR-03 retention profile for the bank event.** Its purpose excludes the economy
  (§1.7).

## 5. Decision test

- **Must decide now:** YES. The three roots block the most packeted work (§3), and the owner asked
  for the bank chain in this batch.
- **Minimum sufficient:** five packets; each builds only what its decision says, the wire packet
  offers nothing, and the map packet wires nothing into the live path.
- **Superseding evidence:** a MAP-LOAD-1 measurement over the ADR-0021 budgets; a privacy review
  that refuses §1.7; a different capability or migration number from the control plane.
- **Deliberately not decided:** MAP-OVERLAY-1, MAP-CUTOVER-1 (including where the digest pin
  lives), ITEM-USE-1, NPC-ACTOR-1, BANK-NPC-1, the house, guild and Market ledger kinds.
