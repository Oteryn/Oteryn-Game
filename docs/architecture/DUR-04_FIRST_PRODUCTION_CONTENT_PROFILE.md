# DUR-04 — First Production Content Profile

- Decision ID: `DUR04-FIRST-PROD-CONTENT-01`
- Profile ID: `FIRST_PRODUCTION_CONTENT_PROFILE/v1`
- Date: 2026-09-09
- Resolves architecture blocker: #433
- Enables, after the separately required registry integration/readback: a fresh bounded continuation of CONTENT #54
- Status: **accepted only when this exact decision is present on protected `main`; a branch/PR copy is proposal evidence only**

## 1. Decision

The first production-capable CONTENT slice is a deliberately narrow **production canary profile**, not a representative world-scale profile and not the permanent World Project / World Bundle design.

It reuses only the semantic shape already proven by VSL-CONTENT-01 and the merged CONTENT evidence seam, while replacing every evidence-only authority marker with a distinct production profile. The profile is intentionally small enough that its externally controlled byte/count limits can be derived from its closed schema rather than copied from test or format-spike limits.

This decision does **not** authorize a live deployment, make gameplay available, select a permanent authoring format, select a permanent runtime bundle format, or turn the current `evidence:*` profile into production.

## 2. Evidence basis

This decision consumes the following protected evidence and contracts:

1. `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`: deterministic typed graph, separate server/client projections, untrusted bounded loader, immutable staging, explicit activation, compatible rollback, complete resource-limit registry, and no ambient script authority.
2. `VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md`: the smallest movement/combat CONTENT semantic slice and explicit non-requirement for scripts/NPCs/quests/houses/market/broad import/final format.
3. Current `apps/game-server/src/content/**`: exact record shape, deterministic ordering, checked arithmetic, SHA-256 artifact/section integrity, strict unknown-critical rejection and fail-closed server/client pairing.
4. Current merged VSL shape: exactly 3 cells, 21 aggregate definitions, 22 server records, 6 client-safe records and 24 counted references. This is used only to define the intentionally tiny canary shape; it is **not** treated as representative production world scale.
5. Current record schema: no known record has more than 8 fields, and every known record kind has an exact field count.
6. Current artifact framing evidence: 24-byte header, two 48-byte section entries, 32-byte trailer, 15 manifest strings and a length-prefixed record body. The first production profile may use a profile-specific replaceable v1 framing with this same bounded shape because a loadable artifact is necessary for the first slice. This does not freeze `.owb`, `.omap`, chunking, compression or a permanent physical schema.
7. Existing production registry precedent uses 128 UTF-8 bytes for bounded semantic identifiers such as the FND-02 client build identifier. `FIRST_PRODUCTION_CONTENT_PROFILE/v1` restricts content keys, revision tokens and presentation/ruleset tokens to ASCII semantic identifiers, so 128 bytes is adopted as an identifier ceiling on that independent production basis, not inherited from `evidence:test-v1`.
8. Content Format Spike #95/#125 is evidence only. Its 64 MiB artifact, 2 MiB raw chunk, 4,096 chunks, 512-byte strings, depth 16, collection 100,000 and 64:1 compression ratio are **not** production limits in this decision.

## 3. `FIRST_PRODUCTION_CONTENT_PROFILE/v1`

### 3.1 Compiler IN

The production compiler accepts exactly one trusted-build, typed in-memory canonical graph. It does **not** parse a permanent serialized World Project.

The graph contains exactly:

- one package identity and one world identity;
- 1 Region definition;
- 1 Area definition;
- 1 Terrain definition;
- 3 Cells on exactly 1 floor;
- 1 local Relocation definition;
- 1 deterministic Behavior policy reference;
- 3 project-owned Presentation/asset semantic references;
- 1 Creature definition;
- 1 Spawn definition with explicit GAME-CHANNEL multiplicity, eligibility and recovery classification, and population ceiling 1;
- 1 Formula/Profile semantic reference to an already accepted gameplay/ruleset policy; CONTENT does not embed guessed formula code;
- 1 Effect definition;
- 1 Ability definition;
- 1 materializable Item definition;
- 1 Loot Table containing exactly 1 Loot Entry;
- 1 XP definition referring to the accepted formula/profile identity;
- 1 deterministic RNG purpose key plus RNG execution-profile identity. No secret/root seed is content.

The compiler must:

- reject fixture-only/synthetic evidence authority markers;
- canonicalize ordering deterministically;
- reject duplicate keys and missing references;
- validate all `REQUIRED_NOW` limits before growth/allocation that depends on attacker- or content-controlled values;
- produce one server-authoritative artifact and one allowlisted client-safe artifact;
- bind exact package/content/map/ruleset/world-policy/compiler/canonicalization/content-lock/provenance/simulation/profile revision identity;
- produce exact SHA-256 digests for both artifacts and their critical sections;
- reject any request for a capability outside this profile.

