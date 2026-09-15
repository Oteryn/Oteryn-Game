# DUR-02 — Persistence v1 Whole-Gate Reconciliation

- Status: **PRE-DECISION RECONCILIATION / NOT ACCEPTED**
- Date: 2026-08-12
- Stable gate: `DUR-02 — Persistence v1`
- Trusted base: `blakinio/Oteryn-v2@710c4b5e00de9f14224a6949c3bc7364f4c724a4`
- Current overall status: `PROPOSED / PLANNED / NOT_STARTED`; this pre-decision analysis task is not an owner-baseline delivery of the gate
- Existing accepted sub-scope: `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md`
- Runtime / PostgreSQL DDL authority: **NONE**
- Decision owner: product/architecture owner

## 1. Why this reconciliation exists

The original `DUR-02 — Persistence v1` backlog was intentionally broad. Since it was written, multiple later gates have taken explicit ownership of pieces that were once grouped under generic persistence:

- `DUR-01` owns durable identifier representation;
- FND-04 owns presence/lease/session/reconnect authority semantics;
- the owner-accepted DUR-02 Character persistence partial baseline owns the profile-neutral Character persistence envelope;
- `ANL-01` owns event/audit semantics and durability classes;
- `GAME-ITEM-01` + `DUR-03` own item/currency conservation and transfer semantics;
- FND-03 owns runtime execution/checkpoint/replay semantics;
- `PERF-01` and operations gates own measured production capacity/latency and operational recovery objectives;
- dedicated gameplay/product domains own market, guild, house and reward behavior.

Keeping every historical bullet under DUR-02 would now create two problems:

1. **false blocking** — implementation could be delayed by questions that no longer belong to persistence architecture;
2. **dual authority** — a generic persistence gate could accidentally redefine semantics already owned by GAME-ITEM, FND-04, ANL, OPS or another domain.

This reconciliation therefore classifies every historical DUR-02 subject and reduces the remaining owner decision to the minimum persistence architecture that must be binding before a real Persistence-v1 implementation package is designed.

## 2. Disposition vocabulary

Every historical subject receives one primary disposition:

- `SATISFIED` — current binding architecture already closes the architecture question; future implementation evidence is not a new architecture decision;
- `RETAIN_DUR02` — a whole-Persistence-v1 architecture decision is still required here;
- `MOVED` — the subject is now owned by another named gate/domain and must not remain a generic DUR-02 blocker;
- `IMPLEMENTATION_DEFERRED` — architecture can safely leave the exact choice to measured implementation/operations evidence.

A `MOVED` subject is not accepted merely by this document. It remains blocked by its destination gate where that gate is still unaccepted.

## 3. Exhaustive historical-subject reconciliation

