# DUR-02 — Profile-Neutral Character Persistence Decision Packet

- Status: **PRE-DECISION ARCHITECTURE / NOT ACCEPTED**
- Date: 2026-08-12
- Stable gate: `DUR-02 — Persistence v1`
- Packet scope: **profile-neutral core Character persistence architecture only**
- Trusted repository base: `blakinio/Oteryn-v2@2913201186d0e38cfc0bf0c9e2c5b83f981a61c6`
- Decision owner: product/architecture owner
- Consumes: ADR-0004, DUR-01, ANL-01, accepted GAME-CHAR Stage A/B, FND-04A/B/C, ADR-0012 and `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`
- Runtime authority: **NONE**
- Does not authorize: PostgreSQL DDL/migrations, database provisioning, runtime persistence code, item/currency persistence, production backup configuration, Platform writes, profile-specific PvP Character state or unresolved Reference values/formulas

## 1. Decision boundary

Accepted GAME-CHAR answers **what Character semantics must be durable**. This packet recommends **how the profile-neutral Character persistence core is physically structured and transactionally protected** without allowing persistence convenience to become gameplay policy.

```text
accepted Character semantics
-> choose relation / transaction / fencing architecture
-> preserve typed profile-extension boundaries
-> keep unresolved profile/ruleset behavior outside schema invariants
-> later implementation proves SQL / migration / runtime behavior
```

### Must this Character persistence architecture be decided now?

**YES.** It blocks safe paper-only design of native Character persistence, FND-04 authority persistence, idempotency, mandatory audit/outbox atomicity, migration/recovery and later VSL-PERSISTENCE-01 implementation planning.

### Does this packet close the whole stable `DUR-02 — Persistence v1` gate?

**NO.**

`FOUNDATION_DECISION_BACKLOG.md` historically defines `DUR-02` more broadly than this Character packet, including Persistence-v1 subjects such as migration mechanism/schema ownership and wider cross-domain consistency boundaries. Later architecture has moved item-conservation semantics toward `GAME-ITEM-01`/`DUR-03`, but no explicit accepted decision has yet declared the entire remaining historical `DUR-02` scope closed by this Character packet.

Therefore a future owner acceptance of this packet must create a **binding partial owner baseline** for the profile-neutral Character persistence sub-scope while the overall stable gate remains:

```text
DUR-02 overall
DecisionStatus       = PROPOSED
DeliveryStatus       = PLANNED after partial-baseline closeout
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
```

A later reconciliation must either close the remaining Persistence-v1 scope or explicitly supersede/narrow it before overall `DUR-02` may become `ACCEPTED`.

### Must SQL DDL or every profile-specific child table be frozen now?

**NO.** This packet freezes ownership, relation families, authority transitions, fencing, transaction boundaries, lock/isolation rules, migration discipline and typed extension requirements. Exact SQL names/syntax, migration library, connection pool, storage parameters and profile-specific child schemas remain implementation or owning-profile work.

## 2. Binding inputs

### 2.1 PostgreSQL and ownership

- PostgreSQL is the authoritative native game relational target.
- Platform and game use separate logical databases, owners, credentials and migration histories.
- There are no cross-database foreign keys.
- Platform may own account/commercial workflow state but may not directly mutate native Character persistence.
- Character Authority is the semantic writer for Character identity/lifecycle/ownership/name/progression/build state.
- Redis/in-memory caches are never authority recovery sources.

### 2.2 Durable identifiers

- native UUIDv7 identities persist as PostgreSQL `uuid`, full 128 bits, nil invalid and non-reused where their semantic contract requires non-reuse;
- persisted `CommandId` preserves the FND-02 nonzero uint64 range as `numeric(20,0)` and is scoped by `GameSessionId`;
- UUID ordering never defines chronology, revision or authority.

### 2.3 Independent fences

```text
CharacterRevision
!= CharacterLease generation
!= GameSessionId
!= connection_generation
!= RuntimeScopeAuthority ownership generation
```

Character semantic mutation validates `CharacterRevision`. FND-04 separately validates who may control/write through current presence/lease/session/transport/runtime authority.

### 2.4 Durable audit

For any mutation whose owning contract requires durable audit:

```text
authoritative mutation + mandatory durable audit evidence
commit together
OR
neither becomes authoritative
```

Publication is at-least-once. Replay is evidence/projection replay and never replays gameplay mutation.

### 2.5 Profile-neutral boundary

Accepted GAME-CHAR forbids:

- treating current Global/Canary/crystalserver/another OTS as unresolved July-28 truth;
- one universal PvP/death persistence profile;
- an untyped JSON/KV/EAV miscellaneous-state escape hatch;
- claiming one Character core schema complete for every future world/profile.

## 3. Persistence shapes considered

### Option A — one wide Character row

**Rejected.** It couples unrelated state/lifecycles, creates nullable migration churn, mixes Character and FND-04 authority, increases root-row coupling and undermines typed child aggregates.

### Option B — generic EAV/JSON Character state