### 3.2 Compiler OUT

The first profile has no:

- filesystem/project-tree parser;
- YAML/RON/JSON5/SQLite/custom authoring serializer decision;
- package dependency resolution or floating dependency graph;
- aliases or alias chains;
- OTBM/Crystal/legacy broad importer;
- scripts or script components;
- NPCs, quests, houses, market definitions or broad catalog support;
- embedded asset blobs;
- chunked/compressed/delta artifact generation;
- signing/CDN/publisher-topology logic;
- verbose unbounded compile/evidence report.

### 3.3 Loader IN

The loader treats both production artifacts as untrusted bytes. Before staging it must verify:

1. artifact byte limit;
2. profile magic/version and zero unknown flags;
3. exactly two known critical sections (`manifest`, `body`); all unknown sections are rejected in v1;
4. checked section table offsets/ranges with no overlap/truncation;
5. section and artifact SHA-256 integrity;
6. exact manifest field count and per-field byte limits;
7. exact record count, record byte limit, known record kinds and exact field count;
8. semantic identifiers, references, source classification and projection allowlist;
9. exact server/client revision compatibility;
10. exact expected artifact digests and revision set supplied by the caller's authorized publication plan.

The loader performs no network fetch and infers no deployment authorization from a digest, test result or repository merge.

### 3.4 Staging IN

Staging is isolated and non-authoritative.

- At most one candidate pair may be staged at a time.
- Raw artifact bytes remain in the immutable artifact store/caller buffer and are separately bounded by artifact limits.
- All heap allocation derived from untrusted decoded content in one staged pair is charged to one 32,768-byte staging budget.
- A compact offset/index representation is required; unbounded per-record/per-string heap growth is forbidden.
- A staged pair carries exact server/client digests, revision set, expected current publication generation and proposed next publication generation.
- Staging success never changes the active publication pointer.

### 3.5 Runtime IN

The first profile authorizes implementation of an **internal CONTENT publication kernel only**:

- immutable lookup of the staged content definitions;
- one atomic publication pointer per world/profile;
- exact content-generation pinning for each newly created authoritative simulation scope;
- retention of at most one draining predecessor generation;
- stale-generation rejection;
- explicit compatible rollback as a new monotonically increasing publication generation.

It does not authorize a CLI, admin endpoint, Platform route, deployment controller, live environment mutation, automatic startup selection or change to `GameplayAvailability::UnavailableBootstrap`.

## 4. Required-now hard maxima

All maxima below are accepted production maxima **only for `FIRST_PRODUCTION_CONTENT_PROFILE/v1`**. Expansion of any semantic count is a new profile/review event; these numbers are not defaults for a future full world.

