# DUR-04 — First Production Content Profile

- Decision: `DUR04-FIRST-PROD-CONTENT-01`
- Profile: `FIRST_PRODUCTION_CONTENT_PROFILE/v1`
- Date: 2026-09-09
- Resolves after protected integration/readback: #433
- Next consumer after registry integration/readback: CONTENT #54
- Status: **accepted only when this exact decision is present on protected `main`; branch/PR copies are proposal evidence only**

## 1. Decision

The first production-capable CONTENT slice is a deliberately narrow **production canary profile**. It is not representative full-world scale and does not select a permanent World Project / World Bundle design.

The semantic shape is the minimum movement/combat shape already proven by VSL-CONTENT-01, but production is a **new trust profile**. `evidence:*`, fixture-only and synthetic authority markers cannot be renamed or promoted into production.

This decision grants no live deployment authority, does not make gameplay available, and does not select a permanent authoring format, permanent bundle format, compression, CDN/signing topology or script runtime.

## 2. Evidence basis

Protected evidence used here:

- DUR-04 requires deterministic typed content, separate server/client projections, bounded untrusted loading, immutable staging, explicit atomic publication, compatible rollback, capability compatibility metadata and registered resource maxima.
- VSL-CONTENT-01 establishes the smallest movement/combat semantic set and explicitly does not require scripts, NPCs, quests, houses, market, broad import or a final physical format.
- Current `apps/game-server/src/content/**` proves deterministic ordering, checked arithmetic, SHA-256 artifact/section integrity, exact record shapes, strict unknown-critical rejection and fail-closed pair checking.
- The merged VSL shape is exactly 3 Cells, 21 semantic definitions, 22 server records, 6 client-safe records and 24 counted references. Those values define only this tiny canary; they are not evidence of full-world production scale.
- The live production resource registry already accepts 128 UTF-8 bytes for a semantic identifier (`FND02-CLIENT-BUILD-ID-BYTES`). This profile uses an ASCII-safe subset and therefore adopts 128 bytes for content semantic identifiers independently of the evidence profile.
- Content Format Spike #95/#125 is evidence only. Its 64 MiB artifact, 2 MiB chunk, 4,096 chunks, 512-byte strings, depth 16, 100,000-entry collection and 64:1 compression ratio are **not** production limits here.

## 3. `FIRST_PRODUCTION_CONTENT_PROFILE/v1`

### Compiler IN

The production compiler receives exactly one trusted-build, typed in-memory graph. It does not parse a permanent serialized World Project.

The graph contains exactly:

- 1 package and 1 world identity;
- 1 Region, 1 Area, 1 Terrain;
- 3 Cells on exactly 1 floor;
- 1 local Relocation;
- 1 deterministic Behavior policy reference;
- 3 project-owned Presentation/asset semantic references;
- 1 Creature;
- 1 Spawn with explicit GAME-CHANNEL multiplicity, eligibility and recovery classification and population ceiling 1;
- 1 Formula/Ruleset profile reference to already accepted gameplay semantics; CONTENT does not embed guessed executable formula code;
- 1 Effect, 1 Ability, 1 materializable Item;
- 1 Loot Table containing exactly 1 Loot Entry;
- 1 XP definition;
- 1 deterministic RNG purpose key and 1 RNG execution-profile identity. Root/secret seed is simulation-owned and is not content.

Compiler obligations:

- reject evidence/fixture/synthetic production authority markers;
- canonicalize deterministically and reject duplicates/missing references;
- apply every `REQUIRED_NOW` bound before content-controlled growth/allocation;
- emit one server-authoritative and one allowlisted client-safe projection;
- bind package/content/map/ruleset/world-policy/compiler/canonicalization/provenance/simulation/content-profile identities, exact Content Lock SHA-256 digest, and exact required runtime/protocol capability-set revisions;
- produce SHA-256 artifact and critical-section digests;
- reject every capability outside this profile.

### Compiler OUT

No filesystem/project-tree parser, serialized authoring format, package dependency resolver, aliases, broad OTBM/Crystal importer, scripts, NPC/quest/house/market content, arbitrary object stacks, embedded asset blobs, compression/chunk/delta generation, signing/CDN logic or unbounded compile/publication report.

### Production record schema