**Rejected.** It weakens constraints, ownership and migration review and becomes a bypass around future semantic gates.

### Option C — full Character event sourcing

**Rejected for Persistence v1.** ADR-0004 chooses current-state tables + revisions + idempotent operations + transactional outbox + bounded critical audit. ANL-01 events remain evidence, not gameplay authority.

### Option D — normalized current-state core + typed child relations/extensions

**RECOMMENDED.** Use one Character root/revision anchor, separate name and FND-04 authority relation families, typed children, durable receipts, retained immutable event evidence plus separate mutable publication state, and explicit typed profile extensions.

## 4. Profile-neutral Character relation families

Logical names below describe ownership; they are not final SQL identifiers.

### 4.1 `character_root`

One row per CharacterId containing only profile-neutral root facts:

- CharacterId;
- current owner AccountId — external Platform-issued reference, no cross-DB FK;
- current WorldId;
- lifecycle `ACTIVE | DELETION_SCHEDULED | RETIRED`;
- monotonic `CharacterRevision`;
- active profile/ruleset interpretation revision references needed to interpret Character state;
- creation/starter context revision references needed for deterministic interpretation/migration;
- accepted lifecycle timestamps/evidence where policy requires them.

It does **not** own items/currency, quest aggregates, Platform entitlements, lease/session generations, generic extension JSON or unresolved Reference arithmetic.

#### Global `CharacterRevision`

Every committed Character-owned **semantic** mutation:

1. locks/revalidates `character_root`;
2. validates expected CharacterRevision when the operation contract supplies one;
3. rejects/reconciles stale state;
4. advances CharacterRevision exactly once for the committed Character semantic transaction;
5. records the resulting revision in durable receipt/audit evidence where applicable.

Several typed Character child rows may change in that transaction; the global CharacterRevision advances once. Pure FND-04 control/session transitions do not advance CharacterRevision unless they also mutate Character semantic state.

CharacterRevision is never derived from UUID order, database transaction ID, wall clock, EventId or lease/session generation.

### 4.2 `account_character_guard`

A game-owned serialization anchor keyed by AccountId.

It is acquired for **every** account-scoped Character operation whose correctness depends on portfolio/quota state, including whenever active policy can change quota eligibility:

- create;
- schedule deletion;
- restore/cancel deletion;
- terminal retirement/finalization;
- account ownership transfer;
- any later quota-affecting lifecycle transition.

It is **not** Account authority, Platform security/entitlement state or an independently authoritative active-count cache.

Portfolio eligibility/count is derived from current Character Authority state under the guard and active policy revision. A cached counter cannot become authority without a later evidence-backed superseding design with transactional and reconciliation invariants.

Multi-account operations lock account guards in canonical full-AccountId byte order.

### 4.3 `character_name_registry`

Separate relation family for:

- display name;
- complete lossless canonical comparison key;
- naming-policy revision;
- semantic claim class such as `CURRENT | FORMER_ALIAS | RESERVED`;
- CharacterId relation;
- policy-owned effective/release evidence.

The game domain computes the canonical key under the accepted naming policy. PostgreSQL enforces equality/uniqueness over the complete result; database collation never invents semantic normalization. Hashes may accelerate lookup but never replace complete-key verification.

#### Naming-policy cutover

One logical global namespace cannot safely run two simultaneously authoritative canonicalization universes that disagree on equality.

Before destination-policy cutover:

1. compute destination canonical keys for every conflict-participating live/reserved claim;
2. detect and resolve/abort all new collisions;
3. validate the destination authoritative uniqueness constraint/index;
4. compatibly cut readers/writers to the destination policy;
5. retain old-policy values only as non-authoritative history/migration evidence after cutover.

The authoritative namespace may not be partitioned by naming-policy revision merely to hide cross-revision conflicts unless an explicit later owner decision intentionally creates separate namespaces.

Application-only `check availability -> later insert` is not correctness. Database uniqueness decides simultaneous name races.

### 4.4 `character_build_core`

Typed Character child relation for profile-neutral build/profession linkage:

- CharacterId;
- stable ruleset-owned vocation/build definition reference where present;
- explicit pre-vocation/unselected state;
- promotion-achievement state/reference;
- build/ruleset definition revision.

CharacterRevision remains the global stale-state fence. Vocation titles/ordinals are ruleset data, not universal engine-schema enums. Stable definition-key representation is consumed from the owning ruleset/content contract.

### 4.5 progression and skill relations

Use typed relations only after the owning ruleset determines which values are authoritative persisted facts and which are deterministic projections.

```text
persisted authoritative fact
!= deterministic derived projection
```

Recommended architecture:

- typed scalar progression relation for universally required persisted Character facts after their ownership/status is accepted;
- typed skill-state relation keyed by the stable ruleset skill-definition key;
- dedicated typed children for later accepted progression systems.

Do not duplicate persisted representations merely because a formula is unknown unless evidence establishes both are independently authoritative.

Migration-sensitive values that cannot safely be recomputed — capacity is the accepted example — preserve the authoritative value plus its interpretation revision.

