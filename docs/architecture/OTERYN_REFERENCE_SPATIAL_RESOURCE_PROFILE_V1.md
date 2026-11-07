# Reference spatial resource candidate v1: one destination

- Status: **PROPOSAL FOR INDEPENDENT REVIEW; NOT ACCEPTED**
- Allocation: #162 comment 5818682264; #504 owner comment 5818657159
- Base: Game main `663052f3139e2e5fff98f904fc1e8a02fd205115`
- Scope: synthetic, non-shipping `REFERENCE_SPATIAL_SINGLE_DESTINATION_CANDIDATE/v1`
- Runtime, registry, production, Movement and target-admission authority: **NONE**

## Problem and constraints

The accepted carrier architecture calls for an exact static destination lookup keyed
by `(WorldId, CoordinateFrameRef, MapRevisionRef, active Content generation,
LogicalCell)`. The resource profile and exact July-28 collision claims remain
unaccepted. This candidate measures the smallest deliberately limited generation:
**one addressed destination cell total**. A cardinal lookup performs one keyed
cell resolution; that fact alone does not establish that a production generation,
Reference corridor or world contains only one cell. A generation with two cells is
unsupported here and needs a separately measured/versioned successor.

The immutable target is `global-tibia-observable-2026-07-28-post-server-save`.
The Python fixture uses made-up identifiers, coordinates and collision values.
No real Reference collision, walkability, floor, placement, footprint or target
continuity is inferred from OTS, minimap or later observations. #483 must admit
the exact field before any production linker can emit a collision fact.
`FIRST_PRODUCTION_CONTENT_PROFILE/v1` and Reference Item artifact v1–v4
retain their existing bytes and activation interpretation. This sidecar has a
unique magic/version and has no production loader.

## Exact candidate encoding

Little-endian, uncompressed, separately typed server (view 1) and client
(view 2) artifacts. A decoder rejects noncanonical manifest JSON, unknown
fields, duplicate keys, extra bytes, unknown versions/views and wrong bindings.
All manifest atoms are UTF-8, maximum 512 encoded bytes for each of world,
frame and map revision; the generation is a canonical lowercase nonzero
SHA-256 hex value. A future production representation must bind these semantic
identities to actual Content Lock/revision provenance and active generation.
The tool accepts an expected binding supplied by its caller and compares every
member; this tool does **not** verify the caller's authority.

| Section | Exact bytes or layout |
|---|---|
| Header | 64: magic 8, version u16, view u8, reserved u8, manifest/index/body u32 lengths, SHA-256 of three concatenated sections, eight zero pad bytes |
| Manifest | canonical sorted compact JSON of four exact binding members; capped at 7,500 bytes |
| Index | u32 count then one 52-byte entry: signed i32 x/y/z, u32 body offset/length, SHA-256 record digest |
| Server body | one 33-byte record: 0/1 exact cell collision fact plus 32-byte nonzero evidence-binding digest |
| Client body | one zero byte, presentation count 0; no collision bit, evidence digest or movement legality |

The digest is an opaque reference to a separately qualified exact-field
`EvidenceBindingRef`; it is **not** a proof or a source verification protocol.
The current production linker has no admitted static-cell collision claim
binding. A production successor must check source provenance, claim class,
target continuity and lock/generation *before* encoding. Neither the Python
caller nor a self-asserted digest can promote an `UNKNOWN`, `CONFLICT`,
`OTS_HYPOTHESIS_ONLY`, missing or post-target-without-continuity claim.

Index address/order is not `PlacementKey`. This minimal cell-fact candidate
accepts no placements, objects, ordered placement relation, presentation
footprint or collision footprint. Unsupported values fail closed at the input
boundary; absence does not mean empty or walkable. When a selected child
needs any of these D1 semantics, this version is insufficient: use a new typed
placement/footprint version preserving stable `PlacementKey` and
`PlacementRef=(PlacementKey, exact_map_revision, TypedDefinitionRef)`,
ordered placements, and distinct presentation/collision footprint relations.
No footprint can be reconstructed from the zero client presentation count.
The current runtime owner alone remains responsible for mutable actor position.

## Finite resource proposal