| ID | Dimension | Unit | Hard maximum | Evidence / derivation | Mandatory boundary obligation |
|---|---|---:|---:|---|---|
| `DUR04-FPC-PACKAGES` | packages per graph/artifact pair | packages | 1 | first profile has one package and no dependency resolver | 1 passes; 2 is rejected before graph resolution |
| `DUR04-FPC-REGIONS` | Region definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected before canonical allocation |
| `DUR04-FPC-AREAS` | Area definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-TERRAINS` | Terrain definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-CELLS` | Cell definitions | cells | 3 | smallest merged movement/collision/relocation VSL shape has start, blocked and target cells | 3 passes; 4 rejected before cell/index allocation |
| `DUR04-FPC-FLOORS` | distinct cell floors | floors | 1 | first profile is a single-floor canary | 1 passes; second distinct floor rejected |
| `DUR04-FPC-RELOCATIONS` | Relocation definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-BEHAVIORS` | Behavior policy references | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-PRESENTATIONS` | project-owned presentation references | definitions | 3 | one creature, item and ability presentation in the proven slice | 3 passes; 4 rejected |
| `DUR04-FPC-CREATURES` | Creature definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-SPAWNS` | Spawn definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-SPAWN-POPULATION` | population per spawn definition | entities | 1 | canary proves one deterministic target without scale assumptions | 1 passes; 2 rejected before runtime allocation |
| `DUR04-FPC-FORMULA-PROFILES` | formula/ruleset profile references | definitions | 1 | one ability/XP policy reference; no embedded formula runtime | 1 passes; 2 rejected |
| `DUR04-FPC-EFFECTS` | Effect definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-ABILITIES` | Ability definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-ITEMS` | Item definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-LOOT-TABLES` | Loot Table definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-LOOT-ENTRIES` | Loot Entries | entries | 1 | one table contains one entry | 1 passes; 2 rejected |
| `DUR04-FPC-XP-DEFINITIONS` | XP definitions | definitions | 1 | closed first-profile semantic shape | 1 passes; 2 rejected |
| `DUR04-FPC-RNG-PURPOSES` | deterministic RNG purpose keys | keys | 1 | one loot RNG purpose; root seed remains simulation-owned | 1 passes; 2 rejected |
| `DUR04-FPC-DEFINITIONS` | aggregate semantic definitions | definitions | 21 | sum of the exact category maxima above | 21 passes; 22 rejected before canonical set allocation |
| `DUR04-FPC-REFERENCES` | aggregate counted semantic references | references | 24 | exact current compiler reference formula over the closed 21-definition shape | 24 passes; 25 rejected before reference collection growth |
| `DUR04-FPC-KEY-BYTES` | ContentKey/PackageKey/WorldId semantic identifier | ASCII bytes | 128 | independent existing production identifier precedent; first profile admits semantic identifiers only | 128 passes; 129 rejected before allocation/copy |
| `DUR04-FPC-FIELD-STRING-BYTES` | revision/policy/presentation/artifact semantic field | ASCII bytes | 128 | same bounded semantic-identifier class; no free-form text or blobs | 128 passes; 129 rejected before allocation/copy |
| `DUR04-FPC-MANIFEST-FIELDS` | manifest fields per artifact | fields | 15 | exact v1 manifest shape | 15 required; any other count rejected |
| `DUR04-FPC-SERVER-RECORDS` | server-authoritative body records | records | 22 | 21 definitions + 1 RNG context | 22 passes; 23 rejected before record vector allocation |
| `DUR04-FPC-CLIENT-RECORDS` | client-safe body records | records | 6 | 3 presentations + creature + ability + item | 6 passes; 7 rejected before record vector allocation |
| `DUR04-FPC-RECORD-BYTES` | one encoded record | bytes | 1,042 | max 8 fields; `2 + 8 * (2 + 128) = 1,042` | 1,042 accepted; 1,043 rejected before record copy |
| `DUR04-FPC-SECTIONS` | critical sections per artifact | sections | 2 | v1 requires exactly manifest + body | exactly 2; 1, 3 or unknown section rejected |
| `DUR04-FPC-SECTION-BYTES` | one section | bytes | 9,626 | exact worst-case server body under fixed record distribution and 128-byte fields | 9,626 accepted; 9,627 rejected before section processing |
| `DUR04-FPC-SERVER-ARTIFACT-BYTES` | server artifact | bytes | 11,728 | `24 header + 96 table + 1,950 manifest + 9,626 body + 32 trailer` | max accepted; max+1 rejected before parse/allocation |
| `DUR04-FPC-CLIENT-ARTIFACT-BYTES` | client artifact | bytes | 3,702 | `24 + 96 + 1,950 + (4 + 6 * 266) + 32` | max accepted; max+1 rejected before parse/allocation |
| `DUR04-FPC-ARTIFACT-PAIR-BYTES` | server + client artifact pair | bytes | 15,430 | `11,728 + 3,702` | pair at max accepted; max+1 rejected before staging |
| `DUR04-FPC-PROJECTIONS` | artifacts/projections per content generation | projections | 2 | exactly server-authoritative + client-safe | exactly 2; missing/extra projection rejected |
| `DUR04-FPC-STAGING-HEAP-BYTES` | heap bytes charged to decoded untrusted staged pair, excluding separately bounded immutable raw artifacts | bytes | 32,768 | worst-case decoded string payload is 14,720 bytes; fixed record/index/offset tables over 28 records, 21 definitions and 24 refs fit well below the remaining budget when represented compactly | charged allocator at 32,768 succeeds; next charged byte fails staging with active unchanged |
| `DUR04-FPC-STAGED-CANDIDATES` | simultaneously staged candidate pairs per world/profile | candidates | 1 | no parallel activation is needed by first slice | second candidate rejected without replacing staged/active state |
| `DUR04-FPC-RESIDENT-GENERATIONS` | distinct runtime content generations resident for one world/profile | generations | 2 | one current generation + at most one draining predecessor | third distinct resident generation blocks activation until predecessor drains |

### 4.1 Exact byte derivation

With the 128-byte semantic field ceiling, a one-field record is at most 132 bytes, two-field 262, three-field 392, four-field 522, five-field 652 and eight-field 1,042 bytes before the outer 4-byte record-length prefix.

