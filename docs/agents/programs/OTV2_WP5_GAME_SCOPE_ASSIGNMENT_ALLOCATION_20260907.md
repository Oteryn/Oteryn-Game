# WP5 Game runtime-scope assignment authority allocation

Coordinator: #162. Source-readiness issue: #319. Programme: #364.
Preparation authority: #319 comment `5575508271`.

## Status

Prospective Game-only allocation. **NOT_ACTIVE**. No runtime/SQL/migration,
Foundation-consumer, production, Platform or external write authority follows
from this document.

```yaml
allocation_id: OTV2-WP5-GAME-SCOPE-ASSIGNMENT-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
allocation_base_main_sha: 494723b0d271e675eb44e63661c8575936ed6144
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_WORK_APPLICATION
accepted_decision: OPS-SCOPE-ASSIGNMENT-FENCING-V1
migration_number: SELECT_AFTER_WP4_PROTECTED_READBACK
foundation_consumer_activation: FORBIDDEN
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
```

## Authority consumed

This allocation consumes the protected scope-assignment/fencing decision and
accepted FND-03/FND-04 boundaries without reopening them:

- one logical runtime scope has one current durable assignment generation;
- receiving GameNode/NodeId cannot mint or advance its own assignment;
- assignment is a Game-owned durable PostgreSQL CAS boundary invoked by an
  independently authenticated/authorized control actor;
- replacement/revoke advances a never-reused positive generation and fences old
  readiness/writers;
- readiness is separate from assignment and requires independently current
  runtime facts under the matched fence;
- commit-before-publish, restart high-water preservation and fail-closed missing
  or regressed assignment state are mandatory.

Current Foundation `ScopeRuntimeFence` / `RuntimeScopeRefV1` consume a supplied
grant. They are not producer authority and are not writable in this allocation.

## Fresh protected facts and serialization

At the preparation base:

- protected main has only immutable migration `0001_admission_reconnect_journal.sql`;
- Child B owns unreleased `0002_fresh_admission_authority.sql`, which already
  introduces the admission runtime-guard relation;
- WP3 owns the shared PostgreSQL qualification target until terminal driver
  acceptance/release;
- therefore this assignment owner must not modify `0002`, B's runtime guard or
  shared tests concurrently;
- the next forward migration number and exact Durability integration hooks are
  selected only after protected WP4 readback/release.

## Prospective producer surfaces

After explicit later Work application and fresh ownership verification:

```text
apps/game-server/src/durability/runtime_scope_assignment.rs     # new owner
apps/game-server/src/durability/mod.rs                          # module/export only
apps/game-server/src/durability/db.rs                           # exact adapter hook only
apps/game-server/src/durability/schema.rs                       # exact codec/schema hook only
apps/game-server/migrations/<NEXT>_runtime_scope_assignment.sql # new after WP4
apps/game-server/tests/runtime_scope_assignment_postgres.rs     # new dedicated target
```

Existing Durability paths and migration namespace are conditional future paths,
not current leases. Use the dedicated PG target rather than
`durability_postgres.rs`.

Foundation `admission.rs`, `admission_recovery_inner.rs`, Server Seam and runtime
composition are explicitly excluded from this producer allocation. A later exact
consumer-integration allocation is required after the producer is protected.

## Required producer contract

Persist one current assignment record per supported `RuntimeScopeRefV1` with at
least:

- exact typed scope;
- positive never-reused `ownership_generation`;
- optional current NodeId holder;
- closed state `ASSIGNED | REVOKED`;
- monotonic source revision/decision identity;
- immutable accepted operation identity/receipt;
- publication/fence binding sufficient to prevent stale readiness restoration.

Absence is distinct from REVOKED. Revoke/restart must not reset generation or
operation high-water. Checked successor overflow rejects permanently/fail closed.

### Command family

- initial assign: authenticated fresh-store/bootstrap authorization, exact scope,
  registered target process-incarnation identity;
- replace: exact prior scope/generation/source/publication CAS, higher generation,
  new target, old readiness fenced;
- revoke: exact prior CAS, higher generation, no holder, readiness fenced;
- read/reconcile: returns authoritative committed assignment/receipt and cannot
  create authority.

Same operation identity + exact command returns the original committed receipt
without another generation. Changed command under the same identity conflicts.
Lost response is reconciled from durable receipt/current assignment; timeout does
not imply abort.

### Atomicity and privileges

Assignment mutation and its required closed runtime-admission fence are one
PostgreSQL transaction. A current ready guard must never remain ready after
replace/revoke. Assignment may be committed while runtime is not ready; that is
closed for admission.

Migration/runtime roles remain least privilege: ordinary GameNode/admission
runtime may read/consume current assignment but cannot INSERT/UPDATE/DELETE the
assignment owner tables or self-grant.

Every mutation locks assignment/fence state in one documented order compatible
with affected Durability writers. No check-then-unlocked-write.

### Restart and rollback

Restore exact assignment/operation high-water before grant consumption. Missing,
regressed or contradictory state fails closed. Reusing a former NodeId or resetting
a table cannot restore ownership. PITR/restore requires the accepted external
non-rollback recovery/fence mechanism before authoritative mutation resumes.

This V1 selects no heartbeat, expiry, automatic failure detector, autoscaling,
orchestration product, process/container topology, live migration or RPO/RTO.
Unresponsive-node replacement remains an explicit authorized CAS command.

## RED/GREEN qualification

RED before implementation must prove current `ScopeRuntimeFence` can consume a
generation but no protected Game producer can establish/reconcile it durably.

GREEN material implementation requires actual PostgreSQL 17.6 evidence for:

- initial assignment, exact replay and conflicting replay;
- two authorized actors racing from one predecessor -> one CAS winner;
- replace/revoke advance generation and atomically force readiness false;
- stale old generation/NodeId cannot publish ready or perform a fenced write;
- wrong scope/NodeId/source/publication binding rejects;
- unauthorized GameNode mutation role is denied by PostgreSQL privileges;
- lost COMMIT response reconciles exactly;
- restart preserves current assignment and high-water;
- injected rollback/regressed generation fails closed;
- generation overflow rejects;
- DB outage/partition cannot make cached ownership sufficient for new mutation or
  readiness publication.

Component fixtures are not production availability evidence. Later consumer
composition must independently prove every activated writer/output boundary is
fenced, not only admission.

## Remaining WP5 composition

This producer does not supply:

- Platform native AccountId/security/signing-trust observations;
- Game Character Authority ownership/world source;
- authenticated native source-descriptor ingestion/high-water restoration for
  Platform security/trust;
- Server Seam or gameplay command registration.

Those remain separate owners and must compose before WP5/G0 readiness.

## Activation sequence

```text
prospective allocation protected
-> WP2 protected
-> WP3 protected/released
-> WP4 protected/released
-> Work chooses exact next migration/path set
-> explicit sole-writer assignment-owner application
-> PostgreSQL TDD implementation + independent review + canonical CI/MQ
-> separate consumer/fence integration allocation
-> compose with Character Authority + Platform security/trust sources
-> WP5 readiness reevaluation
```

## Excluded scope

No Foundation mutation, Platform/external repo, secrets/credentials, production
DB/deployment, B/#335, WP3/#356, Character Authority source, Server Seam/#247,
Cargo/lock, workflow/ruleset/MQ, registry, timing/lease/autoscaling policy or
production readiness claim.

Runtime E2E is NOT_APPLICABLE to this allocation-only document; later material
producer and consumer integration require real database/restart/fencing evidence.
