# DUR-04 — First Production Content Profile

- Decision: `DUR04-FIRST-PROD-CONTENT-01`
- Profile: `FIRST_PRODUCTION_CONTENT_PROFILE/v1`
- Date: 2026-09-09
- Resolves: #433
- Next consumer after registry integration/readback: CONTENT #54
- Status: **accepted only when this exact decision is present on protected `main`; branch/PR copies are proposal evidence only**

## 1. Decision

The first production-capable CONTENT slice is a deliberately narrow **production canary profile**. It is not a representative full-world scale and it does not choose a permanent World Project / World Bundle design.

The profile keeps only the semantic shape already proven by VSL-CONTENT-01 and the merged CONTENT evidence seam, but production is a **new trust profile**. `evidence:*`, fixture-only and synthetic authority markers cannot be renamed or promoted into production.

No live deployment, gameplay availability, permanent authoring format, permanent runtime bundle format, compression, CDN/signing topology or script runtime is authorized by this decision.

## 2. Evidence basis

Protected evidence used by this decision:

- DUR-04 requires a typed deterministic graph, separate server/client projections, an untrusted bounded loader, immutable staging, explicit activation, compatible rollback and registered maxima before implementation acceptance.
- VSL-CONTENT-01 establishes the minimum movement/combat semantic set and explicitly says scripts, NPCs, quests, houses, market, broad import and a final physical format are not required for the first slice.
- Current `apps/game-server/src/content/**` proves deterministic ordering, checked arithmetic, SHA-256 artifact/section integrity, strict unknown-critical rejection and fail-closed server/client pairing.
- The current merged VSL shape is exactly 3 cells, 21 aggregate definitions, 22 server records, 6 client-safe records and 24 counted references. Those values are used to define an intentionally tiny production canary; they are not claimed to represent production world scale.
- Current record kinds have fixed field counts and no known record exceeds 8 fields.
- Current framing evidence is 24-byte header + two 48-byte section-table entries + 32-byte trailer + 15 manifest strings + a length-prefixed record body. A profile-specific replaceable v1 framing with that bounded shape is sufficient to prove production build/load/stage/publication without freezing `.omap`, `.owb` or a permanent container.
- The active resource registry already accepts 128 UTF-8 bytes for a production semantic identifier (`FND02-CLIENT-BUILD-ID-BYTES`). This profile restricts keys/revisions/presentation/ruleset tokens to ASCII semantic identifiers, so 128 bytes is adopted on that independent production-identifier basis, not copied from `evidence:test-v1`.
- Content Format Spike #95/#125 remains evidence only. Its 64 MiB artifact, 2 MiB raw chunk, 4,096 chunks, 512-byte strings, depth 16, collection 100,000 and 64:1 compression ratio are not production limits here.

## 3. Exact first production scope

### Compiler IN

The compiler receives exactly one trusted-build, typed in-memory graph; it does not parse a permanent serialized World Project.

The graph contains exactly:

- 1 package and 1 world identity;
- 1 Region, 1 Area, 1 Terrain;
- 3 Cells on exactly 1 floor;
- 1 local Relocation;
- 1 deterministic Behavior policy reference;
- 3 project-owned Presentation/asset semantic references;
- 1 Creature;
- 1 Spawn with explicit GAME-CHANNEL multiplicity/eligibility/recovery and population ceiling 1;
- 1 Formula/Ruleset profile reference to already accepted gameplay semantics; CONTENT does not embed guessed executable formula code;
- 1 Effect, 1 Ability, 1 materializable Item;
- 1 Loot Table containing exactly 1 Loot Entry;
- 1 XP definition;
- 1 deterministic RNG purpose key and RNG profile identity. Root/secret seed remains simulation-owned.

Compiler obligations:

- reject evidence/fixture/synthetic production authority markers;
- deterministic canonical ordering;
- duplicate/missing-reference rejection;
- all `REQUIRED_NOW` checks before content-controlled growth/allocation;
- one server-authoritative and one allowlisted client-safe projection;
- exact revision binding for package/content/map/ruleset/world-policy/compiler/canonicalization/Content-Lock/provenance/simulation/content-profile identity;
- SHA-256 artifact/critical-section digests;
- rejection of every capability not in this profile.

### Compiler OUT

No filesystem/project-tree parser, serialized authoring format, package dependency resolver, aliases, broad OTBM/Crystal import, scripts, NPC/quest/house/market content, embedded asset blobs, compression/chunk/delta generation, signing/CDN logic or unbounded publication report.

### Loader IN

Production artifacts are untrusted bytes. Before staging the loader validates, in order:

1. total artifact byte limit;
2. profile magic/version and flags;
3. exactly two known critical sections: manifest + body;
4. checked offsets/ranges, no overlap/truncation;
5. section and artifact SHA-256;
6. exact 15-field manifest and per-field byte limits;
7. exact record count, record byte limit, known kind and exact field count;
8. semantic identifiers/references/source classification/projection allowlist;
9. exact server/client revision compatibility;
10. exact expected digests and revision set from the caller's separately authorized publication plan.

No network fetch occurs and integrity evidence never becomes deployment authority.

### Staging IN

- staged content is immutable and non-authoritative;
- maximum one staged pair per world/profile;
- immutable raw artifact bytes are separately bounded and are not charged to the decoded staging heap budget;
- all heap allocations derived from decoded untrusted content are charged to one 32,768-byte budget;
- compact offset/index storage is mandatory; unbounded per-string/per-record heap growth is forbidden;
- staged identity includes exact pair digests/revisions, expected current publication generation and proposed next generation;
- staging never mutates active publication state.

### Runtime IN

Only an **internal CONTENT publication kernel** is in scope:

- immutable content lookup;
- one atomic publication pointer per world/profile;
- exact generation pinning for every new authoritative simulation scope;
- at most one draining predecessor generation;
- stale-generation rejection;
- compatible rollback as a new monotonically increasing publication generation.

There is no public CLI/admin/Platform route, deployment controller, automatic startup selection or change to `GameplayAvailability::UnavailableBootstrap`.

## 4. `REQUIRED_NOW`

Every row below is an accepted production hard maximum only for `FIRST_PRODUCTION_CONTENT_PROFILE/v1`. Any expansion requires a future profile/registry decision.