The production profile is distinct from the existing evidence serializer. It uses the same replaceable length-prefixed framing concept, but its record field domains are production-specific and do not serialize evidence-only fixture flags/labels.

Server record fields are exactly:

| Record | Exact fields | Maximum data bytes per field |
|---|---|---|
| Region / Area / Terrain / LootTable / RngPurpose | semantic key | 128 |
| Cell | key, region, area, terrain, `i32 x`, `i32 y`, `i16 z`, collision | 128,128,128,128,11,11,6,8 |
| Relocation | key, from-cell, to-cell | 128,128,128 |
| Behavior | key, policy revision | 128,128 |
| Presentation | key, asset semantic token | 128,128 |
| Creature | key, behavior, presentation, positive `u32` max-HP | 128,128,128,10 |
| Spawn | key, creature, behavior, cell, population=`1`, recovery, multiplicity, eligibility | 128,128,128,128,1,31,32,15 |
| FormulaProfile | semantic key of already accepted ruleset/formula profile | 128 |
| Effect | key, family=`damage`, formula-profile key | 128,6,128 |
| Ability | key, effect, presentation | 128,128,128 |
| Item | key, presentation, materializable=`true` | 128,128,4 |
| LootEntry | table key, entry key, item key, RNG-purpose key, positive `u32` weight | 128,128,128,128,10 |
| XP | key, formula-profile key, positive `u32` amount | 128,128,10 |
| RngContext | RNG execution-profile revision | 128 |

Client-safe records are exactly three Presentation records plus Creature/Ability/Item client records, each containing exactly two 128-byte-maximum semantic fields.

The enum maxima above are exact current valid literals: collision 8 (`walkable`), recovery 31 (`CHECKPOINTED_RUNTIME_CONTINUITY`), multiplicity 32 (`CHANNEL_LOCAL_SHARED_ELIGIBILITY`), eligibility 15 (`CHARACTER_WORLD`), effect family 6 (`damage`). Decimal maxima follow the Rust scalar domains used by the VSL model: `i32` 11 bytes, `i16` 6 bytes and `u32` 10 bytes. Spawn population is semantically fixed to 1 in this profile.

### Manifest IN

Each projection has exactly 17 length-prefixed manifest fields:

1. fixed profile ID `FIRST_PRODUCTION_CONTENT_PROFILE/v1` (35 bytes);
2. package key;
3. package revision;
4. world ID;
5. content revision;
6. map revision;
7. ruleset revision;
8. world-policy revision;
9. compiler revision;
10. canonicalization revision;
11. exact Content Lock SHA-256 digest as 64 lowercase hexadecimal bytes;
12. provenance revision/summary identity;
13. simulation-profile revision;
14. first-production-content-profile revision;
15. required server runtime capability-set revision;
16. required protocol capability-set revision;
17. fixed projection class (`server-authoritative` or `client-safe`).

Both projections carry the same capability-set revisions and same meaning-bearing revision set. A changed runtime or protocol capability set therefore requires an explicitly compatible artifact/publication expectation; capability identity cannot drift independently and still pass staging.

The profile binds opaque capability-set **revision identities**, not their membership serialization. Direct capability lists are outside v1 and fail closed.

### Loader IN

Production artifacts are untrusted bytes. Before staging, the loader validates in bounded order:

1. total artifact byte limit;
2. profile magic/version and zero unknown flags;
3. exactly two known critical sections: manifest + body;
4. checked offsets/ranges with no overlap/truncation;
5. section and artifact SHA-256 integrity;
6. exact 17-field manifest, exact fixed-field domains and all semantic-field byte limits;
7. exact Content Lock digest syntax/length;
8. exact record count, record byte limit, known production record kind, exact field count and field-domain limits;
9. semantic identifiers/references/source classification/projection allowlist;
10. exact server/client revision and runtime/protocol capability-set compatibility;
11. exact expected artifact digests, revisions, Content Lock digest and capability-set revisions supplied by the caller's separately authorized publication plan.

The loader performs no network fetch and infers no deployment authority from a digest, test result, repository merge or capability identity.

### Staging IN