| # | Historical DUR-02 subject | Primary disposition | Current owner/evidence | Consequence |
|---|---|---|---|---|
| 1 | Rust migration mechanism and schema ownership | `RETAIN_DUR02` | ADR-0004 already fixes game/Platform DB ownership separation and distinct migration histories, but the game-side migration artifact/runner authority model is not yet whole-gate binding | freeze the architecture mechanism, but not a Rust library |
| 2 | Character state model and revision fencing | `SATISFIED` | owner-accepted DUR-02 Character persistence partial baseline + GAME-CHAR | consume `character_root`, CharacterRevision, typed children and fences; do not redesign here |
| 3 | Character lease schema and ownership | `SATISFIED` | FND-04 + owner-accepted Character persistence partial baseline | AccountPresenceClaim/CharacterLease/GameSession/ControlLoss remain separate; implementation evidence later |
| 4 | Inventory/equipment and ground-item transfer transaction boundaries | `MOVED` | `GAME-ITEM-01` + `DUR-03` | no generic DUR-02 item transaction design; value conservation remains blocked there |
| 5 | Idempotency keys and duplicate-command handling | `SATISFIED` | FND-02 CommandId semantics + ANL OperationId/TransactionId + Character persistence receipts; future item/economy specifics belong to DUR-03 | no universal global idempotency key or command table is invented |
| 6 | Isolation levels, locking and retry policy | `RETAIN_DUR02` | Character partial baseline closes Character-specific behavior; whole game persistence still needs a common minimum policy | accept explicit anomaly proof and bounded retry discipline without overriding stricter domain contracts |
| 7 | Transactional outbox boundaries, publication checkpoints, deduplication and recovery | `RETAIN_DUR02` | ANL-01 fixes semantic event/audit behavior; Character partial baseline fixes Character atomicity | freeze one common game-DB substrate boundary; domains still decide which events are mandatory |
| 8 | Critical append-only audit/journal scope and separation from best-effort telemetry | `SATISFIED` | ANL-01 + Character persistence partial baseline | durability/privacy/event-family semantics remain ANL-owned; physical implementation later consumes them |
| 9 | Atomic production of item/currency/security evidence with owning authoritative transaction | `MOVED` | item/currency: `GAME-ITEM-01` + `DUR-03` + ANL-01; other security-sensitive mutations: their owning authoritative domain + ANL-01 | generic DUR-02 supplies shared atomic substrate only; it does not define value/security mutation semantics |
| 10 | Checkpoint interval and maximum accepted progress loss | `RETAIN_DUR02` | FND-03 owns runtime checkpoint/replay; Character persistence accepts commit-before-success; production DR objectives are OPS/PERF-owned | freeze the durable-ack vs runtime-checkpoint/disaster-RPO distinction; do not guess numeric intervals |
| 11 | Market, guild, house and reward consistency classes | `MOVED` | market/economy -> `EXP-ECONOMY-01`; guild/social -> `EXP-SOCIAL-01`; houses -> `EXP-HOUSES-01`; recurring/meta rewards -> `GAME-META-01`; encounter/event rewards -> `EXP-EVENTS-01`; item/currency conservation for any of them -> `DUR-03` | persistence consumes those contracts later; it does not choose their semantics now |
| 12 | Partitioning where justified | `IMPLEMENTATION_DEFERRED` | PERF/implementation evidence | no speculative partitioning/sharding; introduce only from measured load/maintenance evidence with migration plan |
| 13 | Backup, PITR, restore tests, RPO and RTO | `RETAIN_DUR02` | Character partial baseline fixes no-authority-resurrection; numeric targets remain OPS/PERF/product-owned | freeze capability/safety envelope and delegate numbers/topology |
| 14 | Compatible migration rollout and rollback | `RETAIN_DUR02` | Character partial baseline has staged migration discipline but only for its accepted sub-scope | elevate compatible schema-evolution discipline to Persistence-v1 common architecture |

All fourteen historical subjects are accounted for. None is omitted or implicitly accepted by convenience.

## 4. What remains genuinely owned by whole DUR-02

After reconciliation, only six architecture decisions remain necessary for the common Persistence-v1 layer:

1. migration artifact and migration authority model;
2. common transaction isolation/locking/retry principle;
3. shared durable audit/outbox persistence substrate boundary;
4. durable acknowledgement versus runtime checkpoint/disaster progress-loss semantics;
5. PostgreSQL backup/PITR/restore safety envelope;
6. common compatible schema-evolution discipline.

These six decisions are the **minimum whole-DUR-02 closure package** recommended below.

## 5. Decision 1 — game migration artifacts and authority

### Recommendation

Use one authoritative ordered migration history for the current native game database boundary, with project-owned immutable migration artifacts checked into `blakinio/Oteryn-v2`.

Architecture rules:

- game migrations are owned by the native game repository/migration authority, never by Platform;
- Platform and game migration histories remain independent under ADR-0004;
- the canonical migration artifact is explicit, reviewable, deterministic schema/data migration content — normally project-owned SQL plus bounded migration metadata/verification logic where required;
- ORM/runtime automatic schema synchronization (`auto-create`, `auto-update`, `synchronize schema on startup`) is **not** an authoritative production migration mechanism;
- production game runtime credentials do not receive routine DDL authority;
- migrations are applied by a dedicated migration command/job/role with least privilege and an auditable result;
- ordinary game-server startup validates supported schema/migration compatibility and fails closed on unsupported state rather than silently migrating production;
- multiple crates/modules may own migration source fragments, but application ordering resolves into one authoritative game migration ledger/history rather than independent competing schema histories inside the same game database;
- migration identifiers are unique, ordered according to the chosen mechanism and never reused/reinterpreted after merge/release;
- concurrent migration runners must have a single-winner database-visible exclusion mechanism before implementation is accepted.

