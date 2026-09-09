# Oteryn Game repository map

Status: bootstrap map; inspect the exact tree before use.

## Current durable content

- `README.md` — project entry point.
- `docs/architecture/` — accepted native Rust, multichannel, persistence and world/content architecture plus clearly labeled future/proposed architecture candidates.
- `docs/agents/` — agent governance, tasks, evidence and programmes.

## Planned top-level layout

The following layout is a target, not proof of current existence:

```text
apps/
  client/                 native Rust desktop client composition
  web-client/             future browser/WASM composition; create only under an accepted implementation task
  oteryn-studio/          integrated map, asset and content authoring application
services/
  game-server/            authoritative Rust server composition
crates/
  domain-*/               protocol-neutral gameplay/domain crates
  protocol-core/          framing/version/capability primitives
  protocol-oteryn/        native gameplay application-protocol adapter/codec
  world-schema/           canonical native world and spatial types
  world-project/          editable source-project model and migrations
  world-bundle/           compiled read-only runtime bundle
  world-compiler/         deterministic project-to-bundle compiler
  world-validation/       structural, spatial and conversion validation
  world-spatial/          area/subarea/zone/chunk/encounter indexing
  content-registry/       stable keys, packages, aliases and legacy mappings
  asset-pipeline/         project-owned asset import/build pipeline
  editor-*/               Studio commands, history and authoring support
  legacy-intermediate/    bounded normalized migration representation
  legacy-*/               OTBM/OTB/appearance conversion adapters and reports
  world-*/                world/channel/instance runtime components
  persistence-*/          durable state, leases and transaction adapters
  client-*/               shared/client-edge renderer, UI, input, runtime and platform components
content/                   versioned native projects, catalogues, rulesets and scripts
tools/                     migration, fixtures, validation and operations
docs/architecture/        ADRs and architecture baselines
docs/contracts/           public and cross-component contracts
docs/agents/              governance and task state
```

Create paths only when an accepted implementation task owns them. Candidate crate names remain provisional until an owning implementation decision proves the physical split. Do not generate empty architecture merely to match this map.

### Future browser-client boundary

Issue #519 and proposed `docs/architecture/ADR-0017-browser-client-web-ready-boundaries.md` reserve a future browser client as a second composition of the same Rust client semantics. This is **not** current implementation authority.

Current/native work should preserve only the necessary anti-coupling properties:

- shared gameplay/client semantics do not expose Win32, DX12, native socket or DOM types without necessity;
- input semantics use normalized actions rather than only physical platform key codes;
- asset identity is logical/revision-based rather than an absolute local path;
- gameplay/session code stays transport-neutral;
- `protocol-oteryn` remains one application protocol and browser transports require their own accepted transport-profile semantics when they differ from TCP profile `1`;
- Server Seam is not blocked on browser implementation.

If later evidence requires physical browser-specific components, candidate names such as `client-platform-web`, `transport-web` or `renderer-web` are illustrative only. Do not create them until a live task owns the paths and a real consumer exists.

## External repositories and authority

| Repository | Oteryn Game relationship | Default access |
|---|---|---|
| `Oteryn/Oteryn-Game` | canonical native Rust gameplay stack, client surfaces and Oteryn Studio | read/write within task scope |
| `blakinio/Oteryn-v2` | preserved legacy/migration source for pre-organization history and provenance | read-only |
| `Oteryn/Oteryn-Platform` | web/Identity/Game Gateway/World Registry producer | read-only unless separately authorized |
| `blakinio/Otheryn` | C++ behavioral/content reference and migration oracle | read-only unless separately authorized |
| `blakinio/otclient` | existing client/Rust implementation and migration source; FND-01 must inventory and classify it at an exact SHA before designing replacements | read-only unless separately authorized |
| Remere's Map Editor | OTBM behavior/fixture/reference tool; not a target dependency | external evidence only |
| Beats Assets Editor | modern asset/content workflow reference; not a target dependency | external evidence only |
| upstream Canary/OTClient repositories | external evidence only | read-only |

Cross-repository changes require separate task state and PRs in each authorized repository. `Oteryn-Game` must not silently claim that another repository already implements a planned contract.

External editor code, UI and assets require pinned revisions, license/provenance review and explicit implementation-task authority before reuse. Behavioral study and legally permitted compatibility fixtures do not make those projects canonical dependencies.

## Architecture routing

- Native stack and multichannel baseline: `docs/architecture/ADR-0001-native-rust-multichannel-platform.md`.
- Repository and client migration ownership: `docs/architecture/ADR-0002-repository-ownership-and-client-migration.md`.
- Platform/Game Gateway boundary: `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`.
- PostgreSQL and data ownership: `docs/architecture/ADR-0004-postgresql-and-data-ownership.md`.
- Native world format, Oteryn Studio and legacy conversion: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`.
- Proposed browser/WASM/WebGPU client boundary: `docs/architecture/ADR-0017-browser-client-web-ready-boundaries.md` (Issue #519; no implementation authority until accepted/allocated).
- Proposed browser-client staged implementation sequence: `docs/architecture/WEB_CLIENT_IMPLEMENTATION_PLAN.md`.
- Current decision order: `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`.
- Scope/consistency matrix: `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`.
- Otheryn migration strategy: `docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md`.
- Agent cross-repo policy: `docs/agents/CROSS_REPO_CONTRACTS.md`.
- Machine-readable cross-repository revision state: `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`.
- Shared resource-limit, error and failure contracts: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, `FOUNDATION_ERROR_VOCABULARY.md` and `FOUNDATION_FAILURE_SCENARIOS.md`.

## Ownership boundaries

Until code exists, tasks must define exact owned paths. Once workspace crates/services are introduced, add nearer `AGENTS.md` files for high-risk or independently owned areas such as protocol, persistence, server runtime, client runtime, world/content schemas, importers, Studio/editor runtime, assets and deployment.