The accepted Reference eight-skill catalogue does **not** imply either a fixed eight-column table or an opaque skill blob. Physical skill state remains typed, definition-keyed, revisioned and migration-safe.

### 4.6 typed Character-owned profile extensions

Weapon Proficiency, charms, Hunting Tasks, permanent Prey/Hunting slots, Wheel/Promotion Points, Animus and future profile-specific Character facts use dedicated typed relation families/child aggregates after their physical contract is accepted.

Every extension declares:

- semantic owner;
- CharacterId relation;
- stable definition identity/key and its owner;
- schema/definition compatibility revision;
- CharacterRevision interaction;
- transaction/lifecycle boundary;
- migration/rollback;
- privacy/retention where applicable.

No generic `type + JSON/blob` persistence path is allowed as a temporary ownership escape hatch.

## 5. FND-04 authority persistence

FND-04 authority state is physically distinct from Character semantic state.

### 5.1 `account_presence_claim`

Authoritatively keyed by AccountId and enforcing game-domain mandatory-presence exclusion.

Requirements:

- one AccountId has at most one mandatory-presence Character;
- one CharacterId cannot simultaneously be claimed by several accounts;
- current AccountId->CharacterId ownership is still revalidated against Character Authority;
- presence never substitutes for `character_root.account_id`;
- Character ownership never substitutes for presence/control authority.

### 5.2 `character_lease`

Keyed by CharacterId and storing restart-reconstructible lease fencing:

- nonzero monotonic non-reused CharacterLease generation or accepted equivalent;
- current lease/control state;
- holder/session/runtime-scope references required by FND-04;
- restart-reconstructible freshness/expiry evidence once numeric policy is accepted.

Lease generation is never CharacterRevision. Stale generation cannot renew, commit controlled mutation or regain control. Exact TTL/renew/safety values and their final scalar representation remain deferred.

### 5.3 `game_session`

Keyed by GameSessionId when durable continuity requires it.

Stores session-scoped truth only:

- AccountId + CharacterId binding;
- lifecycle/terminality;
- current `connection_generation`;
- current CharacterLease generation fence/reference;
- FND-04 recovery binding revision set as applicable:
  - `protocol_major`;
  - `transport_profile`;
  - `ruleset_revision`;
  - `content_revision`;
  - `map_revision`;
  - `world_policy_revision`;
  - current RuntimeScopeAuthority / runtime-owner generation evidence;
- reconnect-proof/candidate metadata references where the later accepted security implementation requires them.

Actor-wide ControlLoss state is **not duplicated here**. A terminal GameSessionId never revives. GameSessionId is identity, never bearer proof.

### 5.4 `control_loss_continuity`

Single actor-wide owner for restart-safe ControlLoss state:

- non-reused ControlLossEpoch discriminator/ordinal or equivalent;
- authoritative epoch origin;
- original same-session grace deadline where applicable;
- protection eligibility/consumption;
- accepted four-second protection activation/expiry;
- protection re-arm state/deadline where applicable;
- current/terminal GameSession references only as context.

Deadlines survive restart/failover without restart/extension. Process-local monotonic timer values cannot be serialized as restart-stable time; later implementation defines the trusted restart-stable deadline representation and clock-uncertainty/fail-closed behavior.

### 5.5 reconnect proof material

Exact hash/encryption/KMS representation remains deferred by FND-04B. The persistence boundary requires secret/verifier material to remain separate from ordinary Character state/analytics, absent from plaintext logs/audit, predecessor-fenced across generations and frozen by a later security implementation contract before runtime enablement.

### 5.6 atomic fresh-admission authority commit

Fresh admission is one authority linearization boundary, not a series of independently authoritative row writes.

Prechecks may fail fast, but immediately before/atomically with final authority creation the transaction:

1. acquires/validates the current Character root as needed to prevent concurrent ownership/world/lifecycle transition during final admission revalidation, **without advancing CharacterRevision unless semantic Character state changes**;
2. revalidates current AccountId->CharacterId ownership/lifecycle;
3. revalidates current CharacterId->WorldId eligibility;
4. locks/revalidates AccountPresence incumbent state;
5. locks/revalidates CharacterLease/current generation state;
6. revalidates current runtime owner/RuntimeScopeAuthority generation/readiness;
7. revalidates protocol/transport/ruleset/content/map/world-policy/offer dimensions required by FND-04A;
8. revalidates GrantNonce eligibility and mutable authenticated security/trust evidence whose accepted contract requires final revalidation;
9. includes any mandatory durable authority/audit evidence registered for the transition.

Only successful commit atomically:

```text
consumes GrantNonce
+ establishes/advances AccountPresenceClaim
+ establishes/acquires CharacterLease generation
+ creates canonical GameSessionId
+ marks GameSession ACTIVE
+ sets connection_generation = 1
+ establishes initial authoritative session/reconciliation boundary
```

Failure leaves **none** of those candidate authority effects. Precommit/candidate GameSessionId is never reused after failure. Success becomes externally visible only after commit.

