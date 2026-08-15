# Verdict

ACCEPT_WITH_CHANGES

The proposed ecosystem direction is sound, but it is safe only after authority boundaries are made explicit. The strongest part of the proposal is not a new decision: current accepted Oteryn-v2 architecture already intentionally co-locates the native Rust Client, authoritative Rust Game Server, shared domain identifiers/contracts and `protocol-oteryn`, and separately places the canonical native World/Content model, deterministic compiler/bundle/validation pipeline, bounded legacy conversion and Oteryn Studio on the same product side of the Platform boundary.

The proposed future `Oteryn-Game` repository is therefore a valid continuation of the existing canonical repository boundary. The review does **not** recommend separate `Oteryn-Client`, `Oteryn-Server` or `Oteryn-Protocol` repositories now.

Two changes are required before the proposed ecosystem topology should become canonical cross-repository architecture:

1. the future `Oteryn` META repository must be defined as a **cross-repository coordination/contract plane**, not a new owner of local product architecture, runtime code, native client/server E2E implementation or product deployment;
2. `Oteryn-Game -> Oteryn-Atlas` must be frozen as a new explicit, versioned, immutable **artifact projection contract** before Atlas becomes a consumer of game-world data.

This review is documentation-only. It recommends later decisions; it does not rename repositories, create repositories, move code, alter accepted ADR semantics, implement Atlas, or authorize runtime/production work.

# Facts

