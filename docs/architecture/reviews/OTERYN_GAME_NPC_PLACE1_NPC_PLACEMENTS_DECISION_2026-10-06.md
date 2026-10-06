# NPC-PLACE-1 NPC placements: World Project family and World Bundle frame

- Decision: `NPC-PLACE-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES**, after exact-head validation, the independent
  review on the frozen head and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: #1622 6021673251 (NPC-PLACE-1 has never been packeted; the bundle has no NPC shape) and
  the placement part of 6021806127 (Doctor Marrow `POSITION_CONFLICT`).
- Governing: NPC-0 §3.2; NPC-BEHAVIOUR-0 §3.1; ARCH-NPC-PACKETS-1 §1.4; ADR-0021 §4.2, §4.3,
  §4.6; `OTERYN_WORLD_BUNDLE_FORMAT_V1` §3, §8, §9, §11, §13;
  `OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1` §5.
- Amends in this PR, as pending-on-acceptance notes: NPC-0 §3.2 and the NPC-PLACE-1 and
  NPC-TRAVEL-1 brief rows; NPC-BEHAVIOUR-0 §3.1 (actor order wording); ARCH-NPC-PACKETS-1 §1.4 and
  §3; NPC admission §5.
- Amends in NPC-PLACE-1b, with the code: the bundle format (§3.5 lists every edit) and
  `content/world/pins/README.md`.
- Runtime, migration, production and protected-World authority: NONE. Each packet needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Implementation brief

1. **Shape.** NPC placements get their own World Project family, `Npc.Placement`, and their own
   bundle frame, the NPC frame. `spawn::Creature` and the spawn frame are not touched (§2).
2. **Format.** A new frame and a manifest member are a layout change: the bundle becomes
   `OTERYN_WORLD_BUNDLE/v4`, `min_reader_version` 4, and the reader accepts only v4 (§3).
3. **Payload.** The NPC table sorted by NPC key; per NPC, its placements sorted by (native floor,
   `y`, `x`), with a direction. No respawn, no period, no radius (§3.2).
4. **Source.** `content/world/npc-placements/` (index plus shards, frame
   `global-target-2026-09-27`), generated with provenance from the promotion candidates and, for
   an admitted NPC without a candidate row, from the pinned spawn XML its definition names (§4).
5. **Source holds.** An admitted NPC or placement that gets no record is listed in the family's
   held list with a closed reason: `NO_PLACEMENT_SOURCE`, `POSITION_CONFLICT`, `SCHEDULE_VARIANT`
   or `SHARED_CELL`. Exact duplicates collapse into one record (§4.3).
6. **Realization.** A position outside the World fails compilation. Otherwise the compiler writes
   the placement or holds it with a closed reason. A production build stops on any held placement;
   a non-production build lists them in the manifest (§5).
7. **Doctor Marrow.** Held `POSITION_CONFLICT` once the content lane admits its NPC definition;
   only a map-owner record resolves it (§4.3).
8. **Travel.** Destinations are already in the project frame. The compiler fails on one outside
   the World and holds one on a cell that cannot take a character; a production build stops on
   any held destination. Routes are not bundle bytes (§6).
9. **CI.** The compiler now reads `content/npcs/definitions/` and `content/services/travel/`:
   both are added to the compiler input paths and to the classifier's world-bundle prefixes (§7).

Packets:
- **NPC-PLACE-1a** (content lane): the family, the generator, the held list and the pin refresh
  that adding a `content/world/` input requires (§4.4).
- **NPC-PLACE-1b** (impl worker, format and CI-routing review): v4, the compiler lowering, the
  reader, the limits, the manifest and parity output, the input routing and the re-pin. It may
  author against a fixture shard; it merges `main` after 1a merges and re-pins then. The two
  serialize on the pin.
- Then NPC-ACTOR-1 consumes the frame. NPC-TALK-1 waits for NPC-PLACE-1b and MAP-CUTOVER-1b
  (ARCH-NPC-PACKETS-1 §1.4, unchanged order).

## 1. Facts

**PROVEN** (`main` at 97c46d0e)
- `crates/world-bundle/src/spawn.rs`: `Creature {key, period}`, `Point {creature, x, y,
  direction, respawn_ms}`, `Source {key, floor, x, y, points}`. The palette family is `item` or
  `terrain` only (format §3). The bundle is v3 (`bundle.rs` `VERSION = 3`).
- Format §3: the reader rejects unknown manifest fields at every level. Format §11: a change of
  the layout, the manifest fields or the payload grammar is a new version.
- `content/world/pins/oteryn.json` pins one non-production v3 bundle digest and an
  `inputs_digest` over the compiler input paths. The `world_bundle` job of `game-gate` runs
  `pin-check`, which refuses a pin whose either digest is stale.
- The compiler input paths (`tools/world-bundle-compiler/src/main.rs` `INPUT_PATHS`) and the
  classifier's `WORLD_BUNDLE_INPUT_PREFIXES` (`tools/repository/classify_pr_test_lanes.py`) are
  equal, kept so by a test. Neither has `content/npcs/` or `content/services/`.
- The World record `oteryn:world.oteryn` (`content/world/worlds/worlds-00000-00000.json`) declares
  bounds 1340..34264 × 1643..33813 and floors 0-15; the compiler reads it. `ProjectV2Placement`
  (`apps/game-server/src/content/project/v2.rs`) has no direction, and its only disposition is
  `CandidateOnly`. NPC admission §5 keeps NPC placements in the promotion candidates.
- `Spawn.Source` (`content/world/spawns/`) is in the frame `global-target-2026-09-27` with
  `floor` = legacy `z`; the compiler maps `native.floor = -z` (`project.rs`, `compile.rs`). Its
  index carries `held_groups` as `{group_count, path, point_count, sha256}` of a held-list file.
- `tools/content-schema/npc-authoring/convert.py` reads Canary and Crystal NPC spawn XML (centre
  plus offset, direction, `spawntime`, radius, source path); `promotion_candidates.py` merges
  them and arbitrates by the wiki (D6, D8). Its hold reasons hold the whole NPC, which is then not
  admitted, so they never apply to an admitted NPC's placement.
- The World Project admits 1,282 `oteryn:npc.*` keys. 1,112 have a promotion candidate row, with
  1,173 placements, at most 6 for one NPC, 105 of wiki origin with no direction. 10 rows are wiki
  confirmed with no placement. 170 admitted NPCs have no row: 111 from Crystal summer (00ce02a5)
  and 59 bulk records marked `deferred_gameplay: native placement`.
- The candidate placements share 13 cells: 4 are one NPC listed twice at one cell and direction;
  7 are Day and Night variants of one NPC (Dal the Huntress, Fral, Jehan, Kesar twice, Onfroi,
  Wes); 2 are distinct NPCs (Anaztassja's two forms; Awareness of the Emperor and its misspelt
  twin).
- The Crystal summer tree is pinned by `imports/crystalserver/summer-update/source-tree.json`
  (00ce02a5), which lists `data-crystal/world/world-npc.xml` as blob 77d06a85.
- Travel routes (`content/services/travel/`) store `destination.coordinate_frame`
  `global-target-2026-09-27` with `floor` = legacy `z`, the project frame (`project.rs`), and
  ADR-0021 §4.3 lists NPC destinations in the project frame. The frame map is the identity.
- `serve.rs` logs `spawned_actors=0`: no NPC actor exists. MAP-CUTOVER-1a is merged; 1b, which
  serves the bundle World, is not. NPC-CONTENT-1 is merged.
- `NPCBEH0-RL-01`: at most 2,048 NPC actors per channel; every placement is an actor on every
  channel, in NPC key, then position order (NPC-BEHAVIOUR-0 §3.1). It has no
  `RESOURCE_LIMITS_REGISTRY.json` row yet. NPCs block movement like creatures.
- Doctor Marrow: pinned Crystal summer `world-npc.xml` places him at (34010, 32641, 6), the
  post-release Make Believe Reference at (33992, 32662, 6) (#1622 6021806127). Only
  `oteryn:creature.doctor_marrow` exists on `main`; the NPC key is to be admitted by a content
  lane.

**DERIVED**
- The map in the Make Believe area is the pinned pre-release import. A post-release coordinate
  there is not known to be the cell of the same layout.

**UNKNOWN**
- The placement count of the summer and bulk NPCs. 1a measures it against the limits (§3.3).
- How many placements realize: 1b reports it.

## 2. Shape (the escalation's choice)

Ruling: **a separate NPC frame** with an NPC-keyed table.

- An NPC is NPC-owned content (NPC-0, NPC-BEHAVIOUR-0): no respawn, no period, no creature
  definition. A spawn point's fields do not fit it, and a `Creature` row naming an NPC key would
  make the creature reader and CREATURE-AI-0 resolve a key they do not own.
- A separate frame leaves the spawn grammar, its limits rows and its tests unchanged. A typed
  actor table inside the spawn frame would rewrite the spawn grammar for one consumer.
- No palette family is added: an NPC is not a tile entry and has no placement key (format §7).

## 3. Format v4

### 3.1 Layout

- Header `format_version` 4, manifest `format` `"OTERYN_WORLD_BUNDLE/v4"`, `min_reader_version`
  4; the digest domain string names v4.
- A 44-byte NPC row follows the spawn row, in the spawn row's shape: `offset`,
  `compressed_length` (non-zero), `raw_length`, SHA-256 of the frame.
- The NPC frame follows the spawn frame and ends at the digest. It is one canonical zstd frame
  (format §5). A bundle without NPCs still has the row and a frame of the empty table.
- The manifest gains the required member `npcs`: `{npcs, placements, held}`. `npcs` and
  `placements` are the counts of the NPC table and its placements; the frame must hold exactly
  these. `held` is the record keys of the placements the compiler held (§5), sorted and unique,
  like `draft_areas`; it is empty in a production bundle.
- The reader accepts only v4. A v3 bundle is refused like any unknown version. No bundle has been
  published; the one testing pin is rebuilt in NPC-PLACE-1b.

### 3.2 Payload

Varints are LEB128 and canonical (format §5).

- `npc_count`, then per NPC, strictly ascending by key: `key` (varint length and ASCII
  `0x21..=0x7E` bytes, at most 128, starting `oteryn:npc.`) and `placement_count` (at least 1).
- Per placement, strictly ascending by (`floor`, `y`, `x`): `floor` i8 (native, one of the
  manifest's `world.floors`), `y` and `x` (varints) and `direction` u8 (`0` north, `1` east, `2`
  south, `3` west).
- Every placement is inside the World extent. No two placements of the bundle share a cell; the
  reader keeps the set of cells and rejects a repeat across NPCs.
- Counts are checked against the bytes left before anything is reserved; nothing may follow the
  payload. The writer and the reader apply the same validation.
- The order is the canonical actor order of NPC-BEHAVIOUR-0 §3.1, so NPC-ACTOR-1 allocates actor
  ids in frame order.

### 3.3 Limits

| Limit | Hard maximum |
|---|---|
| `NPCPLACE1-RL-01` | 2,048 placements per bundle (= `NPCBEH0-RL-01`); the NPC count is bounded by it |
| `NPCPLACE1-RL-02` | 16 placements per NPC |
| `NPCPLACE1-RL-03` | 1 MiB raw NPC payload, at most 1,024 times its frame; it counts toward the running raw total (format §9) |

- 1b adds the three rows and the `NPCBEH0-RL-01` row to `RESOURCE_LIMITS_REGISTRY.json` and tests
  each at its maximum and at the maximum plus one.
- If 1a measures more than 16 placements for one NPC or more than 2,048 in all, it returns
  `QUESTION`: a larger bound is a decision, not a worker's choice.

### 3.4 Reader order

After the spawn row and frame: the NPC row, the raw and ratio limits, the running raw total, the
frame checksum, the single canonical frame, decompression into exactly `raw_length` bytes, the
payload grammar with the limits of §3.3, the manifest `npcs` counts, and `held` empty when
`build_class` is `production`. Then `dropped_teleports`, as in v3.

### 3.5 Format text that NPC-PLACE-1b amends

| Place | Edit |
|---|---|
| Title, Format ID | v4; v3 joins the retired list |
| §2 table | `format_version` `4`; an NPC row of 44 bytes after the spawn row; frames "then the spawn frame, then the NPC frame, the NPC frame last"; the contiguity sentence names the NPC row |
| §3 table | `format` v4, `min_reader_version` `4`; the `npcs` row of §3.1 |
| §6 | the domain string v4; the digest covers the NPC frame |
| §8 | production: `npcs.held` empty; the writer refuses and the reader rejects it otherwise |
| §9 | the NPC row and frame after the spawn frame, before `dropped_teleports` (§3.4) |
| §13 | "ends at the digest" becomes "is followed by the NPC frame"; v3 is retired |
| new §14 | Format v4: §3.1-§3.4 and §5 of this decision |
| `content/world/pins/README.md` | the pinned digest is a v4 digest |

## 4. Source family (NPC-PLACE-1a)

### 4.1 Records

- Family `Npc.Placement` under `content/world/npc-placements/`: an index (`OTERYN_FAMILY_INDEX/v1`)
  and shards, frame `global-target-2026-09-27`, like `Spawn.Source`.
- A record: `declaration.identity.key` `oteryn:npc_placement.<npc local name>.x<x>_y<y>_z<z>`,
  `npc` (the `oteryn:npc.*` key), `cell` (`x`, `y`, `floor` = legacy `z`), `direction`, and
  `provenance`.
- `provenance`: one row per source that gave this cell: `origin` (`canary`, `crystal` or
  `wiki`), the source repository, revision, path and blob, and the arbitration row of
  `promotion_candidates.py` when there is one.
- A wiki-origin placement has no direction in the source. It is written as north, the engine
  default for a missing attribute, and its provenance says so.
- Not carried: `spawntime` and the spawn radius. NPCs are recreated only at a channel start or a
  reset (NPC-BEHAVIOUR-0 §3.1); the walk radius is the definition's `wander` (NPC-CONTENT-2).

### 4.2 Generator

- A generator under `tools/content-schema/npc-authoring/` writes the family deterministically,
  sorted by record key, for the NPC keys the World Project admits, and for no other key.
- Inputs, per admitted NPC:
  - its promotion candidate placements, when it has a row;
  - otherwise the pinned NPC spawn XML that its definition's provenance names, read by
    `convert.py`: the Crystal summer `world-npc.xml` at 00ce02a5, checked against blob 77d06a85,
    or the pinned Canary file of a bulk record. An NPC whose provenance names no pinned spawn
    file is listed `NO_PLACEMENT_SOURCE`.
- The position-conflict list (§4.3) is a reviewed input file beside the generator.

### 4.3 Source-level holds

- A held placement has no record. The family's held list (`held.json` in the family directory;
  the index carries `{count, path, sha256}`, as `Spawn.Source` does for `held_groups`) names, per
  entry: NPC key, reason and evidence positions with their sources.
- Reasons, closed:
  - `NO_PLACEMENT_SOURCE`: an admitted NPC with no pinned position (the 10 wiki-confirmed rows,
    and any NPC of §4.2 without a spawn file).
  - `POSITION_CONFLICT`: an accepted Reference places the NPC elsewhere than its pinned source.
    The whole NPC is held.
  - `SCHEDULE_VARIANT`: Day and Night variants of one NPC on one cell. Every placement on that
    cell is held until NPC schedules are decided.
  - `SHARED_CELL`: distinct NPCs on one cell. Every placement on that cell is held.
- One NPC listed twice at one cell and direction is one record with both provenance rows, not a
  hold. The same NPC twice at one cell with two directions is `SHARED_CELL`.
- `POSITION_CONFLICT` is resolved only by a map-owner record that names the chosen cell and shows
  that the World Project map at that cell is the matching layout. A newer coordinate never
  replaces the pinned one by itself.
- Doctor Marrow: once the content lane admits `oteryn:npc.doctor_marrow` (6021806127), it is held
  `POSITION_CONFLICT` with both positions as evidence. `oteryn:creature.doctor_marrow` is
  untouched.
- A source-level hold is not a bundle-level hold: the NPC or placement is absent from the World.
  The parity report lists the held list (§5); whether it blocks a release is the release
  checklist's, not the bundle reader's.

### 4.4 Pin

`content/world/npc-placements/` is under the `content/world/` input prefix, so 1a changes
`inputs_digest`. 1a refreshes the pin and, if its content-lock change moves the content revision,
the identity file and the digest, by the pins README procedure (`derive-identity`, then
`pin-check`).

## 5. Realization (NPC-PLACE-1b)

The compiler reads the family, the NPC definitions (`content/npcs/definitions/`) and the World.

- A placement outside the World extent or floors **fails compilation** in every build class
  (ADR-0021 §4.3, NPC-0 §3.2).
- Otherwise the compiler writes a placement when its NPC is admitted and its cell can hold it.
  It holds it with the first reason in this order:
  1. `UnboundNpc`: no admitted NPC definition has the key.
  2. The cell, from the compile-time facts of spawn realization (format §13, item 4): `NoTile`,
     `UnclassifiedTerrain`, `NoGround`, `NotWalkable`, `FloorChange`, `Teleport`.
     `ProtectionZone` is not a reason: NPCs stand in protection zones.
  3. `SpawnPoint`: a realized creature spawn point is on the cell. NPCs block like creatures, so
     the spawn could never place there.
  4. `SharedCell`: two placements that pass items 1-3 on one cell. Every placement on that cell
     is held. After §4.3 this only catches drift.
- A **production** build stops on any held placement (NPC-0 §3.2, the release gate).
- A **non-production** build leaves held placements out, lists their record keys in the manifest
  `npcs.held`, and reports each with its reason in the `parity` and `compile` output
  (`npcs.held_by_reason`).
- The `compile` command's equivalence proof derives the NPC table and `held` from the same inputs
  and compares them with the bundle.
- The `parity` output also reports the family's held list by reason (`npcs.source_held`).

## 6. Travel destinations

- The frame map is the identity (§1). No route is written to the bundle.
- The compiler reads `content/services/travel/` and classifies each route destination:
  - outside the World extent or floors: compilation fails;
  - otherwise the cell reasons of §5 item 2 and `SpawnPoint`, reported as `npcs.routes_held`.
- A production build stops on any held destination (NPC-0 §3.2).
- In a non-production World, NPC-TRAVEL-1 refuses a route whose destination the loaded World's
  collision index does not admit, with the same reasons, so compiler and runtime agree on equal
  inputs. NPC-0's NPC-TRAVEL-1 brief row is amended to say so.

## 7. Runtime and CI boundary

- NPC-PLACE-1b changes the shared reader (`crates/world-bundle`): a new `npc` module and the v4
  constants. The game server's loader keeps working through the reader. It reads the NPC table
  and does nothing with it until NPC-ACTOR-1.
- NPC-ACTOR-1 creates one actor per placement in frame order. It refuses, in a production World,
  a bundle whose `identity.content_revision` differs from the content revision of the NPC
  catalogue loaded at boot; in a non-production World it logs that and creates no NPC actor. A
  placement whose NPC the loaded catalogue holds gets no actor and is logged.
- No wire change. NPCs use `EntityKind::Npc` (NPC-BEHAVIOUR-0 R1).
- 1b adds `content/npcs/definitions/` and `content/services/travel/` to the compiler's
  `INPUT_PATHS` and to `WORLD_BUNDLE_INPUT_PREFIXES`, so an NPC or travel PR runs the
  `world_bundle` lane and re-pins. `tools/repository/classify_pr_test_lanes.py` is in 1b's owned
  paths, with CI-routing review.

## 8. The archived reference packet

`docs/reference/npc-completion-20261004` is CandidateOnly, based on 79b79ae, and is not applied.
NPC-PLACE-1b re-states its three placement cases as compiler tests: a placement of an exact
admitted NPC key is written; a placement naming another or no NPC is held `UnboundNpc`; two
placements on one cell are held `SharedCell`.

## 9. Rejected

- **NPC rows in the spawn creature table.** It overloads `Creature` with a key it does not own
  and gives NPCs respawn and period fields (the escalation's own rule).
- **A typed actor table replacing the spawn frame.** Rewrites the spawn grammar, its limits and
  tests for no creature need.
- **NPC placements as palette entries in sector payloads.** An NPC is an actor, not a tile entry;
  it would get a placement key and enter the item paths.
- **Keeping v3 and amending it in place.** MAP-KIND-CLASS-0 did that for a closed-set value; a
  new frame and a manifest member are a layout change (format §11).
- **Holding a placement outside the World.** ADR-0021 §4.3 and NPC-0 §3.2 fail compilation; the
  one World record covers the whole map, so there is no partial testing World to serve.
- **Keeping one Day or Night variant on a shared cell.** It picks a schedule before schedules
  are decided.
- **Placing Doctor Marrow at either coordinate now.** Two sources disagree, and the Make Believe
  release changed that area after the pinned map. The cell check cannot tell an old walkable
  cell from the right one. A wiki-origin position is not exempt from the cell checks; it is
  written because D6 and D8 accepted the wiki as the arbiter, and no accepted source contradicts
  it.
- **Routes in the bundle.** The runtime can check them against the bundle it already holds.

## 10. Decision test

- **Must decide now:** YES. NPC-ACTOR-1 and NPC-TALK-1 are blocked on NPC-PLACE-1, and 1b's
  format change re-pins the testing bundle. The ARCH-NPC-PACKETS-1 §1.4 precondition is met: the
  map track's bundle World exists as the testing pin and MAP-CUTOVER-1a, and NPC-CONTENT-1 has
  merged.
- **Blocked:** NPC-PLACE-1a and 1b, then NPC-ACTOR-1, NPC-TALK-1 and NPC-QUEST-1.
- **Harder later:** the format version. A bump now costs one re-pin; after the first published
  production bundle it costs a planned World reset (ADR-0021 §4.7).
- **Superseding evidence:** an NPC with more than 16 placements, or more than 2,048 in all; NPC
  schedules (day and night positions, NPC-BEHAVIOUR-0 later work), which need a placement time
  field and a new version; a Reference showing two NPCs on one cell.
- **Deliberately not decided:** NPC schedules, roaming NPCs without a placement, per-channel NPC
  sets, and the release checklist for source-level holds.

## 11. Amendments made in this PR

- **NPC-0 §3.2**, after its last bullet: "Amendment (2026-10-06; NPC-PLACE-1). Placements are the
  `Npc.Placement` family, compiled into the v4 NPC frame. A held position is one of NPC-PLACE-1
  §5 (`UnboundNpc`, the cell reasons, `SpawnPoint`, `SharedCell`); a production build stops on any
  held position or destination. Source-level holds (NPC-PLACE-1 §4.3) are not compiler holds."
- **NPC-0 brief, NPC-PLACE-1 row:** packeted by NPC-PLACE-1 as 1a and 1b.
- **NPC-0 brief, NPC-TRAVEL-1 row:** adds "refuses a route whose destination the loaded World does
  not admit (NPC-PLACE-1 §6)".
- **NPC-BEHAVIOUR-0 §3.1:** "canonical order of the placement key (NPC key, then position)"
  becomes "canonical actor order (NPC key, then native floor, `y`, `x`; NPC-PLACE-1 §3.2)", so it
  is not confused with the format §7 placement key.
- **ARCH-NPC-PACKETS-1 §1.4** NPC-PLACE-1 row and **§3** rejection: "packeted by NPC-PLACE-1
  (2026-10-06); the precondition is met".
- **NPC admission §5:** "Amendment (2026-10-06; NPC-PLACE-1). NPC placements enter as the
  `Npc.Placement` family under the World record `oteryn:world.oteryn`, not as
  `ProjectV2Placement`."

## 12. Before-freeze checklist

1. **Amendments:** §11 in this PR, as pending-on-acceptance notes; the format and pins README text of §3.5 in 1b.
2. **Serialization:** the NPC frame and the manifest member `npcs` (§3); deterministic, written
   and read by one validation; the order is fixed in §3.2.
3. **Restart:** nothing durable. NPCs are recreated from the frame at a channel start or reset.
4. **Typed references:** the NPC key (`oteryn:npc.*`), the placement record key, the cell in the
   project frame and natively, the bundle digest and `content_revision`.
5. **Wire:** none.
6. **Atomic commit:** none at runtime. The bundle digest covers the frame. 1a refreshes the pin
   for its input; 1b changes the format, the routing and the pin together.
