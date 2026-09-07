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
resource_envelope: NATIVE-SOURCE-RESOURCE-ENVELOPE-V1
first_slice_scope: RuntimeScopeRefV1::Channel_ONLY
migration_number: SELECT_AFTER_WP4_PROTECTED_READBACK
foundation_consumer_activation: FORBIDDEN
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
canonical_pg_ci_prerequisite: OTV2-WP5-DEDICATED-POSTGRES-CI-ROUTING-20260907
```

## Authority and resource contracts consumed

This allocation consumes without reopening:

- protected `OPS-SCOPE-ASSIGNMENT-FENCING-V1`;
- accepted FND-03/FND-04 ownership/fencing boundaries;
- protected `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1` and its registered `NASG-*`
  assignment-command resource rows.

The resource envelope is mandatory implementation authority, not optional tuning.
This allocation grants **no registry edit**. The material producer must consume
existing registered NASG bounds exactly, including operation-key/command bytes,
queue and in-flight limits, execution deadlines, retained ambiguity/reconciliation
slot custody and their max/max+1/overflow tests.

Core rules:

- the first slice covers only `RuntimeScopeRefV1::Channel { world_id, channel_id }`;
  the existing `Instance` variant is explicitly unsupported/unallocated;
- one Channel has one current durable assignment generation;
- receiving GameNode/NodeId cannot mint or advance its own assignment;
- assignment is a Game-owned durable PostgreSQL CAS boundary invoked by an
  independently authenticated/authorized control actor;
- **the assignment writer itself**, not the caller, allocates source revision and
  decision identity as checked monotonic successors;
- replacement/revoke advances a never-reused positive ownership generation and
  fences old readiness/writers;
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
- current canonical PG workflows execute only `durability_postgres`; companion
  allocation `OTV2_WP5_DEDICATED_POSTGRES_CI_ROUTING_20260907` must be protected
  and exercised before a dedicated assignment PG target can count as canonical
  acceptance evidence;
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

Persist one current assignment record **only for supported Channel scopes** with
at least:

- exact typed Channel scope: WorldId + ChannelId;
- positive never-reused `ownership_generation`;
- optional current NodeId holder;
- closed state `ASSIGNED | REVOKED`;
- **writer-allocated** positive source revision and writer-issued decision identity;
- immutable accepted operation identity/receipt;
- publication/fence binding sufficient to prevent stale readiness restoration.

Any Instance scope command/read is unsupported and fails closed; no Instance row,
API behavior, schema branch or qualification is authorized in this slice.

Absence is distinct from REVOKED. Revoke/restart must not reset generation,
writer revision/decision or operation high-water. All checked successor overflows
reject permanently/fail closed.

The authenticated control caller supplies intent/preconditions but may not submit,
choose or advance writer source revision/decision identities. Those values are
allocated inside the authoritative writer transition only after exact predecessor
validation; caller-supplied attempts to set/jump them are invalid input and cannot
consume/advance the namespace.

### NASG admission and custody

Every command/reconcile path must enforce the already registered assignment rows
from `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1` before retaining or dispatching work:

- `NASG-OPERATION-KEY`: exact 32 opaque bytes / canonical 43-char unpadded
  base64url at a text boundary;
- `NASG-COMMAND-BYTES`: at most 1,024 retained/encoded bytes per command,
  including actor/scope/predecessor/target/source bindings;
- `NASG-QUEUE`: at most 8 pending commands / 8,192 aggregate retained bytes;
- `NASG-INFLIGHT`: one end-to-end assignment operation slot per logical writer
  registration, 4,096 retained command/result/checkpoint bytes, held through
  completion and ambiguity/reconciliation;
- `NASG-EXECUTION`: 1,000 ms queue wait and 3,000 ms dispatched operation budget
  including DB/lock acquisition; timeout remains ambiguous until authoritative
  reconciliation.

No hidden waiter/retry queue, detached timeout work or new operation key after an
ambiguous result. The exact operation binding is persisted before authoritative
submission, registration custody is fenced across restart, and an occupied
ambiguous slot is reconciled before new work is admitted.

### Command family

- **initial assign**: independently authenticated fresh-store/bootstrap
  authorization, exact Channel scope, and exact target NodeId proven by the
  accepted authenticated process-incarnation registration source;
- **replace**: exact prior Channel/generation/source/publication CAS, with the
  replacement NodeId independently proven as a currently valid registered
  process-incarnation identity under the same target-validation contract; the
  caller cannot nominate an unregistered/stale/mistyped NodeId;
- **revoke**: exact prior CAS, higher generation, no holder, readiness fenced;
- **read/reconcile**: one exact Channel/operation, bounded by NASG result/custody,
  returns authoritative committed assignment/receipt and cannot create authority.

Same operation identity + exact command returns the original committed receipt
without another generation or writer revision. Changed command under the same
identity conflicts. Lost response is reconciled from durable receipt/current
assignment; timeout does not imply abort.

### Atomicity and privileges

Assignment mutation and its required closed runtime-admission fence are one
PostgreSQL transaction. A current ready guard must never remain ready after
replace/revoke. Assignment may be committed while runtime is not ready; that is
closed for admission.

Migration/runtime roles remain least privilege: ordinary GameNode/admission
runtime may read/consume current assignment but cannot INSERT/UPDATE/DELETE the
assignment owner tables, allocate writer revision/decision identities or self-grant.

Every mutation locks assignment/fence state in one documented order compatible
with affected Durability writers. No check-then-unlocked-write.

### Restart and rollback

Restore exact assignment, writer source/decision and operation high-water before
grant consumption. Missing, regressed or contradictory state fails closed.
Reusing a former NodeId or resetting a table cannot restore ownership. PITR/restore
requires the accepted external non-rollback recovery/fence mechanism before
authoritative mutation resumes.

This V1 selects no heartbeat, expiry, automatic failure detector, autoscaling,
orchestration product, process/container topology, live migration or RPO/RTO.
Unresponsive-node replacement remains an explicit authorized CAS command.

## RED/GREEN qualification

RED before implementation must prove current `ScopeRuntimeFence` can consume a
generation but no protected Game producer can establish/reconcile it durably.

GREEN material implementation requires actual PostgreSQL 17.6 evidence **and the
protected dedicated-target CI routing prerequisite** for:

- Channel initial assignment, exact replay and conflicting replay;
- Instance scope assign/replace/read rejected as unsupported;
- NASG max/max+1 for operation key/command/queue/inflight retained bytes plus
  checked arithmetic overflow;
- ninth pending command rejected before retention; second in-flight assignment
  rejected while the first owns its slot;
- queue/execution timeout retains ambiguous slot/custody until reconcile and does
  not spawn a retry or replacement operation key;
- two authorized actors racing from one predecessor -> one CAS winner;
- replace/revoke advance generation and atomically force readiness false;
- stale old generation/NodeId cannot publish ready or perform a fenced write;
- wrong Channel/source/publication binding rejects;
- caller-supplied or jumped source revision/decision identity rejects and does not
  advance the writer namespace;
- replacement target that is unregistered, stale or bound to a different process
  incarnation rejects before authority mutation;
- registered exact replacement target succeeds;
- unauthorized GameNode mutation role is denied by PostgreSQL privileges;
- lost COMMIT response reconciles exactly;
- restart preserves current assignment plus writer/operation high-water and
  reconciles occupied NASG slot before admitting more work;
- injected rollback/regressed generation/revision fails closed;
- generation or writer revision overflow rejects;
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
-> companion dedicated-PG-CI allocation protected
-> WP2 protected
-> WP3 protected/released
-> WP4 protected/released
-> Work chooses exact next migration/path set
-> explicit sole-writer Channel-assignment application with existing NASG rows
-> dedicated PG target exists
-> protected CI routing activated/exercised for that target
-> PostgreSQL TDD implementation + independent review + canonical CI/MQ
-> separate Foundation consumer/fence integration allocation
-> compose with Character Authority + Platform security/trust sources
-> WP5 readiness reevaluation
```

## Excluded scope

No Instance runtime assignment, Foundation mutation, Platform/external repo,
secrets/credentials, production DB/deployment, B/#335, WP3/#356, Character
Authority source, Server Seam/#247, Cargo/lock, registry edits, timing/lease/
autoscaling policy beyond the already registered NASG execution bounds, or
production readiness claim. Workflow source remains separately governed by the
companion CI allocation.

Runtime E2E is NOT_APPLICABLE to this allocation-only document; later material
producer and consumer integration require real database/restart/fencing evidence.
