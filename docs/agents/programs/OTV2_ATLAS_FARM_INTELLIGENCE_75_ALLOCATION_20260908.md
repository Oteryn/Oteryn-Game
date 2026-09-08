# Atlas farm-intelligence source/export allocation

Coordinator: #162. Product/export issue: #75. Current remediation programme: #364.

## State

```yaml
allocation_id: OTV2-ATLAS-FARM-INTELLIGENCE-75-20260908
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
product_issue: 75
allocation_base_main_sha: ae103eb6538f3044659aba3e3af8efbe8014707c
allocation_state: NOT_ACTIVE
preparation_branch: coord/atlas-farm-intel-75-allocation-20260908
prospective_worker_branch: agent/atlas-farm-intelligence-75
worker_launch: NONE_UNTIL_PROTECTED_READBACK_AND_WORK_APPLICATION
risk: MEDIUM_PUBLIC_CONTRACT_STATIC_EXPORT
production_authority: FORBIDDEN
external_repository_write_authority: FORBIDDEN
```

This is an allocation-only document. It creates no implementation writer and grants no present contract, producer, workflow, runtime, production or external-repository mutation authority.

Material application requires exact-head qualification of this allocation, normal protected integration, protected-main readback, a fresh Work ownership/custody check and one explicit Work application to exactly one worker.

## Why this lane is safe to prepare now

Fresh Work preflight on protected `main@ae103eb6538f3044659aba3e3af8efbe8014707c` found the current critical-path writers elsewhere:

- WP3 remains the canonical #351 / PR #356 writer lineage;
- WP4 #329 / PR #335 remains a distinct durability lineage and is not activated by this task;
- Server Seam #247 remains dependency-held on its preserved lineage;
- #418 is already assigned to its sole CONTROL writer and PR #426 changes only two Atlas workflow files plus one repository regression test;
- #420 is already assigned to its own sole CONTROL writer;
- no branch matching the farm-intelligence lane exists, and protected main contains no `tools/game-atlas-farm-intelligence/**` implementation or `oteryn-game-atlas-farm-intelligence-v1` contract.

The post-blocker orchestration architecture explicitly lists Game-owned Atlas farm-intelligence export #75 as an independent later/non-critical lane which does not become a hidden prerequisite for the first gameplay vertical slice unless a live accepted contract proves otherwise.

This allocation therefore uses new path-local surfaces only and does not seize an active writer's files or dependency custody.

## Fresh consumer authority

Game #75 remains the Game-owned upstream requirement for the Atlas Item & Spawn Farm Explorer.

The historical Atlas design package is not stale-only evidence: Oteryn-Atlas PR #120 merged as `7130bea8e95016aa821eefd51b3dba9c18be09c7`, and the normative design is still present on current protected Atlas `main@1e68e3ca2d1bf94c18ed37d79fcf7f0424e31fc2`.

That consumer contract keeps Game authoritative for item identity, creature identity, loot relation/probability/quantity semantics, task identity/type/requirements, explicit weekly classification and any respawn cadence. Atlas owns only deterministic derived joins, estimates and presentation.

The preferred producer identity remains:

```text
oteryn-game-atlas-farm-intelligence-v1
```

Atlas remains `UPSTREAM_BLOCKED` / `UNAVAILABLE` for any capability not proven by an accepted compatible Game publication. This allocation grants no Atlas repository writes.

## Fresh Game source evidence

### Existing creature gameplay export is useful but not sufficient

Protected Game already has terminal `creature-gameplay-profiles-v1` delivery from #136 / PR #138. Its public monster loot rows expose:

```text
item_ref / item_name / item_resolution_state
chance_ppm
min_count
max_count
```

and use the stable creature entity identity seam shared with the static-creature/spawn product.

That product is valid read-only source evidence for #75, but it does not by itself satisfy the newer farm-intelligence acceptance boundary. In particular, #75/Atlas requires proof of exact per-kill roll semantics or an exact per-kill quantity PMF before exact target-quantity probability mathematics is exposed; probability also needs exact ruleset/profile/modifier context and base/static versus live/current classification.

Do not reinterpret `chance_ppm`, `min_count` and `max_count` as a complete PMF merely because they are already public fields.

### Current VSL content loot is non-production evidence

