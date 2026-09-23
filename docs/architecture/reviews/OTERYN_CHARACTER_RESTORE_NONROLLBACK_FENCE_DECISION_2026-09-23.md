# CHARACTER_RESTORE_NONROLLBACK_FENCE_V1

Status: architecture decision candidate for Issue #791. No implementation authority follows until this document is independently reviewed, protected-integrated and read back from `main`.

## Scope

This decision closes the Character Authority no-authority-resurrection gap between ordinary process restart and Character PostgreSQL restore/PITR. It does not select a backup vendor, physical storage product, deployment topology, RPO/RTO, backup cadence or automatic conflict-resolution policy.

Parent control plane: #162  
Source-readiness programme: #319  
Blocked material implementation: #790 / WP5 #414

## Decision

Game Operations owns Character recovery authorization. A Game-owned recovery coordinator maintains one authenticated monotonic recovery register outside the Character PostgreSQL restore unit.

The database stores the recovery generation under which its Character authority was admitted/reconciled, but that local row is only a comparison operand. It cannot certify itself after restore.

Neither Platform Account authority, #415 GameNode registration, S2 source high-water, Character rows, receipts, audit history, caches nor UUID ordering owns this recovery fence.

## External recovery register

The authenticated external record is logically equivalent to:

```text
CharacterRecoveryFenceV1 {
    authority_scope_id
    recovery_generation
    recovery_event_id
    predecessor_generation
    issued_at
    issuer_identity
}
```

Required semantics:

- `authority_scope_id` identifies the authoritative Character persistence scope.
- `recovery_generation` is positive, monotonically increasing when an actual restore/PITR recovery is authorized and is never reused.
- `recovery_event_id` is immutable identity for the exact recovery transition.
- `predecessor_generation` binds the expected prior admitted generation.
- `issuer_identity` is the authenticated configured Game recovery-control authority.
- external state and proof/credentials are retained outside anything restored by Character PostgreSQL PITR/snapshot recovery.

Advancing the register is an atomic compare-and-set over the expected predecessor generation and exact recovery event:

- exactly one strict successor may win;
- lower/reused generation rejects;
- equal generation with changed proof/event is contradiction;
- deletion/reset-to-zero/last-arrival-wins is forbidden;
- ambiguous CAS response is reconciled by rereading the exact predecessor/event identity before retry.

The physical storage mechanism behind the external register remains an Operations implementation choice as long as it meets this non-rollback contract.

## Startup and readiness

Every Game process starts Character authoritative use closed until the recovery fence is reconciled.

Startup reads:

1. authenticated external current recovery fence;
2. database admitted recovery generation/evidence;
3. required schema/migration ledger and Character root/revision/receipt/audit/outbox state;
4. independent #415 process and S2 current-source prerequisites as applicable.

Classification:

| Observation | Classification | Required result |
| --- | --- | --- |
| DB generation equals authenticated external generation and evidence agrees | ordinary restart/failover candidate | reconcile normal invariants, then readiness may open |
| DB generation below external generation | stale/rolled-back DB or incomplete prior recovery | stay closed; authorized recovery/reconciliation required |
| DB generation missing while external history exists | ambiguous rollback | stay closed |
| DB generation above external generation | contradiction/external regression | fail closed |
| equal generation with different immutable recovery evidence | equivocation/corruption | fail closed |
| external proof unavailable/untrusted | unknown | fail closed |

A normal process or NodeId restart does not increment the Character recovery generation.

## Restore / PITR

An actual restore/PITR operation must advance the external recovery register to a strict successor before restored Character state may be reopened as authority.

Authorized recovery:

1. keeps Character admission/current-owner use and owner-sensitive mutation closed;
2. fences/drains old-generation writers;
3. atomically CAS-advances the external register from the expected predecessor to a strict successor generation with one immutable recovery event;
4. treats restored Character/session/lease/transport/receipt/audit rows as historical evidence until reconciled;
5. validates the restored Character owner/world/revision, operation receipts and audit/outbox consistency required by the recovery procedure;
6. terminally fences stale pre-restore GameSession/lease/transport/writer authority;
7. persists the new admitted recovery generation/evidence in Game storage;
8. opens Character readiness only after all required reconciliation succeeds.

Every authoritative Character transaction asserts the currently admitted recovery generation. A transaction that began under an older generation cannot commit after recovery advances.

If stale Character state cannot be safely reconciled, the result is unavailable/closed rather than automatic reconstruction or "latest local row wins."

## Relationship to #415

`NodeIncarnationProof` remains a separate prerequisite:

- it proves which current Game process is allowed to exercise a writer;
- it does not prove whether Character PostgreSQL rolled back;
- #415 registration state itself lives in Game PostgreSQL and can be restored with the database.

A current process may therefore still be rejected because the Character recovery fence is missing/stale. Recovery readiness should bind both current process incarnation and current Character recovery generation without treating either as the other.

## Relationship to S2

S2 retained high-water and current observations protect Platform account-security/signing-trust source ordering. Their useful mechanism pattern—authenticated source, monotonic revision, equality contradiction and fail-closed missing floor—may be reused conceptually.

Their authority namespace and values are not Character recovery authority. A current S2 allow cannot prove that Game-owned Character owner/world history survived restore.

The Platform S3-A witness likewise remains Platform source authority and is not reused for Character recovery.

## Fresh store

A new empty Character store is not authoritative merely because no rows exist.

First activation must bind the store to an authenticated external recovery authority scope/current generation under an explicitly authorized fresh-store procedure before Character bootstrap can commit. Absence of a local admitted generation when external history already exists is a rollback/ambiguity condition, not permission to initialize zero.

## Minimum implementation handoff

After protected integration, Work may amend SAME #790 with the smallest exact surfaces needed for:

- a sealed authenticated `CharacterRecoveryFenceV1` current-source/adapter boundary;
- database retention of admitted recovery scope/generation/event evidence;
- startup reconciliation/readiness gating;
- recovery-generation assertion inside Character bootstrap/current-authority transactions;
- explicit authorized restore reconciliation/CAS path;
- real PostgreSQL restore/restart/concurrency qualification.

Logical minimum may require:

```text
apps/game-server/src/durability/character_authority.rs
apps/game-server/src/durability/db.rs                 # only narrow external-fence adapter/readiness hook if proven necessary
apps/game-server/src/durability/schema.rs             # only proof codec/schema hook if proven necessary
apps/game-server/migrations/<CURRENT_CHARACTER_AUTHORITY_MIGRATION>
apps/game-server/tests/character_authority_postgres.rs
```

Work must grant any additional shared path explicitly after fresh overlap/API proof. No generic witness framework, broker, second Character store or generalized distributed lease service is authorized by this decision.

The external monotonic-register capability and an executable test route are prerequisites for claiming restore qualification. They may be delivered inside an amended #790 only if Work proves and grants the exact owner/path/execution surface; otherwise they land first as one narrow prerequisite.

## Qualification matrix

Real PostgreSQL 17.6 plus an independently controlled external recovery source must prove:

- ordinary restart: DB/external generation equal -> reconcile and reopen without generation increment;
- process replacement with new NodeId at same valid recovery generation -> current process reauthenticates, recovery generation unchanged;
- PITR to older DB generation -> Character admission/current-owner use/mutation remain closed;
- full restore retaining apparently valid Character/session rows -> rows remain non-authoritative;
- authorized restore -> external CAS strict successor, stale authority fenced, then readiness may reopen;
- external register unavailable/untrusted -> fail closed;
- external generation regressed or DB generation greater -> contradiction/fail closed;
- equal generation with different event/proof -> contradiction/fail closed;
- ambiguous recovery CAS -> exact predecessor/event readback, no blind retry;
- restart during incomplete recovery -> remain closed and reconcile same recovery event;
- stale pre-restore Character row -> cannot authorize;
- stale GameSession/lease/transport/writer -> cannot revive;
- old-generation transaction racing recovery -> cannot commit after transition;
- ordinary-restart operation receipt -> exact result reconciliation;
- receipt/audit row present only in rolled-back snapshot -> cannot establish current authority;
- pending committed outbox on ordinary restart -> resumes publication without domain replay;
- rolled-back/inconsistent receipt/audit/outbox -> readiness closed;
- valid bootstrap intent with invalid recovery fence -> zero Character authoritative writes;
- valid S2 allow with invalid Character fence -> still closed;
- valid NodeIncarnationProof with invalid Character fence -> still closed;
- generation arithmetic overflow or malformed proof -> fail closed.

The restore qualification must control PostgreSQL snapshot and external recovery authority independently. Restoring both together does not prove the non-rollback property.

## Explicitly deferred

This decision does not select backup technology, witness storage medium, deployment service, regional topology, RPO/RTO, backup cadence or automated post-disaster Character conflict policy. Those choices may change without weakening the core invariant: restored Character PostgreSQL state never becomes current authority until the distinct external Game recovery fence is proven and required reconciliation completes.