This one-ledger rule applies to the current `oteryn_game` database boundary. A later accepted ADR may create a genuinely separate game-domain database/service with its own migration authority; it must not silently fork the same authoritative schema history.

### Deliberately not frozen

The exact Rust migration crate/library is **not** an architecture decision unless implementation evidence proves it changes these guarantees. It is selected later under dependency/security/maintenance review.

Likewise, this gate does not freeze exact schema names, file naming syntax or deployment orchestrator product.

## 6. Decision 2 — isolation, locking and retries

### Recommendation

Persistence v1 has **no blanket transaction isolation level that substitutes for an invariant proof**.

Common rule:

```text
correctness-sensitive transaction
-> name the invariant and authority rows/constraints
-> prove the anomaly is closed under the selected isolation + locks/constraints
-> use READ COMMITTED only when that proof is explicit
-> otherwise use bounded SERIALIZABLE or a stricter accepted domain mechanism
-> retry/reconcile with the same semantic operation identity
```

Binding principles if accepted:

- `READ COMMITTED` is the normal performance baseline only for transactions whose races are explicitly closed by row/key locks, generation/revision compares and authoritative constraints;
- check-then-write application logic without a database-authoritative conflict mechanism is not sufficient for uniqueness/conservation/lease authority;
- advisory locks are coordination/performance aids only and never the sole durable authority for ownership, uniqueness, quota, lease or value conservation;
- deadlock/serialization retries retain the same OperationId/TransactionId/Command identity where that identity exists; retries do not manufacture a new semantic operation;
- external side effects do not occur inside a blindly retried database closure unless their own idempotency/commit protocol is proven;
- bounded retry/backoff counts are implementation/performance policy, not architecture constants;
- a stricter later domain contract such as DUR-03 may supersede this minimum for its transactions.

This elevates the already accepted Character pattern into a minimum game-persistence principle without pretending that all domains use the same locks or transaction topology.

## 7. Decision 3 — one shared durable audit/outbox substrate boundary

### Recommendation

Use one common game-database **durable event journal + publication state substrate pattern** for authoritative game domains that require ANL-01 durable evidence.

This does not mean one giant event table must remain physically unpartitioned forever. It means one semantic persistence mechanism and one ownership contract are shared rather than every domain inventing incompatible outbox/audit behavior.

Rules:

- ANL-01 remains the owner of EventId, envelope, event-family schema, privacy class, durability class, exact payload-byte semantics and replay rules;
- the owning gameplay transaction decides which durable event(s) are mandatory under its accepted domain contract;
- when durable evidence is mandatory, authoritative mutation + required domain revision/receipt + immutable retained event record + publication enqueue/state commit in the same PostgreSQL transaction or none becomes authoritative;
- publication state is mutable delivery bookkeeping; retained event semantic content is immutable for its accepted retention lifetime;
- publication is at-least-once and EventId-stable; consumers remain idempotent;
- publication claim/checkpoint state is restart-safe: claiming work is not proof of delivery, a publisher crash before confirmed disposition leaves the event retriable after claim expiry/reconciliation, and a delivery attempt cannot delete the immutable event simply because transport submission occurred;
- publication-state transitions must make ambiguous broker/transport outcomes reconcilable without creating a second event identity or reconstructing content;
- publisher retry never reconstructs old event payload from later mutable gameplay state;
- replay rebuilds evidence/projections and never replays authoritative gameplay mutation;
- best-effort telemetry is outside the transaction and may have distinct overload/drop semantics;
- item/currency/economy domains do not become accepted by consuming this substrate — DUR-03 still owns conservation and mandatory value-evidence semantics.