The exact GrantNonce/security-evidence relation layout remains a later FND/security implementation choice; the required same-authority transaction semantics are fixed here.

### 5.7 reconnect/recovery PREPARE persistence

`ReconnectAttemptRef` is idempotency/correlation identity, not authority.

PREPARE uses bounded typed candidate/disposition state binding at least:

```text
GameSessionId
ReconnectAttemptRef
predecessor connection_generation
strict-successor candidate connection_generation
exact authenticated candidate TransportBinding identity/reference
proof class
finite prepared deadline
candidate reconnect-proof metadata/reference where used
PREPARED disposition/reconciliation evidence
```

A raw process-local socket pointer/handle is never persisted as restart-stable authority. If the exact prepared physical transport cannot still be proven current after process loss, that candidate cannot COMMIT and must abort/fail closed into the accepted recovery path.

PREPARE grants **zero** gameplay/liveness/fencing authority and does not advance the current connection generation. Same eligible PREPARE retry reconciles the same logical candidate rather than minting independent successor authority. Abort/expiry/supersession permanently invalidates candidate proof/state.

Prepared resources are bounded; exact limits remain registry/implementation values.

### 5.8 reconnect/recovery atomic COMMIT

Reconnect/recovery authority changes only at COMMIT.

The COMMIT transaction locks/revalidates current GameSession, presence, lease, prepared candidate and **ControlLoss continuity row whenever re-entry/protection entitlement may change**, plus the accepted mutable security/revision facts, including:

- candidate exists, unexpired and bound to exact session/attempt/candidate transport;
- predecessor connection generation still matches PREPARE source;
- session remains reconnect-eligible and original grace is valid for same-session path;
- no healthy current controller regained authority;
- AccountPresenceClaim still denotes the same CharacterId;
- CharacterLease remains current/compatible;
- RuntimeScopeAuthority owner/generation/placement remains current;
- FND-02 reconciliation boundary is safe;
- no newer handoff/takeover/fence/terminal transition supersedes candidate;
- proof-specific mutable security/trust/nonce/revision facts remain current.

Only successful COMMIT atomically:

```text
fences predecessor TransportBinding authority
+ advances/makes candidate connection_generation current
+ binds command/liveness/reconciliation authority to candidate transport
+ activates candidate successor reconnect proof and invalidates predecessor proof where applicable
+ preserves GameSessionId for same-session reconnect
+ preserves actor/gameplay state and domain revisions
+ records re-entry for the current ControlLossEpoch
+ consumes/activates the one eligible 4s protection entitlement at most once
+ consumes RecoveryGrantNonce only on the successful path that requires it
+ records stable ReconnectAttempt disposition for lost-response reconciliation
+ records any mandatory durable authority/audit evidence
```

Failed/stale COMMIT does **not** advance generation, revive predecessor, consume nonce as success, activate proof/protection or roll authority back to PREPARE-time observations.

Lost COMMIT response reconciles from persisted attempt/current-generation authority and never performs a second switch.

### 5.9 post-grace recovery with new GameSession

If the old GameSession is terminal but the same actor remains `PRESENT_UNCONTROLLED`, accepted post-grace recovery creates a **new** GameSessionId at one atomic boundary while preserving the actor.

The transaction locks/revalidates Character ownership/world eligibility where required, presence, lease, runtime authority, recovery evidence and ControlLoss continuity.

Only success:

```text
creates new GameSessionId
+ connection_generation = 1
+ establishes new current reconnect proof/transport authority
+ consumes required RecoveryGrantNonce
+ restores playable control to the same actor
+ preserves any eligible existing ControlLossEpoch/protection entitlement without inventing a new epoch
```

It does not respawn/heal/refill/teleport/reset conditions/cooldowns/combat/threat/aggro or reuse the terminal GameSessionId.

## 6. Idempotency and receipts

### 6.1 Character Authority operation receipts

Retryable cross-system Character mutations use a durable OperationId receipt when the operation requires independent retry identity.

A bounded typed `character_operation_receipt` records:

- OperationId;
- operation kind;
- authenticated caller/semantic request fingerprint;
- CharacterId where known;
- terminal/nonterminal state;
- bounded stable result category;
- resulting CharacterRevision where applicable;
- TransactionId for committed atomic mutation where applicable;
- evidence timestamps.

No arbitrary result JSON is canonical.

Rules:

- same OperationId + same semantic request -> same logical operation/result reconciliation;
- same OperationId + conflicting request -> conflict;
- timeout is not success/failure proof;
- create retry before CharacterId is known resolves to one CharacterId or stable terminal result;
- world/account transfer retry cannot apply twice.

### 6.2 persisted gameplay command dedup

Do not persist every CommandId universally.

Where FND-02/DUR requires durable dedup across an ambiguous commit boundary, identity is exactly:

```text
(GameSessionId, CommandId)
```

Equal numeric CommandId in another GameSession is distinct. Receipt fields and retention remain bounded/operation-owned.