The exact server body distribution is:

- five 1-field records: Region, Area, Terrain, Loot Table, RNG Purpose;
- six 2-field records: Behavior, three Presentations, Formula/Profile, RNG Context;
- five 3-field records: Relocation, Effect, Ability, Item, XP;
- one 4-field Creature;
- one 5-field Loot Entry;
- four 8-field records: three Cells and one Spawn.

Therefore:

`4 + 5*136 + 6*266 + 5*396 + 526 + 656 + 4*1046 = 9,626 bytes`.

The client body has six two-field records:

`4 + 6*266 = 1,600 bytes`.

The manifest has 15 length-prefixed semantic strings:

`15 * (2 + 128) = 1,950 bytes`.

No value above is copied from the current test profile or spike fences.

## 5. `EXCLUDED_FAIL_CLOSED`

These capabilities/resource dimensions are not reachable in `FIRST_PRODUCTION_CONTENT_PROFILE/v1` and must be rejected at the indicated boundary. No input/config/artifact flag can turn them on.

| Capability / resource dimension | Why unreachable in v1 | Required rejection point |
|---|---|---|
| script component bytes | no script definition/section/capability exists | compiler rejects script-bearing graph; loader rejects unknown record/section/capability |
| script instances | no script runtime/linker exists | no construction API; any script capability bit is invalid profile |
| Wasm memories/pages | no Wasm runtime exists | no construction API; unknown script metadata rejected |
| Wasm tables/elements | no Wasm runtime exists | same |
| deterministic fuel | no script execution | same |
| script host-call count | no host ABI | same |
| script query/result collection sizes | no script query imports | same |
| proposed script action-plan count/bytes | no script action plan | same |
| persistent script extension-state bytes | no extension-state capability | schema/compiler reject field/record; no persistence API |
| NPC/quest/house/market definitions | outside first slice | compiler has no accepted production record kind; loader rejects unknown kind |
| arbitrary filesystem/network/process/env access | content is data, not executable authority | no API/capability; artifact token cannot be resolved as ambient I/O |
| network artifact/dependency fetch | release input must already be exact immutable bytes | loader has no network resolver |
| automatic/live activation endpoint | deployment authority is external and separately gated | no public CLI/admin/Platform route; crate-root activation remains unavailable |
| fixture/evidence profile as production | evidence and production trust are distinct | production compiler rejects `evidence:*`, fixture-only/synthetic markers; production loader rejects evidence profile ID |
| unknown optional/critical records or sections | v1 uses exact closed schema | loader rejects all unknown v1 record kinds, fields, flags and sections |
| durable migration other than `COMPATIBLE_NO_MIGRATION` | first slice does not require persisted content reinterpretation | validation rejects any migration requirement/tombstone/removal needing durable change |

## 6. `DEFERRED_REQUIRES_FUTURE_DECISION`

The following DUR-04 resource dimensions remain unselected because the first profile has no path that consumes them. Current v1 input attempting to exercise them still fails closed. A future trigger must define new hard maxima before that capability is implemented or accepted.

| Deferred dimension | Why not needed now | Future trigger requiring a new decision |
|---|---|---|
| serialized World Project source bytes | compiler receives a typed in-memory graph | selecting/implementing an authoring serializer/import boundary |
| source file count/path length/tree depth/nesting | no filesystem/project-tree parser | first file-based World Project/Studio build path |
| package dependency count and dependency depth | exactly one package, no resolver | first multi-package content publication |
| alias count and alias-chain depth | aliases are absent | first alias/deprecation migration capability |
| technical chunk count, chunk bytes, chunk side/floor packing | temporary v1 artifact is record-based, not chunk-packed | permanent/broader runtime bundle or spatial streaming design |
| compressed bytes, decompressed bytes and decompression ratio | v1 forbids compression | first compression codec/container proposal |
| spatial density/index fan-out beyond the fixed 3-cell canary | no broad chunk/spatial allocator exists | first world-scale map/streaming profile |
| embedded asset/blob bytes and asset-pack count | v1 carries semantic presentation keys only | first embedded/client asset packaging decision |
| compiler/import diagnostic report bytes | v1 emits bounded status/digests only | first standardized publication/import report contract |
| permanent World Bundle artifact/container bytes | v1 framing is explicitly replaceable and scoped to this canary | permanent physical-format decision after real import/Studio/runtime evidence |

No deferred row may be implemented with an implicit/unlimited default. Its trigger reopens architecture and registry review.

## 7. Activation and publication semantics

### 7.1 State machine

The only legal progression is:

`RECEIVED_OR_BUILT -> VALIDATED -> STAGED -> ACTIVE`

