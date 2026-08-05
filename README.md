# Oteryn v2

Greenfield implementation of the Oteryn game platform runtime, including a native Rust game server, a native Rust game client and project-owned world/content tooling.

## Architecture baseline

- [ADR-0001: Native Rust Oteryn stack and multichannel-first game server](docs/architecture/ADR-0001-native-rust-multichannel-platform.md)
- [ADR-0002: Repository ownership and native client migration](docs/architecture/ADR-0002-repository-ownership-and-client-migration.md)
- [ADR-0003: Platform Identity, Game Gateway and admission boundary](docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md)
- [ADR-0004: PostgreSQL and data ownership](docs/architecture/ADR-0004-postgresql-and-data-ownership.md)
- [ADR-0005: Native world format, Oteryn Studio and legacy conversion boundary](docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md)
- [Foundation decision backlog](docs/architecture/FOUNDATION_DECISION_BACKLOG.md)
- [Multichannel system scope matrix](docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md)
- [Otheryn reference and migration strategy](docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md)

The current foundation assumes one project-owned gameplay protocol (`protocol-oteryn`), one logical world with one or more gameplay channels, explicit world/channel/instance ownership, a Rust authoritative server and a project-owned native world/content model. OTBM and historical editors are migration/reference inputs rather than target runtime dependencies.

## Agent governance

- [Agent governance index](docs/agents/README.md)
- [Root agent instructions](AGENTS.md)
- [Mandatory bootstrap override](AGENTS.override.md)
- [Repository and planned workspace map](docs/agents/REPOSITORY_MAP.md)
- [Build and test matrix](docs/agents/BUILD_TEST_MATRIX.md)

Governance is validated by `python tools/agents/validate_governance.py` and the `Agent governance` GitHub Actions workflow.

Architecture decisions and implementation programmes are maintained under `docs/architecture/` and `docs/agents/`.
