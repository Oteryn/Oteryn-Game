# MAP-LOAD-PACKET-1: the World Bundle loader packet

```yaml
decision_id: MAP-LOAD-PACKET-1
status: CANDIDATE
date: 2026-10-04
owner: Sol Supervising Architect
requested_by: control plane D491 (owner answer 5a, 2026-10-04): MAP-LOAD-1 moves out of ARCH-BATCH-ROOT-PACKETS-V1 (#1733) into its own decision
writes_on_other_prs: none
```

This decision packets MAP-BUNDLE-2 and MAP-LOAD-1, the root of the map chain (MAP-OVERLAY-1, MAP-WIRE-1,
DEPOT-WIRE-1, NPC-ACTOR-1 and the ground speed source of SPEED-1). It was §1.2-§1.4 and §2.1 of
`OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md` (#1733). Control plane D491 moved it here
with two open findings of #1733:

- P1 4177026515: bind the Terrain catalogue bytes to the bundle pin.
- P2 4177026518: bound the catalogue before parsing.

Codex then found (#1744 P1 4177063031) that a server-read catalogue breaks ADR-0021 D189 (§4.2:
the server reads only the compiled bundle). The owner chose 1a on 2026-10-04: the compiler
writes the terrain semantics into the bundle, as bundle format v2. §1.4 records that ruling.
It answers all three findings, because the server reads no catalogue at all.

It changes no code, no contract and no wire. Its rulings are architecture rulings under ADR-0021
and `OTERYN_WORLD_BUNDLE_FORMAT_V1`. Live PR and Issue state governs. When this was written:

- on `main`: MAP-BUNDLE-1 (the format document, the compiler and its reader in
  `tools/world-bundle-compiler`) and SPEED-1 (#1715, capability 13 and the `GroundSpeedSource`
  seam);
- not built: any runtime map reader.

## 0. Leases and shared files

Neither packet needs a migration, capability, command, event or profile.

| File | Packets | Rule |
|---|---|---|
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | MAP-LOAD-1; ITEM-USE-WIRE-1, BANK-1 and GOLD-FEE-2 (#1733) | each edits only its own rows; the second to merge takes `main` in with a merge commit and keeps both |
| `Cargo.toml`, `Cargo.lock` | MAP-LOAD-1 | one new workspace member (§1.2) |
| `tools/world-bundle-compiler/**`, `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md` | MAP-BUNDLE-2, then MAP-LOAD-1 | serial: MAP-LOAD-1 starts from `main` after MAP-BUNDLE-2 merges |

MAP-BUNDLE-2 starts when this decision merges, and MAP-LOAD-1 starts after MAP-BUNDLE-2. Neither
waits on #1733.

## 1. Rulings

### 1.1 Bundle staging: a CI-built artifact pinned by digest

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

### 1.2 One bundle byte layout, shared by the compiler and the server

`tools/world-bundle-compiler/src/bundle.rs` and `sector.rs` already hold a reader (`read_with`,
`ReadCaps`, `sector::decode`) used by the compiler's own tests. The server must not depend on a
tool crate, and a second reader would let the two drift. MAP-LOAD-1 therefore moves the byte-layout
types, the reader and its caps into one new library crate, `crates/world-bundle`
(`oteryn-world-bundle`), with no dependency on the compiler or the server. The compiler keeps its
writer and depends on the new crate for the layout; `apps/game-server` depends on it for reading.
The move changes no byte and no rule of the format document, and the compiler's tests stay green.

### 1.3 The ground item of a tile, and the ground-speed source

- A tile's ground item is the first top-level entry whose palette entry has `terrain.kind`
  `ground` (§1.4). 2,244 Terrain records are kind `ground`, and each has a KNOWN `ground_speed`;
  border, wall, roof and field records have none.
- A tile with no ground item is not walkable and has no ground speed. The map source returns 0 for
  it, and for a ground item whose `walkable` is KNOWN `false` (the 200 records with speed 0;
  ARCH-ITEM-PACKETS-AMEND-2 §1.11). `player_step_duration` already refuses 0, so nothing paces on
  it.
- MAP-LOAD-1 adds a map-backed `GroundSpeedSource` next to `EngineeringGroundSpeed`. Production
  keeps `EngineeringGroundSpeed` (150) until MAP-CLIENT-1 switches server and client together
  (ADR-0021 amendment). The map source is built and tested, not wired into the live path.
### 1.4 Terrain semantics in the bundle: format v2 (owner 1a)

The v1 bundle carries only a palette key, family and compact id (format §3, §5), and the server's
`TerrainDefinition` holds only a key. So neither gives kind, walkable or ground speed. Under
ADR-0021 D189 (§4.2) the server reads only the compiled bundle, so the compiler resolves the
semantics and writes them into the bundle.

Format §11 makes any change to the manifest fields a new version. The bundle therefore becomes
`OTERYN_WORLD_BUNDLE/v2`, with `format_version` 2 and `min_reader_version` 2:

- **One new field.** Each `palette` entry gains a required field `terrain`. Everything else in v1
  is unchanged in v2: the layout, the payload grammar, the digest rule and every v1 limit.
- **No dual reading.** The compiler writes only v2 and the reader accepts only v2. No v1 bundle
  is stored or consumed anywhere (§1.1), so a v1 bundle is refused like any unknown version.
- **The format document.** It gains a v2 section and keeps its file name.

**The `terrain` field.** The compiler resolves each palette entry with the OPEN-1 rules it
already applies (format §10):

- A `terrain` key resolves to its own record.
- An `item` key resolves to the one record whose `item_pointer` names it, or to none.

It then writes:

- `null` when there is no record: a plain Item, not ground and not walkable;
- otherwise `{"kind", "walkable", "ground_speed"}`:
  - `kind` is the record's KNOWN kind, one of `ground`, `border`, `wall`, `roof` and `field`;
  - for `ground`, `walkable` is a boolean and `ground_speed` is an integer in 0..=1000, with 0
    exactly when `walkable` is false (ARCH-ITEM-PACKETS-AMEND-2 §1.11);
  - for every other kind, both are `null`.

**The compiler fails closed.** It stops on a placed palette entry whose record has an UNKNOWN
`kind`. 65 records have one on `main`. It also stops on a `ground` record with an UNKNOWN
`walkable` or `ground_speed`, on a speed outside 0..=1000, and on speed 0 with `walkable` true.
The existing OPEN-1 rules still refuse a missing record for a Terrain key and two records that
point at one Item key. The parity report adds:
- the count of placed palette entries of each kind;
- the count of placed records with an UNKNOWN kind.

**The reader fails closed.** It rejects a v2 manifest whose `terrain` field is:
- missing;
- of the wrong shape for its `kind`;
- an unknown `kind` value;
- out of range;
- a walkable speed 0;
- carrying an unknown member.

These checks run inside the manifest parse, which `MAP01-BUNDLE-MANIFEST-BYTES` (16 MiB) already
bounds before allocation. Each entry grows by about 60 bytes, so about 20,000 palette entries
add about 1.2 MB, far inside the cap. No new limit row is needed.

**What this removes.** The server reads no Terrain catalogue, so these all go:
- the second pin and `terrain_catalogue_digest`;
- the `content.lock.json` consistency check;
- the six `MAP01-TERRAIN-*` rows.

The one bundle digest (format §6) authenticates the terrain semantics with every other byte.
That closes #1733 P1 4177026515 and P2 4177026518 as well as #1744 P1 4177063031.

### 1.5 Moved to §1.4

The catalogue limits of the previous head are no longer needed, because the server parses no
catalogue (§1.4).

## 2. Packets

### 2.1 MAP-BUNDLE-2 (format v2 and the compiler)

```yaml
task_id: MAP-BUNDLE-2
decision: ADR-0021 §4.2 (D189); OTERYN_WORLD_BUNDLE_FORMAT_V1 §10 and §11; this decision §1.4
worker: oteryn-impl-worker
review: security review of the format change (ADR-0021 §4.8)
branch: allocated by the control plane
base: main (MAP-BUNDLE-1 merged)
migration_lease: none
depends_on: [MAP-BUNDLE-1]
owned_paths:
  - tools/world-bundle-compiler/**                 # v2 writer and reader, terrain resolution, parity counts, regenerated goldens
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md # the v2 section (§1.4); the header's format ID line
validation:
  - cargo test --locked -p oteryn-world-bundle-compiler
  - cargo check --locked --workspace --all-targets
```

Builds §1.4 in the compiler and its reader: the `terrain` field on every palette entry, the
v2 header and manifest, the fail-closed resolution and the parity counts. The format document
gains a "Format v2" section that states the one manifest change and keeps every v1 rule.

Tests:

- a fixture with one entry of each kind compiles into the expected `terrain` values. This
  includes a plain Item (`null`), a walkable ground, a speed 0 non-walkable ground, and a border;
- the compiler refuses each of these: a placed record with an UNKNOWN kind, a ground record with
  an UNKNOWN `walkable` or `ground_speed`, speed 1,001, and speed 0 with `walkable` true. The
  existing OPEN-1 refusals still pass;
- the reader rejects a v1 bundle and each malformed `terrain` value of §1.4, and accepts the
  fixture;
- determinism: two builds of one input are byte-identical. The goldens are regenerated in this
  PR, and every other existing compiler test passes unchanged.

Acceptance: the tests above; the security review on the PR; the parity report on the real map
lists the per-kind counts. If the real map places a record with an UNKNOWN kind, the compile
stops, and the content lane (through the control plane) classifies the record before
MAP-CUTOVER-1.

### 2.2 MAP-LOAD-1

```yaml
task_id: MAP-LOAD-1
decision: ADR-0021 §4.1, §4.2, §4.8 and the §1.11 amendment; OTERYN_WORLD_BUNDLE_FORMAT_V1 §9 and its v2 section; this decision §1.1-§1.4
worker: oteryn-hard-worker
review: security review of the bundle reader (ADR-0021 §4.8)
branch: allocated by the control plane
base: main (MAP-BUNDLE-2 and SPEED-1 merged)
migration_lease: none
depends_on: [MAP-BUNDLE-2, SPEED-1]
owned_paths:
  - crates/world-bundle/**                         # new crate: layout, reader, caps (§1.2)
  - tools/world-bundle-compiler/**                 # moves the v2 reader out; depends on the new crate
  - Cargo.toml                                     # one workspace member
  - Cargo.lock
  - apps/game-server/Cargo.toml                    # the new dependency
  - apps/game-server/src/map/**                    # new: base model, loader, pin check (§1.1, §1.3)
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

- the new crate (§1.2), with the format document's §9 rules: reject the whole bundle on the first
  failure, every MAP01-BUNDLE-* and MAP01-TILE-* cap checked before allocation;
- `WorldBase`: a compact, read-only model of every tile (positions, palette-resolved item compact
  ids, the ground item and its ground speed, the walkable flag), decoded eagerly and shared by
  `Arc` by every channel of the World (ADR-0021 §4.1);
- the load function, which takes only the bundle bytes and the expected pins (bundle digest,
  schema versions, content revision, production flag), reads no other file (D189), and returns a `WorldBase` or a typed error; a production World
  refuses a bundle whose `build_class` is not `production` (missing counts as `non-production`);
- `WorldBase`'s ground item, walkable flag and ground speed, taken only from the palette
  entries' `terrain` field (§1.3, §1.4);
- the map-backed `GroundSpeedSource` (§1.3); production keeps `EngineeringGroundSpeed`.

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
- terrain from the bundle (§1.4): every tile's ground item, walkable flag and ground speed equal
  the fixture's `terrain` values. A tile whose first top-level entry is a border and whose second
  is a ground uses the second. A `null` entry loads as not ground. The loader opens no file
  besides the bundle: the test runs with no `content/` directory present;
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

## 3. What this unblocks

| Work | Was blocked by | After this decision |
|---|---|---|
| MAP-OVERLAY-1, MAP-CUTOVER-1, MAP-WIRE-1/2, DEPOT-WIRE-1, DEPOT-CONTENT-1 | no map reader | packetable after MAP-LOAD-1 (next wave) |
| SPEED-1 map-backed ground speed | no map reader | built by MAP-LOAD-1, wired by MAP-CLIENT-1 |
| MAP-LOAD-1 terrain classification | no terrain semantics in the bundle | MAP-BUNDLE-2 (format v2) |

## 4. Rejected options

- **Keep the reader in the compiler crate.** A server depending on a tool crate, or a second
  reader, is rejected (§1.2).
- **A second server input: the Terrain catalogue pinned by its byte digest.** That was the
  previous head of this decision. It breaks ADR-0021 D189, and it costs a second artifact, a
  second pin, six limit rows and a coupling of the server to the source format (#1744 P1
  4177063031). The owner rejected the D189 amendment it would need (1a, 2026-10-04).
- **A separate terrain table section in the bundle.** Per-palette fields reuse the manifest
  parse and its cap. A new section would add layout, checksums and a reader path for about
  1.2 MB.
- **Ship the whole Terrain record (sight, projectile, floor change, automap).** MAP-LOAD-1 needs
  only kind, walkable and ground speed. Other fields come with their consumers, each in a later
  format version.

## 5. Decision test

1. **Must decide now?** YES. MAP-LOAD-1 is the root of the map chain (§3). It cannot classify
   ground or walkability without terrain semantics, and #1733 cannot carry it through review
   alongside the bank chain (D491).
2. **What concrete work is blocked?** MAP-LOAD-1, and after it MAP-OVERLAY-1, MAP-CUTOVER-1,
   MAP-WIRE-1/2, DEPOT-WIRE-1, DEPOT-CONTENT-1 and the map-backed ground speed of SPEED-1.
3. **What becomes harder or impossible later?**
   - **Fixed semantics.** Terrain semantics are fixed at compile time. A change to a record's
     kind, walkable flag or ground speed takes a new bundle, which is activated only at a
     planned World reset (format §11; ADR-0021 §4.7). It cannot be hot-patched on a running
     World.
   - **Format versions.** Every later Terrain field the server needs (sight, projectile, floor
     change) is a new format version, with a compiler change and regenerated goldens.
   - **The real map.** The compiler now refuses a placed record with an UNKNOWN kind, so the
     real map compiles only once the content lane has classified those records.
   - **Format v1.** It is retired with no reader. That costs nothing now, since no v1 bundle is
     consumed.
4. **What evidence would justify superseding it?**
   - A measured manifest size or parse time near `MAP01-BUNDLE-MANIFEST-BYTES` or
     `MAP01-BASE-LOAD-MS`.
   - A requirement to change terrain semantics without a World reset.
   - A MAP-LOAD-1 measurement above the ADR-0021 budgets.
   - An ADR-0021 amendment of D189.
5. **What is deliberately not decided?**
   - MAP-OVERLAY-1, and MAP-CUTOVER-1, including where the pin lives and the CI artifact job.
   - The client projection.
   - The Terrain fields beyond kind, walkable and ground speed.
   - The classification of the 65 records with an UNKNOWN kind, which belongs to the content lane.