### 6.3 TransactionId

Each logical atomic durable mutation requiring ANL transaction evidence uses one stable TransactionId across ambiguous database-attempt retry/reconciliation. DB attempt IDs/WAL positions/local surrogates never replace it.

## 7. Character-domain transaction architecture

### 7.1 one Character mutation anchor

Every Character-owned semantic durable mutation locks/revalidates `character_root` first. This serializes semantic persistence per Character under one CharacterRevision while unrelated characters remain independent.

FND-04-only control/session transitions do not increment CharacterRevision unless they also mutate Character semantic state.

### 7.2 account-scoped operations

Every lifecycle/ownership operation whose policy may alter portfolio eligibility acquires `account_character_guard` before portfolio evaluation.

Account transfer:

1. lock both account guards in canonical order;
2. revalidate current owner;
3. lock/revalidate Character root;
4. evaluate source/destination policy;
5. revalidate required `ABSENT` / no-playable-lease state;
6. commit AccountId rebinding + CharacterRevision + receipt + mandatory audit atomically.

### 7.3 name operations

Create/rename compute canonical key under active naming policy outside DB collation, rely on complete-key DB uniqueness and atomically bind claim/history with Character mutation/receipt/audit. Simultaneous name races have one DB-authoritative winner.

### 7.4 quiescent high-impact operations

Terminal retirement, world transfer and account ownership transfer retain the Stage-A requirement: actor `ABSENT` and no current playable CharacterLease. Final transaction locks/revalidates presence/lease instead of trusting an earlier precheck.

### 7.5 death/item boundary

DUR-02 may define Character progression consequence persistence but does not decide item/corpse/value conservation. `GAME-ITEM-01`/`DUR-03` must later prove the cross-domain atomic/reconciliation boundary before a path changing both Character and items is implemented. No silent partial success is accepted.

## 8. Isolation, locks and retries

### Recommended isolation rule

PostgreSQL `READ COMMITTED` is acceptable only with **explicit authority anchors + DB constraints** that close every correctness-sensitive anomaly for that operation.

Examples:

- Character root lock -> Character revision serialization;
- account guard -> quota/portfolio serialization;
- unique canonical key -> name conflict;
- presence/lease/session/candidate/ControlLoss locks + generation compare -> FND-04 authority safety;
- unique OperationId / durable command identity -> idempotency.

If an invariant cannot be proven under that model, introduce an accepted lock/constraint anchor or use bounded PostgreSQL `SERIALIZABLE` with same-operation retry/reconciliation.

PostgreSQL default isolation is not itself a correctness proof. Advisory locks may supplement coordination but are never sole authority.

### Lock order

Same-class multi-entity locks use canonical full-identity byte order.

Character-domain cross-class order:

```text
account portfolio guard(s)
-> Character root(s)
-> FND-04 authority rows required for final revalidation
-> existing name rows where row locks are needed
-> typed Character child rows in stable definition-key order
-> receipts / audit / publication inserts
```

Any FND-04 flow that also requires Character root revalidation takes the Character root **before** FND-04 authority rows so it cannot reverse the Character-domain order.

Within FND-04 authority rows, implementation must publish one executable lock-order matrix before SQL code is accepted; it must be consistent across fresh admission, PREPARE/COMMIT, post-grace recovery and Character-domain quiescence checks.

New unique name claims may race on the unique constraint rather than locking a nonexistent row.

Retries after serialization/deadlock/lost response retain the same semantic OperationId/TransactionId. Retry count/backoff remains downstream.

## 9. Durable audit journal and publication state

A mutable outbox row cannot simultaneously be the retained canonical audit record.

### `durable_event_journal`

Stores retained ANL semantic evidence including EventId, event/schema/privacy/retention profile refs, trusted timestamp/context, correlation/causation/operation/transaction links, applicable domain revisions, **exact registered payload bytes**, payload SHA-256 and required CharacterRevision linkage.

While retained, one EventId cannot be rewritten to different semantic content/payload bytes.

An accepted privacy/retention/legal lifecycle may remove/redact data only through its separately governed and audited process; it may not silently rewrite a retained EventId into different semantics.

### `event_publication_state`

Separate mutable state keyed by EventId for pending/published/retry/quarantine status, attempt metadata, retry timing and delivery error/checkpoint state. It is not gameplay/audit truth.

### Atomicity

For every mutation requiring durable audit:

```text
current-state mutation
+ CharacterRevision if Character semantics change
+ receipt where required
+ mandatory durable_event_journal rows
+ publication enqueue state
commit in one PostgreSQL transaction
```

If mandatory journal/enqueue cannot commit, the audited mutation does not become authoritative. Best-effort telemetry is asynchronous/non-blocking.

Publisher uses committed rows only, delivery is at-least-once, EventId deduplicates effects, retries reuse retained exact event bytes and replay cannot mutate gameplay.

## 10. Current state and checkpoints

Normalized root + typed current-state relations at CharacterRevision are canonical Character durability. Do not create a second generic serialized Character snapshot.