Exact table/index/partition/broker and claim-lease implementation remain implementation-owned subject to these invariants and ANL limits.

## 8. Decision 4 — durable acknowledgement, runtime checkpoint and progress loss are different concepts

The historical phrase `checkpoint interval and maximum accepted progress loss` mixed three distinct failure models.

### Recommendation

Freeze them separately:

#### A. Authoritative durable transaction

For a mutation whose contract says it is durable:

```text
success acknowledged
=> owning PostgreSQL transaction committed
=> ordinary process/GameNode restart or failover must reconstruct that committed mutation
```

Persistence v1 intentionally permits **zero application-chosen loss of already acknowledged committed durable mutations during ordinary process/node restart**.

A lost response after commit is reconciled from authoritative state/receipt; it does not undo the commit.

#### B. Runtime simulation checkpoint/replay

FND-03 owns runtime execution/checkpoint/replay semantics for transient/channel simulation state. Runtime checkpoint cadence is not a surrogate for committed Character/item/database durability.

#### C. Disaster restore RPO

Restoring a database to an older PITR point may, by definition, exclude writes committed after that restore point. That is a separately accepted disaster-recovery RPO/product/operations decision, not permission for ordinary runtime to lose acknowledged data.

After a disaster restore, the system must identify/fence the restored authority epoch and never pretend that pre-restore live sessions/leases remain current.

### Numeric policy

Exact checkpoint cadence, WAL/archive cadence, RPO and RTO are not guessed in DUR-02. They require named OPS/PERF/product milestone evidence.

## 9. Decision 5 — backup, PITR and restore safety envelope

### Recommendation

Production Persistence v1 must be **PITR-capable and restore-tested**, while exact operational objectives remain downstream.

Minimum architecture requirements:

- PostgreSQL base-backup + WAL/PITR capability or an equivalently proven continuous recovery mechanism appropriate to the accepted topology;
- backups/archives are protected by least-privilege access and accepted encryption/key-management policy; secrets are not embedded in repository configuration;
- replication/high availability is not treated as a backup substitute;
- restore is exercised automatically or on a registered recurring operations path against named backup artifacts/revisions rather than existing only as documentation;
- restore validation checks schema/migration ledger, Character/FND-04 fences, operation receipts, durable journal/publication state and later domain invariants required by accepted owners;
- restored service starts with admission/authoritative mutation closed until reconciliation passes;
- PITR/disaster recovery establishes a strictly newer non-rollback authority/recovery fence outside values that could have rolled back with the restored database snapshot, or an equivalently proven mechanism;
- pre-restore TransportBinding/GameSession/lease/runtime authority that cannot be proven current is fenced and cannot resurrect;
- replay after restore republishes only committed durable evidence; it never resubmits gameplay commands as if they were uncommitted work.

### Moved operational decisions

The following do **not** block DUR-02 architecture acceptance:

- numeric RPO;
- numeric RTO;
- backup/WAL retention days;
- backup frequency/topology/provider;
- restore-drill frequency;
- storage vendor/region layout.

They belong to OPS/PERF/product/privacy/security policy and must be registered before the corresponding production milestone.

## 10. Decision 6 — common schema evolution discipline

### Recommendation

Elevate the Character partial-baseline migration model to the common native game persistence architecture:

```text
EXPAND
-> MIGRATE / BACKFILL / TRANSFORM
-> VALIDATE
-> CUT OVER compatible readers/writers
-> CONTRACT only when rollback/compatibility window is intentionally closed
```

Rules:

- migrations are deterministic, observable, bounded and restartable/idempotent where a partial run is possible;
- destructive schema contraction does not occur while an old compatible writer/reader may still be serving traffic;
- incompatible writers are fenced before cutover/contract;
- semantic data migrations record source/destination definition/revision semantics and may not silently reinterpret persisted data;
- constraints become authoritative after existing data has been validated or migrated safely;
- large backfills must have bounded batches/progress/resume evidence and cannot monopolize authoritative gameplay transactions without an accepted maintenance plan;
- rollback is a **recovery plan**, not a promise that every migration has a trivial `DOWN` script;
- when reversing a data transformation would be unsafe/lossy, the accepted rollback may be roll-forward repair, compatibility fallback, backup restore or explicit compensating migration;
- migration history is immutable evidence: already released migration identifiers/content are not rewritten to make history look cleaner;
- implementation must test supported mixed-version/cutover windows named by the release contract.