`VALIDATED` means all profile/schema/limit/integrity/semantic/reference/revision/authorization-plan checks have passed. `STAGED` is immutable and non-authoritative.

### 7.2 Commit point

Each world/profile owns one atomic publication pointer:

`PublicationPointer { publication_generation, exact_revision_set, server_digest, client_digest, staged_handle }`.

An activation request supplies:

- the expected current `publication_generation` (or explicit empty bootstrap state);
- the exact staged candidate identity;
- `next_publication_generation = current + 1` using checked arithmetic;
- the exact artifact digests/revision set authorized by the external deployment decision.

The **only commit point** is a compare-and-swap of that complete pointer from the expected current generation to the complete staged candidate. No per-section, per-projection, per-definition or per-scope publication occurs before this swap.

A stale expected generation, mismatched digest/revision, invalid candidate, generation overflow or failed CAS leaves the previous active pointer unchanged.

### 7.3 No partial activation

Server and client projections are one compatibility pair. Activation publishes both identities together. A scope may never observe one projection from generation N and the other from generation N+1.

Existing scopes never have definitions mutated in place.

## 8. Failure, recovery and rollback

### 8.1 Failure before commit

Parse, integrity, limit, semantic, compatibility, authorization-plan or staging failure discards/rejects the candidate. Active content is unchanged. No fallback candidate is selected automatically.

### 8.2 Failure during activation

The atomic publication CAS either succeeds completely or does not occur. A stale/failed CAS leaves current active unchanged. There is no multi-step partially committed state.

If the process terminates after the atomic pointer changed, normal process state is lost; restart follows the restart rule below rather than guessing whether an in-memory action completed.

### 8.3 Restart

Content code does not scan a directory, network, repository, timestamp or generation number and choose “latest”. On restart, authoritative gameplay scope creation remains fail-closed until a separately authorized publication plan supplies an exact artifact pair, exact digest/revision set and a publication generation/floor acceptable to the deployment authority.

The loader revalidates and restages those exact immutable bytes before a new in-process publication commit. If the requested bytes are missing, corrupt, stale, incompatible or unauthorized, no new gameplay scope is allowed to bind them.

### 8.4 Last-good and rollback

A previously verified compatible artifact pair may be selected by an **external authorized publication plan** as the payload of a new activation.

Rollback never decrements `publication_generation`. Example: if generation 8 activates artifact B and artifact A is chosen for rollback, A becomes generation 9. Requests for generation <= current are stale and rejected.

`content_revision` itself is an opaque identity and is not ordered; monotonicity belongs to `publication_generation`.

The first profile accepts only `COMPATIBLE_NO_MIGRATION`, so rollback cannot silently cross a durable-state migration. Any future activation that needs `READ_COMPATIBLE_NORMALIZE`, `EXPLICIT_DATA_MIGRATION`, `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION` or `REMOVED_WITH_EXPLICIT_POLICY` is outside this profile and requires a new decision.

No automatic rollback occurs after a post-commit gameplay failure; an explicit separately authorized next publication is required.

## 9. Coexistence and revision compatibility

Two **distinct resident generations** may coexist only as:

- one current publication for new scopes; and
- at most one draining predecessor pinned by scopes that already existed before the switch.

A single simulation scope is pinned to one exact publication generation for its lifetime. Hot mutation of that scope is forbidden.

A third distinct generation cannot be activated while the predecessor still has pinned scopes. The operator/deployment authority must first drain/terminate those scopes under its separately accepted policy.

Compatibility requires exact equality of the profile/schema version and all revision identities that affect content meaning: content, map, ruleset, world policy, compiler/canonicalization, Content Lock/provenance, simulation profile and the first-production-content profile revision. Server/client pair digests must match the same revision set.

## 10. Trust and security boundary

- SHA-256 proves byte integrity, not publisher/deployment authority.
- The activation caller must provide an exact expected digest/revision set from a separately trusted deployment/control-plane decision; this architecture does not select signing or trust-root topology.
- Passing compiler tests, loader tests, CI or an architecture review never grants production activation authority.
- An `evidence:*` artifact can never be accepted by the production profile.
- Malformed, corrupt, truncated, oversized, unknown, stale, incompatible or unknown-critical content fails closed.
- No content field is interpreted as a filesystem path, URL, process command, environment variable, database statement or authority token.
- Client-safe projection remains allowlist-generated and never becomes server authority.

## 11. Deliberate non-decisions

This decision does **not** choose:

- `.omap`, `.owb`, World Project or World Bundle permanent physical format;
- authoring serialization;
- permanent chunk dimensions/floor packing;
- compression codec;
- patch/delta/CDN layout;
- signing/trust-root/publisher topology;
- script language/runtime/Wasmtime version/WIT ABI;
- broad world/catalog scale;
- production rollout orchestration or live environment policy.

The v1 canary artifact framing is a replaceable implementation profile needed only to exercise deterministic build/load/stage/publication. A future permanent format may require recompilation and a new profile; v1 does not create a perpetual parse-compatibility promise.

## 12. Authority separation

### Architecture implementation authority

After this decision is protected **and** the required registry entries below are separately integrated/read back on protected `main`, a fresh CONTENT #54 allocation may authorize implementation of only this profile and its internal publication kernel.

This decision by itself does not grant a writable CONTENT branch or code mutation authority.

### Repository integration authority

Any architecture, registry or later implementation change is integrated only through normal repository review/checks and FULL Merge Queue/protected-main readback. No direct merge/protection bypass is authorized here.

### Live deployment / production activation authority

**NONE.**

No branch, PR, passing check, protected merge, content artifact, internal activation kernel or CONTENT #54 completion produced by this chain may mutate a live/protected environment without a separate explicit deployment authority.

## 13. Exact serialized registry update after #433 protected readback

After this decision is protected, the next change must append the following entries to `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` without changing the numeric maxima. `updated_at` is set to the actual registry-commit UTC timestamp; all existing unrelated registry rows/rules remain unchanged.