- FACT — Review base `main` is commit `cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e`. The initially observed `5c86773be23059956dc887dc48b19b0228090b40` is the tree SHA of that commit, not the commit SHA; the task record was corrected before this review was written.
- FACT — Repository governance in root `AGENTS.md` and `AGENTS.override.md` requires task -> isolated branch -> PR, prohibits direct writes to `main`, requires full-diff self-review and exact-head validation, and forbids external-repository writes without separate authority.
- FACT — `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md` requires architecture to freeze only decisions that block safe downstream work, preserve explicit extension points and supersede accepted decisions only through explicit newer ADR/contract evidence.
- FACT — The current `Cargo.toml` has 19 workspace members. It contains `apps/client` and client/foundation/Platform-integration/test/tooling crates, but it does not yet contain `services/game-server`, `crates/protocol-oteryn`, canonical world/compiler crates or `apps/oteryn-studio`.
- FACT — `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` reports FND-01/VSL-02 native-client cutover as accepted/lifecycle-closed and `PROVEN`; FND-02 protocol, runtime, persistence and DUR-04 content/world architecture are accepted/lifecycle-closed where recorded but their corresponding runtime implementation remains `NOT_STARTED` unless separately proven.
- FACT — ADR-0001 accepts one native Rust Client, one authoritative Rust Game Server, one project-owned `protocol-oteryn`, Platform control-plane separation and a native multichannel world model.
- FACT — ADR-0002 explicitly makes `blakinio/Oteryn-v2` the canonical repository for the native Rust Client, authoritative Rust Game Server, shared domain/identifier types, `protocol-oteryn` schemas/codecs/fixtures, Rust Platform integration clients and cross-client/server E2E tests. It explicitly rejects permanently keeping the client in a separate repository because of protocol/shared-type drift and coordinated-version cost.
- FACT — ADR-0003 keeps Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry in `blakinio/Oteryn-Platform`; it explicitly rejects moving Game Gateway into Oteryn-v2.
- FACT — ADR-0005 accepts a native Oteryn World/Content model, editable Oteryn World Project, deterministic compiler/validation, immutable World Bundle, bounded legacy importer + Legacy Intermediate Representation, stable content identity and integrated Oteryn Studio. It explicitly rejects OTBM as the canonical editable or runtime format.
- FACT — DUR-04 further requires a typed canonical semantic content graph, exact Content Lock, deterministic compiler, explicit client/server projection, immutable content-addressed World Bundle, bounded fail-closed loader, legacy provenance and importer-boundary Legacy IR. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` is the live status authority marking DUR-04 accepted/lifecycle-closed; the historical DUR-04 document header still contains candidate/in-review wording.
- FACT — ADR-0007 makes Oteryn-v2 the owner of a shared native three-tier E2E platform spanning native client/headless client, Platform/Gateway, game server and persistence, with exact revision evidence.
- FACT — ADR-0008 makes `protocol-canary` reference-only migration evidence and forbids it from the production runtime/dependency/fallback graph.
- FACT — ADR-0011 keeps the migrated native client in Oteryn-v2 and defines its current fail-closed `pre-native-protocol` transition state until the accepted native protocol/runtime/admission implementations exist.
- FACT — ADR-0015 preserves one GameNode = one game-server process but deliberately leaves internal module/crate decomposition and justified adjacent-service extraction evidence-driven.
- FACT — `docs/agents/CROSS_REPO_CONTRACTS.md` currently recognizes Oteryn-Platform, Oteryn-v2, Otheryn and otclient as the relevant repositories and requires one canonical contract owner, exact immutable revisions, producers/consumers and rollout/rollback order for material cross-repository contracts.
- FACT — `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` currently contains Platform/native-gameplay reconciliation and canonical `protocol-oteryn-v1-foundation` entries. It has no Atlas contract.
- FACT — searching the current Oteryn-v2 repository for `Atlas` / `Oteryn-Atlas` returned no canonical reference.
- FACT — an accessible GitHub repository search for repositories owned by `blakinio` matching Oteryn returned `blakinio/Oteryn-v2` and `blakinio/Oteryn-Platform`; it did not return a current `blakinio/Oteryn` META repository or `blakinio/Oteryn-Atlas` repository at review time.
- FACT — `.github/CODEOWNERS` is intentionally narrow and currently enforces owner approval only for repository control-plane paths (`CODEOWNERS`, workflows, repository policy and repository tooling), not for ordinary architecture/runtime/component paths.
- FACT — three active task records were present on the reviewed `main`: the non-owning foundation coordination task and the two lag/disconnect architecture-analysis records. None owns the review paths used by this task.
- FACT — eleven open PRs were observed during review: #191, #239, #240, #268, #269, #270, #271, #272, #273, #276 and #277. The architecture worker PRs are not canonical while unmerged. In particular, PR #273 (`ALPHA-CLIENT-01`) is `DRAFT / OPEN / REWORK` and owns three disjoint paths.

# Agreements

- AGREEMENT — A future four-repository topology consisting of `Oteryn` META, `Oteryn-Game`, `Oteryn-Platform` and `Oteryn-Atlas` is a coherent target **provided authority is one-way and non-overlapping**.
- AGREEMENT — `Oteryn` should be a META repository, not a monorepo and not a Git-submodule composition root. It should own only cross-repository concerns that genuinely need one ecosystem-level authority.
- AGREEMENT — `Oteryn-v2 -> Oteryn-Game` is the correct repository boundary for the native playable game stack and its canonical authoring/build toolchain.
- AGREEMENT — Native Client + authoritative Game Server + `protocol-oteryn` + shared game/domain identifiers should remain in one repository. This is already an accepted architectural choice in ADR-0002 and remains the lowest-drift model while those components intentionally co-evolve.
- AGREEMENT — Canonical World/Content model + source schema + deterministic compiler + runtime bundles + validation must remain with the Game repository. These are game product semantics, not browser-map semantics.
- AGREEMENT — Oteryn Studio should remain in the Game repository for the current stage because it is an authoring frontend over the same canonical schema/registry/compiler/validation core. The GUI must remain a consumer of headless semantic crates rather than becoming their owner.
- AGREEMENT — OTBM is a legacy import format only. It must not become a Game<->Atlas API, shared canonical schema, Atlas datastore contract or permanent runtime format.
- AGREEMENT — `Oteryn-Platform` remains a separate control-plane/application repository. Portal, Identity, Accounts, GameAuth/Game Login Ticket orchestration, Game Gateway and the existing Platform-owned web/application responsibilities do not belong in the native gameplay workspace.
- AGREEMENT — `Oteryn-Atlas` is a sensible separate bounded context for browser map runtime, map search, layers/overlays, POI/NPC/spawn presentation, deep links, indexing and derived publication. Those concerns have a different UX/runtime/release profile from authoritative gameplay and world authoring.
- AGREEMENT — Atlas should consume a safe projection produced from canonical Oteryn world/content, never the canonical project directly and never a Game database or live GameNode as its source of truth.
- AGREEMENT — The preferred world flow is:

  `legacy OTBM/world -> bounded Oteryn-Game importer -> canonical Oteryn World/Content -> deterministic immutable Atlas export -> Oteryn-Atlas ingestion/index -> browser map`.

- AGREEMENT — no separate `Oteryn-Client`, `Oteryn-Server` or `Oteryn-Protocol` repository is justified now.

# Concerns

- CONCERN — The proposed META scope says `cross-repo integration/E2E`. If interpreted as moving the existing native E2E platform or client/server E2E ownership out of the Game repository, it conflicts with ADR-0002 and ADR-0007. META should orchestrate **cross-repository compositions and release evidence**; Game must retain the game-native test harness, scenarios/fixtures owned by game domains and client/server protocol proof.
- CONCERN — The current repository itself contains a file named `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`. Creating a new ecosystem-global META authority without a precise authority split would create two plausible "global" sources of truth. The Game-side register must become explicitly Game-scoped/current-history authority, while only genuinely cross-repository decisions become canonical in META.
- CONCERN — `Oteryn-Atlas` is new to current Oteryn-v2 architecture. There is no accepted Atlas ownership, export schema, versioning contract, public-data policy, compatibility policy or trust boundary yet. Implementing ingestion before that contract would create a new de facto architecture through code.
- CONCERN — Atlas must not ingest OTBM simply because OTBM already contains map data. That would establish a second world-source lineage and allow legacy limitations/IDs to bypass canonical normalization and validation.
- CONCERN — Atlas must not read the canonical World Project directly. The project intentionally includes editor-only metadata, server-authoritative fields, migration/provenance information and future data that may not be public-safe. Atlas needs an explicit allowlisted public projection.
- CONCERN — The Atlas projection can leak gameplay-sensitive or server-only information if "map data" is treated as a blanket public category. Hidden triggers, server-only script inputs, admin/editor metadata, unreleased content, private provenance, anti-abuse data and non-public encounter information must remain excluded unless explicitly classified public.
- CONCERN — A synchronous runtime API from Game Server to Atlas would unnecessarily couple availability, rollout and security domains. Atlas is predominantly a publication/read model and does not need authoritative gameplay latency semantics.
- CONCERN — Calling the renamed repository `Oteryn-Game` is accurate only if "Game" is explicitly defined as the **native playable product + canonical game content authoring/build toolchain**, not just the game server. Without that glossary, Studio and Client can look misplaced.
- CONCERN — Co-location can still become a distributed monolith inside one Git repository. Shared crates must not become an unrestricted dependency bucket. In particular, canonical domain/world crates must not depend on client UI, Studio shell, Platform transport adapters, Atlas types or legacy importer types.
- CONCERN — Studio is the least tightly coupled of the proposed Game components. Keeping it together is appropriate now, but only if the semantic core remains headless and independently testable. Otherwise Studio UI dependencies can contaminate compiler/world crates and make later extraction expensive.
- CONCERN — A single repository does not justify a single artifact version. Client build, server build, `protocol-oteryn` major/schema, world/content revisions, World Bundle format and Atlas export format have different compatibility semantics and must stay separate.
- CONCERN — Renaming `Oteryn-v2` later affects repository coordinates embedded in Cargo metadata, ADRs/contracts, provenance records, external contract locks, links, Actions, package/release metadata and external references. Historical ADR/provenance evidence must not be rewritten to pretend the old coordinate never existed.
- CONCERN — Broadening `.github/CODEOWNERS` into a complete component-ownership map would change current merge-authority semantics and may add mandatory review requirements to ordinary architecture/runtime work. That is a governance decision, not a harmless topology cleanup.
- CONCERN — Future extraction risk grows if `protocol-oteryn`, world schema/compiler or Studio communicate through Rust-internal types with no stable boundaries. Co-location should make atomic change easy without making extraction impossible.

# Requested Changes

1. RECOMMENDATION — Accept the conceptual rename `Oteryn-v2 -> Oteryn-Game`, but do not execute it until a cross-repository topology ADR establishes canonical repository identities, aliases/redirect expectations, historical-reference policy and update order.
2. RECOMMENDATION — Define `Oteryn-Game` as the owner of the **native game product and canonical game-content toolchain**: Client, Server, `protocol-oteryn`, shared game/domain crates, canonical world/content, compiler/bundles/validation, bounded legacy importers and Studio.
3. RECOMMENDATION — Keep Client + Server + Protocol together. Reconsider extraction only when there is concrete evidence of independent ownership/security, a materially different release cadence, multiple independent consumers, build/CI isolation need or an independently deployable boundary that outweighs atomic compatibility changes.
4. RECOMMENDATION — Keep Studio in `Oteryn-Game` now, but require dependency direction `Studio UI -> headless editor/world APIs -> canonical world/schema/compiler`; canonical crates must never depend on the Studio UI shell.
5. RECOMMENDATION — Keep legacy OTBM parsing and Legacy IR in `Oteryn-Game` behind a legacy namespace/module boundary. Canonical schema/compiler/runtime/Atlas-export crates must not depend on OTBM types or legacy numeric identity.
6. RECOMMENDATION — Create `Oteryn-Atlas` only as a derived read-model/presentation product. It must not own canonical world data, content IDs, game rules, authoritative spawns/NPCs, gameplay persistence, legacy import or compilation into runtime World Bundles.
7. RECOMMENDATION — Make the primary Game->Atlas contract an immutable artifact, not a live API. An optional API may later expose discovery/manifest metadata, but it must not become the canonical payload path or grant Atlas access to live Game state/databases.
8. RECOMMENDATION — `Oteryn-Game` should canonically own the Atlas export schema, exporter and golden fixtures because the producer owns the safe projection from canonical world semantics. `Oteryn` META should pin immutable contract revisions and compatibility support; `Oteryn-Atlas` should own ingestion/indexing/consumer validation.
9. RECOMMENDATION — Define a public-visibility projection policy before Atlas export v1. Export only allowlisted fields. Treat every omitted server-only/editor-only field as intentionally unavailable rather than something Atlas may fetch through a secondary channel.
10. RECOMMENDATION — META owns cross-repository ADRs, repository manifest, compatibility matrix, release manifests and **cross-repository E2E orchestration**. It must not own game runtime code, Game's canonical world schema, `protocol-oteryn`, Platform services, Atlas browser code, repository-local ADRs or local CI implementation.
11. RECOMMENDATION — Preserve repository-local E2E ownership: Game owns native client/server/headless protocol E2E mechanics and game-domain scenarios; Platform owns Platform-local contract/integration evidence; Atlas owns Atlas-local ingestion/UI tests. META composes exact immutable revisions into ecosystem E2E/release evidence.
12. RECOMMENDATION — Do not use Git submodules as the normal integration model. Cross-repository composition should use immutable Git SHAs/artifact digests/contract revisions in manifests.
13. RECOMMENDATION — Do not force one SemVer across all Game outputs merely because they share a repository. Release manifests should bind exact compatible client build, server build, protocol revision, world/content/bundle revisions and Atlas export revision.
14. RECOMMENDATION — Keep `.github/CODEOWNERS` unchanged in this documentation delivery. Continue task `owned_paths` and nearest `AGENTS.md` for agent concurrency. If mandatory component reviewers are later desired, change CODEOWNERS only through a dedicated governance decision with explicit effect on merge authority.
15. RECOMMENDATION — Add extraction seams now without extracting repositories: clean dependency direction, protocol/schema fixtures, artifact interfaces, producer/consumer contract tests and no circular Game<->Platform/Atlas dependencies.

# Ownership Matrix

| Area | Target repository owner | Boundary |
|---|---|---|
| Ecosystem repository manifest | `Oteryn` META | Cross-repository identity/coordinates only |
| Cross-repository ADRs | `Oteryn` META | Decisions whose authority spans 2+ repositories |
| Cross-repository compatibility matrix | `Oteryn` META | Pins supported immutable contract/artifact combinations |
| Ecosystem release manifest | `Oteryn` META | Pins exact repository commits/artifact digests; does not build product internals |
| Cross-repository E2E orchestration | `Oteryn` META | Composes repo-owned harnesses/artifacts at exact revisions |
| Native Rust Client | `Oteryn-Game` | Product/runtime owner |
| Authoritative Rust Game Server / GameNode | `Oteryn-Game` | Gameplay authority owner |
| `protocol-oteryn` schema/codecs/golden fixtures | `Oteryn-Game` | Single canonical producer/consumer contract owner |
| Shared game/domain IDs and protocol-neutral crates | `Oteryn-Game` | Must remain independent of UI/transport/legacy formats |
| Canonical Oteryn World/Content semantic model | `Oteryn-Game` | Sole authored game-world source of truth |
| Oteryn World Project source representation | `Oteryn-Game` | Authoring/source-control model |
| World compiler | `Oteryn-Game` | Deterministic lowering/projection authority |
| World Bundle/runtime bundle | `Oteryn-Game` | Runtime artifact authority |
| Canonical world/content validation | `Oteryn-Game` | Headless semantic validation authority |
| OTBM parser | `Oteryn-Game` | Legacy/import-only boundary; bounded/untrusted input |
| Legacy Intermediate Representation | `Oteryn-Game` | Importer-boundary type only; never canonical schema |
| Legacy semantic mapping/conversion reports | `Oteryn-Game` | Converts pinned legacy evidence into canonical model |
| Oteryn Studio semantic/editor core | `Oteryn-Game` | Uses the same canonical world/validation/compiler APIs |
| Oteryn Studio desktop/UI shell | `Oteryn-Game` for now | Extraction candidate only after evidence-based trigger |
| Atlas public-export semantic schema | `Oteryn-Game` | Producer-owned safe projection contract |
| Atlas exporter / full snapshot builder | `Oteryn-Game` | Deterministic projection from canonical model/bundle inputs |
| Atlas incremental-delta builder, if later accepted | `Oteryn-Game` | Must bind exact base and target export digests |
| Atlas export cross-repo lock/support matrix | `Oteryn` META | Pins producer schema/artifact revisions and consumer support |
| Atlas artifact ingestion/parser validation | `Oteryn-Atlas` | Treats artifact as untrusted consumer input |
| Atlas indexing/search/read models | `Oteryn-Atlas` | Rebuildable derived data |
| Browser map runtime/UI | `Oteryn-Atlas` | Presentation/runtime owner |
| Map layers/overlays/deep links | `Oteryn-Atlas` | Presentation semantics only |
| POI/NPC/spawn public presentation | `Oteryn-Atlas` | Only data explicitly exposed by Game public projection |
| Portal | `Oteryn-Platform` | Web/application control plane |
| Identity / Accounts / OAuth / MFA | `Oteryn-Platform` | Credential/account authority |
| GameAuth / Game Login Ticket producer | `Oteryn-Platform` | Pre-admission control-plane authority |
| Game Gateway / World Registry | `Oteryn-Platform` | Routing/admission orchestration, not gameplay authority |
| Game persistence / authoritative mutable world state | `Oteryn-Game` | Never Atlas/Platform/META |
| Canary/Otheryn/otclient historical evidence | existing legacy/reference repositories | Read-only evidence; no target authority |

# Conflicts With Current ADRs

| ADR/path | Assessment | Required treatment |
|---|---|---|
| `docs/architecture/ADR-0001-native-rust-multichannel-platform.md` | COMPATIBLE | Proposed Game/Platform split preserves its native Client/Server/Protocol and Platform authority model. |
| `docs/architecture/ADR-0002-repository-ownership-and-client-migration.md` | COMPATIBLE WITH ONE CLARIFICATION | Strongly supports one Game repo for Client+Server+Protocol/shared types. META must not take over its cross-client/server E2E ownership; META may orchestrate broader cross-repo E2E. Repository-coordinate wording will need a later rename/alias update, not semantic reversal. |
| `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md` | COMPATIBLE | Platform responsibilities in the proposal align with the accepted boundary. |
| `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md` | COMPATIBLE | Strongly supports canonical world/compiler/legacy importer/Studio remaining on the Game side. Atlas export is a new downstream projection and is not currently defined. |
| `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` | POTENTIAL CONFLICT IF META SCOPE IS TOO BROAD | Preserve Game-owned native E2E platform/harness/evidence; define META as composition/orchestration for multi-repo tests and releases. A new cross-repo ADR should state this split. |
| `docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md` | COMPATIBLE | Canary remains reference-only and must not appear in Game<->Atlas contracts. |
| `docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md` | COMPATIBLE | Native client remains in Game; current implementation state remains distinct from target repository ownership. |
| `docs/architecture/ADR-0013-platform-database-technology-independence.md` | COMPATIBLE | Repository topology does not collapse Platform/game persistence ownership. |
| `docs/architecture/ADR-0015-gamenode-implementation-shape-not-yet-frozen.md` | COMPATIBLE | Repository co-location does not freeze internal GameNode decomposition or prevent later evidence-backed adjacent services. |
| `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` | COMPATIBLE / EXTENSION REQUIRED | Canonical typed graph, deterministic projection and immutable artifact rules are the correct upstream foundation. Atlas needs a new explicit public projection/export contract; Atlas must not consume the canonical graph directly. |
| `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` | AUTHORITY AMBIGUITY UNDER FUTURE META | No current conflict while META does not exist. Before META becomes canonical, define which decisions remain Game-local and which ecosystem-global decisions move to META authority. Preserve historical evidence. |
| `docs/agents/CROSS_REPO_CONTRACTS.md` | COMPATIBLE / UPDATE REQUIRED LATER | Existing policy already provides the correct single-owner/producer/consumer/rollout discipline, but its repository inventory has no META or Atlas yet. Update only after the topology ADR is accepted and repositories actually exist. |
| `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` | NO CONFLICT / NEW ENTRY REQUIRED LATER | Current lock has no Atlas contract. Add a Game->Atlas lock only after a merged immutable export contract exists. |
| `.github/CODEOWNERS` | COMPATIBLE AS-IS | It is intentionally merge-authority-focused. Do not silently repurpose it as a full architecture ownership map. |

No currently accepted ADR requires separate Client, Server or Protocol repositories. No currently accepted ADR makes OTBM a canonical runtime or cross-repository world contract. No current ADR establishes `Oteryn-Atlas`; that is a real new cross-repository decision rather than an implicit rename of existing architecture.

# Cross-Repository Contracts Needed

1. **ECOSYSTEM-REPO-TOPOLOGY-01** — cross-repository ADR defining `Oteryn` META, `Oteryn-Game`, `Oteryn-Platform`, `Oteryn-Atlas`, canonical responsibilities, prohibited ownership overlap, repository aliases/rename transition and the rule against submodule-based composition.
2. **GAME-ATLAS-EXPORT-01** — canonical producer-owned immutable Atlas export contract: schema identity, public visibility policy, limits, deterministic encoding/canonicalization, full snapshot semantics, optional delta semantics, provenance and error classes.
3. **GAME-ATLAS-COMPAT-01** — consumer compatibility rules: supported schema/format ranges, required capabilities, unknown-critical-field behavior, World/Map/Content revision binding and fail-closed rejection.
4. **GAME-ATLAS-PROVENANCE-01** — provenance/integrity envelope tying export digest to exact Game source/compiler/exporter identity, Content Lock, source World Bundle or canonical world revision, projection profile and source/import provenance summary.
5. **GAME-ATLAS-ROLLBACK-01** — publication/rollback contract: immutable retention, atomic active-pointer switch, consumer index rebuild rules, incompatible rollback rejection and no in-place mutation of published artifacts.
6. **GAME-ATLAS-DELTA-01** — only if incremental publication is needed: delta explicitly names base export digest and target export digest, can be verified before activation and never replaces the ability to recover from a complete full snapshot.
7. **ECOSYSTEM-COMPATIBILITY-MATRIX-01** — META-owned machine-readable mapping of compatible Game client/server/protocol/world/export revisions, Platform contract revisions and Atlas consumer versions. It must pin immutable revisions, not mutable branches.
8. **ECOSYSTEM-RELEASE-MANIFEST-01** — META-owned release manifest schema pinning exact repository commits and artifact digests without forcing one shared version number.
9. **ECOSYSTEM-E2E-OWNERSHIP-01** — explicit split between repository-owned test harnesses/scenarios and META-owned cross-repository orchestration/evidence aggregation.
10. **PUBLIC-WORLD-DATA-01** — classification contract for which canonical game fields may appear in public Atlas exports and which server-only/editor-only/security-sensitive fields are prohibited.

The existing Platform<->Game protocol/admission contracts should be referenced/pinned by META rather than duplicated or re-authored there.

# Migration Risks

## P0

- P0 — **dual world source of truth** if Atlas is allowed to ingest OTBM, canonical source files independently, live Game databases or manually edited map truth instead of one Game-produced projection;
- P0 — **server/private data disclosure** if the Atlas projection is generated by exclusion/blacklist rather than explicit public allowlisting;
- P0 — **authority split-brain** if META and Game both claim canonical ownership of the same ADR/protocol/world contract or if Atlas gains authority over canonical World/Content;
- P0 — **artifact trust failure** if Atlas activates unbounded/unverified external payloads or published mutable artifacts without digest/provenance verification.

## P1

- P1 — moving native E2E implementation to META and thereby weakening exact Game client/server protocol/runtime proof required by ADR-0007;
- P1 — repository rename performed before contract locks, Cargo metadata, Actions, external references and historical alias policy are reconciled;
- P1 — Game/Atlas schema drift if export schema and compatibility support are duplicated in both repositories rather than producer-owned and pinned;
- P1 — Atlas rollback serving an artifact whose world/content/export revision is incompatible with the active consumer/index schema;
- P1 — circular repository dependency if Game build requires Atlas or Platform artifacts while those repositories simultaneously require a newly built Game artifact;
- P1 — Studio or legacy importer types leaking into canonical world/runtime APIs and making later extraction or format evolution unsafe.

## P2

- P2 — CI cost and queue pressure from a large Game repository if every path change runs every Client/Server/Studio/world job without risk/path selection;
- P2 — unnecessary synchronized releases if repository tags are mistaken for protocol/world/client/server compatibility identity;
- P2 — Studio UI dependency growth contaminating headless compiler/validation crates;
- P2 — long incremental Atlas delta chains increasing recovery and debugging complexity without periodic complete snapshot baselines;
- P2 — duplicate cross-repository ADR copies drifting between META and local repositories instead of one canonical source plus immutable references.

## P3

- P3 — naming ambiguity around `Oteryn-Game` because it also contains Studio and canonical world tooling;
- P3 — documentation/link churn caused by the future repository rename;
- P3 — developer discovery friction from multiple repositories if the META manifest/navigation is incomplete;
- P3 — increased review noise if CODEOWNERS is broadened prematurely rather than using existing task/path ownership mechanisms.

# Open Decisions

1. Must `Oteryn-Game` become the accepted future canonical repository name, with `Oteryn-v2` retained as historical coordinate/redirect evidence, before any rename is executed?
2. Is the `Oteryn` META repository explicitly limited to cross-repository governance/contracts/manifests/orchestration, while repository-local ADRs, native Game E2E mechanics and product implementations remain owned locally?
3. Does the project accept `artifact-first` as the mandatory Game->Atlas source-of-truth boundary, with any future API limited to discovery/control metadata rather than canonical map payload/state?
4. Does `Oteryn-Game` own the Atlas export schema/exporter/fixtures while META owns only immutable locking/compatibility metadata and Atlas owns ingestion/presentation?
5. What exact public-data classification is allowed in Atlas export v1, especially for spawn/NPC/encounter/quest-related fields that may reveal server-authoritative or intentionally hidden semantics?
6. Should Oteryn Studio remain in the Game repository until an explicit extraction trigger is observed, with headless world/editor/compiler APIs required now to preserve that option?
7. What is the exact E2E responsibility split between Game-local native E2E and future META cross-repository orchestration so ADR-0007 remains satisfied without duplicating orchestration systems?

The physical Atlas serialization/container, artifact transport/CDN, exact signing mechanism, exact delta cadence and Studio UI technology do **not** need to be decided by this topology review. They should remain deferred until a concrete implementation/consumer gate requires them and the existing architecture discipline can evaluate evidence.

# Final Recommendation

Adopt the proposed ecosystem topology **with the requested authority corrections**:

- `Oteryn` = thin ecosystem META/control **documentation and integration coordination** repository: cross-repo ADRs, repository manifest, immutable compatibility/release manifests and cross-repo E2E orchestration only;
- `Oteryn-Game` = current `Oteryn-v2` product boundary: native Client + authoritative Server + `protocol-oteryn` + shared domain/IDs + canonical World/Content + deterministic compiler/bundles/validation + bounded legacy import + Oteryn Studio;
- `Oteryn-Platform` = web/control-plane application boundary: Portal + Identity + Accounts + GameAuth + Game Gateway + World Registry and related Platform modules;
- `Oteryn-Atlas` = derived browser-map product: ingestion/index/search/layers/POI/NPC/spawn presentation/deep links, consuming only a safe versioned Game-produced artifact.

Do **not** split Client, Server or Protocol into independent repositories now. The existing accepted architecture chose co-location for good reasons: protocol and shared types co-evolve, client/server compatibility can be validated on one exact commit and drift is easier to prevent. Extend that same repository boundary to canonical World/Content and Studio as already accepted, while enforcing internal dependency direction so co-location does not become accidental coupling.

Before creating/renaming repositories or implementing Atlas ingestion, accept one cross-repository topology ADR and one Game->Atlas export contract. The Atlas contract should be **artifact-first, immutable, content-addressed, deterministic, provenance-rich, versioned, full-snapshot recoverable and fail-closed**, with optional base-digest-bound deltas only as an optimization. Atlas must treat the artifact as untrusted input and must never obtain canonical world authority through OTBM, direct World Project access, Game databases or live GameNode APIs.

`Oteryn-Game` is the recommended name. `Oteryn-Core` is less precise and risks implying Platform/Atlas are secondary implementation details; `Oteryn-Native` couples repository identity to current implementation technology. `Oteryn-Game` remains understandable if its repository description/glossary explicitly says **native playable game stack + canonical game-content authoring/build toolchain**, not merely "server".

The resulting architecture preserves current accepted Oteryn-v2 decisions, adds Atlas as a clean downstream projection boundary, gives META a useful but deliberately non-invasive role, and keeps future extraction possible without paying the coordination cost of premature repository fragmentation today.