- at most one candidate pair may be staged per world/profile;
- staging is immutable and non-authoritative;
- raw artifact bytes remain separately bounded by the artifact-pair limit and are not charged to the decoded staging heap;
- all heap allocation derived from decoded untrusted content is charged to one 32,768-byte staging budget;
- compact offset/index representation is mandatory; unbounded per-string/per-record allocation is forbidden;
- the staged pair binds exact digests/revisions/Content Lock/runtime+protocol capability revisions plus expected current publication generation and proposed next generation;
- activation transfers/reuses the immutable decoded allocation; it must not create an uncharged second decoded copy.

### Runtime IN

Only an **internal CONTENT publication kernel** is in scope:

- immutable lookup;
- one atomic publication pointer per world/profile;
- exact content-generation pinning for each authoritative simulation scope;
- at most one draining predecessor generation;
- checked monotonic publication generation;
- stale-generation rejection;
- compatible rollback as a new higher publication generation.

There is no public CLI/admin/Platform route, deployment controller, automatic startup selection or change to `GameplayAvailability::UnavailableBootstrap`.

## 4. `REQUIRED_NOW` hard maxima

Every row is accepted only for `FIRST_PRODUCTION_CONTENT_PROFILE/v1`. Expansion requires a new architecture/registry decision.

| ID | Dimension | Unit | Hard max | Evidence / derivation | Boundary / denial obligation |
|---|---|---|---:|---|---|
| `DUR04-FPC-PACKAGES` | packages | packages | 1 | one-package canary, no resolver | 1 pass; 2 reject before graph resolution |
| `DUR04-FPC-REGIONS` | Regions | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-AREAS` | Areas | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-TERRAINS` | Terrains | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-CELLS` | Cells | cells | 3 | start + blocked + relocation-target VSL shape | 3 pass; 4 reject before cell/index allocation |
| `DUR04-FPC-FLOORS` | distinct floors | floors | 1 | single-floor canary | second distinct floor reject |
| `DUR04-FPC-RELOCATIONS` | Relocations | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-BEHAVIORS` | Behavior refs | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-PRESENTATIONS` | Presentation refs | definitions | 3 | creature + ability + item presentation | 3 pass; 4 reject |
| `DUR04-FPC-CREATURES` | Creatures | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-SPAWNS` | Spawns | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-SPAWN-POPULATION` | population per Spawn | entities | 1 | deterministic canary without scale assumption | 1 pass; 2 reject before runtime allocation |
| `DUR04-FPC-FORMULA-PROFILES` | Formula/Ruleset refs | definitions | 1 | one accepted policy reference | 1 pass; 2 reject |
| `DUR04-FPC-EFFECTS` | Effects | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-ABILITIES` | Abilities | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-ITEMS` | Items | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-LOOT-TABLES` | Loot Tables | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-LOOT-ENTRIES` | Loot Entries | entries | 1 | one entry in one table | 1 pass; 2 reject |
| `DUR04-FPC-XP-DEFINITIONS` | XP definitions | definitions | 1 | closed canary | 1 pass; 2 reject |
| `DUR04-FPC-RNG-PURPOSES` | RNG purpose keys | keys | 1 | one deterministic loot purpose | 1 pass; 2 reject |
| `DUR04-FPC-DEFINITIONS` | aggregate semantic definitions | definitions | 21 | exact sum of categories | 21 pass; 22 reject before set allocation |
| `DUR04-FPC-REFERENCES` | aggregate semantic references | references | 24 | exact current compiler reference formula over closed graph | 24 pass; 25 reject before collection growth |
| `DUR04-FPC-KEY-BYTES` | semantic key/id | ASCII bytes | 128 | independent production semantic-identifier precedent | valid 128 pass; 129 reject before retain/copy |
| `DUR04-FPC-FIELD-STRING-BYTES` | semantic revision/policy/presentation/capability field | ASCII bytes | 128 | same bounded semantic-identifier class | valid 128 pass; 129 reject before retain/copy |
| `DUR04-FPC-SHA256-HEX-BYTES` | Content Lock digest | lowercase hex bytes | 64 | SHA-256 is exactly 32 bytes / 64 hex bytes | exactly 64 valid lowercase hex pass; 63/65/non-hex reject |
| `DUR04-FPC-MANIFEST-FIELDS` | manifest fields | fields | 17 | exact production manifest above | only 17 accepted |
| `DUR04-FPC-SERVER-RECORDS` | server records | records | 22 | 21 definitions + RNG context | 22 pass; 23 reject before vector allocation |
| `DUR04-FPC-CLIENT-RECORDS` | client-safe records | records | 6 | 3 presentations + creature + ability + item | 6 pass; 7 reject before vector allocation |
| `DUR04-FPC-RECORD-BYTES` | one encoded record payload | bytes | 609 | exact maximum valid Spawn record | valid 609-byte Spawn pass; 610 reject before record copy/decode |
| `DUR04-FPC-SECTIONS` | critical sections | sections | 2 | manifest + body | missing/extra/unknown reject |
| `DUR04-FPC-SECTION-BYTES` | one section | bytes | 6,905 | exact maximum valid server body | valid 6,905 body pass; 6,906 reject before processing |
| `DUR04-FPC-SERVER-ARTIFACT-BYTES` | server artifact | bytes | 9,002 | exact valid-domain framing derivation | valid 9,002 pass; 9,003 reject before parse/allocation |
| `DUR04-FPC-CLIENT-ARTIFACT-BYTES` | client artifact | bytes | 3,688 | exact valid-domain framing derivation | valid 3,688 pass; 3,689 reject before parse/allocation |
| `DUR04-FPC-ARTIFACT-PAIR-BYTES` | server + client pair | bytes | 12,690 | 9,002 + 3,688 | valid max pair pass; 12,691 reject before staging |
| `DUR04-FPC-PROJECTIONS` | projections per generation | projections | 2 | exact server + client pair | missing/extra projection reject |
| `DUR04-FPC-STAGING-HEAP-BYTES` | decoded untrusted heap per staged/resident generation | bytes | 32,768 | exact 23,736-byte charged upper workset below | next charged byte rejects; active unchanged |
| `DUR04-FPC-STAGED-CANDIDATES` | simultaneous staged pairs | candidates | 1 | no parallel activation needed | second reject without replacing staged/active |
| `DUR04-FPC-RESIDENT-GENERATIONS` | distinct resident generations | generations | 2 | current + one draining predecessor | third cannot activate until drain |
| `DUR04-FPC-LIVE-DECODED-CONTENT-BYTES` | aggregate decoded CONTENT heap per world/profile including residents plus one staged candidate | bytes | 98,304 | `(2 resident + 1 staged) * 32,768` | any allocation/copy exceeding aggregate cap reject; publication state unchanged |

### Exact valid-domain byte derivation

A record is `kind:u8 + field_count:u8 + Σ(u16 length + field bytes)`. The largest valid record is Spawn:

`2 + 4*(2+128) + (2+1) + (2+31) + (2+32) + (2+15) = 609 bytes`.

Other important valid maxima are Cell 566, LootEntry 534, Creature 404, Relocation/Ability 392, XP 274, Effect 270, Item 268, Behavior/Presentation 262 and one-field records 132 bytes. These are derived from actual valid field domains, not from an impossible eight-arbitrary-string envelope.

For the exact 22-record production body, including each outer `u32` record-length prefix, the valid worst case is **6,905 bytes**. The six two-semantic-field client records remain **1,600 bytes** total.

The 17-field manifest contains 14 variable semantic fields at 128 bytes, one fixed 35-byte profile ID, one exact 64-byte Content Lock digest and the projection literal. Therefore:

- server manifest: `17*2 + 14*128 + 35 + 64 + 20 = 1,945` bytes;
- client manifest: `17*2 + 14*128 + 35 + 64 + 11 = 1,936` bytes.

Using the replaceable v1 framing already evidenced by CONTENT (`24-byte header + 2*48-byte section table + sections + 32-byte trailer`):

- server artifact: `24 + 96 + 1,945 + 6,905 + 32 = 9,002` bytes;
- client artifact: `24 + 96 + 1,936 + 1,600 + 32 = 3,688` bytes;
- pair: `12,690` bytes.

A boundary fixture for each artifact maximum must use only valid production-domain values (128-byte valid distinct semantic identifiers, exact 64-byte lowercase-hex lock digest, valid maximal enum literals and valid maximal scalar representations). Tests may not pad unknown/free-form fields to manufacture the maximum.

### Exact staging-memory derivation

Raw artifact bytes are independently bounded by 12,690 bytes and are outside the decoded allocator.

Maximum decoded field data is 11,976 bytes:

- server manifest data 1,911 + server record field data 6,627;
- client manifest data 1,902 + client record field data 1,536.

There are at most 117 decoded strings/field slices (server 17+71; client 17+12). The prescribed compact staging upper workset is:

- field/string descriptors: `117*8 = 936`;
- record descriptors: `28*8 = 224`;
- definition index: `21*8 = 168`;
- reference index: `24*8 = 192`;
- artifact/section/revision descriptors: <=2,048;
- bounded validation scratch: <=8,192.

Total charged upper bound is `11,976 + 936 + 224 + 168 + 192 + 2,048 + 8,192 = 23,736` bytes. The 32,768-byte hard cap leaves 9,032 bytes for alignment/fixed allocator bookkeeping. An implementation representation that cannot fit must fail this profile rather than raise the limit.

Activation reuses the staged allocation, so two resident generations consume at most 65,536 decoded bytes. With one distinct staged candidate concurrently present, aggregate decoded CONTENT heap is mechanically capped at 98,304 bytes. A post-activation copy that would exceed this aggregate cap is forbidden.

No numeric maximum above is inherited from the evidence test profile or format spike.

## 5. `EXCLUDED_FAIL_CLOSED`

These capabilities/resource dimensions have no v1 construction/runtime path and cannot be enabled by input/config/artifact flags:

| Capability/dimension | Why unreachable | Required rejection |
|---|---|---|
| script component/module bytes/count | no script definition/section | compiler rejects; loader rejects unknown record/section/capability |
| live script instances | no script runtime/linker | no construction API |
| Wasm memories/pages/tables/elements | no Wasm runtime | no accepted metadata/API |
| script fuel/host calls/query-result/action-plan budgets | no script execution/ABI | no accepted metadata/API |
| persistent script extension state | no extension-state capability | schema/compiler rejects |
| `script_execution_profile_revision` | authoritative scripts absent | production manifest has no such field; unknown field rejected |
| WIT world/interface requirement | no script/WIT consumer exists in v1 | no WIT field/capability accepted; first script/WIT proposal reopens architecture |
| authored arbitrary objects-per-cell collection | no object-stack collection in Cell schema | unknown field/record rejected |
| NPC/quest/house/market/broad catalog definitions | outside first slice | no accepted production record kind |
| ambient filesystem/network/process/environment access | content is data, not ambient authority | no API resolves tokens as I/O authority |
| network artifact/dependency fetch | exact immutable bytes must already be supplied | loader has no resolver |
| fixture/evidence profile as production | trust profiles are distinct | production compiler/loader reject evidence/fixture/synthetic authority markers |
| unknown records/fields/flags/sections | closed schema | reject before staging |
| durable migration other than `COMPATIBLE_NO_MIGRATION` | first slice needs no durable reinterpretation | validation rejects |
| automatic/latest-wins startup selection | external publication plan owns authorization | no scan/latest resolver |
| public live activation endpoint | deployment authority is outside CONTENT | no CLI/admin/Platform/live route |

## 6. `DEFERRED_REQUIRES_FUTURE_DECISION`

These dimensions have no numeric selection now because v1 has no path consuming them. Attempts remain fail-closed until a future accepted decision adds limits.

| Deferred dimension | Why not needed now | Future trigger |
|---|---|---|
| serialized World Project/source bytes | typed in-memory compiler input | first serialized authoring/import format |
| source file count/path bytes/tree/nesting | no filesystem/project parser | first Studio/file-tree path |
| serialized package bytes | no serialized package boundary | first source package format |
| package dependency count/depth | one package, no resolver | first multi-package publication |
| aliases/count/chain depth | no aliases | first alias/deprecation capability |
| world coordinate magnitude/extent-derived allocation | sparse 3-cell set; no allocation may derive from coordinate magnitude | first dense range/extent/streaming design |
| dense object count per cell | arbitrary authored object collection excluded | first object-stack/density capability |
| chunk count/raw chunk bytes/chunk dimensions/floor packing/density | flat replaceable canary framing | first chunked/broad-world bundle |
| permanent spatial-index count/bytes/fan-out | in-memory compact indices are count-bounded | first permanent bundle/spatial index |
| compressed/decompressed bytes and expansion ratio | compression forbidden | first compression proposal |
| embedded asset/blob bytes and asset-pack count | semantic Presentation refs only | first embedded/client asset package |
| compiler/import report bytes | bounded status/digests only | first standardized report contract |
| direct runtime/protocol capability-list count/bytes | v1 binds only opaque accepted capability-set revisions | first artifact that directly enumerates capabilities |
| permanent World Bundle total bytes/version window | v1 framing is replaceable | permanent physical-format decision after real import/Studio/runtime evidence |
| patch/delta count/bytes | no patching | first patch/delta design |
| CDN object count/bytes | no CDN | first distribution design |
| signing-chain/keyset/signature count/bytes | exact expected digests are supplied externally; signing topology unselected | first signing/trust-root decision |

No deferred dimension may receive an implicit/unlimited implementation default.

## 7. Activation and publication semantics

The only legal progression is:

`RECEIVED_OR_BUILT -> VALIDATED -> STAGED -> ACTIVE`.

`VALIDATED` means all profile/schema/limit/integrity/semantic/reference/revision/Content-Lock/capability/authorized-expectation checks pass. `STAGED` is immutable and non-authoritative.

### Commit point

Each world/profile owns one atomic publication pointer containing:

`publication_generation + exact revision/capability set + server digest + client digest + staged handle`.

`PublicationGeneration` is a checked `u64`. Generation 0 is reserved as the no-publication floor. A first-ever authorized bootstrap supplies floor 0 and may commit generation 1. Thereafter an activation must supply the externally authoritative last-committed floor `N` and propose exactly `N.checked_add(1)`. Missing floor, stale/lower/equal generation, gap, overflow or failed CAS fails closed.

The **only commit point** is one compare-and-swap of the complete pointer from the expected floor/current publication to the complete staged pair. No section, projection, definition or scope is published separately.

## 8. Failure, restart, recovery and rollback

- **Before commit:** parse/integrity/limit/semantic/compatibility/authorization/staging failure discards or rejects the candidate; active is unchanged; there is no automatic fallback.
- **During activation:** CAS succeeds completely or not at all. Failure preserves the prior active pointer; there is no partial activation.
- **Restart:** CONTENT never scans a directory/network/repository/timestamp and selects “latest”. A separately authorized publication plan must supply exact artifact bytes/digests/revisions/capability revisions **and the durable last-committed publication floor N**. Missing/ambiguous floor is fail-closed. The exact bytes are revalidated/restaged; the next publication may only be `N+1` by checked arithmetic. Thus process restart cannot reset monotonicity.
- **Last-good/rollback:** previously verified compatible bytes may be selected only by a separately authorized publication plan and become a **new higher generation**. Example: B at generation 8 -> A bytes at generation 9. Content revision itself is opaque and unordered.
- Only `COMPATIBLE_NO_MIGRATION` is accepted. Normalize/explicit migration/incompatible/removal semantics require a future decision. No automatic post-commit rollback exists.

Durability/authority for the publication floor belongs to the external deployment/control plane and is not granted or implemented by this CONTENT decision.

## 9. Coexistence/version semantics

At most two distinct resident generations may coexist: current + one draining predecessor. A simulation scope is pinned to one exact generation for its lifetime; hot in-place mutation is forbidden. A third distinct generation cannot activate until the predecessor drains.

Server/client artifacts must exactly agree on profile/schema, package/content/map/ruleset/world-policy/compiler/canonicalization/Content-Lock/provenance/simulation/content-profile revisions and required runtime/protocol capability-set revisions. A scope can never observe mixed projections/generations.

## 10. Security/trust boundary

- SHA-256 proves byte integrity, not publisher/deployment authority.
- Runtime/protocol capability revision matching proves compatibility identity, not authority.
- Caller expectations come from a separately trusted deployment/control-plane decision; signing topology is deliberately unselected.
- Passing tests, review, CI, Merge Queue or protected integration never grants live activation authority.
- Evidence/fixture artifacts cannot be accepted by the production profile.
- Malformed/corrupt/truncated/oversized/unknown/stale/incompatible/unknown-critical input fails closed.
- No content field is interpreted as a filesystem path, URL, command, environment variable, SQL statement or authority token.
- Client-safe projection is allowlist-generated and never server authority.

## 11. Deliberate non-decisions

This decision does **not** choose permanent `.omap`/`.owb`/World Project/World Bundle format, authoring serialization, permanent chunk/floor packing, compression, patch/delta/CDN, signing/trust-root topology, script runtime/WIT ABI, broad-world scale or live rollout orchestration.

The v1 framing is replaceable and scoped to this canary. A future permanent format may require recompilation/new profile; v1 creates no perpetual parse-compatibility promise.

## 12. Authority separation

**Architecture implementation authority:** only after this decision is protected **and** its exact registry append is separately protected/read back may a fresh #54 allocation authorize this bounded implementation. This decision alone grants no CONTENT write branch.

**Repository integration authority:** normal repository control plane only — exact-head review, required checks, FULL Merge Queue and protected-main readback. No direct merge/protection bypass.

**Live deployment / production activation authority:** **NONE**. No branch, PR, CI pass, protected merge, artifact or internal publication kernel may mutate a live/protected environment without separate explicit deployment authority.

## 13. Mechanical next steps

### Exact serialized registry append

`docs/architecture/DUR-04_FIRST_PRODUCTION_CONTENT_PROFILE_REGISTRY_APPEND.json` is the canonical append packet matching the live registry schema. After fresh protected registry readback, its rows must be appended without changing maxima/units/boundary obligations; any ID collision or contradictory current DUR-04 row reopens reconciliation rather than being overwritten.

### Fresh CONTENT #54 allocation after registry protected readback

Writable runtime paths, and no others unless a concrete same-scope compile dependency is proven:

- `apps/game-server/src/content/model.rs`
- `apps/game-server/src/content/compiler.rs`
- `apps/game-server/src/content/artifact.rs`
- `apps/game-server/src/content/production.rs` (new if useful)
- `apps/game-server/src/content/mod.rs`
- `apps/game-server/src/content/tests.rs`

`apps/game-server/src/lib.rs` is not allocated to enable gameplay/live activation; `GameplayAvailability::UnavailableBootstrap` stays unchanged.

Implementation obligations:

- preserve `EvidenceLimits`, evidence fixtures and evidence compile target as non-production;
- add distinct `FirstProductionContentLimitsV1` bound exactly to protected registry rows;
- add a distinct production graph/compile target/profile; never rename evidence types/markers into production;
- implement the exact production record/17-field manifest schema and capability-set bindings above;
- add bounded production staging and enforce 32,768 per-generation + 98,304 aggregate decoded heap limits;
- add checked `PublicationGeneration` plus explicit durable external restart floor semantics;
- add internal/testable `ContentPublicationKernelV1` with exact expected-generation CAS;
- add scope pin/release enforcing max two resident generations;
- expose no public deployment activation surface.

Exact-head tests:

1. table-driven at-limit and max+1 denial for every registry row, with **valid-domain** at-limit artifacts/records;
2. deterministic byte-identical compile under enumeration shuffle;
3. duplicate/missing-reference and malformed graph rejection;
4. production rejects evidence/fixture/synthetic authority markers;
5. server/client projection leak, revision mismatch and runtime/protocol capability-set mismatch rejection;
6. invalid Content Lock digest, corrupt/truncated/unknown/overflow/oversize rejection before staging;
7. scripts/WIT/compression/chunks/source files/dependencies/aliases and all excluded/deferred capabilities fail closed;
8. staging allocator and aggregate decoded-memory exhaustion preserve active state;
9. valid pair reaches ACTIVE only at the complete-pointer CAS;
10. pre-commit failure/stale/gapped/overflow CAS preserves previous active generation;
11. no mixed-generation scope; existing scopes remain pinned;
12. third resident distinct generation rejected until drain;
13. rollback only as new higher generation to previously verified compatible bytes;
14. non-`COMPATIBLE_NO_MIGRATION` rejected;
15. restart with missing/stale publication floor cannot activate and cannot reset monotonicity;
16. compile-fail/public-surface regression proves no live activation API;
17. `GameplayAvailability::UnavailableBootstrap` remains true.

## 14. #433 resolution condition

#433 is `RESOLVED` only after this decision and append packet pass:

- genuinely independent exact-head architecture/security review with `P0=0 / P1=0 / P2=0`;
- applicable canonical CI/governance;
- zero unresolved material review threads;
- normal FULL Merge Queue;
- protected-main readback.

Then the exact handoff is:

`#433 resolved -> RESOURCE_LIMITS_REGISTRY serialized update -> review/CI/FULL MQ/readback -> fresh CONTENT #54 allocation`

No step grants live deployment authority.
