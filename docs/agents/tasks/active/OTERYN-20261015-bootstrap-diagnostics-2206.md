# OTERYN-20261015-bootstrap-diagnostics-2206

```yaml
task_id: OTERYN-20261015-bootstrap-diagnostics-2206
title: Repair preproduction bootstrap diagnostics and temporal row-lock privileges
mode: REPAIR
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/preprod-bootstrap-diagnostics-2206
pr: null
issue: 2206
base_sha: b2f65117ac8bfb05a98aa7d7d76f3ae20d923565
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex standalone root task for client PR 1942
created_at: 2026-10-15T07:49:53Z
updated_at: 2026-10-15T08:05:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/node/serve.rs
  - apps/game-server/migrations/0088_temporal_schedule_row_lock.sql
  - apps/game-server/tests/support/spell_item_temporal_pg_cases.sql
  - apps/game-server/tests/support/spell_field_chain_pg_cases.sql
  - docs/agents/tasks/active/OTERYN-20261015-bootstrap-diagnostics-2206.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2208](https://github.com/Oteryn/Oteryn-Game/pull/2208). Temporal schedule row-lock privilege repair and static bootstrap diagnostic stages are merged. Evidence: `apps/game-server/src/node/serve.rs`.

Remaining: Live bootstrap/QA Character acceptance remains an external operational fact not proved by repository source; the merged repair must not be reimplemented. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## Outcome

Governing issue: #2206. Continue the owner's QA provisioning request for #1942.
Persist the narrow schedule row-lock privilege repair and identify which bootstrap
boundary returns unavailable. This candidate does not claim Character creation or
five vocations are working.

## Architecture and source of truth

- PROVEN: Game 7411e612 deployed successfully, Actions run 38697082633.
- PROVEN: PostgreSQL rejected runtime temporal schedule row locks; SELECT/INSERT
  alone do not permit FOR UPDATE. Both schedule tables retain immutable UPDATE/
  DELETE and TRUNCATE guards. Preproduction received UPDATE(item_instance_id) on
  each table, with no payload-column, DELETE or TRUNCATE grant.
- PROVEN: authenticated Platform bootstrap intent reads succeed. One account
  security read took 5.552 seconds, exceeding the unchanged 3-second exchange bound.
- UNKNOWN: exact failing bootstrap boundary; existing diagnostics collapse it to
  unavailable. New diagnostics contain only static stage/reason classes.
- PROVEN: no QA Character or bootstrap receipt committed at the last checkpoint.
- PROVEN: normal web creation is a separate default-off path. Game's deployed
  node config has no character_commands section; Platform main 2184dfae config
  and the merged step-3 task explicitly retain U-CC7/U-CC9 enablement gates.
  Contract sections 13-14 require recovery reconciliation acceptance and the
  backlog enablement packet; this repair does not silently activate that path.

## High-risk authority/recovery qualification

The permission delta permits row locking of existing immutable schedule rows;
the existing guard remains the independent mutation boundary. No new authority,
session, receipt, fence, source facts or time bounds are introduced.
Applicable operators: missing row-lock grant; attempted identity UPDATE; payload
UPDATE; DELETE; TRUNCATE. Tests use the actual restricted runtime role and real
schedule rows. Each mutation is isolated. Existing fixture guards retain receipt,
audit, early-expiry and chain-order negative cases. Session/generation/World
substitution is NOT_APPLICABLE to this permission-only repair: those consumers
are unchanged. Required independent exact-head deep review remains pending.

## Acceptance criteria

- [x] Missing runtime lock privilege reproduced on PostgreSQL 17.6.
- [x] Both restricted-role schedule locks succeed with the forward migration.
- [x] Identity UPDATE still fails at the immutable trigger; payload UPDATE,
  DELETE and TRUNCATE remain unauthorized for both schedule types.
- [x] Bootstrap logs only static failure stage/reason classes.
- [ ] Required Rust checks, exact-head CI and independent review pass.
- [ ] Live bootstrap succeeds after the actual remaining cause is repaired.

## Validation

- PostgreSQL 17.6 RED: new runtime-role lock case fails with SQLSTATE 42501 on
  game_spell_item_temporal_schedules before migration 0088.
- PostgreSQL 17.6 GREEN: `python tools/content-schema/native-gameplay/verify_house_privacy_pg.py --container qa-bootstrap-pg` PASS, including both runtime-role locks and mutation rejection cases.
- Rust formatting: PASS on product files. Strict server all-target Clippy and
  server tests: pending; the parallel run was stopped at the cloud memory limit
  and validation continues with CARGO_BUILD_JOBS=1 and test debug symbols off.
- `python tools/agents/validate_governance.py`: PASS.
- `python -m unittest discover -s tools/agents/tests`: PASS, 124 tests.
- Full changed-file self-review: PASS; sole payload delta is one narrow grant
  per existing schedule table and static diagnostics; no consumer decision,
  authority, freshness, receipt or immutable guard is changed.

## Context checkpoint

Single writer on the named canonical branch; API-native new-candidate publication
uses one complete-delta tree, one sole-parent commit and one non-force ref update.
No local Git candidate is being published. No protected integration is authorized
by this record. Preserve the private QA credentials and all prior operation IDs;
reconcile receipts before issuing a new expired uncommitted intent.

Next action: finish deterministic validation on the frozen candidate and request
the required independent review. Exact SHA, canonical PR and check results are
retained in immutable PR comments after publication, not self-referential commits.
Live QA provisioning remains unfinished; normal web enablement still requires
the accepted owning-contract gates.
