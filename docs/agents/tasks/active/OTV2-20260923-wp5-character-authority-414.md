# OTV2-20260923-wp5-character-authority-414

```yaml
task_id: OTV2-20260923-wp5-character-authority-414
title: WP5 Character authority first slice
mode: REPAIR
status: implementing
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
  - apps/game-server/src/character_bootstrap_intent.rs
  - apps/game-server/src/native_admission_source/descriptor.rs
  - apps/game-server/src/native_admission_source/mod.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/character_authority_audit.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/migrations/0005_character_authority.sql
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/native_admission_source_postgres.rs
  - apps/game-server/tests/runtime_scope_assignment_postgres.rs
  - apps/game-server/tests/native_admission_source_transport.rs
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

- Status: IMPLEMENTING. The Platform producer for `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1` is protected-integrated as Oteryn/Oteryn-Platform#1407 (`9147bfd3a771762a6646fd87b9172cdb3a6c9a01`). The Game consumer is implemented here under the #795 handoff.
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

## Authenticated bootstrap intent consumer (#795)

- `BootstrapCommand` is gone. `bootstrap_character` consumes only a `CharacterBootstrapIntentV1`, and that type has no field constructor. It is produced only by strictly decoding the complete Platform decision:
  - exactly the 13 contract members, with no unknown, duplicate or missing member;
  - only the `OPERATOR_CONTROL_PLANE_BOOTSTRAP` variant, with the fixed issuer, operation and audience;
  - canonical UUIDs and decimals;
  - a source validity window of 1–300 s.
- The decoder does not authenticate by itself, like the S1 codec. Authenticated bytes come from the purpose-separated TLS 1.3 mTLS read `native_admission_source::read_character_bootstrap_intent`:
  - it uses its own `Operation` path, `/internal/v1/game-auth/character-bootstrap-intents/read`, and an issuer-bound descriptor;
  - every non-200 response is bounded unavailability.
- A new bootstrap proves all of the following in one transaction before committing:
  - the recovery fence;
  - the current #415 incarnation;
  - an unexpired, non-future intent at trusted Game time;
  - current allowed S1/S2 account security for the intent's AccountId, read from the S2 fresh-admission floor within the FND-04 5 s window and share-locked;
  - the retained source high-water: a lower revision is stale, and an equal revision is a contradiction.
- The same transaction advances the high-water and commits the root, the receipt, the audit event and the outbox row.
- The receipt binds the complete intent, including decision id, source revision and source times.
- An exact retry returns the committed result. Changed reuse conflicts.
- `reconcile_character_bootstrap` resolves an uncertain outcome by operation identity, without re-authorizing.
- Integrity requires that the high-water exists exactly when receipts exist and that it equals the newest receipt's decision. A restore therefore cannot drop or regress it, and absence is never read as zero. The high-water row only advances and cannot be truncated.
- Shared paths allocated for this consumer:
  - the S1 transport `descriptor.rs` and `mod.rs`: one operation variant and the narrow read;
  - its transport test crate.
- Qualification on local PostgreSQL 17.6:
  - `authenticated_intent_qualification_matrix` covers the #795 matrix: missing, denied and stale S2; exact and concurrent retries; changed reuse; stale and contradictory revisions; expired and future intents; injected outbox failure rolling back the high-water; concurrent distinct operations; restart; a replaced #415 proof; and a dropped or regressed restored high-water;
  - mutation checks confirm that disabling the S2 gate, the high-water comparison or the high-water presence check each fails the matrix.
