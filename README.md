# Oteryn v2

Greenfield implementation of the Oteryn game platform runtime, including a native Rust game server and a native Rust game client.

## Architecture baseline

- [ADR-0001: Native Rust Oteryn stack and multichannel-first game server](docs/architecture/ADR-0001-native-rust-multichannel-platform.md)
- [Multichannel system scope matrix](docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md)
- [Otheryn reference and migration strategy](docs/architecture/OTHERYN_REFERENCE_MIGRATION_PLAN.md)

The current foundation assumes one project-owned gameplay protocol (`protocol-oteryn`), one logical world with one or more gameplay channels, explicit world/channel/instance ownership and a Rust authoritative server.

## Agent governance

- [Agent governance index](docs/agents/README.md)
- [Root agent instructions](AGENTS.md)
- [Mandatory bootstrap override](AGENTS.override.md)
- [Repository and planned workspace map](docs/agents/REPOSITORY_MAP.md)
- [Build and test matrix](docs/agents/BUILD_TEST_MATRIX.md)

Governance is validated by `python tools/agents/validate_governance.py` and the `Agent governance` GitHub Actions workflow.

Architecture decisions and implementation programmes are maintained under `docs/architecture/` and `docs/agents/`.
