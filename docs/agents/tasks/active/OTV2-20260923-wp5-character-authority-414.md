# OTV2-20260923-wp5-character-authority-414

```yaml
task_id: OTV2-20260923-wp5-character-authority-414
title: WP5 Character authority first slice
mode: REPAIR
status: blocked
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp5-character-authority-414
issue: 319
pr: 790
base_sha: ec8803bd1a37600acd0ff544811871e8fb2c40cf
head_sha: f3e24c3bb42f2be67005a4a38b10435dd1c2a72d
final_head_sha: null
final_head_frozen_at: null
owner: work-coordinator
created_at: 2026-09-23T00:00:00Z
updated_at: 2026-09-23T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/mod.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/character_authority_audit.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/migrations/0005_character_authority.sql
  - apps/game-server/tests/character_authority_postgres.rs
  - Cargo.toml
  - apps/game-server/Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260923-wp5-character-authority-414.md
public_contracts: []
depends_on:
  - 791
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- Status: BLOCKED on #791
- Coordinator: #162
- Source programme: #319
- Branch: `agent/wp5-character-authority-414`
- Admission main: `ec8803bd1a37600acd0ff544811871e8fb2c40cf`
- Scope: first authoritative Character bootstrap/current read with atomic receipt, registered durable audit payload and outbox.
- Excluded: transfer, world transfer, rename, retirement, Platform writes, Foundation composition and Server Seam.

## Owned paths

The exact paths are those in Work application #162 comment `5793512204`, including the bounded three-file prost dependency lease recorded by #247 comment `5793517381`.

## Qualification

Focused Rust/protobuf checks run locally. The registered `character_authority_postgres` target and whole candidate require PostgreSQL 17.6 and canonical repository CI before integration. Independent exact-head review, Ready, Merge Queue and closeout remain coordinator-owned.
