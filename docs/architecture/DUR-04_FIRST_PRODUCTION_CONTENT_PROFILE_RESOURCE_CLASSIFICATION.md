# DUR-04 first production content profile — complete resource classification

Normative companion to `DUR-04_FIRST_PRODUCTION_CONTENT_PROFILE.md` for `DUR04-FIRST-PROD-CONTENT-01` / `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.

Every potential resource dimension identified by DUR-04, VSL-CONTENT-01, the current CONTENT seam, the Content Format Spike inventory, or the production publication design has exactly one classification below. `DEFERRED_REQUIRES_FUTURE_DECISION` input still fails closed in v1; deferred means only that the future numeric/design choice is intentionally not selected now. `EXCLUDED_FAIL_CLOSED` has no v1 construction/parse/runtime capability.

A mechanically derived aggregate may reuse the registry rows that bound its factors only where that relation is exact. There is no fourth/unlimited class.

## `REQUIRED_NOW`

| Potential dimension | Registry owner / hard maximum | Unit | Evidence | Fail-closed boundary |
|---|---|---|---|---|
| package count | `DUR04-FPC-PACKAGES=1` | packages | one-package canary, no resolver | second package rejected before graph resolution |
| Region count | `DUR04-FPC-REGIONS=1` | definitions | closed canary | second rejected |
| Area count | `DUR04-FPC-AREAS=1` | definitions | closed canary | second rejected |
| Terrain count | `DUR04-FPC-TERRAINS=1` | definitions | closed canary | second rejected |
| Cell count | `DUR04-FPC-CELLS=3` | cells | start + blocked + relocation target | fourth rejected before index allocation |
| logical cell-index entries | `DUR04-FPC-CELLS=3` | entries | exactly one sparse index entry per Cell | fourth cannot be created |
| distinct floor count | `DUR04-FPC-FLOORS=1` | floors | single-floor canary | second distinct floor rejected |
| Relocation count / relocation candidates | `DUR04-FPC-RELOCATIONS=1` | definitions/candidates | one relocation globally | second rejected |
| Behavior refs | `DUR04-FPC-BEHAVIORS=1` | definitions | closed canary | second rejected |
| Presentation/project asset refs | `DUR04-FPC-PRESENTATIONS=3` | definitions | creature + ability + item | fourth rejected |
| Creature count | `DUR04-FPC-CREATURES=1` | definitions | closed canary | second rejected |
| Spawn count | `DUR04-FPC-SPAWNS=1` | definitions | closed canary | second rejected |
| population per Spawn | `DUR04-FPC-SPAWN-POPULATION=1` | entities | smallest deterministic canary | value 2 rejected before runtime allocation |
| Formula/Ruleset refs | `DUR04-FPC-FORMULA-PROFILES=1` | definitions | one opaque accepted policy ref | second rejected |
| Effect count | `DUR04-FPC-EFFECTS=1` | definitions | closed canary | second rejected |
| Ability count | `DUR04-FPC-ABILITIES=1` | definitions | closed canary | second rejected |
| Item count | `DUR04-FPC-ITEMS=1` | definitions | closed canary | second rejected |
| Loot Table count | `DUR04-FPC-LOOT-TABLES=1` | definitions | closed canary | second rejected |
| Loot Entry count | `DUR04-FPC-LOOT-ENTRIES=1` | entries | one entry in one table | second rejected |
| XP definition count | `DUR04-FPC-XP-DEFINITIONS=1` | definitions | closed canary | second rejected |
| RNG purpose count | `DUR04-FPC-RNG-PURPOSES=1` | keys | one deterministic loot purpose | second rejected |
| aggregate semantic definitions / unique definition keys / definition-index entries | `DUR04-FPC-DEFINITIONS=21` | definitions/keys/entries | exact category sum; one unique key per definition | 22nd rejected before set/index growth; duplicate key rejected |
| aggregate semantic references / reference-index entries | `DUR04-FPC-REFERENCES=24` | references/entries | exact compiler formula | 25th rejected before collection growth |
| ContentKey/PackageKey/WorldId length | `DUR04-FPC-KEY-BYTES=128` | ASCII bytes | independent production semantic-ID precedent | 129 rejected before retain/copy; invalid atom rejected |
| revision/policy/presentation/RNG/capability-set identity length | `DUR04-FPC-FIELD-STRING-BYTES=128` | ASCII bytes | bounded semantic-ID class | 129 rejected before retain/copy |
| Content Lock digest length | `DUR04-FPC-SHA256-HEX-BYTES=64` | lowercase hex bytes | exact SHA-256 representation | 63/65/non-hex rejected |
| manifest field count | `DUR04-FPC-MANIFEST-FIELDS=17` | fields | exact profile/package/revision/lock/capability/projection schema | any non-17 count rejected |
| runtime capability compatibility identity | `DUR04-FPC-FIELD-STRING-BYTES=128` | ASCII bytes | manifest field 15 binds required server runtime capability-set revision | mismatch/oversize rejected before staging |
| protocol capability compatibility identity | `DUR04-FPC-FIELD-STRING-BYTES=128` | ASCII bytes | manifest field 16 binds required protocol capability-set revision | mismatch/oversize rejected before staging |
| server record count | `DUR04-FPC-SERVER-RECORDS=22` | records | 21 definitions + one RNG context | 23 rejected before vector allocation |
| client-safe record count | `DUR04-FPC-CLIENT-RECORDS=6` | records | 3 Presentation + Creature + Ability + Item | 7 rejected before vector allocation |
| maximum valid encoded record payload | `DUR04-FPC-RECORD-BYTES=609` | bytes | valid Spawn: four 128-byte keys + fixed population/enums | 610 rejected before copy/decode; 609 boundary fixture must be semantically valid |
| section count | `DUR04-FPC-SECTIONS=2` | sections | exact manifest + body | missing/extra/unknown rejected |
| maximum valid section bytes | `DUR04-FPC-SECTION-BYTES=6905` | bytes | exact valid-domain server body | 6906 rejected before processing |
| server artifact / compiler server binary output | `DUR04-FPC-SERVER-ARTIFACT-BYTES=9002` | bytes | exact v1 framing + valid server manifest/body maxima | 9003 rejected before parse/allocation |
| client artifact / compiler client binary output | `DUR04-FPC-CLIENT-ARTIFACT-BYTES=3688` | bytes | exact v1 framing + valid client manifest/body maxima | 3689 rejected before parse/allocation |
| server+client pair / aggregate compiler binary output | `DUR04-FPC-ARTIFACT-PAIR-BYTES=12690` | bytes | exact 9002+3688 | 12691 rejected before staging |
| projection count | `DUR04-FPC-PROJECTIONS=2` | projections | exactly server-authoritative + client-safe | missing/third projection rejected |
| decoded heap per staged/resident generation | `DUR04-FPC-STAGING-HEAP-BYTES=32768` | bytes | exact charged worst-case workset 23736 under prescribed compact representation | next charged byte rejects; no uncharged activation copy |
| simultaneous staged candidate pairs | `DUR04-FPC-STAGED-CANDIDATES=1` | candidates | no parallel activation needed | second rejected without replacing state |
| resident generations | `DUR04-FPC-RESIDENT-GENERATIONS=2` | generations | current + one draining predecessor | third activation denied until drain |
| aggregate decoded CONTENT heap across residents + staged candidate | `DUR04-FPC-LIVE-DECODED-CONTENT-BYTES=98304` | bytes | exact `(2+1)*32768` composition | allocation/copy beyond aggregate cap rejected; active unchanged |
| compiler semantic workset | mechanically bounded by `DEFINITIONS=21`, `REFERENCES=24`, `SERVER-RECORDS=22`, `CLIENT-RECORDS=6` | items | no recursive or external retry domain | any new input-controlled pass/collection is a new resource dimension and fails review until registered |
| malformed loader workset | mechanically bounded by `ARTIFACT-PAIR-BYTES=12690`, `SECTIONS=2`, record/field bounds | bytes/items | loader work is bounded over already admitted lengths; no recursive parser | reject lengths/counts before allocation/iteration |
| staging validation workset | mechanically bounded by pair/count rows plus `STAGING-HEAP-BYTES=32768` | bytes/items | one candidate and compact bounded indices | exhaustion rejects staging and preserves active |

### Valid-domain record-size closure

The production serializer does not use `8 * 128` as a record-domain assumption. Exact maxima are: Spawn 609; Cell 566; LootEntry 534; Creature 404; Relocation/Ability 392; XP 274; Effect 270; Item 268; Behavior/Presentation 262; one-semantic-field records 132. Numeric and enum fields are bounded by their actual valid Rust/literal domains. Therefore the 609/6905/9002/3688/12690 acceptance boundaries are constructible with valid content.

### Memory closure

Decoded semantic data is at most 11,976 bytes; 117 compact field descriptors, 28 record descriptors, 21 definition-index entries, 24 reference entries, <=2,048 artifact/revision descriptors and <=8,192 validation scratch produce a charged upper workset of 23,736 bytes. The remaining 9,032 bytes inside 32,768 cover alignment/fixed allocator bookkeeping. Activation reuses the immutable staged allocation. Two residents are <=65,536 bytes and residents plus one staged candidate are <=98,304 bytes; a second decoded activation copy is forbidden.

## `EXCLUDED_FAIL_CLOSED`

| Potential dimension/capability | Why unreachable in v1 | Rejection/non-activation proof |
|---|---|---|
| authored arbitrary objects-per-cell count | Cell has no object collection | compiler has no field; loader rejects unknown field/record |
| script component/module bytes/count | no script definition/section | compiler/loader reject |
| script instance count | no script linker/runtime | no construction API |
| Wasm memory/pages/tables/elements | no Wasm runtime | no accepted field/API |
| script fuel | no script execution | no accepted field/API |
| script host-call count | no host ABI | no accepted field/API |
| script query/result collection count/bytes | no script query import | no accepted field/API |
| script action-plan count/bytes | no action-plan API | no accepted field/API |
| persistent extension-state bytes | no extension-state capability | schema/compiler reject |
| `script_execution_profile_revision` | scripts absent | field not in exact 17-field manifest; unknown field rejected |
| WIT world/interface requirement | no script/WIT consumer | no field/capability; first script/WIT design reopens architecture |
| NPC/quest/house/market definitions/state | outside first slice | no accepted production kind/state surface |
| arbitrary broad catalog records | closed semantic profile | extra/unknown category rejected |
| ambient filesystem/network/process/environment access | content is data | no API resolves content as ambient authority |
| network artifact/dependency fetch | exact bytes supplied by caller; no dependency resolver | no network resolver |
| evidence/fixture profile as production | distinct trust profile | production compile/load rejects evidence/fixture/synthetic authority markers |
| unknown record/field/flag/section | exact closed v1 syntax | reject before staging |
| migration other than `COMPATIBLE_NO_MIGRATION` | no durable reinterpretation | activation validation rejects |
| automatic/latest startup selection | publication requires exact authorized plan/floor | no scan/latest resolver |
| public live activation/deployment-controller surface | no live deployment authority | no CLI/admin/Platform/live route; gameplay bootstrap remains unavailable |

## `DEFERRED_REQUIRES_FUTURE_DECISION`

| Potential dimension | What remains unchosen | Why unnecessary now | Trigger for new decision |
|---|---|---|---|
| serialized World Project/source bytes | authoring input byte ceiling | typed in-memory graph | first serialized authoring/import boundary |
| source file count | project file count | no filesystem parser | first Studio/file-tree path |
| source path/name bytes | path/name limit | paths are not content input | first file-based project format |
| source tree/nesting depth | parser recursion/tree depth | no recursive source parser | first recursive/file-tree format |
| generic source collection entries | generic collection cap | all v1 collections have named exact bounds | first new generic source collection |
| serialized package bytes | source package blob size | no serialized package | first source package format |
| package dependency count/depth | dependency graph fan-out/depth | one package, no resolver | first multi-package publication |
| aliases/count/chain depth | alias/deprecation graph | aliases absent | first alias capability |
| world coordinate magnitude / width / height / bounding span | dense-world extent budget | sparse three-cell set; no allocation/scan derives from magnitude; arithmetic checked | first dense extent/range/streaming design |
| dense objects-per-cell capacity | object stack/density model | arbitrary object collection excluded | first object-stack capability |
| chunk count/raw bytes/side/floor packing/density | permanent chunk model | flat replaceable canary framing | first chunked/broad-world bundle |
| permanent spatial-index count/bytes/fan-out | bundle index layout | only compact in-memory count-bounded indices | first permanent bundle/spatial index |
| compressed bytes/decompressed bytes/ratio | compression limits | compression forbidden | first compression proposal |
| embedded asset/blob bytes and asset-pack count/bytes | packaged asset boundary | semantic Presentation refs only | first embedded/client asset packaging |
| compiler/import diagnostic report bytes | report ceiling | bounded status/digests only | first standardized report |
| direct runtime capability-list entries/bytes | membership serialization | v1 binds only opaque capability-set revision | first artifact that enumerates runtime capabilities |
| direct protocol capability-list entries/bytes | membership serialization | same | first artifact that enumerates protocol capabilities |
| permanent World Bundle bytes/version compatibility window | durable physical contract | v1 is replaceable | permanent format after real import -> canonical world -> Studio/edit -> compile -> server/client evidence |
| patch/delta count/bytes | patch distribution budget | no patching | first patch/delta design |
| CDN object count/bytes | distribution budget | no CDN | first CDN design |
| signing chain/keyset/signature count/bytes | trust topology | exact expected digests are external; signing unselected | first signing/trust-root decision |

## Completeness invariant

There is no implicit unlimited v1 resource. Any future implementation that introduces an externally controlled count, depth, length, byte size, allocation-sensitive density, retry/work collection, compressed/decompressed size, capability-membership collection or script budget that cannot map to one `REQUIRED_NOW` row above **fails review and reopens DUR-04 before implementation acceptance**.