Exact zero-downtime guarantees are not implied. Maintenance versus online migration is chosen per migration/release evidence.

## 11. Subjects deliberately removed from the whole-DUR-02 blocking set

### Item / inventory / ground / currency

`GAME-ITEM-01` defines item semantics; `DUR-03` defines transfer/conservation/anti-duplication. Whole DUR-02 may provide shared database primitives but cannot accept those semantics on their behalf.

### Market / guild / houses / rewards

The persistence layer consumes the contract of the originating gameplay domain:

- market/economy semantics: `EXP-ECONOMY-01`;
- guild/social semantics: `EXP-SOCIAL-01`;
- house semantics: `EXP-HOUSES-01`;
- recurring/meta reward semantics: `GAME-META-01`;
- encounter/event reward semantics: `EXP-EVENTS-01`;
- whenever any of those mutate item/currency value, `DUR-03` remains the conservation/anti-duplication authority.

A generic database gate must not choose whether an auction, guild roster, house or reward is world-global, channel-local, escrowed, compensating or strongly serialized.

### Partitioning / sharding

No table/domain is partitioned merely because the architecture expects scale. Partitioning must be justified by measured cardinality, write/read workload, retention, maintenance or failure-isolation evidence. `PERF-01` and the later implementation can introduce it with a migration/rollback plan.

### Exact Rust DB/migration technology

Library selection is downstream as long as the selected stack can prove the accepted transaction, migration, typing, cancellation, timeout and security requirements. A future implementation finding may reopen the architecture only if the library/driver imposes a real correctness constraint.

## 12. Minimum whole-DUR-02 closure decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

Accept these six rules as the remaining whole-Persistence-v1 architecture:

1. **Migration authority:** one game-owned ordered migration history for the current native game database boundary; immutable project-owned explicit migration artifacts; dedicated least-privilege migrator; no production runtime auto-schema-sync; exact Rust migration library deferred.
2. **Transaction correctness:** no blanket isolation as proof; READ COMMITTED only with explicit anomaly-closing locks/constraints, otherwise bounded SERIALIZABLE/stricter accepted mechanism; same semantic identity across retry.
3. **Common audit/outbox substrate:** one ANL-compatible durable journal + mutable crash-safe publication-state/checkpoint pattern shared by authoritative game domains; mandatory evidence commits atomically with its owning mutation; telemetry stays separate.
4. **Progress-loss separation:** acknowledged durable mutation means committed and reconstructible across ordinary process/node restart; FND-03 runtime checkpoint/replay is separate; disaster restore RPO is a separately measured operational/product policy.
5. **PITR/restore envelope:** production Persistence v1 is PITR-capable and restore-tested, starts restored authority fail-closed, and uses a newer non-rollback recovery fence/equivalent before admission resumes; numeric RPO/RTO/cadence stay OPS/PERF-owned.
6. **Schema evolution:** game-wide expand -> migrate/backfill -> validate -> cut over -> contract discipline with writer fencing, resumable data work, explicit semantic migrations and evidence-based rollback/recovery rather than mandatory simplistic down-migrations.

Accept the reconciliation dispositions in section 3 at the same time:

- Character/lease persistence questions are already satisfied by accepted Character/FND-04 architecture;
- item/currency/value conservation moves to GAME-ITEM/DUR-03;
- market/guild/house/reward consistency moves to the exact domain gates named above;
- event/audit semantic scope remains ANL-01;
- partitioning and exact library choices are implementation/performance decisions unless evidence later proves an architecture constraint.

## 13. Recommended effect if owner accepts

If the owner accepts section 12 and the owner-baseline lifecycle later closes:

```text
DUR-02 — Persistence v1
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
DDL/migrations       = NOT_AUTHORIZED by architecture acceptance alone
```