These are **candidate ceilings for the one-cell sidecar only**, not a measured
world or Reference corridor footprint. The 7,500 manifest cap is an inherited
conservative carrier budget with explicit pre-slice rejection, **not a reachable
maximum for this four-field grammar**. The three-atom 512-ASCII-byte fixture
encodes a 1,687-byte manifest; escaping can vary actual length, so that
fixture is not an exact grammar maximum. This is not evidence that a Reference
manifest uses those atoms. u32 section limits and checked u64 arithmetic
apply before slicing or allocation.

| Dimension | Candidate maximum | Boundary |
|---|---:|---|
| cells / destinations per generation | 1 | count 2 rejected |
| coordinate axes | signed i32 [-2,147,483,648, 2,147,483,647] | either adjacent out-of-range rejected |
| world/frame/map revision atom | 512 UTF-8 bytes each | 513 rejected |
| generation / evidence digest | 32 raw bytes each | zero, malformed or wrong expected binding rejected |
| index entries / bytes | 1 / 56 | `4 + 52*1`; count 2 rejected |
| server/client body bytes | 33 / 1 | record length 34/2 rejected |
| manifest bytes | 7,500 conservative rejection budget | forged header 7,501 rejected before slicing; actual maximum under this grammar is smaller and not proven tight |
| server/client artifact bytes | 7,653 / 7,621 | `64 + 7,500 + 56 + 33/1`; max+1 rejected |
| generation pair encoded bytes | 15,274 conservative envelope | checked sum and component limits; hypothetical maximum components pass, max+1 component rejects |

The artifact ceilings are *safe envelope arithmetic*, not tight reachable maxima
or observed output lengths. There is no actual max-byte fixture at the envelope;
the test verifies section rejection from forged headers and component/pair
checked arithmetic separately. Fixture outputs: manifest 203, server 356, client 324, pair 680 bytes.
Max-length ASCII atom fixture: manifest 1,687, server 1,840, client 1,808
bytes. No resident-memory, loading-time, arbitrary generation count, full
Reference source cardinality, or general production artifact budget is claimed.
The cell is looked up via one exact address after a bounded index parse
(1 entry); no sublinear or general N lookup claim follows.

Run reproducibly from repository root:

```text
python3 tools/reference-spatial-resource-profile/spatial_resource_profile_self_test.py
python3 tools/reference-spatial-resource-profile/spatial_resource_profile.py --output docs/agents/evidence/OTV2-20260924-reference-spatial-resource-measurement-504.json
```

Four self-test groups cover encode/decode determinism, the four exact manifest
bindings, address and evidence mismatch, view separation, cardinal 1/2,
signed coordinate bounds, UTF-8 atom 512/513, unsupported D1 fields,
forged section length max+1/count/offset/record, oversized artifacts,
component/pair envelope arithmetic, trailing bytes, and checked
addition/multiplication overflow. They do not claim a reachable manifest,
artifact or pair exact-maximum byte fixture. A later production codec needs its own
adversarial decode, source admission and runtime end-to-end qualification.

## Options and decision timing

| Option | Trade-off |
|---|---|
| This one-destination separate sidecar | Small, closed and measurable first lookup shape; deliberately rejects any broader generation and offers no presentation placements |
| Broader multi-cell indexed generation | Useful for corridor content, but requires selected Reference field evidence, generation cardinality and placement/footprint/resource measurements absent today |
| Widen bootstrap or Item v1–v4 carrier in place | Historical decoding and activation ambiguity; incompatible with preserved profiles |

**Must decide now? NO for production acceptance.** This candidate can be
reviewed now, but #504 cannot accept a general Reference spatial resource
profile from synthetic N=1 arithmetic. The bounded structural child is the
only conceivable user; even it may require a separate fixture carrier.
Prematurely fixing one-cell production capacity would force a version
migration as soon as a second destination is selected. Reopen after #483
exact-field admission and a selected generation corpus establish actual
cardinality, placements/footprints and byte measurements; separately
register limits and authorize production implementation. No permanent
world encoding, chunk size, Reference geography, RL-03, actor movement,
target collision value, placement order or footprint is decided here.

`REFERENCE_SPATIAL_CARRIER_ARCHITECTURE = OWNER_ACCEPTED_SEMANTICS_ONLY`
`REFERENCE_SPATIAL_RESOURCE_PROFILE = CANDIDATE_NOT_ACCEPTED`
`REFERENCE_SPATIAL_PRODUCTION_ADMISSION = BLOCKED_ON_483_AND_RESOURCE_PROFILE`