`apps/game-server/src/content/**` currently contains VSL evidence-only content structures and fixtures, including fixture loot weights. The model identifies its profile as non-production evidence. These files may be inspected read-only to understand native semantic direction, but their fixture values must not become farm-intelligence product facts or Reference-parity evidence.

### Current platform catalogue is not a repair source

The existing Game Platform catalogue explicitly describes its committed unsupported fixture as non-activatable and does not claim a complete native item, creature, loot, quest/task or related gameplay inventory. Platform catalogue data must not be promoted into missing Game farm authority.

## Exact prospective material lease after protected application

The future sole #75 worker may own only these paths:

```text
docs/contracts/OTERYN_GAME_ATLAS_FARM_INTELLIGENCE_V1.md
tools/game-atlas-farm-intelligence/**
docs/agents/tasks/active/OTV2-20260908-atlas-farm-intelligence-75.md
docs/superpowers/plans/2026-09-08-atlas-farm-intelligence-75.md
```

The new tool subtree includes its producer, validator, focused tests and bounded generated test fixtures only. It does not include another repository's data or proprietary/publication-forbidden assets.

No existing workflow is allocated. No existing Game Atlas producer is writable under this lease.

If a dedicated workflow, an existing workflow trigger, an existing producer/helper, an existing public contract, `apps/game-server/**`, root Cargo/workspace, registry, or another shared path proves materially necessary, the worker must stop before mutation with exact `SHARED_LEASE_REQUIRED = path :: reason`. It must not widen its own scope.

## Read-only dependency evidence

The material worker may inspect current protected sources, including at minimum:

```text
docs/contracts/OTERYN_GAME_ATLAS_CREATURE_GAMEPLAY_PROFILES_V1.md
tools/game-atlas-creature-gameplay/**
tools/game-atlas-creatures/**
tools/game-platform-catalog/**
apps/game-server/src/content/**
docs/architecture/**
```

It may also read the accepted Atlas Item & Spawn Farm Explorer design/plan/review on protected `Oteryn/Oteryn-Atlas`, but receives no external write authority.

Legacy/reference evidence may be read only through already accepted Game migration/reference boundaries and with exact provenance. Browser/runtime parsing of legacy OTBM/Lua/XML or external sites is forbidden.

## Source-qualification gate before strong capability claims

Before publishing a capability as `SUPPORTED`, the worker must prove the exact source/provenance and semantics for that family. Missing proof yields `PARTIAL`, `UNSUPPORTED`, `UNKNOWN` or the contract's reviewed equivalent; it never becomes an empty-success claim.

### 1. Items

Prove a Game-owned stable public item identity and display name. Existing unresolved `item_ref: null` rows remain unresolved. Do not hash display names, legacy IDs, client IDs or file paths into fake native item identity.

### 2. Creature identity

Bind loot relations to the existing stable creature entity identity seam that can be joined to the protected static placement product. Names are display facts only.

### 3. Loot probability

Prove the exact denominator/roll semantics and the exact context under which the probability is authoritative. Distinguish static/base evidence from live/current modified chance. If current source evidence only proves a bounded/static representation, publish exactly that limitation.

### 4. Loot quantity

Classify each relation as one of the source-proven forms needed by the Atlas consumer, such as:

- exact fixed quantity on a successful roll;
- exact finite discrete per-kill quantity distribution;
- bounded quantity with unproven internal distribution;
- unsupported quantity semantics.

Do not synthesize a uniform distribution from `min_count`/`max_count`. Exact expected-kill / hitting-time capability is unavailable when the exact PMF/process is not proven.

### 5. Placement/farm-supply semantics

Read current placement publications rather than duplicating them. Prove or preserve placement weight, alternative-spawn semantics, conditional/event/quest/world-change activation and stable spawn-group identity where the consumer requires them. Equal geometry is not stable spawn-group identity.

If those semantics are absent from accepted current placement products, farm capacity/yield capability remains partial or unsupported; #75 does not seize existing placement-producer ownership to add them.

### 6. Task semantics

Search accepted Game-owned source evidence for stable task identity, delivery-item requirements, creature-kill requirements, grouped/multi-requirement structure, credit semantics and explicit weekly classification.

Do not flatten richer grouped semantics into a single guessed row. If no accepted authoritative task catalogue is present, publish the task capability as unsupported/unknown rather than inventing one.

### 7. Respawn cadence

Do not promote an existing static field or legacy value into live respawn authority without an accepted source contract proving its exact semantics and context. Missing proof means `UNSUPPORTED`/`UNKNOWN`.

