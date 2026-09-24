# First Production Content Profile decision

- Date: 2026-09-09
- Tracking: Issue #433, unblocks a fresh bounded continuation of CONTENT #54 only after the separately serialized resource-limit update is protected on `main`.
- Protected base used for this decision: `main@10ce3393a51dac14105b831040e1f4faa3ca565f`.
- Decision ID: `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.
- Status on this branch: architecture candidate pending independent exact-head architecture/security review, canonical CI/governance, FULL Merge Queue, and protected-main readback.
- Architecture implementation authority after protected acceptance: **bounded to the exact first-production compiler/loader/staging/runtime seam described here**.
- Repository integration authority: **normal reviewed PR + required checks + FULL Merge Queue only**.
- Live deployment / production activation authority: **NONE**. This decision does not authorize a release, production environment mutation, traffic, credential use, publisher trust, CDN publication, or a live content switch.

## 1. Decision test

**Must decide now? YES.** CONTENT #54 cannot truthfully move from the merged non-production evidence seam to a production-capable first slice while DUR-04 has no accepted production limit profile or complete activation/recovery boundary.

**Concrete downstream work blocked:** the exact serialized `RESOURCE_LIMITS_REGISTRY.json` update and, only after its protected readback, a fresh bounded allocation for CONTENT #54.

**What becomes harder if chosen incorrectly:** a test/evidence profile could accidentally become production, a temporary carrier could become a permanent World Bundle compatibility format, loader allocations could become unbounded, or an in-process content API could accidentally acquire deployment authority.

**Evidence that can supersede this decision:** a reviewed real import -> canonical world -> Studio/edit -> compile -> server/client load/render E2E format decision; representative broad-content sizing; a security finding; a future scripting decision; or a future rollout/migration contract requiring capabilities explicitly excluded here.

**Deliberately not decided:** permanent World Project/World Bundle encoding, file extension, compression, chunking, CDN, signing topology, Studio source format, broad legacy import, script runtime, hot rollout with live scopes, migration-bearing content changes, and broad-world production capacity.

## 2. Verified evidence baseline

This decision consumes the following protected facts without promoting historical evidence to authority:

- DUR-04 requires bounded compiler/loader staging, exact revision binding, immutable artifacts, fail-closed unknown critical input, separated loading/activation, and registry hard maxima for every applicable externally controlled resource. It deliberately selected no numeric production ceilings and grants no live activation authority.
- VSL-CONTENT-01 requires only the minimum movement/combat semantic set and explicitly excludes scripts, broad content, Studio, legacy-import breadth, and a permanent physical format.
- The current protected `apps/game-server/src/content/**` is still the repaired non-production evidence seam: ordinary release compilation is rejected and runtime activation types are test-only.
- The current `RESOURCE_LIMITS_REGISTRY.json` contains no DUR-04 first-production rows.
- The terminal Content Format Spike #95/#125 measured synthetic 32x32, 64x64 and 128x128 worlds across all three candidate representations, enforced a 512-byte string fence with negative oversized-string evidence, and retained `SPIKE_RESULT != OWNER_FORMAT_DECISION`.
- The existing `evidence:test-v1` limits are test-only and are **not** used as production values by this decision.

Two spike values are reused, with explicit bounded rationale:

1. **1,024 cells** — the smallest terminally measured 32x32 world footprint. It is adopted only as the aggregate cell ceiling for this first production slice. It is not a chunk-size, floor-packing, broad-world or capacity claim.
2. **512 bytes per semantic text atom** — the terminal spike already proved deterministic rejection above this fence. The first profile carries identifiers/revisions/metadata tokens only; it excludes prose blobs, source documents, scripts and raw assets. Therefore 512 bytes is a conservative security ceiling, not a content-capacity estimate.

Every other `REQUIRED_NOW` maximum below is a direct consequence of the exact profile shape plus checked arithmetic; none is copied from `evidence:test-v1` or from the spike's 64 MiB artifact fence.

## 3. Exact first-production scope

`FIRST_PRODUCTION_CONTENT_PROFILE/v1` is the smallest production-capable semantic slice that exercises the accepted movement/combat content boundary without selecting a permanent physical format.

### 3.1 Compiler/build path — IN

The first-production compiler accepts exactly one **typed in-memory production source graph**. It does not parse a production source file format.

The graph contains:

- exactly one package and one `WorldId`;
- exactly one technical Region-like footprint;
- one Area and one Terrain definition;
- one floor, with an x-span of at most 32 cell positions, a y-span of at most 32 cell positions, and at most 1,024 explicit cells total;
- exactly one same-scope relocation;
- exactly one behavior definition;
- exactly three presentation metadata definitions: creature, item and ability;
- exactly one creature, spawn, product formula profile, effect, ability, item, loot table, loot entry, XP definition and RNG purpose;
- one RNG-profile/context metadata record with **no seed, secret, entropy source or mutable RNG state in content**;
- exact package/content/map/ruleset/world-policy/compiler/canonicalization/content-lock/provenance/SIM/profile revisions;
- required GAME-CHANNEL multiplicity/eligibility and spawn-recovery classification;
- product-release formula/policy references only. Fixture/evidence/test-only formula or profile markers are rejected.

The compiler must canonicalize enumeration order, validate the exact cardinality/resource profile, reject duplicate/unresolved/incompatible definitions, and emit exactly one server-authoritative artifact plus one client-safe artifact.

### 3.2 Temporary bootstrap artifact carrier — IN, non-permanent

The first production slice may use a versioned internal bootstrap artifact profile sufficient to exercise actual ordinary-release bytes and loader safety. It is **not** the permanent `.owb`, World Bundle or World Project format and creates no compatibility promise beyond `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.

The carrier has exactly two critical sections: manifest and body. The manifest has exactly 17 bounded fields:

1. artifact profile ID;
2. package key;
3. package revision;
4. world ID;
5. content revision;
6. map revision;
7. ruleset revision;
8. world-policy revision;
9. compiler revision;
10. canonicalization revision;
11. Content Lock revision/digest token;
12. provenance revision/digest token;
13. SIM profile revision;
14. first-production content profile revision;
15. projection class;
16. durable migration class;
17. required capability profile.

The only accepted migration class is `COMPATIBLE_NO_MIGRATION`. The required capability profile is the closed first-profile capability set; unknown/additional critical capabilities fail closed.

The current evidence carrier's stable semantic record families may be reused behind a **different production profile ID/version** only where they implement this decision. `VSL_BUNDLE_EVIDENCE_PROFILE`, `evidence:*`, synthetic fixture markers and test-only activation are never renamed into production.

### 3.3 Loader/staging/runtime path — IN

The loader:

- receives exactly one server artifact and one client-safe artifact as an explicit pair;
- checks projection-specific total byte maxima before peer/input-sized allocation;
- validates profile/header/counts/checked offsets/integrity/manifest/revisions/capabilities/semantic references before constructing staged runtime state;
- rejects evidence/test/synthetic profiles, malformed/corrupt/oversize/unknown-critical input, wrong projection, pair mismatch and stale/incompatible requested generation;
- constructs at most one staged candidate while retaining at most one current active generation;
- never fetches source packages or artifacts from the network;
- never selects `latest`, scans a directory for a preferred revision, or infers deployment authority from a valid digest.

Runtime consumers can read only the exact immutable active generation and its exact revision/digest set.

### 3.4 OUT of the first profile

The following are not silently optional; they are unavailable unless a future accepted decision explicitly adds them:

- serialized production World Project/source parser;
- multiple packages, package dependencies, aliases or alias nesting;
- multiple floors or multiple technical Regions;
- chunk tables, chunk streaming, dense cell/object packing or artifact-controlled spatial candidate indexes;
- compression/decompression;
- raw asset/blob payloads;
- scripts/Wasm/WIT/component loading or execution;
- NPC/quest/shop/event/house/market/broad monster/item/spell content;
- legacy import or migration tooling;
- durable content migration classes other than `COMPATIBLE_NO_MIGRATION`;
- gameplay-relevant hot reload while any authoritative scope is live;
- simultaneous authoritative generations;
- CDN, signing, release trust topology or publisher authenticity decisions;
- live deployment/activation.

## 4. Mechanical maxima derivation

Let `C = 1,024` cells.

The exact non-cell definition count for this profile is 18:

`1 region + 1 area + 1 terrain + 1 relocation + 1 behavior + 3 presentations + 1 creature + 1 spawn + 1 formula + 1 effect + 1 ability + 1 item + 1 loot table + 1 loot entry + 1 XP definition + 1 RNG purpose = 18`.

Therefore:

- maximum unique canonical content definitions / `ContentKey`s = `C + 18 = 1,042`;
- maximum server body records = `1,042 definitions + 1 RNG context = 1,043`;
- maximum client body records = `3 presentations + 1 creature + 1 ability + 1 item = 6`;
- maximum references, using the current semantic graph's exact reference formula, = `3*C + 2 relocation + 2 creature + 3 spawn + 1 effect + 2 ability + 1 item + 3 loot-entry + 1 XP = 3,087`.

A semantic atom is at most 512 bytes. A record has at most eight fields. With a two-byte record header and a two-byte length prefix per field:

`max_record_bytes = 2 + 8 * (2 + 512) = 4,114`.

The production-bootstrap manifest has 17 strings:

`max_manifest_section_bytes = 17 * (2 + 512) = 8,738`.

The server body upper bound is intentionally calculated as if **every** server record used the eight-field worst case:

`max_server_body_bytes = 4 + 1,043 * (4 + 4,114) = 4,295,078`.

With the existing bounded carrier framing shape of 24-byte header, two 48-byte section entries and 32-byte artifact digest trailer, fixed framing is 152 bytes. Therefore:

- `max_server_artifact_bytes = 152 + 8,738 + 4,295,078 = 4,303,968`;
- `max_client_body_bytes = 4 + 6 * (4 + 4,114) = 24,712`;
- `max_client_artifact_bytes = 152 + 8,738 + 24,712 = 33,602`;
- `max_generation_pair_bytes = 4,303,968 + 33,602 = 4,337,570`.

For a conservative decoder-work bound, treating every record as eight fields:

`max_decoded_field_instances = 2 * 17 manifest fields + 1,043 * 8 server fields + 6 * 8 client fields = 8,426`.

These are hard security/implementation bounds for this first profile. They are not throughput, player-count, RSS, latency, broad-world or permanent-format capacity claims.

## 5. Complete resource-dimension classification

Every DUR-04/VSL-CONTENT-01 dimension applicable or potentially applicable to the first slice is classified exactly once below.

### 5.1 `REQUIRED_NOW`

| Resource dimension | Unit | Hard maximum | Evidence / derivation | Mandatory boundary behavior |
|---|---:|---:|---|---|
| packages in typed production graph | packages | 1 | first-profile shape | second package rejected before merge/resolution |
| worlds in one generation | worlds | 1 | first-profile shape | second WorldId rejected |
| unique ContentKeys/definitions | definitions | 1,042 | 1,024 cells + 18 exact non-cell definitions | 1,043 rejected before sort/lowering |
| semantic references | references | 3,087 | exact current graph reference formula at profile maxima | 3,088 rejected before resolution growth |
| x-axis footprint span | cell positions | 32 | smallest terminal spike footprint side | span 33 rejected; no dense allocation |
| y-axis footprint span | cell positions | 32 | smallest terminal spike footprint side | span 33 rejected; no dense allocation |
| distinct floors | floors | 1 | minimum VSL slice; multi-floor not needed | second distinct z plane rejected |
| cells | cells | 1,024 | smallest terminally measured 32x32 spike footprint | 1,025 rejected before canonicalization/lowering |
| Region definitions | definitions | 1 | first-profile shape | second Region rejected |
| Area definitions | definitions | 1 | first-profile shape | second Area rejected |
| Terrain definitions | definitions | 1 | first-profile shape | second Terrain rejected |
| relocation definitions | definitions | 1 | minimum VSL movement requirement | second relocation rejected |
| behavior definitions | definitions | 1 | minimum VSL AI requirement | second behavior rejected |
| presentation definitions | definitions | 3 | one each for creature/item/ability | fourth rejected |
| creature definitions | definitions | 1 | minimum VSL combat requirement | second creature rejected |
| spawn definitions | definitions | 1 | minimum VSL source requirement | second spawn rejected |
| product formula profiles | definitions | 1 | one product-release formula reference | second or fixture-only profile rejected |
| effect definitions | definitions | 1 | minimum VSL combat requirement | second effect rejected |
| ability definitions | definitions | 1 | minimum VSL combat requirement | second ability rejected |
| item definitions | definitions | 1 | minimum VSL durable-item requirement | second item rejected |
| loot tables | definitions | 1 | minimum VSL loot requirement | second table rejected |
| loot entries | definitions | 1 | minimum VSL loot requirement | second entry rejected |
| XP definitions | definitions | 1 | minimum VSL progression requirement | second XP definition rejected |
| RNG purpose keys | definitions | 1 | minimum deterministic loot purpose | second purpose rejected |
| artifact sections per projection | sections | 2 | bootstrap carrier design: manifest + body | section count other than 2 rejected |
| manifest fields per artifact | fields | 17 | exact first-production manifest | missing/extra field rejected as incompatible profile |
| server body records | records | 1,043 | 1,042 definitions + 1 RNG context | 1,044 rejected before record allocation |
| client body records | records | 6 | exact allowlisted client projection | seventh record rejected; non-allowlisted type rejected |
| one encoded record | bytes | 4,114 | 8-field worst case at 512-byte strings | 4,115 rejected before record allocation |
| one section | bytes | 4,295,078 | worst-case server body | +1 rejected before retaining section |
| one semantic key | ASCII bytes | 512 | terminal spike string fence, restricted identifier-only profile | 513 rejected before retaining key |
| one revision/metadata atom | ASCII bytes | 512 | terminal spike string fence, no prose/blob source | 513 rejected before retaining string |
| server projection artifact | bytes | 4,303,968 | exact arithmetic above | +1 rejected before parse/allocation |
| client-safe projection artifact | bytes | 33,602 | exact arithmetic above | +1 rejected before parse/allocation |
| server+client generation pair | bytes | 4,337,570 | sum of projection maxima | cumulative +1 rejected before staging both |
| artifacts per generation | artifacts | 2 | exactly server + client-safe | missing/third artifact rejected |
| concurrently staged candidate generations | generations | 1 | no batch/hot rollout required | second staged candidate returns `CONFLICT`/capacity denial |
| resident active+staged generations | generations | 2 | one active + one non-authoritative candidate | third generation cannot be retained |
| decoded field instances per pair | fields | 8,426 | 2 manifests + worst-case record fields | 8,427th field cannot be materialized |

All overflow/maximum violations fail closed with the repository's capacity/error vocabulary before untrusted-size allocation or authoritative publication. Semantic incompleteness or wrong exact cardinality below a maximum fails as invalid/incompatible content rather than being padded or inferred.

### 5.2 `EXCLUDED_FAIL_CLOSED`

| Capability/resource | Why unreachable in v1 | Mandatory rejection boundary |
|---|---|---|
| package dependencies | one-package profile has no dependency graph | typed source validator requires dependency count zero; artifact capability/profile cannot declare dependencies |
| aliases / alias nesting | not needed by the minimum slice | source/compiler reject any alias declaration; loader has no alias record kind |
| per-cell object collections / dense object packing | first profile carries explicit cells plus semantic definitions only | unknown object-collection record/section is critical and rejected |
| raw asset/blob payloads | client projection carries metadata tokens only | compiler rejects blob-bearing fields; loader rejects unknown blob section/kind |
| serialized source input to the production compiler | permanent World Project source format is deliberately unselected | ordinary production API accepts only the typed graph; no file/parser entry point exists |
| script components, WIT imports, Wasm instances or script persistent state | scripts are not required for first movement/combat content | compiler rejects script references/capability profile; loader rejects script sections/required capabilities; runtime exposes no script executor |
| durable migration classes other than `COMPATIBLE_NO_MIGRATION` | first slice must not need schema/value migration | compile/stage rejects any other class before activation eligibility |
| live-scope hot reload | VSL explicitly does not require gameplay hot reload | activation requires the GameNode/scope set to be drained/quiescent; live scope causes `CONFLICT` |
| more than one authoritative content generation | first profile avoids mixed-generation gameplay | runtime exposes one active pointer only; no second active generation can be published |
| batch/corpus staging API | malformed corpus is test evidence, not production input | one call accepts exactly one pair; no list/directory/batch API |
| network-time source/artifact fetch or `latest` resolution | DUR-04 forbids unresolved runtime fetch and floating latest | content module has no network resolver or latest selector; absence is fail-closed |
| legacy import | not required to prove first production seam | no importer path in allocation; legacy-shaped critical input rejected |

None of these can be accidentally enabled by configuration, an artifact flag, a source token or successful evidence tests. Adding any of them requires a new accepted profile decision and a new bounded allocation.

### 5.3 `DEFERRED_REQUIRES_FUTURE_DECISION`

These dimensions receive **no numeric production value from this decision** because the first profile has no implementation path that consumes them. Their future trigger is explicit.

| Deferred dimension | Why not needed now | Future trigger requiring a decision |
|---|---|---|
| permanent World Project source bytes, source file count and source nesting depth | first production compiler accepts a typed graph only | owner-accepted permanent source representation or production source parser allocation |
| permanent package dependency count/depth | v1 is one package with dependencies excluded | first multi-package production content requirement |
| permanent World Bundle chunk count/raw-chunk bytes | bootstrap carrier has no chunks | accepted final/next bundle representation introducing chunking |
| compressed bytes, decompressed bytes and decompression ratio | v1 carrier is uncompressed | any accepted compression-enabled artifact profile |
| permanent spatial index count/candidate density | v1 uses direct keyed explicit cells and references | artifact-controlled spatial index or streaming/locality design |
| broad-world floor/Region/cell maxima beyond this profile | first slice is deliberately capped at one floor/Region and 1,024 cells | broad production world import or representative full-world sizing |
| raw asset/package blob bytes | v1 carries presentation metadata only | accepted asset packaging/distribution contract |
| compiler source-to-bundle report bytes / Studio save report | no production source parser/Studio path exists | production authoring/Studio compile pipeline allocation |
| script component bytes, component count, instance count, memory/table limits, fuel, host calls, query/result/action-plan counts/bytes and extension-state bytes | script runtime is excluded fail-closed in v1 | explicit DUR-04 script/WIT/engine implementation decision |
| CDN patch/download/signature envelope sizing | this decision grants no distribution/signing topology | accepted release/distribution/signing architecture |

Until the named trigger is accepted, input that would require one of these deferred dimensions cannot enter this profile and therefore fails closed rather than using an implicit/unlimited value.

## 6. Exact `RESOURCE_LIMITS_REGISTRY.json` serialized append

After this decision is protected on `main`, the registry update is a **separate serial PR**. It must preserve all existing entries and append exactly the objects below, in this order. Its `updated_at` is set to the actual UTC timestamp of that registry mutation; no other existing top-level field or existing entry changes.

```json
[
  {"id":"DUR04-FIRST-PROD-PACKAGES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Packages in FIRST_PRODUCTION_CONTENT_PROFILE/v1 typed graph","unit":"packages","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject a second package before dependency/key merge or canonicalization.","client_visible":false,"boundary_tests":["1 package accepted","2 packages rejected before merge/canonicalization"]},
  {"id":"DUR04-FIRST-PROD-WORLDS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"World contexts in one first-production generation","unit":"worlds","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject a second WorldId before graph lowering.","client_visible":false,"boundary_tests":["1 WorldId accepted","second WorldId rejected"]},
  {"id":"DUR04-FIRST-PROD-CONTENT-KEYS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Unique canonical ContentKey definitions","unit":"definitions","hard_maximum":1042,"configurable_range":{"minimum":21,"maximum":1042},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Count with checked arithmetic before sort/set construction and lowering.","client_visible":false,"boundary_tests":["1,042 accepted when all family limits pass","1,043 rejected before canonicalization growth","checked-sum overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-REFERENCES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Semantic references in one first-production graph/artifact","unit":"references","hard_maximum":3087,"configurable_range":{"minimum":24,"maximum":3087},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Count with checked arithmetic before reference-vector/set materialization.","client_visible":false,"boundary_tests":["3,087 accepted when semantically valid","3,088 rejected before resolution growth","reference-count overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-X-SPAN","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Inclusive x-axis span of explicit first-profile cells","unit":"cell positions","hard_maximum":32,"configurable_range":{"minimum":1,"maximum":32},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Validate min/max coordinates using checked span arithmetic; do not allocate a dense grid from coordinates.","client_visible":false,"boundary_tests":["span 32 accepted","span 33 rejected","coordinate span arithmetic overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-Y-SPAN","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Inclusive y-axis span of explicit first-profile cells","unit":"cell positions","hard_maximum":32,"configurable_range":{"minimum":1,"maximum":32},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Validate min/max coordinates using checked span arithmetic; do not allocate a dense grid from coordinates.","client_visible":false,"boundary_tests":["span 32 accepted","span 33 rejected","coordinate span arithmetic overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-FLOORS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Distinct z/floor planes in one first-production generation","unit":"floors","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject a second distinct floor before spatial lowering.","client_visible":false,"boundary_tests":["one floor accepted","second distinct floor rejected"]},
  {"id":"DUR04-FIRST-PROD-CELLS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Explicit cells in one first-production generation","unit":"cells","hard_maximum":1024,"configurable_range":{"minimum":3,"maximum":1024},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Check cell count before clone/sort/lowering or staged record allocation.","client_visible":false,"boundary_tests":["1,024 cells accepted when spans and semantics pass","1,025 rejected before canonicalization/lowering"]},
  {"id":"DUR04-FIRST-PROD-REGIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Region definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Enforce exact first-profile cardinality before lowering.","client_visible":false,"boundary_tests":["1 Region accepted","2 Regions rejected"]},
  {"id":"DUR04-FIRST-PROD-AREAS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Area definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Enforce exact first-profile cardinality before lowering.","client_visible":false,"boundary_tests":["1 Area accepted","2 Areas rejected"]},
  {"id":"DUR04-FIRST-PROD-TERRAINS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Terrain definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Enforce exact first-profile cardinality before lowering.","client_visible":false,"boundary_tests":["1 Terrain accepted","2 Terrains rejected"]},
  {"id":"DUR04-FIRST-PROD-RELOCATIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Relocation definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess relocation definitions before reference collection.","client_visible":false,"boundary_tests":["1 relocation accepted","2 relocations rejected"]},
  {"id":"DUR04-FIRST-PROD-BEHAVIORS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Behavior definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess behavior definitions before canonicalization.","client_visible":false,"boundary_tests":["1 behavior accepted","2 behaviors rejected"]},
  {"id":"DUR04-FIRST-PROD-PRESENTATIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Presentation metadata definitions","unit":"definitions","hard_maximum":3,"configurable_range":{"minimum":3,"maximum":3},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Allow exactly creature/item/ability presentation metadata; no raw asset payload allocation.","client_visible":true,"boundary_tests":["3 allowlisted presentation definitions accepted","4th presentation rejected"]},
  {"id":"DUR04-FIRST-PROD-CREATURES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Creature definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess creature definitions before lowering.","client_visible":false,"boundary_tests":["1 creature accepted","2 creatures rejected"]},
  {"id":"DUR04-FIRST-PROD-SPAWNS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Spawn definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess spawns before source/reference validation.","client_visible":false,"boundary_tests":["1 classified spawn accepted","2 spawns rejected"]},
  {"id":"DUR04-FIRST-PROD-FORMULA-PROFILES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Product formula profile definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess profiles; fixture/evidence/test-only profile markers remain invalid independently of count.","client_visible":false,"boundary_tests":["1 product-release formula profile accepted","2 profiles rejected","fixture-only profile rejected"]},
  {"id":"DUR04-FIRST-PROD-EFFECTS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Effect definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess effects before lowering.","client_visible":false,"boundary_tests":["1 effect accepted","2 effects rejected"]},
  {"id":"DUR04-FIRST-PROD-ABILITIES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Ability definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess abilities before lowering/client projection.","client_visible":true,"boundary_tests":["1 ability accepted","2 abilities rejected"]},
  {"id":"DUR04-FIRST-PROD-ITEMS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Item definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess item definitions before lowering/client projection.","client_visible":true,"boundary_tests":["1 item accepted","2 items rejected"]},
  {"id":"DUR04-FIRST-PROD-LOOT-TABLES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Loot table definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess loot tables before lowering.","client_visible":false,"boundary_tests":["1 loot table accepted","2 loot tables rejected"]},
  {"id":"DUR04-FIRST-PROD-LOOT-ENTRIES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Loot entries","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject a second loot entry before reference/RNG validation.","client_visible":false,"boundary_tests":["1 loot entry accepted","2 loot entries rejected"]},
  {"id":"DUR04-FIRST-PROD-XP-DEFINITIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"XP definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess XP definitions before lowering.","client_visible":false,"boundary_tests":["1 XP definition accepted","2 XP definitions rejected"]},
  {"id":"DUR04-FIRST-PROD-RNG-PURPOSES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Content-declared deterministic RNG purpose keys","unit":"definitions","hard_maximum":1,"configurable_range":{"minimum":1,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject excess purpose keys before canonicalization; content carries no seed/entropy state.","client_visible":false,"boundary_tests":["1 RNG purpose accepted","2 purposes rejected"]},
  {"id":"DUR04-FIRST-PROD-SECTIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Critical sections per bootstrap projection artifact","unit":"sections","hard_maximum":2,"configurable_range":{"minimum":2,"maximum":2},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Validate section count before section-table allocation; profile requires exactly manifest and body.","client_visible":false,"boundary_tests":["2 known critical sections accepted","0/1/3 sections rejected","unknown critical section rejected"]},
  {"id":"DUR04-FIRST-PROD-MANIFEST-FIELDS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Manifest fields per first-production artifact","unit":"fields","hard_maximum":17,"configurable_range":{"minimum":17,"maximum":17},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Require exactly the versioned 17-field manifest before retaining metadata.","client_visible":false,"boundary_tests":["17 profile-defined fields accepted","16 or 18 rejected as incompatible profile"]},
  {"id":"DUR04-FIRST-PROD-SERVER-RECORDS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Server-authoritative body records","unit":"records","hard_maximum":1043,"configurable_range":{"minimum":22,"maximum":1043},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Check count before record-vector allocation and parsing.","client_visible":false,"boundary_tests":["1,043 records accepted when semantic-family limits pass","1,044 rejected before vector allocation"]},
  {"id":"DUR04-FIRST-PROD-CLIENT-RECORDS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Client-safe body records","unit":"records","hard_maximum":6,"configurable_range":{"minimum":6,"maximum":6},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Check count before record-vector allocation; only allowlisted client record kinds are valid.","client_visible":true,"boundary_tests":["6 allowlisted records accepted","7 rejected before allocation","server-only record kind rejected independently"]},
  {"id":"DUR04-FIRST-PROD-RECORD-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One encoded bootstrap body record","unit":"bytes","hard_maximum":4114,"configurable_range":{"minimum":1,"maximum":4114},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject declared record length before slicing/copying/field decode.","client_visible":false,"boundary_tests":["4,114-byte record accepted when semantically valid","4,115 rejected before record allocation"]},
  {"id":"DUR04-FIRST-PROD-SECTION-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One bootstrap artifact section","unit":"bytes","hard_maximum":4295078,"configurable_range":{"minimum":1,"maximum":4295078},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Validate section length and checked range before retaining or parsing section bytes.","client_visible":false,"boundary_tests":["4,295,078-byte section accepted only when total artifact also passes","4,295,079 rejected","offset+length overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-KEY-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One namespaced semantic key","unit":"ASCII bytes","hard_maximum":512,"configurable_range":{"minimum":1,"maximum":512},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject length before retaining/copying key; syntax validation follows.","client_visible":false,"boundary_tests":["512-byte valid key accepted","513 bytes rejected before retention"]},
  {"id":"DUR04-FIRST-PROD-STRING-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One revision/profile/presentation metadata atom","unit":"ASCII bytes","hard_maximum":512,"configurable_range":{"minimum":1,"maximum":512},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Reject length before retaining/copying; raw blobs and prose source are outside this profile.","client_visible":true,"boundary_tests":["512-byte valid metadata atom accepted","513 bytes rejected before retention","invalid/non-profile text syntax rejected"]},
  {"id":"DUR04-FIRST-PROD-SERVER-ARTIFACT-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One server-authoritative bootstrap artifact","unit":"bytes","hard_maximum":4303968,"configurable_range":{"minimum":1,"maximum":4303968},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Check server slice length before header parse or input-sized allocation.","client_visible":false,"boundary_tests":["4,303,968 bytes accepted only when nested limits pass","4,303,969 rejected before parse/allocation"]},
  {"id":"DUR04-FIRST-PROD-CLIENT-ARTIFACT-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"One client-safe bootstrap artifact","unit":"bytes","hard_maximum":33602,"configurable_range":{"minimum":1,"maximum":33602},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Check client slice length before header parse or input-sized allocation.","client_visible":true,"boundary_tests":["33,602 bytes accepted only when nested/client-allowlist limits pass","33,603 rejected before parse/allocation"]},
  {"id":"DUR04-FIRST-PROD-GENERATION-PAIR-BYTES","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Combined server and client-safe artifact bytes in one staging request","unit":"bytes","hard_maximum":4337570,"configurable_range":{"minimum":2,"maximum":4337570},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Use checked addition of the two already projection-bounded inputs before staging either as a generation pair.","client_visible":false,"boundary_tests":["4,337,570 cumulative bytes accepted only when both projection maxima pass","4,337,571 rejected","pair-byte checked-add overflow rejected"]},
  {"id":"DUR04-FIRST-PROD-ARTIFACTS-PER-GENERATION","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Artifacts in one first-production generation","unit":"artifacts","hard_maximum":2,"configurable_range":{"minimum":2,"maximum":2},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Require exactly one server and one client-safe artifact; no third projection/blob artifact can be retained.","client_visible":false,"boundary_tests":["exact server+client pair accepted","missing projection rejected","third artifact rejected"]},
  {"id":"DUR04-FIRST-PROD-CONCURRENT-STAGED-GENERATIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Concurrent non-authoritative staged candidate generations","unit":"generations","hard_maximum":1,"configurable_range":{"minimum":0,"maximum":1},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Serialize staging; reject a second candidate before retaining its pair/runtime indexes.","client_visible":false,"boundary_tests":["one staged candidate accepted","second concurrent candidate rejected without replacing current active generation"]},
  {"id":"DUR04-FIRST-PROD-RESIDENT-GENERATIONS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Resident content generations in the content catalog, active plus staged","unit":"generations","hard_maximum":2,"configurable_range":{"minimum":0,"maximum":2},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"At most one active and one staged generation may retain artifact/runtime state; no third generation is cached.","client_visible":false,"boundary_tests":["active+staged=2 accepted","third resident generation rejected","failed candidate is discarded without changing active"]},
  {"id":"DUR04-FIRST-PROD-DECODED-FIELDS","owner_contract":"OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md","resource":"Decoded manifest/body field instances processed for one staged server+client pair","unit":"fields","hard_maximum":8426,"configurable_range":{"minimum":1,"maximum":8426},"failure_category":"CAPACITY_EXCEEDED","allocation_impact":"Track cumulative decoded fields with checked arithmetic; nested record/profile checks occur before each retained field.","client_visible":false,"boundary_tests":["8,426 fields accepted only when all nested limits pass","8,427th field cannot be materialized","cumulative counter overflow rejected"]}
]
```

The registry PR must also add contract tests that prove every appended object has the required registry fields, unique ID, finite positive hard maximum, unambiguous unit, and named boundary tests. It must not change any unrelated existing limit.

## 7. Activation, publication and commit semantics

The first profile uses this exact state machine:

```text
received/built
    -> validated
    -> staged
    -> active
```

### `received/built`

Bytes or typed compiler output exist, but have no runtime authority. A successful compile or test is not a deployment approval.

### `validated`

Both artifacts independently pass profile-specific byte/count checks, integrity, exact manifest/revision/capability checks and semantic validation; then the pair passes exact revision identity and expected-generation binding. Evidence/test/synthetic profiles are rejected.

### `staged`

One immutable `StagedGeneration` is complete off to the side. It may contain bounded derived runtime indexes. It is non-authoritative and cannot be observed by gameplay consumers.

### `active`

Activation is permitted only at a **quiescent boundary**: no authoritative ChannelRuntime/InstanceRuntime scope may be live against the content catalog, and new scope admission is held while the commit is attempted.

The content subsystem requires an externally supplied `AuthorizedContentGeneration` describing the exact server/client digests, exact revision set, expected current activation sequence/digest (or explicit empty-start state), and a strictly newer `activation_sequence`. The content subsystem can validate this object but **cannot mint it, select its target revision, discover `latest`, or infer it from a passing build/test**. The origin/delivery/authenticity of that live authorization remains outside this decision and therefore cannot be exercised in a live environment under this authority.

The commit point is one non-fallible in-process publication under the catalog's exclusive commit lock:

1. recheck quiescence;
2. recheck expected-current generation/sequence;
3. recheck staged generation matches the exact authorization;
4. atomically replace the single active immutable generation pointer with the staged generation;
5. release the commit lock/admission hold.

No fallible external I/O, migration, client distribution, database write or network publication is allowed after step 4 as part of the activation transaction. Observability emitted afterward is evidence only and cannot retroactively change commit success.

An activation with stale expected-current evidence, a non-increasing `activation_sequence`, wrong digest/revision, incompatible profile, or a live scope fails closed without changing the active generation.

## 8. Failure, recovery, restart and rollback

### Before commit

Any compile, receive, validate, stage, compatibility, quiescence, authorization or expected-current failure discards/retains the candidate only as non-authoritative bounded state. The current active generation is unchanged.

### During activation

There is no partially published state. All checks happen before the single pointer replacement. A stale/racing request loses the expected-current comparison and fails. Once the pointer replacement occurs, the new generation is active; there is no subsequent fallible step in the content transaction.

### Restart

Content does not persist or reconstruct live authority by itself. On process restart the catalog begins with **no active generation and not-ready state**. It never activates a cached artifact or highest revision automatically.

The external runtime/deployment boundary may provide:

- one exact currently authorized primary generation; and
- optionally one exact last-known-good fallback generation.

Each is revalidated from immutable local/release bytes under this profile. A prior `LastKnownGoodReceipt` is evidence, not current authority: it may identify the fallback bytes/revisions, but activation still requires current external authorization with a strictly newer `activation_sequence`.

If the primary is missing/invalid/incompatible, the node may activate the explicitly authorized fallback only when it is also `COMPATIBLE_NO_MIGRATION`. Otherwise the node remains not ready. It never searches for another older generation.

### Rollback

Rollback is a new activation event at a quiescent boundary, not a decrement of authority generation. It may target only the explicitly authorized last-known-good artifact pair and only when both current and target are `COMPATIBLE_NO_MIGRATION`.

The rollback request must use a **strictly greater `activation_sequence`** than the current activation. Content revision identifiers may therefore refer to an older verified artifact while the authority/fencing sequence remains monotonic. An unauthorized older artifact, a migration-bearing target, or any non-monotonic activation sequence is rejected.

The content subsystem returns a deterministic `LastKnownGoodReceipt` containing the committed activation sequence plus exact digest/revision identity. Durable retention of that receipt, if desired, belongs to the future accepted deployment/operations owner; the content subsystem gets no DDL or live control-plane write authority here.

## 9. Coexistence/version semantics

Two generations may coexist **only** as:

- one currently active immutable generation; plus
- one fully staged **non-authoritative** candidate while the GameNode is quiescent.

Two authoritative content generations do not coexist in `FIRST_PRODUCTION_CONTENT_PROFILE/v1`.

- Existing live scopes prevent activation entirely.
- No scope begins while the commit admission hold is active.
- After commit, every newly created scope pins the one exact active generation/revision set.
- A later candidate cannot be staged if doing so would create a third resident generation.
- The previous artifact can be reloaded later for an explicitly authorized rollback; it is not kept as a second active generation.

Hot rollout with old and new live scopes, per-scope generation coexistence and migration-bearing rollout are future decisions.

## 10. Security and trust boundary

The following are binding:

- `VSL_BUNDLE_EVIDENCE_PROFILE`, `evidence:*`, fixture-only formula/policy identifiers, synthetic provenance and `synthetic://` presentation markers are rejected by the production profile.
- A passing test/evidence profile cannot be renamed or wrapped into production.
- Unknown critical section/record/capability/profile input fails closed.
- Malformed/corrupt/truncated/overlapping/oversize input fails before authoritative publication and before input-sized allocation.
- The bootstrap artifacts remain integrity-addressed. Integrity does **not** prove publisher authenticity.
- No source package or artifact is fetched from the network by the production runtime.
- No config flag can enable scripts, compression, dependencies, blobs, hot live-scope reload or another excluded capability.
- Content validation, compiler success, CI success, independent review or protected integration does not grant live deployment authority.

## 11. Authority separation

### Architecture implementation authority

After this decision **and** its registry rows are both protected on `main`, a fresh CONTENT #54 allocation may implement only the typed first-production source/profile, deterministic compile/projection, bounded bootstrap artifact loader, non-authoritative staging, quiescent atomic runtime publication interface, restart/rollback mechanics and tests described here.

### Repository integration authority

Implementation and registry changes use ordinary repository branches/PRs, applicable independent exact-head review, canonical CI/governance and FULL Merge Queue. No direct push/merge, ruleset weakening or bypass is authorized.

### Live deployment / production activation authority

**NONE.** No merged code, profile, digest, `AuthorizedContentGeneration` test fixture, protected registry value or successful CI job can itself authorize use in a live environment. The accepted live deployment/control-plane owner must separately authorize a concrete generation and environment in the future.

## 12. Exact fresh CONTENT #54 allocation after registry readback

The fresh allocation must be generated only after the serialized registry update has passed review/CI/FULL MQ and protected-main readback. Its maximum write set is:

- `apps/game-server/src/content/mod.rs`
- `apps/game-server/src/content/model.rs`
- `apps/game-server/src/content/compiler.rs`
- `apps/game-server/src/content/artifact.rs`
- new `apps/game-server/src/content/production.rs`
- new `apps/game-server/src/content/activation.rs`
- `apps/game-server/src/content/tests.rs`
- new `apps/game-server/tests/content_first_production.rs`
- the fresh task record for CONTENT #54 under `docs/agents/tasks/active/`

No Cargo/lockfile, migration/DDL, protocol, Foundation authority, Platform, Atlas, workflow/ruleset, WP3/WP4/WP5 or Server Seam path is implicitly leased.

Required implementation symbols/behaviors:

- a production limit/profile type that cannot accept `evidence:*`;
- typed `FirstProductionContentSource` / exact profile cardinality validation;
- production compile target distinct from evidence/test target;
- production bootstrap artifact profile ID/version with the 17-field manifest;
- projection-specific server/client hard ceilings and exact pair ceiling;
- fail-closed production staging that rejects evidence/synthetic/unknown-critical input;
- `StagedGeneration`;
- single-candidate staging guard and two-resident-generation guard;
- `AuthorizedContentGeneration` consumer interface with no content-owned mint/latest selection;
- quiescence + expected-current + monotonic activation sequence checks;
- single active immutable generation pointer and exact commit point;
- `LastKnownGoodReceipt` as evidence only;
- restart-not-ready and explicitly authorized fallback path;
- rollback as a new higher activation sequence;
- no live-scope hot reload and no migration-bearing activation.

Mandatory focused tests before broader repository gates:

1. exact minima and each family maximum accepted;
2. every `REQUIRED_NOW` max+1 rejected at the earliest safe boundary;
3. 512-byte key/string accepted, 513 rejected before retention;
4. 1,024 cells accepted, 1,025 rejected; x/y span 32 accepted, 33 rejected; second floor rejected;
5. checked derivation/conformance tests prove `1042/3087/1043/6/4114/4295078/4303968/33602/4337570/8426` match the protected registry;
6. same production graph compiles byte-identically after source enumeration shuffle;
7. evidence/test/synthetic profile and fixture-only formula/presentation markers rejected by ordinary release;
8. duplicate/unresolved/wrong projection/pair mismatch/corrupt/truncated/unknown-critical/oversize input fails closed;
9. source dependencies, aliases, compressed/chunk/blob/script sections/capabilities rejected and cannot be enabled by config;
10. staging a second concurrent candidate or retaining a third generation is denied without changing active;
11. activation with a live scope, stale expected-current generation, wrong digest/revision or non-increasing activation sequence leaves active unchanged;
12. commit publishes one complete generation only; there is no partial server/client activation;
13. pre-commit failure preserves current active generation;
14. restart begins not-ready and does not auto-select a cached/latest artifact;
15. explicit primary restart activation revalidates bytes; invalid primary cannot activate;
16. explicitly authorized last-known-good fallback can activate only as `COMPATIBLE_NO_MIGRATION` under a newer activation sequence;
17. rollback to unauthorized older, migration-bearing or non-monotonic target is rejected;
18. no second authoritative generation can coexist; new scope sees only the exact post-commit generation;
19. client-safe projection contains no server-only record/field;
20. public release API does not expose an evidence/test activation path and does not mint live deployment authority.

The fresh allocation must additionally inherit the applicable `apps/game-server/AGENTS.md` high-risk exact-head independent review requirement.

## 13. #433 resolution checklist mapping

1. `FIRST_PRODUCTION_CONTENT_PROFILE/v1`: exact compiler/loader/staging/runtime IN/OUT is defined in §§3 and 12.
2. Every potential DUR-04/VSL resource dimension is classified in §5.
3. Every `REQUIRED_NOW` item has unit, exact hard max, evidence/derivation, boundary obligation and fail-closed behavior in §§4-6.
4. Excluded capabilities explain unreachability and rejection boundary in §5.2; they cannot be config/artifact-enabled.
5. Deferred dimensions identify the missing future decision and trigger in §5.3.
6. `received/built -> validated -> staged -> active`, exact commit point, stale/incompatible rejection and no partial activation are in §7.
7. Pre-commit failure, activation race, restart, last-known-good fallback, rollback and monotonic activation sequencing are in §8.
8. Coexistence is explicit: one active plus one non-authoritative staged candidate only; never two authoritative generations (§9).
9. Evidence != production, malformed/oversize/unknown-critical fail-closed and deployment authority separation are in §§10-11.
10. No permanent World Project/World Bundle, compression, CDN, signing topology or script runtime is selected (§§1,3,5).
11. Architecture implementation, repository integration and live deployment authorities are separately defined (§11).
12. The exact serialized registry append and exact post-readback CONTENT #54 bounded allocation are in §§6 and 12.
13. This decision is not `RESOLVED` until its exact PR head receives independent architecture/security review with P0/P1/P2=0, applicable canonical CI/governance, normal FULL Merge Queue integration and protected-main readback.

## 14. Resolution state

Until criterion 13 is proven on the exact final head, Issue #433 remains open.

If criterion 13 passes, the only valid closeout handoff is:

`#433 resolved -> RESOURCE_LIMITS_REGISTRY serialized update -> review/CI/FULL MQ/readback -> fresh CONTENT #54 allocation`

This decision does not authorize skipping or combining those serial steps.