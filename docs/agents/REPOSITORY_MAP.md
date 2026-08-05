# Oteryn v2 repository map

Status: bootstrap map; inspect the exact tree before use.

## Current durable content

- `README.md` — project entry point.
- `docs/architecture/` — accepted native Rust/multichannel architecture.
- `docs/agents/` — agent governance, tasks, evidence and programmes.

## Planned top-level layout

The following layout is a target, not proof of current existence:

```text
apps/
  client/                 native Rust desktop client composition
services/
  game-server/            authoritative Rust server composition
crates/
  domain-*/               protocol-neutral gameplay/domain crates
  protocol-core/          framing/version/capability primitives
  protocol-oteryn/        native gameplay wire adapter
  world-*/                world/channel/instance runtime components
  persistence-*/          durable state, leases and transaction adapters
  client-*/               renderer, UI, input and client state components
content/                   versioned maps, catalogues, rulesets and scripts
tools/                     migration, fixtures, validation and operations
docs/architecture/        ADRs and architecture baselines
docs/contracts/           public and cross-component contracts
docs/agents/              governance and task state
```

Create paths only when an accepted implementation task owns them. Do not generate empty architecture merely to match this map.

## External repositories and authority

| Repository | Oteryn v2 relationship | Default access |
|---|---|---|
| `blakinio/Oteryn-v2` | target native Rust gameplay stack | read/write within task scope |
| `blakinio/Oteryn-Platform` | web/Identity/Game Gateway/World Registry producer | read-only unless separately authorized |
| `blakinio/Otheryn` | C++ behavioral/content reference and migration oracle | read-only unless separately authorized |
| `blakinio/otclient` | existing client/Rust implementation reference and migration source | read-only unless separately authorized |
| upstream Canary/OTClient repositories | external evidence only | read-only |

Cross-repository changes require separate task state and PRs in each authorized repository. `Oteryn-v2` must not silently claim that another repository already implements a planned contract.

## Architecture routing

- Native stack and multichannel baseline: `docs/architecture/ADR-0001-native-rust-multichannel-platform.md`.
- Scope/consistency matrix: `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`.
- Otheryn migration strategy: `docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md`.
- Agent cross-repo policy: `docs/agents/CROSS_REPO_CONTRACTS.md`.

## Ownership boundaries

Until code exists, tasks must define exact owned paths. Once workspace crates/services are introduced, add nearer `AGENTS.md` files for high-risk or independently owned areas such as protocol, persistence, server runtime, client runtime, content/assets and deployment.
