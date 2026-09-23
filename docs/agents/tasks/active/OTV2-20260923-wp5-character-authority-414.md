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
base_sha: 929397df6e33ceabf10ed862fe835517e7d40887
head_sha: f4b1ef9f42710cc19f0b6e4c68a12e1e9c431f36
final_head_sha: null
final_head_frozen_at: null
owner: claude
created_at: 2026-09-23T00:00:00Z
updated_at: 2026-09-23T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/mod.rs
  - apps/game-server/src/character_recovery_fence.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/character_authority_audit.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/migrations/0005_character_authority.sql
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/native_admission_source_postgres.rs
  - apps/game-server/tests/runtime_scope_assignment_postgres.rs
  - Cargo.toml
  - apps/game-server/Cargo.toml
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260923-wp5-character-authority-414.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- Status: BLOCKED on the protected Platform producer mapping for `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1`; the Character recovery decision from merged #795 is implemented here without inventing that separate prerequisite.
- Writer: Claude, per the #790 handoff (comment `5796230826`); ChatGPT writer custody released.
- Coordinator: #162
- Source programme: #319
- Branch: `agent/wp5-character-authority-414`
- Admission main: `929397df6e33ceabf10ed862fe835517e7d40887` (protected #795)
- Scope: first authoritative Character bootstrap/current read with atomic receipt, registered durable audit payload and outbox.
- Excluded: transfer, world transfer, rename, retirement, Platform writes, Foundation composition and Server Seam.

## Owned paths

The original exact paths are those in Work application #162 comment `5793512204`, including the bounded three-file prost dependency lease recorded by #247 comment `5793517381`. The protected #795 SAME-line amendment adds only `apps/game-server/src/character_recovery_fence.rs` and the minimal `apps/game-server/src/lib.rs` export.

## Qualification

Focused Rust/protobuf checks run locally. The registered `character_authority_postgres` target and whole candidate require PostgreSQL 17.6 and canonical repository CI before integration. Independent exact-head review, Ready, Merge Queue and closeout remain coordinator-owned.

## Retention and custody repair (Claude)

- Registered `CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1` is now enforced:
  - every audit record carries `expires_at = occurred_at + P90D`;
  - the only deletion is ordinary expiry of an unheld record, which deletes the event, envelope and payload with no analytics copy;
  - receipts keep the stable EventId but no payload copy.
- Explicit legal holds record reason, authorizing actor, start and affected event. A hold blocks expiry until its single release.
- Publication is at-least-once: an event stays pending until its exact acknowledgement, which is a one-way, idempotent state change.
- Identities are allocated by the database (UUIDv7: millisecond time plus random bits).
- Bootstrap fails closed unless the proving NodeId incarnation is current. The previous code ignored the proof result.
- The three existing path-including PostgreSQL crates add `domain` and `character_recovery_fence` path modules, so the Character API compiles and is exercised under the registered target instead of being excluded with `cfg(not(test))`.