```json
[
  {"id":"DUR04-FPC-PACKAGES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"packages per first production content graph/artifact pair","unit":"packages","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"invalid production content profile","allocation_impact":"reject before package graph allocation/resolution","client_visible":false,"boundary_tests":"1 accepted; 2 rejected before graph resolution"},
  {"id":"DUR04-FPC-REGIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Region definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-AREAS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Area definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-TERRAINS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Terrain definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-CELLS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Cell definitions","unit":"cells","hard_maximum":3,"configurable_range":{"min":3,"max":3},"failure_category":"content resource limit exceeded","allocation_impact":"reject before cell/spatial index allocation","client_visible":false,"boundary_tests":"3 accepted; 4 rejected"},
  {"id":"DUR04-FPC-FLOORS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"distinct cell floors","unit":"floors","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before spatial index allocation","client_visible":false,"boundary_tests":"one floor accepted; second distinct floor rejected"},
  {"id":"DUR04-FPC-RELOCATIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Relocation definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-BEHAVIORS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Behavior policy references","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-PRESENTATIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"project-owned Presentation references","unit":"definitions","hard_maximum":3,"configurable_range":{"min":3,"max":3},"failure_category":"content resource limit exceeded","allocation_impact":"reject before projection allocation","client_visible":false,"boundary_tests":"3 accepted; 4 rejected"},
  {"id":"DUR04-FPC-CREATURES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Creature definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-SPAWNS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Spawn definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before spawn/runtime index allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-SPAWN-POPULATION","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"population per Spawn definition","unit":"entities","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before runtime population allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-FORMULA-PROFILES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"formula/ruleset profile references","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-EFFECTS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Effect definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-ABILITIES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Ability definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-ITEMS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Item definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-LOOT-TABLES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Loot Table definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before loot allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-LOOT-ENTRIES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"Loot Entries","unit":"entries","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before loot-entry allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-XP-DEFINITIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"XP definitions","unit":"definitions","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-RNG-PURPOSES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"deterministic RNG purpose keys","unit":"keys","hard_maximum":1,"configurable_range":{"min":1,"max":1},"failure_category":"content resource limit exceeded","allocation_impact":"reject before RNG-purpose allocation","client_visible":false,"boundary_tests":"1 accepted; 2 rejected"},
  {"id":"DUR04-FPC-DEFINITIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"aggregate semantic definitions","unit":"definitions","hard_maximum":21,"configurable_range":{"min":21,"max":21},"failure_category":"content resource limit exceeded","allocation_impact":"reject before canonical definition set allocation","client_visible":false,"boundary_tests":"21 accepted; 22 rejected"},
  {"id":"DUR04-FPC-REFERENCES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"aggregate counted semantic references","unit":"references","hard_maximum":24,"configurable_range":{"min":24,"max":24},"failure_category":"content resource limit exceeded","allocation_impact":"reject before reference collection allocation","client_visible":false,"boundary_tests":"24 accepted; 25 rejected"},
  {"id":"DUR04-FPC-KEY-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"ContentKey PackageKey WorldId semantic identifier","unit":"ASCII bytes","hard_maximum":128,"configurable_range":{"min":1,"max":128},"failure_category":"content resource limit exceeded","allocation_impact":"reject before identifier allocation/copy","client_visible":false,"boundary_tests":"128 bytes accepted; 129 rejected"},
  {"id":"DUR04-FPC-FIELD-STRING-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"revision policy presentation and artifact semantic field","unit":"ASCII bytes","hard_maximum":128,"configurable_range":{"min":1,"max":128},"failure_category":"content resource limit exceeded","allocation_impact":"reject before field allocation/copy","client_visible":false,"boundary_tests":"128 bytes accepted; 129 rejected"},
  {"id":"DUR04-FPC-MANIFEST-FIELDS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"manifest fields per artifact","unit":"fields","hard_maximum":15,"configurable_range":{"min":15,"max":15},"failure_category":"invalid production content artifact","allocation_impact":"reject before manifest field allocation","client_visible":false,"boundary_tests":"exactly 15 accepted; any other count rejected"},
  {"id":"DUR04-FPC-SERVER-RECORDS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"server-authoritative body records","unit":"records","hard_maximum":22,"configurable_range":{"min":22,"max":22},"failure_category":"content resource limit exceeded","allocation_impact":"reject before record vector allocation","client_visible":false,"boundary_tests":"22 accepted; 23 rejected"},
  {"id":"DUR04-FPC-CLIENT-RECORDS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"client-safe body records","unit":"records","hard_maximum":6,"configurable_range":{"min":6,"max":6},"failure_category":"content resource limit exceeded","allocation_impact":"reject before record vector allocation","client_visible":false,"boundary_tests":"6 accepted; 7 rejected"},
  {"id":"DUR04-FPC-RECORD-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"one encoded production content record","unit":"bytes","hard_maximum":1042,"configurable_range":{"min":1,"max":1042},"failure_category":"content resource limit exceeded","allocation_impact":"reject before record copy/decode","client_visible":false,"boundary_tests":"1042 bytes accepted; 1043 rejected"},
  {"id":"DUR04-FPC-SECTIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"critical sections per artifact","unit":"sections","hard_maximum":2,"configurable_range":{"min":2,"max":2},"failure_category":"invalid production content artifact","allocation_impact":"reject before section table allocation","client_visible":false,"boundary_tests":"exactly manifest+body accepted; missing extra or unknown section rejected"},
  {"id":"DUR04-FPC-SECTION-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"bytes per artifact section","unit":"bytes","hard_maximum":9626,"configurable_range":{"min":1,"max":9626},"failure_category":"content resource limit exceeded","allocation_impact":"reject before section processing/allocation","client_visible":false,"boundary_tests":"9626 accepted; 9627 rejected"},
  {"id":"DUR04-FPC-SERVER-ARTIFACT-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"server production content artifact","unit":"bytes","hard_maximum":11728,"configurable_range":{"min":1,"max":11728},"failure_category":"content resource limit exceeded","allocation_impact":"reject before artifact parse/allocation","client_visible":false,"boundary_tests":"11728 accepted; 11729 rejected"},
  {"id":"DUR04-FPC-CLIENT-ARTIFACT-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"client-safe production content artifact","unit":"bytes","hard_maximum":3702,"configurable_range":{"min":1,"max":3702},"failure_category":"content resource limit exceeded","allocation_impact":"reject before artifact parse/allocation","client_visible":false,"boundary_tests":"3702 accepted; 3703 rejected"},
  {"id":"DUR04-FPC-ARTIFACT-PAIR-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"server plus client production artifact pair","unit":"bytes","hard_maximum":15430,"configurable_range":{"min":1,"max":15430},"failure_category":"content resource limit exceeded","allocation_impact":"reject before pair staging","client_visible":false,"boundary_tests":"15430 accepted; 15431 rejected"},
  {"id":"DUR04-FPC-PROJECTIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"artifacts projections per content generation","unit":"projections","hard_maximum":2,"configurable_range":{"min":2,"max":2},"failure_category":"invalid production content pair","allocation_impact":"reject before pair staging","client_visible":false,"boundary_tests":"exact server+client pair accepted; missing or extra projection rejected"},
  {"id":"DUR04-FPC-STAGING-HEAP-BYTES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"decoded untrusted-content heap charged to one staged pair excluding separately bounded immutable raw artifacts","unit":"bytes","hard_maximum":32768,"configurable_range":{"min":1,"max":32768},"failure_category":"content staging resource exhausted","allocation_impact":"bounded staging allocator refuses the allocation and leaves active state unchanged","client_visible":false,"boundary_tests":"charged usage through 32768 allowed; next charged byte rejects staging; active unchanged"},
  {"id":"DUR04-FPC-STAGED-CANDIDATES","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"simultaneously staged candidate pairs per world profile","unit":"candidates","hard_maximum":1,"configurable_range":{"min":0,"max":1},"failure_category":"content staging capacity exhausted","allocation_impact":"reject second candidate without replacing staged or active state","client_visible":false,"boundary_tests":"one staged candidate accepted; second rejected"},
  {"id":"DUR04-FPC-RESIDENT-GENERATIONS","owner_contract":"DUR-04 / FIRST_PRODUCTION_CONTENT_PROFILE/v1","resource":"distinct runtime content generations resident per world profile","unit":"generations","hard_maximum":2,"configurable_range":{"min":1,"max":2},"failure_category":"content generation capacity exhausted","allocation_impact":"block activation until draining predecessor is released","client_visible":false,"boundary_tests":"current plus one draining predecessor accepted; third distinct generation rejected"}
]
```

