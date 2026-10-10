# OTV2-20261010-guild-0-contract

```yaml
task_id: OTV2-20261010-guild-0-contract
title: "GUILD-0 contract: Oteryn Game Guilds v1 candidate (docs only)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/guild-0-20261010
issue: 1622
pr: 1955
base_sha: 348b2b76
head_sha: "exact frozen head in the #1622 FREEZE entry"
final_head_sha: "exact frozen head in the #1622 FREEZE entry"
owner: GUILD-0 worker for the #1622 control plane
created_at: 2026-10-10
updated_at: 2026-10-10
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_GAME_GUILDS_V1.md
  - docs/agents/tasks/archive/OTV2-20261010-guild-0-contract.md
public_contracts: [oteryn-game-guilds-v1]
depends_on: []
blocks: [GUILD-1, GUILD-WIRE-1, GUILD-CHAT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A candidate integration contract for guilds (`docs/contracts/OTERYN_GAME_GUILDS_V1.md`) that
fixes authority and ownership (Game sole authority, Platform read-only), durable invariants,
the command set and results, member, creature-information and public projections, guild chat,
failure modes and 32 conformance cases, built on the accepted GUILD-0 decision. No runtime,
migration or wire allocation.

## Architecture and source of truth

- PROVEN: GUILD-0 accepted by ACCEPT-SOCIAL-MAP-0; GUILD-1 packet in SOCIAL-MAP packets §2.2.
- DERIVED: `decline` intent (GUILD-1 packet lists it, GUILD-0 §9 oneof does not); the
  `GUILD_BADGES` domain shape for creature information.
- UNKNOWN: Q1 (creature information content), sent to the control plane.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs-only candidate contract; it performs no fenced mutation, grants no
authority and interprets no persisted recovery evidence. The implementing children (GUILD-1) carry
the qualification.

## Acceptance criteria

- [x] Contract covers authority, durable state, commands, projections, failure modes and
  conformance cases, and cites reference evidence.
- [x] No re-decision of GUILD-0; conflicts resolve in favour of GUILD-0.

## Excluded scope

Runtime code, migrations, wire number leases, registries, Platform and Atlas changes.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
