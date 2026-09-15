# Oteryn ecosystem repository topology — senior developer / programmer / project manager second pass

- Date: 2026-08-15
- Review base: `blakinio/Oteryn-v2@dcc4a7773a48ea07720ae3f19f090bcfee2d266b`
- Previous review: `docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_REVIEW_2026-08-15.md`
- Scope: architecture/delivery review only
- Runtime implementation authority: **NONE**
- Repository reorganization authority: **NONE**
- External repository write authority: **NONE**

## Executive verdict

**UPHOLD `ACCEPT_WITH_CHANGES`, WITH STRONGER DELIVERY CONSTRAINTS.**

The first-pass repository topology is technically sound. A second pass from senior development, maintenance and project-management perspectives does **not** justify splitting the native Client, Server or `protocol-oteryn`, and it does not justify moving canonical World/Content, compiler/validation or Oteryn Studio out of the current Game-side repository today.

The strongest correction is about **timing and execution**, not the logical boundaries:

> Freeze the topology and cross-repository contracts first; do not turn repository reorganization into a parallel product programme while the native runtime is largely `NOT_STARTED`.

The target topology remains:

```text
Oteryn              -> thin ecosystem coordination/meta plane
Oteryn-Game         -> native game product + canonical game-content toolchain
Oteryn-Platform     -> web/application/control plane
Oteryn-Atlas        -> derived browser-map/read-model product
```

However, physical creation/renaming/extraction must be demand-driven and sequenced around product evidence. A box on the target diagram is not by itself a reason to create a repository, CI surface, release train or governance surface now.

## Verified basis

- FACT — Current `main` for this second pass is `dcc4a7773a48ea07720ae3f19f090bcfee2d266b`.
- FACT — The first-pass topology review is merged and lifecycle-closed. Its verdict is `ACCEPT_WITH_CHANGES`.
- FACT — Root governance defines Oteryn-v2 as the greenfield native Rust gameplay stack containing authoritative Rust server, native Rust client, shared Rust domain/protocol/tooling crates and one `protocol-oteryn`; Platform remains external for portal/Identity/Game Gateway/World Registry.
- FACT — ADR-0002 explicitly chooses one canonical Rust repository for native Client, authoritative Server, shared domain/identifier types, `protocol-oteryn` schemas/codecs/fixtures and cross-client/server E2E because permanent separation increases coordinated-version and drift cost.
- FACT — ADR-0005 explicitly places the native World/Content model, editable World Project, deterministic compiler/validation, immutable World Bundle, bounded legacy conversion and integrated Oteryn Studio on the same Game-side architecture boundary.
- FACT — ADR-0005 also requires canonical schema/domain crates to remain independent from Tauri/UI/SQL/network/renderer implementation details.
- FACT — `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` marks the migrated Rust client/workspace as `PROVEN`, while `protocol-oteryn`, GameNode/runtime, persistence and DUR-04 world/compiler/Studio implementation remain `NOT_STARTED` unless separately proven.
- FACT — `ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` already identifies the primary programme risk as mature architecture plus absent complete native vertical runtime and excessive simultaneous scope pressure.
- FACT — The current root Cargo workspace has 19 members and contains the native client/foundation/supporting crates, but no implemented Game Server, `protocol-oteryn`, canonical world/compiler or Studio workspace members yet.
- FACT — `BUILD_TEST_MATRIX.md` already uses path-proportional Rust validation behind one stable `Merge gate / validate` context; documentation-only changes do not automatically run the whole Rust matrix.
- FACT — `.github/CODEOWNERS` is intentionally restricted to repository control-plane paths and is not a general component-maintainer map.
- FACT — current `main` contains three active task records; current open PRs also include multiple unmerged architecture worker proposals and dependency-update PRs. Those branches are not canonical architecture until merged.

## What the first pass got right

### 1. `Oteryn-v2 -> Oteryn-Game` is the correct high-cohesion boundary