### 8. Provenance and completeness

Bind every source family to exact repository/source revisions, digests and capability/completeness state. One supported subsection must not make sibling subsections appear complete.

### 9. Resource bounds

Census the exact admitted source corpus before freezing producer hard limits. Do not copy unrelated Atlas limits or invent production ceilings. Generated/test fixtures remain within separate explicitly test-only limits when needed.

## Producer/contract requirements after source qualification

The new Game-owned product must be deterministic, bounded and public-safe.

At minimum:

- canonical UTF-8 JSON with deterministic key/record ordering;
- exact contract/schema revision and Game producer revision;
- source/provenance tuple(s) and semantic digest;
- explicit capability/completeness states;
- stable Game-owned item/creature/task identities only when proven;
- lossless integer/rational probability representation appropriate to the proven source semantics;
- explicit quantity-model classification and exact PMF only when proven;
- no floating-point probability authority;
- bounded record/string/file counts and bytes;
- duplicate/dangling/conflicting identity rejection;
- safe relative output paths only;
- deterministic double-export byte/digest equality;
- corruption, malformed input, unsupported schema and hard-limit rejection;
- no wall-clock time, machine paths, runner IDs, secrets or private/live player state in canonical output;
- default-deny publication allowlist;
- truthful empty versus unsupported distinction.

A current-source snapshot may be produced only when the admitted source is available and provenance-qualified. Otherwise the product must publish truthful capability metadata and the exact blocker without fabricating facts.

## TDD / qualification obligations

The material worker must create focused RED/GREEN tests within its owned tool subtree for at least:

- deterministic ordering/bytes/digests independent of source enumeration order;
- stable item and creature identity binding;
- unresolved item identity remaining unresolved;
- exact probability denominator/roll validation where supported;
- quantity fixed/PMF/bounded/unsupported distinction;
- zero-yield/unreachable target handling where exact PMF capability exists;
- duplicate/dangling/conflicting relations;
- task delivery/kill/grouped requirements when supported and explicit unsupported task capability otherwise;
- explicit weekly semantics versus ordinary/custom kill target;
- provenance mismatch and mixed-generation rejection;
- malformed/corrupt/oversized product rejection;
- no script execution, web scraping or browser-side legacy parsing;
- canonical capability-state semantics for empty, partial, unsupported and unknown families.

Material public-contract changes require exact-head whole-diff self-review and the independent review required by current repository policy. Canonical exact-head repository checks must pass unchanged.

Runtime gameplay E2E is `NOT_APPLICABLE` when the final delivery remains a static export/read-model only; the PR must state the concrete reason. The export itself still requires real deterministic producer/validator execution against its admitted source or an explicit blocked/unsupported classification.

## Explicit exclusions

No authority is granted for:

- WP2/WP3/WP4/WP5 or Server Seam implementation paths;
- #418 / PR #426 workflow or regression paths;
- #420 CONTROL paths;
- any `.github/workflows/**` change;
- existing `tools/game-atlas-creature-gameplay/**`, `tools/game-atlas-creatures/**`, static/fullworld/semantic-search producer semantics or identity helpers;
- `apps/game-server/src/content/**`, domain/runtime, Cargo/workspace, migrations, registries, shared composition roots or production server code;
- Atlas/Platform/META/Reference repository writes;
- production/live data, credentials, deployment or protected environment changes;
- TibiaRoute/wiki/fansite/browser-runtime authority;
- Reference parity claims;
- guessed task/weekly/respawn/drop formulas;
- farm-time/KPH/spatial-cluster calculations, which remain Atlas-derived consumer behavior.

## Activation / integration lifecycle

```text
this allocation-only document
-> exact-head self-review
-> independent review appropriate to the control/public-contract allocation risk
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work active-owner/path/custody check
-> explicit application to ONE sole #75 worker
-> worker source-qualification gate
-> RED/GREEN producer + contract only within exact lease
-> exact-head whole-diff self-review + required independent review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> #75 capability/readiness reconciliation
```

The allocation does not make #75 a prerequisite for WP3, WP4, WP5, G0, Server Seam, Movement or Combat. It does not activate Atlas #114 and does not claim an Atlas consumer deployment.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.

Runtime product E2E is `NOT_APPLICABLE` to this allocation-only document because it changes no executable product/runtime surface.