If later runtime/profile systems need a broader consistent checkpoint, use a manifest containing CharacterId, CharacterRevision boundary, relevant ruleset/content/map/world-policy/runtime-owner revisions and references to **typed owner-specific checkpoint components**. No arbitrary payload blob.

A Character success response is never authoritative before PostgreSQL commit. Lost success response is reconciled from durable receipt/current state rather than blindly repeated. This is a logical commit/ack invariant, not a numeric DR RPO promise.

## 11. Restart, crash and disaster restore

### Ordinary restart

Committed Character state/revision, required FND-04 authority continuity, receipts and mandatory audit/publication evidence are reconstructed from PostgreSQL before authority resumes. Redis/in-memory state cannot restore authority by itself.

Restart/failover never resets/reuses CharacterRevision, lease generation, connection generation, ControlLossEpoch, grace/protection deadlines or terminal GameSession and never replays a committed semantic operation.

If authority cannot be proven, fail closed or use the accepted fresh/recovery path.

### PITR/disaster restore

An older restored snapshot cannot assume restored sessions/leases remain current.

Before gameplay authority resumes:

1. keep admission/mutation closed;
2. establish a strictly newer non-rollback recovery/authority fence outside values that could have rolled back with the snapshot, or prove an equivalent mechanism;
3. fence pre-restore TransportBinding/session/lease authority that cannot be proven current;
4. reconcile Platform security/account state and current routing/world revisions;
5. verify Character/name/FND-04/receipt/audit/publication integrity;
6. publish/replay committed evidence only, never gameplay commands;
7. reopen traffic only after restore validation passes.

Exact recovery-fence issuer/storage is deferred to DUR/OPS/security. The architecture requirement is **no authority resurrection**.

### Backup/RPO/RTO

Require PostgreSQL backup + WAL/PITR capability, automated restore verification, integrity checks across Character/name/FND-04/receipt/audit/publication state and exact artifact/revision evidence. Numeric production RPO/RTO remains OPS/PERF/milestone-owned.

## 12. Migration and compatibility

Game DB has one game-owned migration history; Platform migrations never apply it.

Incompatible change follows:

```text
EXPAND
-> resumable/idempotent BACKFILL or TRANSFORM
-> VALIDATE
-> CUT OVER compatible readers/writers
-> CONTRACT only after rollback window closes
```

Ruleset/profile/content changes that alter persisted interpretation require explicit source/destination revisions and deterministic migration/compatibility evidence. A revision change never silently reinterprets state.

Migration IDs are never reused; backfills are resumable/bounded; constraints become authoritative only after validation; incompatible writers are fenced before destructive contract; rollback order is known before cutover; domain identities do not change due to storage migration.

Definition references keep stable identity or explicit deterministic mapping. Database row positions/enum ordinals are never durable definition identity.

## 13. Referential integrity, retirement and privacy

Game-owned typed child relations use CharacterId referential integrity where both sides share game migration authority. Exact FK deferrability/cascade syntax is implementation-owned.

Ordinary Character retirement is a semantic state transition, not `DELETE CASCADE`. Child state does not disappear solely because lifecycle becomes RETIRED unless its semantic/retention contract requires removal.

Physical cleanup never makes CharacterId reusable; sufficient tombstone/provenance evidence remains.

Privacy deletion/anonymization is separately owned by DATA-PRIVACY-01 and cannot be modelled as identity reuse or implicit removal of mandatory security/audit evidence. Name-history retention/public exposure needs explicit policy before production.

## 14. Query/index intent

Implementation must provide authoritative access paths for:

- CharacterId root;
- Characters by AccountId + lifecycle/policy eligibility;
- Characters by WorldId where required;
- complete canonical name-key conflict lookup;
- CharacterLease by CharacterId;
- AccountPresenceClaim by AccountId plus unique claimed CharacterId;
- GameSession by GameSessionId/current Character where recovery requires it;
- ReconnectAttempt candidate/disposition reconciliation;
- OperationId receipt;
- `(GameSessionId, CommandId)` durable dedup where implemented;
- pending publication without scanning full audit history;
- authorized EventId/TransactionId/CharacterRevision audit lookup.

Full identities/keys are authoritative. UUIDv7 order may improve locality but never defines chronology. Hash-only/partial indexes never replace full equality verification. High-cardinality IDs do not become ordinary metrics labels.

## 15. Future implementation failure outcomes

