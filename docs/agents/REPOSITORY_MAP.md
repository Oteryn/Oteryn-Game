# Oteryn Game repository map

Status: current physical routing map. Verify the exact tree and nearest `AGENTS.md` before mutation.

## Current top-level layout

- `apps/client/` — native Rust desktop-client composition root.
- `apps/game-server/` — authoritative Rust Game Server composition root, including migrations and runtime/content modules.
- `crates/` — shared Foundation, identity, simulation, diagnostics, client/runtime/platform, input, renderer, synthetic-asset and test-support crates registered in the workspace.
- `tools/` — repository governance, architecture checks, evidence generators, importers, validators and focused developer tools. A tool is not runtime authority unless an accepted contract says so.
- `tests/` — repository-level fixtures, security tests and pre-native/native-client evidence.
- `experiments/` — bounded prototypes and bakeoffs; not production authority.
- `docs/architecture/` — accepted ADRs/baselines plus clearly marked proposals and historical snapshots.
- `docs/contracts/` — durable public, cross-component and machine-readable contracts.
- `docs/agents/` — routed governance, prompts, programmes, task records and retained evidence.
- `docs/migration/` and `docs/repository/` — migration and repository-engineering guidance.

The workspace members in `Cargo.toml` and the checked-in tree are physical truth. Candidate names in architecture or plans are not proof that a crate, service or application exists.

## Proposed or future surfaces

- A browser/WebAssembly client remains proposed under Issue #519 and `docs/architecture/ADR-0018-browser-client-web-ready-boundaries.md`; it is not current implementation authority.
- Oteryn Studio and any new editor, service, crate or deployment surface require an accepted owning task before paths are created.
- `services/game-server/` is not the current server path; the implemented composition root is `apps/game-server/`.

Do not create empty directories or placeholder crates merely to match a plan.

## External repositories and authority

| Repository | Oteryn Game relationship | Default access |
|---|---|---|
| `Oteryn/Oteryn-Game` | canonical native Rust gameplay stack, client surfaces and current Game tooling | read/write within task scope |
| `Oteryn/Oteryn-Platform` | web identity, Game Gateway, World Registry and commercial/control-plane producer | read-only unless separately authorized |
| `blakinio/Oteryn-v2` | preserved legacy/migration provenance | read-only |
| `blakinio/Otheryn` | C++ behavioral/content reference and migration oracle | read-only unless separately authorized |
| `blakinio/otclient` | client implementation and migration/reference evidence | read-only unless separately authorized |
| upstream Canary/OTClient and editor projects | external comparison/evidence only | read-only |

Cross-repository work requires explicit authority and a separate lifecycle in every mutated repository. Reference code, UI and assets also require pinned revisions plus applicable license/provenance review before reuse.

## Architecture routing

- Native stack and multichannel baseline: `docs/architecture/ADR-0001-native-rust-multichannel-platform.md`.
- Repository/client migration ownership: `docs/architecture/ADR-0002-repository-ownership-and-client-migration.md`.
- Platform/Game Gateway boundary: `docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md`.
- PostgreSQL/data ownership: `docs/architecture/ADR-0004-postgresql-and-data-ownership.md`.
- Native world format and Oteryn Studio boundary: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`.
- Scope consistency: `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`.
- Otheryn migration: `docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md`.
- Cross-repository policy and revision state: `docs/agents/CROSS_REPO_CONTRACTS.md` and `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json`.
- Shared limits and failure contracts: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, `FOUNDATION_ERROR_VOCABULARY.md` and `FOUNDATION_FAILURE_SCENARIOS.md`.

## Ownership routing

Current nearest-path instructions exist at repository root, `apps/game-server/`, `crates/simulation-determinism/` and `docs/agents/`. Add another nearest `AGENTS.md` only when a real independently owned or high-risk boundary needs rules that are not already supplied by root, META or accepted domain contracts.