| ID | Dimension | Unit | Hard max | Evidence / derivation | Boundary / denial obligation |
|---|---|---|---:|---|---|
| `DUR04-FPC-PACKAGES` | packages | packages | 1 | one-package profile, no resolver | 1 pass; 2 reject before graph resolution |
| `DUR04-FPC-REGIONS` | Regions | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-AREAS` | Areas | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-TERRAINS` | Terrains | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-CELLS` | Cells | cells | 3 | start + blocked + relocation target VSL shape | 3 pass; 4 reject before cell/index allocation |
| `DUR04-FPC-FLOORS` | distinct floors | floors | 1 | single-floor canary | second floor rejected |
| `DUR04-FPC-RELOCATIONS` | Relocations | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-BEHAVIORS` | Behavior refs | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-PRESENTATIONS` | Presentation refs | definitions | 3 | creature + ability + item presentation | 3 pass; 4 reject |
| `DUR04-FPC-CREATURES` | Creatures | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-SPAWNS` | Spawns | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-SPAWN-POPULATION` | population per spawn | entities | 1 | deterministic canary without scale assumption | 1 pass; 2 reject before runtime allocation |
| `DUR04-FPC-FORMULA-PROFILES` | formula/ruleset refs | definitions | 1 | one accepted policy ref | max pass; max+1 reject |
| `DUR04-FPC-EFFECTS` | Effects | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-ABILITIES` | Abilities | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-ITEMS` | Items | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-LOOT-TABLES` | Loot Tables | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-LOOT-ENTRIES` | Loot Entries | entries | 1 | one entry in one table | max pass; max+1 reject |
| `DUR04-FPC-XP-DEFINITIONS` | XP definitions | definitions | 1 | closed canary shape | max pass; max+1 reject |
| `DUR04-FPC-RNG-PURPOSES` | RNG purpose keys | keys | 1 | one deterministic loot purpose | max pass; max+1 reject |
| `DUR04-FPC-DEFINITIONS` | aggregate definitions | definitions | 21 | sum of exact category counts | 21 pass; 22 reject before set allocation |
| `DUR04-FPC-REFERENCES` | aggregate references | references | 24 | current compiler formula over the closed shape | 24 pass; 25 reject before collection growth |
| `DUR04-FPC-KEY-BYTES` | semantic key/id | ASCII bytes | 128 | independent production identifier precedent | 128 pass; 129 reject before allocation/copy |
| `DUR04-FPC-FIELD-STRING-BYTES` | revision/policy/presentation field | ASCII bytes | 128 | same semantic-identifier class; no free text/blob | 128 pass; 129 reject before allocation/copy |
| `DUR04-FPC-MANIFEST-FIELDS` | manifest fields | fields | 15 | exact v1 manifest | only 15 accepted |
| `DUR04-FPC-SERVER-RECORDS` | server records | records | 22 | 21 definitions + RNG context | 22 pass; 23 reject before vector allocation |
| `DUR04-FPC-CLIENT-RECORDS` | client-safe records | records | 6 | 3 presentations + creature + ability + item | 6 pass; 7 reject before vector allocation |
| `DUR04-FPC-RECORD-BYTES` | one record | bytes | 1,042 | max 8 fields: `2 + 8*(2+128)` | 1,042 pass; 1,043 reject before copy/decode |
| `DUR04-FPC-SECTIONS` | sections | sections | 2 | exact manifest + body | missing/extra/unknown reject |
| `DUR04-FPC-SECTION-BYTES` | section bytes | bytes | 9,626 | exact worst-case server body | max pass; max+1 reject before processing |
| `DUR04-FPC-SERVER-ARTIFACT-BYTES` | server artifact | bytes | 11,728 | exact framing calculation below | max pass; max+1 reject before parse/allocation |
| `DUR04-FPC-CLIENT-ARTIFACT-BYTES` | client artifact | bytes | 3,702 | exact framing calculation below | max pass; max+1 reject before parse/allocation |
| `DUR04-FPC-ARTIFACT-PAIR-BYTES` | artifact pair | bytes | 15,430 | 11,728 + 3,702 | max pass; max+1 reject before staging |
| `DUR04-FPC-PROJECTIONS` | artifacts/projections per generation | projections | 2 | exact server + client pair | missing/extra reject |
| `DUR04-FPC-STAGING-HEAP-BYTES` | decoded untrusted staging heap | bytes | 32,768 | explicit sub-budget below | next charged byte rejects, active unchanged |
| `DUR04-FPC-STAGED-CANDIDATES` | simultaneous staged pairs | candidates | 1 | no parallel activation needed | second candidate rejected, previous state preserved |
| `DUR04-FPC-RESIDENT-GENERATIONS` | distinct resident generations | generations | 2 | current + one draining predecessor | third distinct generation blocks activation |

### Exact byte derivation

A record field is encoded as two length bytes plus at most 128 data bytes. The largest known record has eight fields, so its maximum is `2 + 8*(2+128) = 1,042` bytes before the outer 4-byte record-length prefix.

The exact server-record distribution is five 1-field, six 2-field, five 3-field, one 4-field, one 5-field and four 8-field records. Therefore the body maximum is:

`4 + 5*136 + 6*266 + 5*396 + 526 + 656 + 4*1046 = 9,626 bytes`.

The six client records are all two-field records:

`4 + 6*266 = 1,600 bytes`.

The manifest maximum is `15*(2+128) = 1,950 bytes`.

Thus:

- server artifact: `24 + 2*48 + 1,950 + 9,626 + 32 = 11,728` bytes;
- client artifact: `24 + 2*48 + 1,950 + 1,600 + 32 = 3,702` bytes;
- pair: `15,430` bytes.

### Exact staging-memory derivation

Raw artifact bytes are already bounded by the 15,430-byte pair limit and remain outside the decoded staging allocator.

Worst-case decoded semantic field payload is 14,720 bytes:

- server: 15 manifest + 73 record fields = 88 fields; `88*128 = 11,264` bytes;
- client: 15 manifest + 12 record fields = 27 fields; `27*128 = 3,456` bytes.

The required compact staging representation has these additional upper budgets:

- 115 string descriptors * 8 bytes = 920;
- 28 record descriptors * 8 bytes = 224;
- 21 definition-index entries * 8 bytes = 168;
- 24 reference entries * 8 bytes = 192;
- artifact/section/revision descriptors: <=2,048 bytes;
- bounded validation scratch: <=8,192 bytes.

Total charged upper bound is `14,720 + 920 + 224 + 168 + 192 + 2,048 + 8,192 = 26,464` bytes. The accepted 32,768-byte hard cap leaves 6,304 bytes for alignment/fixed allocator bookkeeping. An implementation representation that cannot fit must fail this profile rather than raise the limit.

No numeric maximum above is inherited from the evidence test profile or Content Format Spike.

## 5. `EXCLUDED_FAIL_CLOSED`

These resource dimensions/capabilities are absent from v1 and cannot be activated by input, config or artifact metadata:

| Dimension/capability | Why unreachable | Fail-closed boundary |
|---|---|---|
| script component bytes | no script definition/section | compiler rejects graph; loader rejects unknown record/section/capability |
| script instance count | no script runtime/linker | no construction API; script capability invalid |
| Wasm memory/pages | no Wasm runtime | script metadata invalid |
| Wasm table/elements | no Wasm runtime | script metadata invalid |
| script fuel | no script execution | no accepted field/API |
| script host-call count | no host ABI | no accepted field/API |
| script query/result collection count/bytes | no script query imports | no accepted field/API |
| script action-plan count/bytes | no action-plan API | no accepted field/API |
| persistent extension-state bytes | no extension-state/persistence capability | schema/compiler rejects it |
| NPC/quest/house/market/broad catalog records | outside first slice | unknown production record kind rejected |
| ambient filesystem/network/process/env access | content is non-authoritative data | no capability/API; tokens are not paths/URLs/commands |
| network artifact/dependency fetch | exact bytes must already be supplied | loader has no network resolver |
| fixture/evidence artifact as production | trust profiles are distinct | production compiler/loader reject evidence/synthetic/fixture authority markers |
| unknown records/fields/flags/sections | v1 is a closed schema | loader rejects all unknown v1 critical or optional syntax |
| migration other than `COMPATIBLE_NO_MIGRATION` | canary adds no persisted reinterpretation | validation rejects migration-required activation |
| automatic/live activation endpoint | deployment authority is external | no public CLI/admin/Platform/live route |

## 6. `DEFERRED_REQUIRES_FUTURE_DECISION`

These potential DUR-04 dimensions deliberately receive no numeric limit now because no v1 code path consumes them. Attempts remain fail-closed until a future accepted decision adds limits.

| Deferred dimension | Why unnecessary now | Trigger for new decision |
|---|---|---|
| serialized World Project source bytes | typed in-memory compiler input | first serialized authoring/import format |
| source file count/path length/tree/nesting depth | no filesystem/project parser | first Studio/file-tree build path |
| package dependency count/depth | one package, no resolver | first multi-package publication |
| alias count/alias depth | no aliases | first alias/deprecation migration capability |
| chunk count/raw chunk bytes/chunk dimensions/floor packing | record-based replaceable canary framing | first chunked/broad-world runtime bundle |
| compressed/decompressed bytes and ratio | compression forbidden in v1 | first compression proposal |
| broad spatial density/index fan-out | only fixed 3-cell sparse canary | first world-scale spatial/streaming profile |
| embedded asset/blob bytes and asset-pack count | presentation keys only | first embedded/client asset package |
| compile/import report bytes | bounded status/digests only | standardized publication/import report |
| permanent World Bundle total bytes | v1 has no perpetual compatibility promise | permanent physical-format decision after real import/Studio/runtime evidence |

A deferred dimension may not be implemented with an implicit or unlimited default.

## 7. Activation boundary

Only this progression is legal:

`RECEIVED_OR_BUILT -> VALIDATED -> STAGED -> ACTIVE`.

`VALIDATED` means profile/schema/limits/integrity/semantics/references/revision pair and exact authorized-publication expectation all pass. `STAGED` is immutable and non-authoritative.

### Commit point

Each world/profile owns one atomic publication pointer containing:

`publication_generation + exact revision set + server digest + client digest + staged handle`.

Activation provides expected current generation (or explicit empty bootstrap), the staged candidate, and `next_generation = current + 1` using checked arithmetic. The **only commit point** is one compare-and-swap of the complete pointer. No section/projection/definition/scope is published separately.

Invalid, stale, incompatible, oversized or failed-CAS content never becomes active. Failure leaves the prior pointer unchanged.

## 8. Failure, recovery and rollback

### Before commit

Any parse/integrity/limit/semantic/compatibility/authorization-plan/staging failure rejects the candidate. Active state is unchanged and no fallback is auto-selected.

### During activation

Atomic CAS succeeds completely or not at all. Stale expected generation, generation overflow or CAS failure preserves current active state. There is no partial activation.

### Restart

CONTENT never scans a directory/network/repository/timestamp/generation and chooses “latest”. After restart, gameplay scope creation remains fail-closed until a separately authorized publication plan supplies exact artifact bytes/digests/revisions and an acceptable publication generation/floor. Those exact bytes are revalidated and restaged before the in-process commit.

Missing/corrupt/stale/incompatible/unauthorized bytes remain inactive.

### Last-good / rollback

A previously verified compatible artifact pair may be selected only by a separately authorized publication plan and is activated as a **new higher publication generation**. Rollback never decrements generation. Example: B at generation 8 -> rollback to bytes of A at generation 9.

`content_revision` is opaque, not orderable. Monotonicity is the publication generation.

Only `COMPATIBLE_NO_MIGRATION` is accepted in this profile. Any normalize/explicit migration/incompatible/removal policy is a future decision. No automatic post-commit rollback occurs.

## 9. Coexistence/version rules

At most two distinct generations may coexist:

- current generation for newly created scopes;
- one draining predecessor pinned by scopes that existed before the switch.

A scope is pinned to one exact generation for its lifetime; hot in-place mutation is forbidden. A third distinct generation cannot activate until the predecessor drains.

Compatibility requires exact profile/schema and all meaning-bearing revision identities: content, map, ruleset, world policy, compiler/canonicalization, Content Lock/provenance, simulation profile and this content profile. Server/client digests must bind the same revision set.

## 10. Security/trust boundary

- SHA-256 proves byte integrity, not publisher/deployment authority.
- Caller-supplied expected digests/revisions must come from a separately trusted deployment/control-plane decision; signing topology remains unselected.
- Tests/CI/review/merge do not grant production activation authority.
- Evidence/fixture artifacts can never be accepted as production.
- malformed/corrupt/truncated/oversize/stale/incompatible/unknown input fails closed;
- no content string becomes a filesystem path, URL, command, environment variable, SQL statement or authority token;
- client-safe projection remains allowlist-generated and never becomes server authority.

## 11. Deliberate non-decisions

This decision does not choose permanent `.omap`/`.owb`/World Project/World Bundle format, authoring serialization, permanent chunks/floor packing, compression, patch/delta/CDN, signing/trust-root topology, script runtime/WIT ABI, broad world scale or live rollout orchestration.

The v1 framing is replaceable and scoped only to this canary. A future permanent format may require recompilation/new profile; v1 creates no perpetual parse-compatibility promise.

## 12. Authority separation

**Architecture implementation authority:** after this decision is protected and the exact registry append is separately merged/read back, a fresh #54 allocation may authorize only this bounded implementation. This decision alone grants no CONTENT write branch.

**Repository integration authority:** normal repository control plane only — required review/checks, FULL Merge Queue and protected-main readback. No direct merge/protection bypass.

**Live deployment / production activation authority:** **NONE**. No branch, PR, CI pass, protected merge, artifact or internal publication kernel may mutate a live/protected environment without separate explicit deployment authority.

## 13. Mechanical next steps

### Exact serialized registry append

`docs/architecture/DUR-04_FIRST_PRODUCTION_CONTENT_PROFILE_REGISTRY_APPEND.json` is the canonical mechanically prepared append packet. It matches the live registry shape (`configurable_range.minimum/maximum`, array `boundary_tests`), contains every `REQUIRED_NOW` row above and may be serialized only after fresh protected registry readback. Its numeric maxima/units/boundary obligations may not be changed during serialization without reopening #433.

### Fresh CONTENT #54 allocation after registry protected readback

Writable runtime paths, and no others unless a concrete same-scope compile dependency is proven:

- `apps/game-server/src/content/model.rs`
- `apps/game-server/src/content/compiler.rs`
- `apps/game-server/src/content/artifact.rs`
- `apps/game-server/src/content/production.rs` (new if useful)
- `apps/game-server/src/content/mod.rs`
- `apps/game-server/src/content/tests.rs`

`apps/game-server/src/lib.rs` is not allocated to enable gameplay/live activation; `GameplayAvailability::UnavailableBootstrap` stays unchanged.

Required symbols/semantics:

- preserve `EvidenceLimits`, `CompileTarget::Evidence` and evidence fixtures as non-production;
- add distinct `FirstProductionContentLimitsV1` bound exactly to protected registry rows;
- add distinct production profile/compile target, never rename `evidence:*`;
- production semantic validation for every exclusion/defer fence;
- replaceable v1 server/client artifact profile;
- bounded staging allocator;
- checked `PublicationGeneration`;
- internal/testable `ContentPublicationKernelV1` with expected-generation CAS;
- scope pin/release with max two resident generations;
- no public deployment activation surface.

Exact-head test obligations:

1. table-driven at-limit and max+1 denial for every registry row;
2. deterministic byte-identical compile under enumeration shuffle;
3. duplicate/missing-reference and malformed semantic graph rejection;
4. production rejects evidence/fixture/synthetic authority markers;
5. projection leak and revision-pair mismatch rejection;
6. corrupt/truncated/unknown/overflow/oversize rejection before staging;
7. scripts/compression/chunks/source files/dependencies/aliases and all excluded/deferred capabilities fail closed;
8. staging allocator exhaustion preserves active state;
9. valid pair advances `validated -> staged -> active` only at atomic commit;
10. pre-commit failure/stale CAS preserves previous active generation;
11. no mixed-generation scope;
12. third distinct resident generation rejected until drain;
13. rollback only as a new higher publication generation to verified compatible bytes;
14. non-`COMPATIBLE_NO_MIGRATION` rejected;
15. restart cannot auto-select content;
16. compile-fail/public-surface regression proving no live activation API;
17. `GameplayAvailability::UnavailableBootstrap` remains true.

## 14. #433 resolution condition

#433 is `RESOLVED` only after this decision and its serialized append packet pass:

- genuinely independent exact-head architecture/security review with `P0=0 / P1=0 / P2=0`;
- applicable canonical CI/governance;
- zero unresolved material review threads;
- normal FULL Merge Queue;
- protected-main readback.

Then the exact handoff is:

`#433 resolved -> RESOURCE_LIMITS_REGISTRY serialized update -> review/CI/FULL MQ/readback -> fresh CONTENT #54 allocation`

No step grants live deployment authority.