| Condition | Required disposition |
|---|---|
| stale CharacterRevision | conflict; no Character mutation |
| stale lease/session/runtime generation | conflict; no stale control/mutation |
| duplicate identical OperationId | same logical result/reconciliation |
| same OperationId + different request | conflict; no reinterpretation |
| duplicate durable `(GameSessionId, CommandId)` | same command; no second effect |
| same numeric CommandId in another GameSession | distinct command |
| canonical name conflict | one DB-authoritative winner |
| naming-policy migration collision | cutover blocked until resolved |
| quota/lifecycle race | serialized under account guard |
| concurrent fresh admissions | at most one atomic authority winner; no partial candidate authority |
| reconnect PREPARE retry | same logical candidate/disposition; zero authority |
| stale/failed reconnect COMMIT | no authority switch/nonce success/protection manufacture |
| lost reconnect COMMIT response | reconcile persisted attempt/current generation; no second switch |
| DB unavailable | no authoritative mutation; no Redis/in-memory fallback |
| deadlock/serialization failure | bounded same-operation retry |
| mandatory audit unavailable in transaction | mutation does not commit |
| publication dependency down after commit | durable backlog/retry; committed gameplay stays committed |
| unsupported schema/profile/definition revision | fail closed |
| ambiguous Character commit response | reconcile receipt/current state |
| restored stale session/lease authority | fenced; no authority resurrection |

Raw PostgreSQL errors are not public contracts.

## 16. Security boundaries

- game DB runtime credentials cannot write Platform DB;
- Platform credentials cannot write Character/FND-04 authority relations;
- read projections use explicit read-only APIs/views/contracts;
- reconnect secrets never enter ordinary audit/analytics;
- knowing AccountId/CharacterId/GameSessionId is never authorization;
- ordinary corrections do not depend on manual raw SQL; later support/GM mutation uses typed audited domain commands.

## 17. Implementation evidence required later

This packet is paper-only. Future implementation must prove at least:

### Identity / constraints

- full UUID and persisted CommandId round trips;
- no cross-semantic ID substitution;
- CharacterId non-reuse;
- canonical name race and naming-policy migration collision detection.

### Character revision / quota

- stale CharacterRevision rejection;
- concurrent Character mutations produce one revision order;
- every quota-affecting lifecycle/create/transfer race respects policy;
- opposing transfers/rename-transfer do not split ownership or corrupt state.

### FND-04 authority

- concurrent fresh admissions have one final-boundary winner and no partial authority;
- fresh-admission Character root revalidation prevents concurrent transfer/lifecycle TOCTOU without advancing CharacterRevision solely for authority state;
- stale lease/runtime/session generation is rejected;
- PREPARE has zero authority and same retry returns the same logical candidate;
- process-local transport handles are never restored as durable candidate authority;
- COMMIT atomically switches binding/generation/proof/attempt disposition and consumes protection entitlement at most once under the ControlLoss lock;
- failed/stale COMMIT changes no authority;
- lost COMMIT response reconciles without a second switch;
- post-grace recovery creates a new GameSession and preserves actor state;
- actor-wide ControlLoss has one authority, survives restart and does not reset deadlines;
- full FND-04 binding revision set is revalidated on recovery.

### Idempotency

- duplicate create/rename/delete/restore/world-transfer/account-transfer has one effect;
- conflicting OperationId payload fails closed;
- durable gameplay command duplicate cannot double-apply.

### Audit/publication

- mutation + exact mandatory audit bytes + publication enqueue are atomic;
- crash after commit/before publish redelivers same EventId/bytes;
- duplicate publish yields one consumer effect;
- conflicting retained EventId content fails integrity checks;
- backlog cannot be discarded for capacity;
- replay cannot mutate gameplay;
- retention/privacy lifecycle cannot silently rewrite retained event semantics.

### Crash / restore / migration

- restart preserves Character/FND-04 fences and deadlines;
- terminal GameSession never revives;
- stale transport cannot regain authority;
- expand/backfill/validate/cutover/contract is demonstrated;
- interrupted backfill resumes idempotently;
- restore drill verifies Character/name/FND-04/receipt/audit/publication integrity;
- PITR/restore cannot resurrect pre-restore authority.

### Access/privacy

- Platform write credentials rejected from game authority tables;
- public/read projection cannot leak unauthorized ownership mapping;
- a pseudonymous analytics family cannot silently fall back to raw CharacterId/AccountId; restricted player-linked audit remains controlled by its explicit privacy class.

## 18. Owner decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

Accept these seventeen rules as a **binding partial DUR-02 baseline for the profile-neutral Character persistence sub-scope**:

1. **Normalized current-state**, not event sourcing: `character_root` is Character identity/lifecycle/owner/world/global-revision lock anchor; typed children hold distinct state.
2. **One CharacterRevision** advances once per committed Character semantic transaction and remains independent from FND-04 authority generations.
3. **Account guard rows** serialize every quota-affecting portfolio/lifecycle operation without becoming Account authority or count truth.
4. **Global name registry** stores complete domain-generated canonical key + policy revision; DB uniqueness decides races; naming-policy cutover validates all destination collisions before one new authoritative policy becomes active.
5. **Separate FND-04 relations** keep AccountPresenceClaim, CharacterLease, GameSession and actor-wide ControlLoss continuity as distinct authorities; no duplicated ControlLoss truth.
6. **Atomic fresh admission** commits GrantNonce/presence/lease/new GameSession/connection generation as one revalidated authority boundary, including Character root revalidation; failed admission leaves no partial authority.
7. **Reconnect/recovery PREPARE has zero authority**, uses bounded typed candidate/disposition state and never treats a process-local socket handle as durable authority.
8. **Reconnect/recovery COMMIT is the only binding switch** and atomically fences predecessor, advances connection generation, binds candidate transport/proof, preserves continuity, records attempt outcome and consumes the one eligible 4s protection entitlement at most once under ControlLoss fencing.
9. **Post-grace recovery uses a new GameSessionId** while preserving the same authoritative actor and any eligible existing ControlLoss semantics.
10. **Typed profile extensions only**; no generic JSON/KV/EAV misc-state escape hatch.
11. **Durable receipts** use OperationId for retryable Character workflows; `(GameSessionId, CommandId)` is persisted only where an actual durable gameplay boundary requires it.
12. **Explicit lock/isolation proof**: Character root + account/FND-04/name constraints close anomalies; READ COMMITTED is acceptable only with proof, otherwise bounded SERIALIZABLE; advisory locks are never sole authority.
13. **Retained immutable audit semantics + separate mutable publication state** commit atomically with mandatory audited mutations; privacy/retention lifecycle remains separately governed.
14. **Normalized current state is the Character checkpoint**; additional checkpoints reference typed owner-specific components; no generic snapshot blob and no acknowledged success before commit.
15. **No authority resurrection after restore**: PITR/disaster restore requires a newer non-rollback recovery fence/equivalent proof before admission resumes.
16. **Staged migration** uses expand -> migrate/backfill -> validate -> cut over -> contract; retirement, physical deletion and privacy erasure remain separate; CharacterId is never reused.
17. **Profile-neutral core only**: unresolved Reference values/formulas/profile-specific PvP/world facts and numeric operational policies stay outside core schema invariants and remain gated by their owners.

## 19. Effect if owner later accepts this packet

Because the stable `DUR-02 — Persistence v1` gate remains broader than this packet, acceptance should create a dedicated record such as:

```text
DUR-02 profile-neutral Character persistence sub-scope
Owner baseline        = OWNER_ACCEPTED PARTIAL BASELINE
Binding scope         = sections 4-18 of this packet
ImplementationStatus  = NOT_STARTED
Runtime / DDL authority = NONE

DUR-02 overall
DecisionStatus        = PROPOSED
DeliveryStatus        = PLANNED after partial-baseline closeout
ImplementationStatus  = NOT_STARTED
```

Acceptance would unblock a separately authorized implementation-design package **within the accepted Character persistence sub-scope** and provide stable typed extension contracts.

It would **not**:

- mark overall `DUR-02` accepted;
- create/migrate PostgreSQL tables;
- accept `GAME-ITEM-01` or `DUR-03`;
- define item/currency atomicity;
- select a PvP/world profile;
- make unresolved Reference behavior true;
- authorize runtime Character persistence;
- choose Rust migration/ORM/database libraries;
- set production RPO/RTO/backup frequency;
- authorize Platform or production changes.

A later full-DUR-02 reconciliation must identify the remaining stable Persistence-v1 subjects, account for later gate splits such as GAME-ITEM/DUR-03, and either accept them or explicitly supersede/narrow the historical DUR-02 scope before the overall DecisionStatus may become `ACCEPTED`.

## 20. Supersession / reopening

A later proposal may reopen an accepted partial-baseline clause only with named evidence such as:

- a proven correctness anomaly not closed by the lock/constraint/authority-transition model;
- measured contention showing one Character root revision is an unacceptable bottleneck plus a proven equivalent partitioned fence;
- an accepted profile semantic requiring new typed durable state;
- evidence that a deferred Reference formula/value constrains physical representation/atomicity;
- a PostgreSQL limitation discovered during implementation requiring an equivalent safer design;
- security/privacy/restore evidence requiring stronger separation/fencing;
- explicit later architecture superseding ADR-0004 or this accepted partial scope.

ORM defaults, convenience, current Global schema, Canary/crystalserver tables or generic JSON flexibility are not sufficient evidence.

## 21. Deliberately not decided

- remaining whole-gate DUR-02 Persistence-v1 subjects outside this Character packet;
- exact SQL table/column/index/constraint/schema names;
- migration framework/library;
- ORM/query builder/Rust DB crate;
- connection-pool settings;
- exact scalar representation of CharacterRevision/lease/session generations where semantics allow equivalent non-reused forms;
- exact stable ruleset/content definition-key scalar representation where its owner has not frozen it;
- profile-specific child table layouts;
- item/currency/market/house schema;
- partitioning/sharding;
- operational retention/backup/RPO/RTO/retry values;
- reconnect-secret hashing/KMS;
- exact non-rollback disaster-recovery fence implementation/issuer;
- physical GrantNonce/security-evidence relation layout;
- production topology;
- runtime implementation.

Until the owner accepts or modifies section 18, this document remains **PRE-DECISION ARCHITECTURE / NOT ACCEPTED** and `DUR-02` overall remains `PROPOSED / PLANNED / NOT_STARTED`.