From a senior developer perspective, Client + Server + Protocol are exactly the components where atomic changes have unusually high value:

- protocol/schema changes can be implemented and tested against both ends on one exact commit;
- shared semantic identifiers do not need package publication or cross-repository version coordination for every change;
- compatibility fixtures and headless/native E2E can test one coherent source revision;
- refactors across domain/wire/client projection boundaries remain reviewable as one change rather than synchronized PR chains.

Splitting those components now would optimize organization before there is evidence of organizational scale that needs the split.

### 2. Canonical World/Content + compiler + validation belong with Game

These components define gameplay semantics and the runtime input contract. Moving them to Atlas, Platform or META would invert ownership and make a presentation/control-plane repository authoritative over game semantics.

OTBM parser and Legacy IR belong next to the canonical conversion boundary, but must stay downstream-only dependencies:

```text
legacy importer -> canonical model
canonical model -X-> legacy IR
runtime/compiler -X-> OTBM types
Atlas export    -X-> OTBM types
```

### 3. Atlas should remain a separate derived product

Atlas has a materially different workload and release profile:

- browser delivery;
- indexing/search;
- map-specific presentation;
- public-data classification;
- rebuildable derived read models;
- potentially different frontend/runtime technologies.

That is a real repository boundary, not merely a directory preference.

### 4. Platform remains a real separate bounded context

Identity, Accounts, GameAuth/Game Login Ticket, Game Gateway and web/application responsibilities have different security, deployment and ownership characteristics from authoritative gameplay. Recombining them to reduce repository count would worsen isolation.

## Second-pass corrections and stronger constraints

### Correction 1 — do not make physical topology migration compete with the first native vertical slice

This is the most important project-management finding.

The target topology may be accepted before runtime implementation, because wrong repository authority can create expensive migration later. But **physical repository operations should not become an independent critical-path programme unless a concrete downstream consumer needs them**.

The current programme already has a known product risk: architecture maturity exceeds runtime proof. Repository rename/create/migration work has real cost:

- references and provenance need reconciliation;
- branches/PRs can become stale;
- CI/release configuration needs duplication or migration;
- agents and maintainers must relearn ownership locations;
- cross-repo contract bookkeeping grows before producing gameplay evidence.

RECOMMENDATION — accept/freeze the topology contract now, but schedule physical operations in migration waves around product milestones rather than performing all topology work immediately.

### Correction 2 — META is a capability that should appear when it has a real neutral workload

The first pass correctly limits META authority. The second pass narrows **when the repository itself should exist**.

A META repository is justified when at least one real cross-repository responsibility cannot be cleanly owned by an existing repository without creating authority ambiguity, for example:

- Game + Platform + Atlas compatibility matrix;
- ecosystem release manifest spanning immutable artifacts from multiple repos;
- neutral cross-repository E2E composition;
- cross-repository ADR that genuinely governs multiple independently released products.

Until then, creating an empty or mostly-documentation META repository adds navigation, CI/governance and ownership overhead without removing a current blocker.

RECOMMENDATION — `Oteryn` META is part of the target architecture, but physical creation should be gated by its first concrete cross-repository contract/release/E2E consumer. Do not create placeholder structure merely to make the diagram complete.

### Correction 3 — one Git repository must not imply one release unit

`Oteryn-Game` should be one source repository but several independently identifiable deliverables:

- native Client build;
- Game Server build;
- `protocol-oteryn` major/schema contract;
- World Project/schema revision;
- World Bundle format/artifact digest;
- Oteryn Studio build;
- Atlas export schema/artifact revision.

A repository tag may be useful as source provenance, but it must not replace these compatibility identities.

RECOMMENDATION — use release manifests to bind exact compatible deliverables. Avoid a policy where every Studio-only or compiler-only change implies a Client+Server product release.

### Correction 4 — repository ownership is not equivalent to Git blob storage

ADR-0005 correctly gives Game ownership of content/assets. The second pass adds an operational distinction:

> `Oteryn-Game` owning canonical content does not mean every large binary source, generated bundle, cache, audio file or release artifact must live directly in ordinary Git history.

Heavy binary storage can create:

- clone/fetch cost;
- repository-history bloat;
- poor diff/review behavior;
- expensive CI cache invalidation;
- accidental inclusion of generated or rights-sensitive assets.

RECOMMENDATION — keep schemas, manifests, source relationships, provenance and reviewable textual/structured content under Game authority, but select large-blob/artifact storage separately when real asset sizes and authoring workflows are known. Do not prematurely create `Oteryn-Content`; first preserve the ownership boundary while leaving storage mechanics open.

### Correction 5 — Studio stays in Game, but must be a separate dependency and release island

Studio is still the weakest-cohesion member of `Oteryn-Game`. Keeping it together is correct now because its highest-value dependencies are the canonical schema, validation, compiler, asset pipeline and map renderer.

The code architecture must nevertheless make this dependency direction enforceable:

```text
Studio shell / UI
        |
        v
headless editor commands / preview APIs
        |
        v
world-project / content-registry / validation / compiler

canonical world/compiler crates
        -X-> Studio/Tauri/web UI
```

The Studio binary should be independently buildable and releasable. Client/server CI must not become dependent on Studio GUI availability merely because the source lives in the same repository.

### Correction 6 — Game owns Atlas export semantics, but ecosystem support requires consumer proof

The first pass assigns Atlas export schema/exporter ownership to Game. That remains the best single-owner model because only Game can safely project canonical semantic data into a public representation.

The second pass adds a control against unilateral producer breakage:

- Game may define and publish a new export schema revision;
- Atlas must prove consumer parsing/limits/indexing against that immutable revision;
- META compatibility/release manifests, once they exist, may mark a Game-export/Atlas-consumer pair ecosystem-supported only after both producer and consumer evidence exists.

This preserves one schema owner without turning producer ownership into permission to break a consumer silently.

### Correction 7 — full cross-repository E2E must not run on every local PR

A future META orchestration layer should not become a central CI tax or single point of developer friction.

Recommended gate placement:

- repository-local PRs: local focused/unit/integration/E2E according to affected paths;
- contract-affecting PRs: producer/consumer contract fixtures and targeted cross-repo compatibility evidence;
- protected main/nightly: selected wider ecosystem journeys;
- release candidate: complete named ecosystem manifest + cross-repository E2E.

A local Studio UI change should not require Platform+Game+Atlas system bring-up. A Game admission/protocol contract change may legitimately require targeted Platform/Game compatibility evidence.

### Correction 8 — CODEOWNERS and engineering accountability are different problems

Current CODEOWNERS is intentionally an enforced merge-authority mechanism for control-plane paths. Broadening it to every component would change repository governance and introduce review bottlenecks.

The project still needs human/agent discoverability for ownership as the Game repository grows.

RECOMMENDATION — if needed, add a **non-enforcing component maintainer/ownership map** separately from CODEOWNERS. Continue using task `owned_paths` as concurrency leases. Change CODEOWNERS only when mandatory approval semantics are deliberately desired.

## Repository-boundary decision by component

| Component | Keep in `Oteryn-Game` now? | Senior-dev/PM rationale |
|---|---|---|
| Native Client | **YES** | High atomic-change value with protocol/server/shared types; separate Windows build/release unit is enough for now. |
| Game Server | **YES** | Core authoritative product; no separate repository benefit proven. |
| `protocol-oteryn` | **YES** | Highest drift risk if split; schema/fixtures need one exact client/server source revision. |
| Shared game/domain crates | **YES** | Correct shared semantic owner; require strict dependency direction. |
| Canonical World/Content model | **YES** | Game semantics/source authority. |
| World compiler/validation/bundle | **YES** | Must evolve atomically with canonical model and runtime consumers. |
| OTBM/legacy importers | **YES, bounded** | Migration boundary belongs next to canonical conversion but must not contaminate canonical/runtime dependencies. |
| Oteryn Studio semantic core | **YES** | Tight coupling to canonical authoring/validation/compiler APIs. |
| Oteryn Studio UI shell | **YES FOR NOW** | Separate release/build island; future extraction is plausible if measured friction appears. |
| Atlas exporter/schema | **YES** | Producer-owned safe projection from canonical semantics. |
| Browser Atlas runtime/index/search | **NO** | Separate derived product and different operational lifecycle. |
| Platform services | **NO** | Existing independent control-plane/security boundary. |
| META orchestration | **NO, separate when needed** | Neutral cross-repo authority, but physical repository should be demand-triggered. |

