# DUR-02 — Persistence v1 Owner Baseline

- Status: **OWNER_ACCEPTED**
- Gate: `DUR-02 — Persistence v1`
- Owner decision date: 2026-08-12
- Owner decision time: 09:41 +02:00
- Source type: `USER_SOURCE`
- Decision packet: `DUR-02_PERSISTENCE_V1_WHOLE_GATE_RECONCILIATION.md`
- Consumes: `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`
- DecisionStatus after terminal delivery: **ACCEPTED**
- DeliveryStatus after terminal closeout: **LIFECYCLE_CLOSED**
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- PostgreSQL DDL/migration execution authority: **NONE**

## 1. Owner acceptance

After the lifecycle-closed whole-gate reconciliation reduced the historical Persistence-v1 catch-all to six genuinely common persistence decisions and explicitly classified all fourteen historical DUR-02 subjects, the owner was presented the complete recommendation and replied:

> tak

This is binding acceptance of:

- the six-rule minimum whole-DUR-02 closure package;
- all fourteen historical subject dispositions;
- the exact destination ownership assigned to moved subjects;
- the implementation-unblocking boundary described by the reconciliation;
- all explicit exclusions and downstream numeric/tooling decisions;
- the supersession/reopening discipline of the reconciliation packet.

The accepted Character persistence partial baseline remains binding and is consumed by this whole-gate baseline; it is not superseded or weakened.

## 2. Canonical whole-gate status

After this owner-baseline delivery is merged and its task lifecycle is terminally closed:

```text
DUR-02 — Persistence v1
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migrations       = NOT_AUTHORIZED by architecture acceptance alone
```

Therefore:

```text
DUR-02 architecture accepted
!= PostgreSQL schema implemented
!= migration files executed
!= Rust persistence runtime implemented
!= server runtime implemented
!= production persistence enabled
```

A later implementation package must explicitly receive implementation authority and prove the applicable migration, concurrency, crash, restore, access-control and E2E behavior on exact revisions.

## 3. Accepted rule 1 — migration authority and immutable migration history

For the current native game database boundary (`oteryn_game`), use one authoritative ordered migration history with project-owned explicit migration artifacts checked into `blakinio/Oteryn-v2`.

Binding rules:

- native game migrations are owned by the game repository/migration authority, never Platform;
- Platform and game migration histories remain separate under ADR-0004;
- migration artifacts are explicit, reviewable and immutable after release/merge;
- production game runtime startup never performs authoritative `auto-create`, `auto-update` or schema synchronization;
- production runtime credentials do not receive routine DDL authority;
- schema migration runs through a dedicated least-privilege migration command/job/role with auditable result;
- ordinary server startup validates supported schema/migration compatibility and fails closed on unsupported state;
- multiple game crates/modules may contribute migration source, but the current `oteryn_game` boundary resolves to one authoritative migration ledger/history;
- migration identifiers/content are not reused or silently reinterpreted after release;
- concurrent migration execution requires a database-visible single-winner exclusion mechanism before implementation is accepted.

A genuinely separate future game persistence authority/database/service may use a distinct migration history only after an explicit architecture decision establishes that boundary.

The exact Rust migration crate/library, file naming syntax and deployment orchestrator remain implementation choices unless later evidence shows they alter correctness.

## 4. Accepted rule 2 — transaction isolation, locking and retry correctness

Persistence v1 has no blanket isolation level that substitutes for an invariant proof.

Binding rule:

```text
correctness-sensitive transaction
-> name invariant + authority rows/constraints
-> prove anomaly closure under selected isolation + locks/constraints
-> READ COMMITTED only when that proof is explicit
-> otherwise bounded SERIALIZABLE or stricter accepted domain mechanism
-> retry/reconcile under the same semantic operation identity
```

Consequences:

- application-only check-then-write is not sufficient for uniqueness, ownership, lease, quota or value-conservation correctness;
- advisory locks are never the sole durable authority for ownership, uniqueness, quota, lease or value conservation;
- deadlock/serialization retries preserve OperationId/TransactionId/Command identity where one exists;
- retries do not manufacture a new semantic operation;
- external side effects cannot live inside blindly retried transactions unless their own idempotency/commit protocol is proven;
- retry/backoff counts remain implementation/performance policy;
- stricter domain contracts such as DUR-03 may impose stronger transaction rules.

## 5. Accepted rule 3 — common durable audit/outbox substrate

Use one ANL-compatible semantic substrate pattern for game-domain mandatory durable evidence:

```text
authoritative mutation
+ required revision/receipt
+ immutable retained durable event record
+ mutable crash-safe publication enqueue/checkpoint state
= one owning PostgreSQL transaction
```

Binding rules:

