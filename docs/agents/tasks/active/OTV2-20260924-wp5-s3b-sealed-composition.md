# OTV2-20260924-wp5-s3b-sealed-composition

```yaml
task_id: OTV2-20260924-wp5-s3b-sealed-composition
title: WP5 S3-B sealed composition of the real protected owners
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp5-s3b-sealed-composition-319
issue: 319
pr: 815
base_sha: 5d4686f736acae407c114f42cc03e55680b14119
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude
created_at: 2026-09-24T00:00:00Z
updated_at: 2026-09-24T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/fresh_admission_composition.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/durability/native_admission_source.rs
  - apps/game-server/src/durability/runtime_scope_assignment.rs
  - apps/game-server/tests/wp5_s3b_composition.rs
  - .github/workflows/wp5-s3b-composition.yml
  - tools/qualification/wp5_s3b/run.sh
  - tools/qualification/wp5_s3b/compose.override.yml
  - tools/qualification/wp5_s3b/nginx.conf
  - docs/agents/tasks/active/OTV2-20260924-wp5-s3b-sealed-composition.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- Status: IMPLEMENTING. Prerequisites are all on protected main:
  - S3-A #760;
  - S2 #757;
  - #415;
  - #414 (#790, `5d4686f`);
  - the Platform producer Oteryn/Oteryn-Platform#1407 (`9147bfd`).
- Writer: Claude. Coordinator: #162. Source programme: #319.
- Scope: seal the fresh-admission composition of the real owners. Every admission fact comes from its owner:
  - Platform account security and fresh signing trust: exact S1 responses, stored in S2 under the #415 custody fence;
  - Character ownership and world: #414, read inside the composition transaction through the reconciled capability and recovery fence;
  - Channel ownership and readiness: the #415 assignment plus holder-attested readiness;
  - durable admission: the existing FreshAdmissionStore commit.
- Characters are bootstrapped from real Platform `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1` decisions:
  - the Platform operator command issues them;
  - they are read over the purpose-separated mTLS route;
  - the Game-owned interpretation is configured through the operator-only SQL procedure.
- Excluded: Platform writes, production, credentials, Foundation production composition, Server Seam and WP5 G0.

## Owned paths

- New S3-B paths: the composition module, the harness, the workflow, and the topology overlay. The overlay has the Platform intent environment variables and the nginx route. The protected S3-A topology files are reused unchanged.
- Minimal visibility changes (`pub(super)`) in S2 (`fence_custody`) and #415 (`channel_scope`, `scope_key`, `load_assignment`, `writer_high_water`, `require_history_matches_high_water`), so the composition can resolve owner facts in its own transaction.
- One #414 in-transaction reader, `load_current_character`, which asserts the recovery fence in the caller's transaction. `record_for` becomes `pub(super)`.

- Commit-boundary revalidation. `FreshAdmissionStore` gains `commit_revalidated`, an owner-revalidation hook that runs inside the commit transaction; `commit` is unchanged. `commit_composed_fresh_admission` uses the hook to recompute the sealed composition from the current S2 floors, the #414 Character under the recovery fence, the #415 assignment and the guards. It rejects the commit as stale authority unless the result equals the prepared composition. This addresses Codex finding 4090536307.

## Qualification

- Canonical: `WP5 S3-B sealed composition qualification` workflow on the exact head.
  - Real Platform `9147bfd` behind TLS 1.3 mTLS nginx/FastCGI with MariaDB.
  - PostgreSQL 17.6 service.
  - Result line `S3B_RESULT=COMPOSED_PASS`.
- The harness checks:
  - intent endpoint purpose separation: the native-evidence client identity is refused, and a native-evidence descriptor is refused;
  - an unknown operation reads as unavailable;
  - exact intent replay;
  - two fresh admissions, stale-composition rejection after #415 replacement, and fencing of the superseded holder;
  - an S2 owner change between compose and commit is rejected as stale authority at the commit boundary.
- Local: fmt, clippy `-D warnings`, and the full `oteryn-game-server` suite on PostgreSQL 17.6 are green.
- The local composed run is not evidence: Docker Hub returned 429 for anonymous pulls in this environment.

## Qualification evidence

The workflow run on `6a2dda59498242153fa96959f02224266d36abbd` (Actions job `107519972291`) produced `S3B_RESULT=COMPOSED_PASS`:

- `postgres_server_version_num=170006 platform=9147bfd3a771762a6646fd87b9172cdb3a6c9a01`
- `platform_intents=issued count=2 path=operator_command ttl_seconds=300`
- `runtime_assignment=assigned holder=A generation=1 readiness=attested`
- `character_interpretation=configured revision=1 path=operator_procedure`
- `intent_reads wrong_client_identity=refused native_evidence_descriptor=refused unknown_operation=unavailable`
- `character_authority=bootstrapped characters=2 intents=platform_operator_issued replay=exact audit=durable_outbox`
- `fresh_admission=committed account=1 holder=A sources=platform_s2,character_414,runtime_415`
- `stale_composition_after_replacement=rejected_stale_authority`
- `superseded_holder=fenced compose=refused s2_custody=refused`
- `owner_change_between_compose_and_commit=rejected_stale_authority`
- `fresh_admission=committed account=2 holder=B generation=2 custody=claimed`
- `cleanup=complete containers=removed volumes=removed ephemeral_keys=removed`
