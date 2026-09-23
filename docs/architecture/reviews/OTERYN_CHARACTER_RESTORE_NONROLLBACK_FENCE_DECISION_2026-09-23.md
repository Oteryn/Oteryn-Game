# CHARACTER_RESTORE_NONROLLBACK_FENCE_V1

Status: architecture decision candidate for Issue #791. No implementation authority follows until this document is reviewed, protected-integrated and read back from `main`.

## Scope

This decision closes the Character Authority no-authority-resurrection gap for process restart versus PostgreSQL restore/PITR. It selects an authority contract, not a backup vendor, production topology, RPO/RTO target or deployment product.

Parent control plane: #162  
Source-readiness programme: #319  
Blocked implementation: #790 / WP5 material #414  
Related accepted authority: DUR-02 restore safety, Character Authority #414, protected #415 and S2.

## Decision

Game Operations / the Game control plane owns a distinct **Character recovery-fence authority outside the Character PostgreSQL restore unit**.

The runtime consumes a sealed `CharacterRecoveryFenceObservationV1` from an owning adapter that authenticates that external authority. Character PostgreSQL retains only the last accepted binding needed to compare current proof with the database state. A database snapshot, Character row, receipt, audit row, UUID ordering, #415 registration row or S2 source row cannot construct the sealed observation.

The physical durable medium used by the external authority is deliberately not selected here. Its acceptance requirement is semantic: the authority state and credential/proof needed to establish the current recovery decision must not roll back as part of restoring the Character PostgreSQL snapshot.

## Recovery-fence model

The external authority retains one immutable Game Character-store identity and a monotonically nondecreasing positive recovery generation. Each authenticated observation binds:

- expected Game recovery authority identity;
- Character-store identity;
- recovery generation;
- decision identity for the exact generation/state;
- one state:
  - `FRESH_STORE_AUTHORIZED`;
  - `CONTINUITY_CONFIRMED`;
  - `RESTORE_RECONCILIATION_REQUIRED`;
  - `RESTORE_RECONCILED`;
  - `REVOKED_OR_UNAVAILABLE`.

Generation arithmetic is checked and never wraps. Equal generation with a different authenticated decision/state is a conflict. Lower generation is stale and cannot authorize.

The database retains the accepted Character-store identity, accepted recovery generation and decision identity/state needed to detect mismatch. Those retained values are historical comparison state, not the external authority itself.

## Fresh store

An empty/new Game Character store is not self-authorizing merely because Character tables are empty.

First activation requires an authenticated external `FRESH_STORE_AUTHORIZED` observation for a new store identity/generation. The Character bootstrap authorization may be consumed only after this proof is accepted.

This is separate from S2 fresh-store provenance and #415 node registration. Those owners preserve their own namespaces and authority meanings.

## Ordinary restart

An ordinary process/node restart does not automatically imply a database restore.

Authoritative Character reads/mutations may resume only when the owning recovery adapter authenticates an external observation for the same Character-store identity that is compatible with the database's last accepted recovery binding and explicitly reports continuity as current.

A new NodeId, successful #415 registration or process restart alone is never continuity proof.

The external authority may advance the recovery generation across operational restarts. If it does, Game accepts the successor only through the same authenticated compare-and-record path; lower/equal-conflicting values fail closed.

## Restore / PITR

A restore/PITR procedure must make the external authority distinguish the restored database from ordinary uninterrupted continuity before authoritative Character mutation can resume.

When the external observation is ahead of, incompatible with, or explicitly marks restore against the database's retained binding:

1. Character mutation and any admission/current-owner use that depends on Character Authority stay closed;
2. restored Character rows, receipts and audit/outbox history remain historical evidence only;
3. #415 current process proof and S2 current Platform security/trust may be re-established, but neither makes restored Character owner/world state current;
4. an explicit Game recovery/reconciliation procedure validates the restored Character owner/world/revision/receipt/audit state required by the accepted recovery plan;
5. only the owning external authority can issue a `RESTORE_RECONCILED` successor decision for the exact store identity/generation after that procedure;
6. Game atomically records the reconciled recovery binding before reopening Character mutation/current-authority consumption.

If the external authority cannot prove its own retained high-water, the store identity is ambiguous, generation regresses, or reconciliation result is unknown, authoritative Character use remains unavailable. There is no reset-to-zero or "latest local row wins" fallback.

This decision deliberately permits fail-closed manual/operational recovery instead of inventing an automatic reconstruction authority.

## Relationship to #415 and S2

Protected #415 is required to prove the current Game process/incarnation for writers. Its registration tables are in Game PostgreSQL and may roll back with that database; they are not the Character recovery fence.

Protected S2 preserves Platform native security/trust source ordering and current observations. It does not own Game Character owner/world history and is not the Character recovery fence.

Both can be mandatory independent prerequisites after recovery. Neither may be reinterpreted as the external Character recovery authority.

The S3-A Platform witness demonstrates a useful non-rollback mechanism pattern but belongs to the Platform native-evidence producer. Its data/authority namespace is not reused for Character recovery.

## Minimum Game implementation handoff

After protected integration, Work may allocate the smallest consumer implementation needed to:

- define the sealed Character recovery-fence observation/current-source capability;
- retain the accepted store identity/generation/decision binding in the existing Character authority persistence;
- require a current compatible recovery observation before `bootstrap_character`, current Character authority reads used for admission, and future owner-sensitive Character mutations;
- expose an explicit recovery/reconciliation acceptance operation that cannot be called using restored local rows alone;
- keep ordinary API consumers fail-closed while recovery status is unresolved.

The owning adapter must authenticate a source outside the PostgreSQL restore unit. This architecture does not require a generic witness framework and does not grant authority to reuse Platform witness bytes or #415/S2 state.

A new small Game-owned recovery-source module/path is acceptable only if fresh compiler/API evidence proves no existing owner seam can carry the sealed observation without changing another authority's meaning. Work must allocate any such path explicitly.

## Qualification matrix

At minimum, real PostgreSQL 17.6 plus an independently controlled recovery-source fixture must prove:

- fresh store cannot bootstrap without authenticated fresh-store recovery proof;
- ordinary restart with matching current external continuity proof preserves exact Character state/receipts/outbox;
- missing external source fails closed;
- wrong authority/store identity fails closed;
- lower generation fails closed;
- equal generation with changed decision/state fails closed;
- external generation/restore decision ahead of restored DB closes Character mutation and admission-owner use;
- restoring an older database snapshot while keeping the external recovery source current does not resurrect a rolled-back Character owner/world/revision;
- #415 re-registration after restore does not reopen Character authority by itself;
- current S2 account-security/trust after restore does not reopen Character authority by itself;
- restored receipts/audit rows cannot self-certify currentness;
- explicit recovery reconciliation reopens only the exact accepted store/generation after the required state validation;
- ambiguous/lost reconciliation response is retried/reconciled by the same decision identity and cannot produce two recovery generations;
- concurrent Character mutation versus recovery-state transition serializes so no mutation commits under a superseded recovery fence;
- recovery-generation overflow or malformed proof fails closed;
- the external proof/credential is not logged or exposed through public projections.

The restore test must control the PostgreSQL snapshot and external recovery source independently. A test that restores both together cannot prove the non-rollback property.

## Explicitly deferred

This decision does not select:
- backup/PITR vendor or storage medium;
- recovery-service deployment topology;
- RPO, RTO or backup cadence;
- automatic Character conflict resolution after a disaster;
- cross-region replication design.

Those can change without altering the core rule: restored Character PostgreSQL state never becomes current authority until a distinct external Game recovery authority proves continuity or explicitly completes reconciliation.