The existing Character persistence partial baseline becomes a binding sub-baseline consumed by the accepted whole gate; it is not superseded or weakened.

Acceptance would unblock **separately authorized Persistence-v1 implementation design and implementation packages** for common game persistence infrastructure and already accepted Character/FND-04 persistence semantics.

It would **not**:

- authorize immediate PostgreSQL schema/migration execution;
- authorize production DB credentials or deployment;
- accept GAME-ITEM-01 or DUR-03;
- permit durable item/currency/value mutation before DUR-03;
- choose market/guild/house/reward semantics;
- set production RPO/RTO/capacity/backup cadence;
- choose a Rust database/migration crate;
- select partitioning/sharding;
- authorize Platform database changes;
- fill unresolved Reference behavior;
- authorize production traffic.

A later implementation task must explicitly state which accepted DUR-02 sub-scope it implements and prove migration, concurrency, crash, restore, access-control and E2E behavior on exact revisions.

## 14. Does DUR-02 acceptance require GAME-ITEM-01 first?

**No, after this reconciliation.**

The historical backlog coupled inventory/ground transfer concerns into DUR-02, but later architecture now has an explicit `GAME-ITEM-01 -> DUR-03` ownership chain. Waiting for item semantics to accept common Persistence-v1 would recreate the dual-authority problem this reconciliation is intended to remove.

Therefore:

```text
whole DUR-02 accepted
-> common persistence + Character/FND-04 implementation may be separately authorized

GAME-ITEM-01 accepted
+ DUR-03 accepted
-> item/currency/value persistence may be implemented/proven
```

This separation permits server/persistence foundation work to begin earlier without pre-accepting item conservation.

## 15. Relationship to the start of real server construction

This reconciliation is intentionally designed to shorten the path to executable work.

FND-02, FND-03 and FND-04 architecture are already accepted. Once whole DUR-02 is owner-accepted, the core protocol/runtime/session/persistence architecture no longer needs another generic foundation design pass before a separately authorized implementation programme can start.

A complete gameplay vertical slice still requires its additional movement/combat/item/content prerequisites, but the server foundation does **not** need to wait for every Playable Alpha or expansion gate before code exists.

A future implementation authorization may therefore safely split construction into bounded real components such as:

```text
server bootstrap / GameNode shell
-> protocol-oteryn transport/runtime adapter
-> admission + GameSession + CharacterLease
-> PostgreSQL migration/persistence substrate
-> Character/FND-04 persistence
-> minimal ChannelRuntime
-> later movement/combat/item/content vertical-slice increments
```

That future transition is an explicit implementation-authority decision; this reconciliation itself remains paper-only.

## 16. Supersession / reopening

A later decision may reopen the accepted whole-DUR-02 core only with named evidence such as:

- a PostgreSQL concurrency anomaly not closed by the accepted transaction policy;
- implementation evidence showing the common journal/publication substrate cannot satisfy an accepted domain requirement;
- a migration tool/database limitation that changes correctness rather than convenience;
- measured performance/maintenance evidence requiring partitioning or a different persistence topology;
- security/privacy/legal requirements requiring stronger data separation or retention behavior;
- an accepted domain contract demonstrating a historical subject was assigned to the wrong owner;
- disaster-recovery evidence requiring a stronger non-rollback authority fence.

ORM defaults, library preference, OTS database schemas, convenience or desire to reduce review are not supersession evidence.

## 17. Explicitly not decided

- SQL DDL/table/index/constraint names;
- exact Rust PostgreSQL library, ORM/query builder or migration crate;
- database pool settings;
- numeric timeouts/retry/backoff;
- exact RPO/RTO/backup cadence/retention;
- exact backup vendor/topology;
- partition/shard keys;
- read replica/caching topology;
- item/currency/market/guild/house/reward schemas or semantics;
- profile-specific PvP Character extensions;
- production deployment;
- runtime implementation.

Until the owner accepts or modifies section 12, overall `DUR-02` remains **PROPOSED / PLANNED / NOT_STARTED** and this document is only the reconciliation/decision packet.