- ANL-01 remains owner of EventId, event envelope, schema/version, durability/privacy class, payload-byte semantics and replay rules;
- each gameplay/domain contract decides which events are mandatory;
- publication is at-least-once and EventId-stable;
- consumers remain idempotent;
- publisher claim/checkpoint state is restart-safe and mutable, while retained event semantic content is immutable during its accepted retention lifetime;
- claiming/submitting work is not proof of delivery;
- publisher crash leaves work retriable/reconcilable;
- ambiguous broker outcome reuses the same EventId and exact semantic content rather than reconstructing a new event;
- attempted publication never deletes the immutable retained event evidence merely because submission was attempted;
- replay rebuilds evidence/projections and never replays authoritative gameplay mutation;
- best-effort telemetry stays outside the mandatory authoritative transaction;
- item/currency/value conservation remains DUR-03-owned even when it consumes the common substrate.

Physical table/index/partition/broker choices remain downstream.

## 6. Accepted rule 4 — durable acknowledgement, runtime checkpoint and disaster RPO are separate

For a mutation whose contract declares it durable:

```text
success acknowledged
=> owning PostgreSQL transaction committed
=> ordinary process/GameNode restart or failover must reconstruct the committed result
```

Persistence v1 permits no application-chosen loss of already acknowledged committed durable mutations during an ordinary process/node restart.

A lost response after commit is reconciled from durable state/receipts; it does not justify replaying the semantic mutation blindly.

Separately:

- FND-03 owns runtime simulation checkpoint/replay semantics for transient/channel execution state;
- a runtime checkpoint is not a substitute for committed Character/item/database durability;
- disaster PITR to an older point may exclude later committed writes according to a separately accepted/measured disaster-recovery RPO;
- disaster RPO is not permission for ordinary runtime to lose acknowledged committed data.

Exact runtime checkpoint cadence, WAL/archive cadence, RPO and RTO remain downstream OPS/PERF/product policy.

## 7. Accepted rule 5 — PITR and restore safety envelope

Production Persistence v1 must be PITR-capable and restore-tested before production-readiness claims.

Binding architecture requirements:

- PostgreSQL base-backup + WAL/PITR capability or an equivalently proven continuous-recovery mechanism appropriate to the accepted topology;
- backup/archive access follows least privilege and accepted encryption/key-management policy;
- replication/high availability is not treated as a backup substitute;
- restore is exercised against named backup artifacts/revisions on a registered operations path rather than existing only as documentation;
- restore validation includes schema/migration ledger, Character/FND-04 fences, operation receipts, durable journal/publication state and all later domain invariants required by accepted owners;
- restored service starts with admission and authoritative mutation closed until reconciliation passes;
- disaster restore establishes a strictly newer non-rollback recovery/authority fence outside values that could have rolled back with the database snapshot, or an equivalently proven mechanism;
- pre-restore TransportBinding/GameSession/lease/runtime authority that cannot be proven current is fenced and cannot resurrect;
- post-restore replay republishes committed durable evidence only and never resubmits gameplay commands as uncommitted work.

Numeric RPO/RTO, backup cadence/retention, provider/topology and restore-drill frequency remain OPS/PERF/product/privacy/security-owned.

## 8. Accepted rule 6 — common compatible schema evolution

Native game persistence follows:

```text
EXPAND
-> MIGRATE / BACKFILL / TRANSFORM
-> VALIDATE
-> CUT OVER compatible readers/writers
-> CONTRACT only after the rollback/compatibility window is intentionally closed
```

Binding rules:

- migrations are deterministic, observable and bounded;
- partial/resumable work is restartable/idempotent where applicable;
- destructive contraction does not occur while an old compatible reader/writer may still serve traffic;
- incompatible writers are fenced before cutover/contract;
- semantic data migrations carry explicit source/destination definition/revision meaning and never silently reinterpret persisted data;
- new authoritative constraints are enabled only after existing data is safely validated/migrated;
- large backfills use bounded batches/progress/resume evidence and do not monopolize gameplay transactions without an accepted maintenance plan;
- rollback is a recovery plan, not a requirement that every migration has a simplistic reversible `DOWN` script;
- unsafe/lossy reversal may require roll-forward repair, compatibility fallback, restore or compensating migration;
- released migration history remains immutable evidence;
- implementation tests every supported mixed-version/cutover window required by the release contract.

Exact zero-downtime guarantees are not implied; maintenance versus online migration remains evidence-driven per migration/release.

## 9. Binding fourteen-subject reconciliation

The historical DUR-02 scope is reconciled as follows:

| # | Historical subject | Binding disposition | Owner / consequence |
|---|---|---|---|
| 1 | migration mechanism and schema ownership | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 1 + ADR-0004 |
| 2 | Character state model and revision fencing | `SATISFIED` | GAME-CHAR + Character persistence partial baseline |
| 3 | Character lease/session persistence ownership | `SATISFIED` | FND-04 + Character persistence partial baseline |
| 4 | inventory/equipment/ground-item transfer transaction boundaries | `MOVED` | GAME-ITEM-01 + DUR-03 |
| 5 | idempotency keys and duplicate-command handling foundation | `SATISFIED` | FND-02 + ANL identities + Character operation receipts; later domain specifics remain domain-owned |
| 6 | isolation levels, locking and retry policy | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 2 |
| 7 | transactional outbox, publication checkpoint, deduplication and recovery | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 3 + ANL-01 semantic ownership |
| 8 | critical audit/journal versus best-effort telemetry semantics | `SATISFIED` | ANL-01 + Character persistence partial baseline |
| 9 | atomic item/currency/security evidence with authoritative mutation | `MOVED` | owning mutation domain + ANL-01; item/currency conservation in DUR-03 |
| 10 | checkpoint interval / maximum accepted progress loss | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 4; numeric runtime/DR values remain downstream |
| 11 | market/guild/house/reward consistency classes | `MOVED` | economy -> EXP-ECONOMY-01; guild/social -> EXP-SOCIAL-01; houses -> EXP-HOUSES-01; recurring/meta rewards -> GAME-META-01; encounter/event rewards -> EXP-EVENTS-01; item/currency conservation -> DUR-03 |
| 12 | partitioning where justified | `IMPLEMENTATION_DEFERRED` | PERF/implementation evidence; no speculative sharding |
| 13 | backup, PITR, restore tests, RPO and RTO | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 5 for capability/safety; numeric objectives downstream |
| 14 | compatible migration rollout and rollback | `RETAIN_DUR02 -> ACCEPTED` | whole DUR-02 rule 6 |

A `MOVED` disposition does not accept the destination gameplay gate. It prevents generic Persistence-v1 from becoming a second semantic owner.

## 10. Consequence for GAME-ITEM-01 and DUR-03

Whole DUR-02 acceptance does **not** require GAME-ITEM-01/DUR-03 to be accepted first.

The ownership chain is now explicit:

```text
accepted DUR-02
-> common persistence substrate + accepted Character/FND-04 persistence may be separately implemented

GAME-ITEM-01 accepted
+ DUR-03 accepted
-> durable item/currency/value mutation may be implemented/proven
```

Until GAME-ITEM-01 and DUR-03 are accepted, no server implementation may claim durable item/currency/value conservation merely because DUR-02 is accepted.

## 11. Implementation transition boundary

This acceptance removes the final generic architecture blocker before a **separately authorized server/persistence foundation implementation programme** for already accepted scopes.

A future implementation programme may be decomposed into bounded executable increments such as:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content vertical-slice increments
```

However, this baseline itself grants **no implementation authority**. A separate owner authorization/task must explicitly permit code/DDL/migrations and define its exact implementation scope and evidence.

## 12. Preserved authority boundaries

- ADR-0004 remains authoritative for PostgreSQL and separate Platform/game ownership.
- DUR-01 remains authoritative for durable identifier representation.
- GAME-CHAR and the Character persistence partial baseline remain authoritative for Character semantics/storage envelope.
- FND-04 remains authoritative for account presence, GameSession, CharacterLease, reconnect/recovery and ControlLoss semantics.
- FND-03 remains authoritative for runtime execution/checkpoint/replay semantics.
- ANL-01 remains authoritative for event/audit semantic contracts and privacy/durability classes.
- GAME-ITEM-01 + DUR-03 retain item/currency/value semantics and conservation.
- domain gates retain market/social/house/reward semantics.
- PERF/OPS/product/privacy/security retain numeric production recovery, capacity, backup-retention and related operational policy.

Persistence is a correctness substrate; it does not invent gameplay semantics.

## 13. Explicitly not decided

This owner baseline does not choose:

- SQL table/column/index/constraint names;
- exact Rust PostgreSQL driver, ORM/query builder or migration library;
- database pool settings;
- numeric timeouts/retry/backoff;
- exact RPO/RTO/backup cadence/retention;
- backup provider/topology/region layout;
- partition/shard keys;
- read replica/cache topology;
- item/currency/market/guild/house/reward schemas or gameplay semantics;
- profile-specific PvP Character extensions;
- production deployment topology;
- unresolved Reference mechanics.

## 14. Supersession and reopening

A later accepted decision may reopen a specific clause only with named evidence such as:

- a PostgreSQL concurrency anomaly not closed by the accepted transaction policy;
- implementation evidence showing the common durable journal/publication substrate cannot satisfy an accepted domain invariant;
- a database/migration-tool limitation that changes correctness rather than convenience;
- measured performance/maintenance evidence requiring partitioning or another persistence topology;
- security/privacy/legal requirements requiring stronger separation, retention or restore behavior;
- an accepted domain contract proving a historical subject was assigned to the wrong owner;
- disaster-recovery evidence requiring a stronger non-rollback authority fence;
- an explicit owner decision superseding ADR-0004 or this baseline.

ORM defaults, library preference, OTS database schemas, convenience or desire to shorten review are not sufficient supersession evidence.

## 15. No runtime or production authority

This owner acceptance does not authorize:

- PostgreSQL DDL or migration execution;
- Rust persistence adapters/repositories;
- GameNode/server runtime implementation;
- protocol listener/transport implementation;
- FND-04 runtime admission/session/lease implementation;
- database provisioning/credentials;
- durable item/currency/value mutation;
- Platform database changes;
- production backup/restore configuration;
- production deployment or traffic.

It closes the architecture gate so a **later explicitly authorized** implementation task can begin from binding contracts rather than another generic persistence design pass.
