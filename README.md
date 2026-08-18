# Oteryn Game

Canonical native Rust game repository for Oteryn, migrated with preserved Git history from `blakinio/Oteryn-v2`, including the game server, native client and project-owned world/content tooling.

## Architecture baseline

Start with the [canonical architecture index](docs/architecture/README.md) and [current foundation status](docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md).

Core baseline:

- [ADR-0001: Native Rust Oteryn stack and multichannel-first game server](docs/architecture/ADR-0001-native-rust-multichannel-platform.md)
- [ADR-0003: Platform Identity, Game Gateway and admission boundary](docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md)
- [ADR-0004: PostgreSQL and data ownership](docs/architecture/ADR-0004-postgresql-and-data-ownership.md)
- [ADR-0005: Native world format, Oteryn Studio and legacy conversion boundary](docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md)
- [ADR-0006: Game Intelligence, analytics and audit](docs/architecture/ADR-0006-game-intelligence-analytics-and-audit.md)
- [ADR-0009: GameNode capacity, deployment and recovery](docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md)
- [ADR-0014: TCP-default, QUIC-opt-in dual gameplay transport strategy](docs/architecture/ADR-0014-dual-gameplay-transport-tcp-default-quic-opt-in.md)
- [2026-08-10 architecture review refinements](docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md)
- [Foundation decision backlog](docs/architecture/FOUNDATION_DECISION_BACKLOG.md)
- [Multichannel system scope matrix](docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md)

The target uses one project-owned gameplay application protocol (`protocol-oteryn`), one logical world with one or more gameplay channels, explicit world/channel/instance ownership, an authoritative Rust server and a project-owned native world/content model. OTBM and historical editors remain migration/reference inputs rather than target runtime dependencies.

Current registered gameplay transport remains TCP + TLS 1.3 profile `1`. QUIC v1 + TLS 1.3 is the accepted future player-opt-in transport target with TCP retained as the safe baseline, but functional QUIC admission/recovery is blocked until the protocol transport registry and FND-04 fresh/recovery grant profiles are explicitly reconciled and accepted. Both transports remain one `protocol-oteryn` application protocol.

Architecture acceptance does not imply implementation or production activation. See the [three-axis architecture status model](docs/architecture/ARCHITECTURE_STATUS_MODEL.md).

## Agent governance

- [Agent governance index](docs/agents/README.md)
- [Root agent instructions](AGENTS.md)
- [Mandatory bootstrap override](AGENTS.override.md)
- [Repository and planned workspace map](docs/agents/REPOSITORY_MAP.md)
- [Build and test matrix](docs/agents/BUILD_TEST_MATRIX.md)

Governance is validated by `python tools/agents/validate_governance.py` and the `Agent governance` GitHub Actions workflow.

Architecture decisions and implementation programmes are maintained under `docs/architecture/` and `docs/agents/`.

## Licensing

Oteryn Game source code, tooling, schemas, configuration and technical documentation are licensed under the [Mozilla Public License 2.0](LICENSE) unless a more specific notice applies.

Creative game assets are not automatically covered by MPL-2.0 and remain reserved unless separately licensed. The Oteryn names, logos and branding are also outside the software license grant.

See the complete [licensing policy](docs/repository/LICENSING.md), [creative asset notice](LICENSE-ASSETS.md) and [trademark notice](TRADEMARKS.md).