## Explicit future extraction triggers

The correct strategy is **design extraction seams now, extract repositories only on evidence**.

### Client extraction trigger

Reconsider `Oteryn-Client` only if several of these become true:

- client and server are owned/released by genuinely independent teams;
- Windows/desktop dependency and CI load repeatedly blocks unrelated server delivery despite path-based CI;
- stable protocol/package compatibility is mature enough that atomic source changes are no longer the dominant safety mechanism;
- client release cadence materially diverges from Game source cadence;
- separate security/distribution governance creates a real boundary.

### Protocol extraction trigger

Reconsider `Oteryn-Protocol` only if:

- multiple independent repositories/languages become first-class producers/consumers;
- protocol schemas/SDKs require their own governed public release lifecycle;
- Game repository ownership creates measurable adoption or release blockers;
- independent protocol security ownership becomes necessary.

One extra consumer alone is not enough if atomic Game client/server evolution remains more valuable.

### Studio extraction trigger

Reconsider `Oteryn-Studio` only if:

- Studio gets a materially independent roadmap/team/release train;
- its GUI dependency graph materially degrades Game CI/build/maintenance despite isolation;
- external tooling needs the headless editor APIs independently;
- packaging/security/licensing boundaries differ enough to justify separate governance.

If extraction happens, canonical schema/compiler/validation should normally remain in Game; Studio becomes a consumer of versioned headless contracts.

### Content repository extraction trigger

Do **not** create `Oteryn-Content` now. Reconsider a separate authored-content repository only if content production obtains a genuinely different workflow:

- many non-code content authors;
- distinct permissions/review cadence;
- very large file/storage behavior incompatible with code repository operations;
- independent content release trains;
- localization/media production that materially overwhelms source-code workflows.

Even then, Game should remain owner of the canonical schema/compiler/validation contract. A content repository would contain authored instances/packages, not redefine the model.

### Server/service extraction trigger

Repository co-location does not prevent adjacent deployment services. Follow ADR-0015: extract a separate deployable service only when there is real independent authority, security, data ownership, scaling or failure-domain evidence. Do not create a repository merely because a service may later be a separate process.

## Recommended migration sequence

### Wave 0 — freeze architecture, move nothing

- accept one cross-repository topology ADR;
- define exact repository responsibilities and forbidden overlaps;
- define repository rename/history/alias/provenance policy;
- define Game->Atlas artifact-first boundary at the semantic level;
- keep implementation and production unchanged.

**PM rationale:** this prevents accidental architecture while preserving product velocity.

### Wave 1 — make `Oteryn-Game` internally extraction-safe

Before physical split/creation pressure grows:

- enforce dependency directions around protocol/domain/world/compiler/Studio;
- maintain path-proportional CI;
- keep independent Client/Server/Studio release identities;
- keep generated/release artifacts out of ordinary source history;
- inventory all current repository-coordinate references before rename.

No separate repositories are required for this wave.

### Wave 2 — rename current repository only when coordination cost is bounded

Conceptually rename `Oteryn-v2` to `Oteryn-Game` after:

- open PR/task migration impact is inventoried;
- cross-repository locks/provenance/URLs/package metadata/workflows are enumerated;
- historical references have an explicit preservation policy;
- rollback/correction plan is defined.

