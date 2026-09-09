# DUR-04 first production content profile — complete resource classification

Normative companion to `DUR-04_FIRST_PRODUCTION_CONTENT_PROFILE.md` for decision `DUR04-FIRST-PROD-CONTENT-01` / profile `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.

Every potential resource dimension identified by DUR-04, VSL-CONTENT-01, the merged CONTENT evidence seam, or the Content Format Spike inventory has exactly one classification below. A dimension listed as `DEFERRED_REQUIRES_FUTURE_DECISION` is still rejected by v1 input; “deferred” means its future numeric/design choice is intentionally not made now. A dimension listed as `EXCLUDED_FAIL_CLOSED` has no v1 construction/parse/runtime capability at all.

Where multiple semantic names are the same enforced scalar, the table names the single registry row that owns the bound. No duplicate registry row is required merely to alias the same count.

## `REQUIRED_NOW`

| Potential dimension | Registry owner / hard maximum | Unit | Evidence | Boundary and fail-closed obligation |
|---|---|---|---|---|
| package count | `DUR04-FPC-PACKAGES = 1` | packages | one-package canary, no dependency resolver | second package rejected before graph resolution |
| Region count | `DUR04-FPC-REGIONS = 1` | definitions | closed VSL canary | max+1 rejected before definition allocation |
| Area count | `DUR04-FPC-AREAS = 1` | definitions | closed VSL canary | max+1 rejected |
| Terrain count | `DUR04-FPC-TERRAINS = 1` | definitions | closed VSL canary | max+1 rejected |
| cell count | `DUR04-FPC-CELLS = 3` | cells | start + blocked + relocation-target minimum VSL shape | fourth cell rejected before cell/index allocation |
| logical cell-index entries | `DUR04-FPC-CELLS = 3` | entries | sparse index has exactly one entry per admitted cell | fourth entry cannot be created because fourth cell is rejected |
| distinct floor count | `DUR04-FPC-FLOORS = 1` | floors | single-floor canary | second distinct floor rejected before spatial index allocation |
| Relocation count | `DUR04-FPC-RELOCATIONS = 1` | definitions | closed VSL canary | max+1 rejected |
| relocation lookup candidate count | `DUR04-FPC-RELOCATIONS = 1` | candidates | only one relocation exists in the entire profile | a second candidate cannot exist because the second relocation is rejected |
| Behavior references | `DUR04-FPC-BEHAVIORS = 1` | definitions | closed VSL canary | max+1 rejected |
| presentation / project-owned asset-reference count | `DUR04-FPC-PRESENTATIONS = 3` | definitions | creature + ability + item presentation | fourth rejected |
| Creature count | `DUR04-FPC-CREATURES = 1` | definitions | closed VSL canary | max+1 rejected |
| Spawn count | `DUR04-FPC-SPAWNS = 1` | definitions | closed VSL canary | max+1 rejected |
| runtime population represented by one Spawn | `DUR04-FPC-SPAWN-POPULATION = 1` | entities | smallest deterministic production canary | value 2 rejected before runtime population allocation |
| Formula/Ruleset profile reference count | `DUR04-FPC-FORMULA-PROFILES = 1` | definitions | one opaque reference to already accepted gameplay semantics | max+1 rejected; executable formula code is not content |
| Effect count | `DUR04-FPC-EFFECTS = 1` | definitions | closed VSL canary | max+1 rejected |
| Ability count | `DUR04-FPC-ABILITIES = 1` | definitions | closed VSL canary | max+1 rejected |
| Item count | `DUR04-FPC-ITEMS = 1` | definitions | closed VSL canary | max+1 rejected |
| Loot Table count | `DUR04-FPC-LOOT-TABLES = 1` | definitions | closed VSL canary | max+1 rejected |
| Loot Entry count | `DUR04-FPC-LOOT-ENTRIES = 1` | entries | one entry in one table | max+1 rejected |
| XP definition count | `DUR04-FPC-XP-DEFINITIONS = 1` | definitions | closed VSL canary | max+1 rejected |
| deterministic RNG-purpose count | `DUR04-FPC-RNG-PURPOSES = 1` | keys | one loot RNG purpose; root seed remains simulation-owned | max+1 rejected |
| aggregate semantic definition count | `DUR04-FPC-DEFINITIONS = 21` | definitions | exact sum of category counts | 22 rejected before set allocation |
| unique content-key count | `DUR04-FPC-DEFINITIONS = 21` | keys | every admitted semantic definition has exactly one unique key; duplicate keys already fail validation | the 22nd key requires a 22nd definition and is rejected; duplicate key fails closed independently |
| aggregate semantic reference count | `DUR04-FPC-REFERENCES = 24` | references | exact current compiler formula over the closed 21-definition graph | 25 rejected before reference-collection growth |
| semantic key / PackageKey / WorldId byte length | `DUR04-FPC-KEY-BYTES = 128` | ASCII bytes | independent active production semantic-identifier precedent; v1 allows semantic IDs only | 129 rejected before retain/copy; invalid/non-ASCII form rejected |
| revision/policy/presentation/RNG-profile field byte length | `DUR04-FPC-FIELD-STRING-BYTES = 128` | ASCII bytes | same bounded semantic-identifier class; no free-form text/blob | 129 rejected before retain/copy |
| manifest field count | `DUR04-FPC-MANIFEST-FIELDS = 15` | fields | exact v1 framing | 14/16 rejected; only 15 accepted |
| server body record count | `DUR04-FPC-SERVER-RECORDS = 22` | records | 21 definitions + RNG context | 23 rejected before record-vector allocation |
| client-safe body record count | `DUR04-FPC-CLIENT-RECORDS = 6` | records | three presentations + creature + ability + item | 7 rejected before vector allocation |
| one encoded record length | `DUR04-FPC-RECORD-BYTES = 1,042` | bytes | largest record has 8 exact fields: `2 + 8*(2+128)` | 1,043 rejected before copy/decode |
| artifact section count | `DUR04-FPC-SECTIONS = 2` | sections | exact manifest + body framing | missing/extra/unknown section rejected |
| one artifact section length | `DUR04-FPC-SECTION-BYTES = 9,626` | bytes | exact worst-case server body distribution | 9,627 rejected before processing/allocation |
| server-authoritative artifact length / compiler server output | `DUR04-FPC-SERVER-ARTIFACT-BYTES = 11,728` | bytes | exact fixed-framing derivation in the decision | max+1 rejected before parse/allocation |
| client-safe artifact length / compiler client output | `DUR04-FPC-CLIENT-ARTIFACT-BYTES = 3,702` | bytes | exact fixed-framing derivation | max+1 rejected before parse/allocation |
| aggregate artifact-pair bytes / compiler total binary output | `DUR04-FPC-ARTIFACT-PAIR-BYTES = 15,430` | bytes | exact server + client maxima | max+1 rejected before staging |
| projection count per generation | `DUR04-FPC-PROJECTIONS = 2` | projections | exactly server-authoritative + client-safe | missing/third projection rejected |
| artifact pair count per staged generation | `DUR04-FPC-STAGED-CANDIDATES = 1` | pairs | one staged candidate is sufficient | second simultaneous pair rejected, previous staged/active unchanged |
| decoded staging-memory budget | `DUR04-FPC-STAGING-HEAP-BYTES = 32,768` | bytes | exact 26,464-byte charged upper workset + 6,304 bytes alignment/fixed bookkeeping margin under prescribed compact representation | next charged byte rejects staging; active unchanged |
| staged candidate count | `DUR04-FPC-STAGED-CANDIDATES = 1` | candidates | no parallel activation required | second candidate rejected |
| resident content generation count | `DUR04-FPC-RESIDENT-GENERATIONS = 2` | generations | one current + one draining predecessor | third distinct generation cannot activate until predecessor drains |
| logical definition-index entries | `DUR04-FPC-DEFINITIONS = 21` | entries | compact runtime index is one entry per admitted definition | 22nd entry cannot exist because 22nd definition is rejected |
| client-safe presentation/item/ability/creature record count | `DUR04-FPC-CLIENT-RECORDS = 6` and `DUR04-FPC-PRESENTATIONS = 3` | records / refs | exact allowlisted client projection | extra record/ref rejected before projection staging |
| client-safe projection bytes | `DUR04-FPC-CLIENT-ARTIFACT-BYTES = 3,702` | bytes | exact framing derivation | max+1 rejected |
| compiler semantic workset count | owned jointly by `DEFINITIONS=21`, `REFERENCES=24`, `SERVER-RECORDS=22`, `CLIENT-RECORDS=6` | bounded items | compiler has no recursion or input-controlled extra pass domain; every loop is over one of these bounded collections | implementation tests must prove no unregistered input-controlled collection/recursion; any such addition reopens architecture |
| loader malformed-input workset | owned jointly by `ARTIFACT-PAIR-BYTES=15,430`, `SECTIONS=2`, record and field bounds | bounded bytes/items | loader work is linear/fixed over already registered untrusted lengths and has no recursive parser | reject declared maxima before allocation/iteration; malformed input cannot open an unbounded loop |
| staging validation workset | owned jointly by pair bytes/counts and `STAGING-HEAP-BYTES=32,768` | bytes/items | validation can visit only the admitted pair and compact indices | allocator/workset overflow rejects staging and preserves active state |

The three derived-workset rows above are not additional independently configurable resources: they are mechanically bounded by the named registry rows. An implementation that introduces a new externally controlled pass count, retry count, recursion depth, collection or report buffer has introduced a new resource dimension and fails this profile until registered.

## `EXCLUDED_FAIL_CLOSED`

| Potential dimension/capability | Why unreachable in v1 | Rejection point / non-activation proof |
|---|---|---|
| authored objects-per-cell collection count | Cell records carry terrain/collision/coordinates/references but no arbitrary object collection | compiler production graph has no such collection; unknown field/record rejected by loader |
| script component/module bytes | no script definition/section | compiler rejects script-bearing graph; loader rejects unknown record/section/capability |
| script component/module count | no script model | same boundary |
| live script instance count | no script linker/runtime | no construction API exists |
| Wasm memory/page count | no Wasm runtime | no accepted field/API; script metadata invalid |
| Wasm table/element count | no Wasm runtime | same |
| script fuel | no script execution | no accepted field/API |
| script host-call count | no host ABI | no accepted field/API |
| script query/result collection count/bytes | no script query import | no accepted field/API |
| script proposed action-plan count/bytes | no action-plan API | no accepted field/API |
| persistent extension-state bytes | no extension-state or script persistence capability | schema/compiler reject it; no durability API |
| NPC definitions | outside first production slice | no accepted production kind; unknown kind rejected |
| quest definitions/state | outside first production slice | no accepted production kind/state surface |
| house definitions/state | outside first production slice | no accepted production kind/state surface |
| market definitions/state | outside first production slice | no accepted production kind/state surface |
| arbitrary broad catalog records | closed exact semantic profile | unknown/extra category rejected before emission/staging |
| ambient filesystem read/write | content is data, never ambient authority | no content API maps token to filesystem authority |
| ambient network access | no script/network capability | no accepted API |
| process/env access | no executable/host capability | no accepted API |
| network artifact fetch | caller must already supply exact immutable bytes | loader has no network resolver |
| network dependency fetch | no dependency resolver | no network resolver |
| evidence/fixture profile as production | evidence and production trust profiles are distinct | production compiler/loader reject `evidence:*`, fixture-only and synthetic authority markers |
| unknown record kinds/field counts/flags/sections | closed v1 schema | loader rejects before staging |
| durable migration requiring state reinterpretation | v1 accepts only `COMPATIBLE_NO_MIGRATION` | validation rejects every other migration class |
| automatic/latest-wins startup selection | publication authority must be explicit | restart has no directory/network/repository scan or “latest” resolver |
| public live activation endpoint / deployment-controller count | architecture grants no live deployment authority | no CLI/admin/Platform/live route is part of allocation; crate-root gameplay availability remains closed |

## `DEFERRED_REQUIRES_FUTURE_DECISION`

| Potential dimension | What is deliberately not chosen | Why unnecessary for first slice | Trigger that requires a new decision |
|---|---|---|---|
| serialized World Project/source bytes | maximum serialized authoring input | compiler receives typed in-memory graph | first serialized authoring/import format |
| source file count | number of files in a project | no filesystem/project parser | first Studio/file-tree build path |
| source path/file-name bytes | path/name limit | no paths are content input | first file-based project format |
| source tree/nesting depth | parser/tree recursion limit | no recursive source parser | first recursive/file-tree authoring format |
| source collection-entry count not represented by typed definitions | generic collection limit | all admitted collections have named exact semantic bounds | first new generic source collection |
| serialized package bytes | bytes per source package | package is a typed graph identity, not a serialized blob | first serialized package boundary |
| package dependency count | dependency fan-out | exactly one package, no resolver | first multi-package publication |
| package dependency depth | transitive graph depth | no dependency graph | first dependency resolver |
| alias count | aliases/deprecations | aliases absent | first alias/deprecation capability |
| alias-chain depth | recursive alias resolution | no alias resolver | first alias capability |
| world coordinate magnitude / width / height / bounding-box span | dense-world dimension limit | v1 uses a sparse three-cell set; no allocation, scan or index capacity may be derived from coordinate magnitude, and coordinate arithmetic must be checked | first dense world, range scan, extent-derived allocation or spatial streaming design |
| dense objects-per-cell capacity | object-density model | arbitrary authored object collection is excluded | first authored object stack/density capability |
| technical chunk count | bundle chunk fan-out | v1 framing is flat record-based | first chunked runtime bundle |
| raw/decompressed chunk bytes | chunk payload ceiling | no chunks | first chunked bundle |
| chunk side / floor packing | permanent spatial packing geometry | no chunk packing | first chunked/spatial-streaming design |
| chunk density / sparse-dense allocation policy | allocation-sensitive spatial density | only sparse three-cell canary | first world-scale spatial profile |
| permanent serialized spatial-index count/bytes/fan-out | bundle index layout | only compact in-memory indices bounded by Cell/Definition counts | first permanent bundle/spatial index |
| compressed artifact/chunk bytes | compressed-size ceiling | compression is forbidden by v1 | first compression proposal |
| decompressed artifact/chunk bytes | decompressed-size ceiling | compression is forbidden by v1 | first compression proposal |
| decompression ratio | expansion-ratio ceiling | no decompressor exists | first compression proposal |
| embedded asset/blob bytes | asset payload ceiling | v1 carries semantic Presentation refs only | first embedded/client asset packaging |
| asset-pack count/bytes | packaged asset boundary | no asset pack | first asset-pack design |
| compiler/import diagnostic report bytes | report/evidence output ceiling | v1 emits only bounded status/digests | first standardized publication/import report |
| permanent World Bundle total bytes | durable container-scale ceiling | v1 framing is replaceable and has no perpetual compatibility promise | permanent physical-format decision after real import -> canonical world -> Studio/edit -> compile -> server/client evidence |
| permanent World Project / World Bundle format version count/compat window | long-lived physical compatibility policy | first canary needs only exact v1 profile match | first permanent format/versioning commitment |
| patch/delta count/bytes | patch distribution budget | no patching in v1 | first patch/delta proposal |
| CDN object count/bytes | distribution topology budget | no CDN chosen | first CDN/distribution design |
| signing-chain/keyset/signature count/bytes | trust topology resource limits | publication plan supplies exact expected digests; digest is not authority | first signing/trust-root topology decision |

## Completeness invariant

For `FIRST_PRODUCTION_CONTENT_PROFILE/v1` there is no fourth class and no implicit unlimited resource. A future implementation proposal that cannot map an externally controlled count, depth, length, bytes, allocation-sensitive density, retry/work collection, compressed/decompressed size or script budget to one `REQUIRED_NOW` registry row above **must fail review** and reopen DUR-04 architecture before code acceptance.
