# MAP-LOAD-PACKET-1: the World Bundle loader packet

```yaml
decision_id: MAP-LOAD-PACKET-1
status: CANDIDATE
date: 2026-10-04
owner: Sol Supervising Architect
requested_by: control plane D491 (owner answer 5a, 2026-10-04): MAP-LOAD-1 moves out of ARCH-BATCH-ROOT-PACKETS-V1 (#1733) into its own decision
writes_on_other_prs: none
```

This decision packets MAP-LOAD-1, the root of the map chain (MAP-OVERLAY-1, MAP-WIRE-1,
DEPOT-WIRE-1, NPC-ACTOR-1 and the ground speed source of SPEED-1). It was §1.2-§1.4 and §2.1 of
`OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md` (#1733). Control plane D491 moved it here
with two open findings of #1733, which §1.4 and §1.5 answer:

- P1 4177026515: bind the Terrain catalogue bytes to the bundle pin.
- P2 4177026518: bound the catalogue before parsing.

It changes no code, no contract and no wire. Its rulings are architecture rulings under ADR-0021
and `OTERYN_WORLD_BUNDLE_FORMAT_V1`. Live PR and Issue state governs. When this was written:

- on `main`: MAP-BUNDLE-1 (the format document, the compiler and its reader in
  `tools/world-bundle-compiler`) and SPEED-1 (#1715, capability 13 and the `GroundSpeedSource`
  seam);
- not built: any runtime map reader.

## 0. Leases and shared files

MAP-LOAD-1 needs no migration, capability, command, event or profile.

| File | Packets | Rule |
|---|---|---|
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | MAP-LOAD-1; ITEM-USE-WIRE-1, BANK-1 and GOLD-FEE-2 (#1733) | each edits only its own rows; the second to merge takes `main` in with a merge commit and keeps both |
| `Cargo.toml`, `Cargo.lock` | MAP-LOAD-1 | one new workspace member (§1.2) |

MAP-LOAD-1 starts when this decision merges. It does not wait on #1733.

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
### 1.4 The Terrain catalogue input, pinned by the digest of its bytes

The bundle carries only a palette key, family and compact id (format §3, §5), and the server's
`TerrainDefinition` holds only a key, so neither gives kind, walkable or ground speed. The bundle
format stays v1. The loader takes a second input: the Terrain catalogue of the World Project
(`content/world/terrain/terrain-*.json`) from the same source checkout the bundle was compiled
from.

**Binding (#1733 P1 4177026515).** The shards carry no `project_revision`, and
`content/world/manifest.json` does not inventory them. `content/world/content.lock.json` therefore
does not authenticate their bytes, so a revision or lock check alone could accept stale or edited
shards. The catalogue is instead pinned by a digest over its exact bytes, like the bundle:

- `terrain_catalogue_digest` is SHA-256 over the domain tag `OTERYN_TERRAIN_CATALOGUE_DIGEST/v1`
  and a NUL byte. Then, for each shard in ascending byte order of its file name, it adds:
  - the name length as a u32 little-endian;
  - the name bytes;
  - the byte length as a u64 little-endian;
  - the exact file bytes, unparsed.
- The catalogue is exactly the files that match `terrain-*.json`. `index.json` is a directory
  note and is not part of it.
- The function lives in `crates/world-bundle`, so the compiler and the server share one
  definition. The compiler computes it over the shards it read and prints it next to the bundle
  digest. CI records both from the one build (§1.1).
- The loader's caller passes the expected catalogue digest as a pin, beside the bundle digest.
  The loader hashes the bytes it was given and refuses the pair on any difference, before it
  parses a shard.
- As a consistency check, the loader also refuses a pair whose `content.lock.json`
  `project_revision` differs from the bundle's `identity.content_revision`. That check is not
  the authentication; the digest is.
- Where both pins live in the World configuration binds MAP-CUTOVER-1, as for the bundle digest.

**Resolution.**

- A palette entry of family `terrain` resolves to the record with that key.
- One of family `item` resolves to the one Terrain record whose `item_pointer` names that key, or
  to none, which loads as not ground and not walkable.
- The loader reads `kind`, `walkable` and `ground_speed` from the resolved record.
- It refuses the bundle when:
  - a `terrain` palette key has no record;
  - two records point at one Item key;
  - a `ground` record has an UNKNOWN `walkable` or `ground_speed`.

### 1.5 Catalogue limits (#1733 P2 4177026518)

The MAP01-BUNDLE-* caps cover only bundle bytes, and the World Project source profile defers
physical-source maxima. MAP-LOAD-1 therefore registers these rows in
`RESOURCE_LIMITS_REGISTRY.json` (owner contract ADR-0021, failure category `CAPACITY_EXCEEDED`,
not client-visible, consumer the game server World loader):

| Row | Maximum | Today |
|---|---|---|
| `MAP01-TERRAIN-SHARD-COUNT` | 64 shards | 21 |
| `MAP01-TERRAIN-SHARD-BYTES` | 4 MiB per shard | 537,179 |
| `MAP01-TERRAIN-CATALOGUE-BYTES` | 32 MiB in all | about 8.7 MiB |
| `MAP01-TERRAIN-RECORDS` | 65,536 records | 8,612 |
| `MAP01-TERRAIN-JSON-DEPTH` | 16 nesting levels | 6 |
| `MAP01-TERRAIN-STRING-BYTES` | 1 KiB per string or key | 103 |

Each limit is enforced before allocation:

- The shard count and file sizes are checked from metadata before any file is read.
- Bytes are read into a buffer bounded by the remaining catalogue budget.
- Depth, string length and record count are checked during a bounded parse, which stops at the
  first excess.

An excess refuses the whole catalogue with a typed error, and the World stops before admission.
The maxima are hard and not configurable. Raising one is a registry change.

## 2. Packet

### 2.1 MAP-LOAD-1

```yaml
task_id: MAP-LOAD-1
decision: ADR-0021 §4.1, §4.2, §4.8 and the §1.11 amendment; OTERYN_WORLD_BUNDLE_FORMAT_V1 §9; this decision §1.1-§1.5
worker: oteryn-hard-worker
review: security review of the bundle reader (ADR-0021 §4.8)
branch: allocated by the control plane
base: main (MAP-BUNDLE-1 and SPEED-1 merged)
migration_lease: none
depends_on: [MAP-BUNDLE-1, SPEED-1]
owned_paths:
  - crates/world-bundle/**                         # new crate: layout, reader, caps (§1.2)
  - tools/world-bundle-compiler/**                 # moves the reader out; depends on the new crate; prints the catalogue digest (§1.4)
  - Cargo.toml                                     # one workspace member
  - Cargo.lock
  - apps/game-server/Cargo.toml                    # the new dependency
  - apps/game-server/src/map/**                    # new: base model, loader, pin check, Terrain catalogue reader (§1.4, §1.5)
  - apps/game-server/src/movement/speed.rs         # the map-backed GroundSpeedSource
  - apps/game-server/src/world_runtime.rs          # the Arc<WorldBase> handle only; no live wiring
  - apps/game-server/tests/map_load_*.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json   # MAP01-BASE-LOAD-MS, -BASE-RSS-BYTES, -VIEWPORT-US: measured value and evidence; the new MAP01-TERRAIN-* rows (§1.5)
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
- the load function, which takes the bundle bytes, the Terrain catalogue bytes (§1.4) and the
  expected pins (bundle digest, catalogue digest, schema versions, content revision, production
  flag) and returns a `WorldBase` or a typed error; a production World
  refuses a bundle whose `build_class` is not `production` (missing counts as `non-production`);
- `terrain_catalogue_digest` in the new crate, and the compiler printing it next to the bundle
  digest (§1.4);
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
- the Terrain catalogue (§1.4): the pinned bytes are accepted; one changed byte in any shard, a
  missing, extra or renamed shard, and a catalogue whose lock revision differs from the bundle's
  `content_revision` are each refused; the compiler and the server compute the same digest for
  the fixture; a `terrain` palette key without a record, two records pointing at one Item
  key, and a `ground` record with an UNKNOWN `walkable` or `ground_speed` each refuse the bundle;
  an Item palette key with no Terrain record loads as not ground;
- each MAP01-TERRAIN-* limit (§1.5) at its maximum (accepted) and maximum + 1 (refused), checked
  before the bytes or the parsed model are allocated;
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
the new crate; the MAP01-TERRAIN-* rows are registered.

## 3. What this unblocks

| Work | Was blocked by | After this decision |
|---|---|---|
| MAP-OVERLAY-1, MAP-CUTOVER-1, MAP-WIRE-1/2, DEPOT-WIRE-1, DEPOT-CONTENT-1 | no map reader | packetable after MAP-LOAD-1 (next wave) |
| SPEED-1 map-backed ground speed | no map reader | built by MAP-LOAD-1, wired by MAP-CLIENT-1 |

## 4. Rejected options

- **Keep the reader in the compiler crate.** A server depending on a tool crate, or a second
  reader, is rejected (§1.2).
- **Version the bundle format to carry terrain semantics.** It would move the same data into
  every bundle and needs a format v2 and a compiler change. The pinned catalogue needs neither.
- **Authenticate the catalogue by `content.lock.json`.** The lock does not cover the shard bytes
  (§1.4).
- **Add the shards to `content/world/manifest.json`.** That is a World Project source change owned
  by the content lane, and it still needs a server-side digest check. The byte digest is
  sufficient alone.
- **Parse the catalogue unbounded and rely on file sizes in CI.** The loader runs before
  admission on the node, so its own caps must hold (§1.5).

## 5. Decision test

- **Must decide now:** YES. MAP-LOAD-1 is the root of the map chain (§3), and #1733 cannot carry
  it through review alongside the bank chain (D491).
- **Minimum sufficient:** one packet. It builds the reader, the model, the pins and the limits,
  and it wires nothing into the live path.
- **Superseding evidence:** any of these would reopen this decision:
  - a MAP-LOAD-1 measurement above the ADR-0021 budgets;
  - a content-lane decision that puts the Terrain catalogue under the manifest;
  - a bundle format v2.
- **Deliberately not decided:** MAP-OVERLAY-1, and MAP-CUTOVER-1 (including where both pins
  live).