## 14. Fresh CONTENT #54 allocation after registry protected readback

Only after the registry update above has separately passed review/CI/FULL Merge Queue and protected-main readback, the coordinator may issue one fresh allocation with these writable runtime paths and no others unless a concrete compile error proves a same-scope path is necessary:

- `apps/game-server/src/content/model.rs`
- `apps/game-server/src/content/compiler.rs`
- `apps/game-server/src/content/artifact.rs`
- `apps/game-server/src/content/production.rs` (new, if separation is useful)
- `apps/game-server/src/content/mod.rs`
- `apps/game-server/src/content/tests.rs`

`apps/game-server/src/lib.rs` is **not** allocated for enabling gameplay or live activation; `GameplayAvailability::UnavailableBootstrap` remains unchanged by this CONTENT slice.

Expected implementation symbols/obligations:

- keep `EvidenceLimits`/`CompileTarget::Evidence` and evidence fixtures non-production;
- add a distinct `FirstProductionContentLimitsV1` bound exactly to the protected registry values;
- add `FIRST_PRODUCTION_CONTENT_PROFILE/v1` metadata and a distinct production compile target; do not rename an `evidence:*` target;
- add production semantic validation that rejects fixture/synthetic authority markers and all excluded/deferred capabilities;
- add the replaceable v1 server/client artifact profile with exact limits above;
- add bounded production staging using one charged allocator budget;
- add `PublicationGeneration` with checked monotonic increment;
- add an internal/testable `ContentPublicationKernelV1` with expected-generation CAS semantics;
- add scope pin/release semantics enforcing at most two resident generations;
- do not expose a crate-root/public deployment activation API and do not add a Platform/admin/CLI/live route.

Mandatory tests on the exact implementation head:

1. a table-driven at-limit and `max+1` denial test for every registry row above;
2. deterministic byte-identical compile under input enumeration shuffle;
3. duplicate/missing reference and malformed semantic graph rejection;
4. production rejects `evidence:*`, fixture-only and synthetic authority markers;
5. server/client projection leak negative and revision-pair mismatch rejection;
6. corrupt/truncated/unknown-section/unknown-record/unknown-flag/overflow/oversize rejection before staging;
7. scripts, compression, chunks, source files/dependencies/aliases and other excluded/deferred capabilities fail closed;
8. staging allocator exhaustion leaves active state unchanged;
9. valid pair transitions `validated -> staged -> active` only at the atomic pointer commit;
10. pre-commit failure and stale CAS preserve the previous active generation;
11. no scope observes mixed generations; existing scopes remain pinned to the draining predecessor;
12. third resident distinct generation is rejected until draining completes;
13. rollback reuses a previously verified compatible artifact only through a new higher `PublicationGeneration`; lower/equal generation requests fail;
14. migration class other than `COMPATIBLE_NO_MIGRATION` fails profile validation;
15. restart/bootstrap path cannot select latest/old content automatically and remains fail-closed without an exact authorized publication plan;
16. compile-fail/public-surface regression proving no live activation API is exposed;
17. `GameplayAvailability::UnavailableBootstrap` remains true.

Required integration evidence remains: whole-diff self-review, genuinely independent exact-head architecture/security review with `P0=0 / P1=0 / P2=0`, applicable canonical CI/governance, zero unresolved material threads, normal FULL Merge Queue and protected-main readback.

## 15. #433 completion rule and handoff

#433 is resolved only after this architecture decision itself has passed its required exact-head independent review, canonical checks, FULL Merge Queue and protected-main readback.

Then the exact handoff is:

`#433 resolved -> RESOURCE_LIMITS_REGISTRY serialized update -> review/CI/FULL MQ/readback -> fresh CONTENT #54 allocation`

No step in that handoff grants live deployment/production activation authority.