Do not combine rename with runtime refactors, client/server protocol changes or world-format implementation.

### Wave 3 — define and prove Game->Atlas export before Atlas becomes authoritative to users

Order:

```text
canonical Game world/content
 -> public allowlisted projection
 -> deterministic immutable full Atlas artifact
 -> consumer parser/limits tests
 -> Atlas indexing/read model
 -> browser presentation
```

Start with full snapshots. Add deltas only when measured publication size/time justifies them. A delta must bind exact base and target digests and full-snapshot recovery must remain available.

Integrity digest/provenance is required from the first contract. Production authenticity/signing policy may remain a later release-security gate if it is not required for the first isolated proof, but hashes must never be represented as publisher authentication.

### Wave 4 — create META when the first real neutral cross-repo workload exists

Create the META repository when it can immediately own at least one real artifact such as:

- ecosystem repository manifest;
- cross-repository topology ADR;
- immutable compatibility matrix;
- ecosystem release manifest;
- neutral E2E composition.

Do not move repository-local architecture or CI implementations there.

## CI and release model for a larger `Oteryn-Game`

### CI

Preserve one protected aggregate merge context while making sub-gates impact-aware.

Target path/risk classes should eventually distinguish at least:

- shared domain/protocol;
- client-only;
- server-only;
- world/compiler/content semantics;
- Studio-only;
- legacy importer;
- documentation/governance.

Shared protocol/domain/world-schema changes legitimately fan out to multiple gates. Studio shell changes should not fan out to server E2E unless a shared semantic contract changed.

The impact model should be driven by declared dependency/contract relationships, not only simple directory prefixes, so changing a shared crate still triggers its downstream consumers.

### Releases

Use independent artifact identities and an immutable compatibility manifest. A coherent ecosystem release may look conceptually like:

```text
game_source_commit
client_artifact_digest + client_build_id
server_artifact_digest + server_build_id
protocol_major + schema_revision
world_bundle_digest + world/content revisions
studio_build_id (when relevant)
platform_commit/artifacts
atlas_export_schema + export_digest
atlas_consumer_build
```

The manifest is a compatibility statement, not a requirement that every component was rebuilt or version-bumped simultaneously.

## Agent ownership and project-management implications

The current task-path lease model is appropriate for concurrent architecture work, but the second pass identifies two operational requirements as implementation grows:

1. **shared-crate coordination must be explicit** — protocol/domain/world-schema changes can affect several component owners and should be treated as high fan-out paths;
2. **component accountability should be discoverable without broadening merge gates** — a non-enforcing maintainer map can name primary/backup reviewers, affected release units and escalation owners.

Avoid permanent component branches. Prefer bounded task branches and small mergeable vertical slices, because long-lived Client/Server/World branches would recreate cross-repository drift inside one repository.

## Project-management risk register

### P0

- dual or ambiguous world authority between Game source, legacy OTBM and Atlas;
- public Atlas export leaking server-only/security-sensitive/unreleased content;
- META overriding repository-local authority or becoming a second source of truth;
- repository migration combined with unrelated runtime changes such that rollback/provenance cannot be cleanly reconstructed.

### P1

- repository-reorganization work delaying first native vertical runtime without protecting an immediate invariant;
- Game mega-CI where every small change triggers Client+Server+Studio+world work;
- producer-only Atlas schema rollout without consumer compatibility proof;
- large/generated binary content committed into normal Git history by default;
- one release/version identifier incorrectly representing client, server, protocol, world and Studio compatibility;
- long-lived component branches creating internal drift.

### P2

- Studio GUI dependencies leaking into canonical/headless crates;
- overuse of META for routine local decisions;
- premature `Oteryn-Content`/`Oteryn-Protocol`/`Oteryn-Studio` repositories increasing coordination cost;
- component ownership being invisible because CODEOWNERS is intentionally narrow;
- cross-repo E2E becoming mandatory for low-risk local PRs and slowing delivery.

### P3

- naming/documentation churn from `Oteryn-v2 -> Oteryn-Game`;
- temporary duplicated links/manifests during migration;
- developer navigation cost across Game/Platform/Atlas/META without a concise repository manifest.

## Decision timing test

### Must the logical repository topology be decided now?

**YES.**

It affects the future owner of canonical World/Content, legacy conversion, Atlas export and cross-repository contracts. Allowing those boundaries to emerge implicitly through implementation would create expensive dual authority.

### Must every physical repository be created/renamed now?

**NO.**

Current runtime implementation is largely not started. Physical topology work should happen when it unlocks a concrete consumer/migration or materially reduces current coordination risk.

### What downstream work is blocked without the logical decision?

- safe Atlas extraction/implementation;
- correct placement of OTBM import and canonical world/compiler work;
- neutral cross-repository compatibility/release governance;
- future repository rename planning without history/provenance ambiguity.

### What becomes harder if the logical boundary is delayed?

- Atlas may accidentally adopt OTBM or direct World Project access;
- Studio/world/compiler dependencies may grow in the wrong direction;
- Platform or META may duplicate game contracts;
- source-of-truth ownership becomes expensive to unwind.

### What evidence should trigger supersession/extraction later?

- measured CI/build/clone/release friction;
- independent teams and roadmaps;
- multiple independent protocol/tool consumers;
- security/data/deployment boundaries;
- large-content production workflows incompatible with the code repository;
- repeated cross-component delivery blockers that cannot be solved through dependency isolation and path-based CI.

### Deliberately not decided

- final large-asset storage technology;
- Git LFS versus object/artifact storage;
- Atlas physical serialization/container;
- CDN/publication transport;
- signing implementation/trust root;
- exact CI provider/cache strategy;
- exact META repository creation date;
- exact extraction thresholds expressed as arbitrary numbers;
- Studio viewport/UI technology beyond already accepted evidence-driven selection.

## Delta versus the first-pass review

The first-pass review is **not superseded**. This second pass strengthens it with the following execution rules:

1. topology authority should be frozen before implementation, but physical repo migration must not outrank native vertical-slice delivery by default;
2. META should be created only when it has an immediate neutral cross-repository consumer;
3. one Game repository must expose several independent release units rather than one synchronized product version;
4. Game ownership of content/assets does not mandate ordinary Git storage for large/generated binary data;
5. Studio must behave as an independently buildable/releasable dependency island inside Game;
6. Game-owned Atlas schema becomes ecosystem-supported only after Atlas consumer compatibility evidence;
7. cross-repo E2E belongs on contract/release-critical paths, not every local PR;
8. engineering ownership discovery should be separated from enforced CODEOWNERS semantics;
9. future Client/Protocol/Studio/Content splits require explicit measured extraction triggers.

## Final recommendation

Keep the first-pass target topology and **do not fragment `Oteryn-Game` now**.

The practical engineering strategy should be:

```text
one high-cohesion Game source repository
+ strict internal dependency boundaries
+ independent build/release units
+ path/dependency-aware CI
+ artifact-first external contracts
+ demand-driven repository extraction
```

`Oteryn-Game` remains the recommended future name because it accurately describes ownership of the playable native game product and its canonical content-authoring/build toolchain. The repository description must make that scope explicit so Studio/world tooling do not look accidental.

Do not create `Oteryn-Client`, `Oteryn-Server`, `Oteryn-Protocol`, `Oteryn-Studio` or `Oteryn-Content` today. Preserve extraction seams and revisit only when measured delivery, ownership, security or storage evidence makes the coordination cost of co-location greater than the current atomic-change advantage.

Create `Oteryn-Atlas` as a genuinely separate product when its Game-produced public artifact contract is ready for a consumer. Create `Oteryn` META when it has an immediate neutral cross-repository contract/release/E2E responsibility. Until then, do not spend product velocity on repository boxes that have no executable owner or consumer.
