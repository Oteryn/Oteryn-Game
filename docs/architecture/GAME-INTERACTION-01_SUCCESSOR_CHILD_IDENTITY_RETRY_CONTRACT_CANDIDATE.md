# GAME-INTERACTION-01 successor contract candidate — child identity, retry and reconciliation

- Status: `PROPOSED / NONCANONICAL`
- Gate: `GAME-INTERACTION-01` (same gate; no new global gate ID)
- Successor issue/task: #274 / `OTV2-20260815-game-interaction-successor-r1`
- Successor draft PR: #277
- Predecessor issue/PR: #262 / draft PR #269
- Predecessor reviewed head: `71253a8d5805ed37ec451e40e2c7200c38031a52`
- Base: `main@cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e`
- Mode: `PAPER-ONLY / CONTRACT`
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`
- Implementation authority: `NONE`

## 1. Successor boundary

This candidate is a fresh bounded successor for the unchanged `GAME-INTERACTION-01` gate.

The predecessor task reached `repair_cycles_for_current_gate: 3`. The final coordinator review of draft PR #269 identified two remaining material findings after that budget was exhausted. Repository governance therefore forbids another predecessor repair cycle.

Normative governance consequences:

- issue #262, PR #269, predecessor branch and predecessor-owned files are read-only evidence to this worker;
- this candidate resolves only child fan-out/cascade identity and cross-owner/client retry/error semantics;
- no predecessor lifecycle state is rewritten;
- no new global gate ID is created;
- this candidate is intended for Architecture Coordinator integration with the sound predecessor contract, not as a replacement for all predecessor design.

## 2. Preserved predecessor invariants

This successor MUST preserve:

1. server-authoritative target resolution;
2. typed/versioned interaction state machines;
3. explicit semantic scope, lifetime, reset and recovery;
4. GAME-ITEM + DUR-03 item/value authority;
5. GAME-ABILITY effect authority;
6. movement/relocation/handoff delegation without choosing its final owner here;
7. named coupled workflows rather than a generic multi-owner transaction;
8. bounded cascades, with numeric bounds owned elsewhere;
9. DUR-04 proposal-only guest scripts;
10. deterministic semantic revisions/order/RNG under SIM-DETERMINISM;
11. no generic mutable process-global/`GLOBAL` interaction scope.

## 3. Consumed contracts

This candidate consumes without redefining:

- `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md`;
- `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`;
- `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`;
- `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md`;
- `docs/architecture/FND-04C_ERROR_DIAGNOSTICS_FAILURE_COMPATIBILITY_CONTRACT.md`;
- `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`;
- `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`;
- `docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`;
- accepted GAME-ITEM-01 authority contracts;
- an eventually accepted GAME-ABILITY-01 owning contract for ability-owned effect internals.

FND-02 remains the client-command idempotency/order authority. GAME-INTERACTION MUST NOT invent a competing client command identity.

## 4. Identity terms

### 4.1 `RootSourceOccurrenceRef`

A stable authoritative source occurrence accepted by its owning domain, for example:

- client action -> FND-02 `CommandRef`;
- movement-derived interaction -> committed authoritative movement occurrence reference;
- timer -> stable timer occurrence identity;
- domain workflow -> accepted owner operation/transaction/cause reference;
- event -> accepted event occurrence identity.

A client hint, transport packet, UI selection, callback object or pointer is never a root authority identity.

### 4.2 `OwnerOperationRef`

A stable typed operation/transaction/attempt identity owned by a delegated domain, for example a DUR-03 TransactionId/OperationId.

GAME-INTERACTION MUST NOT create one generic global `OperationId` to replace owner-specific identities.

### 4.3 `SemanticRevisionContext`

The exact immutable behavior-affecting artifact/revision set bound to a logical occurrence, as required by SIM/content/owner contracts.

### 4.4 `AuthorityFenceEvidence`

Current runtime ownership generation and mutable state/domain revisions used to prove that the current owner may apply or complete work.

Authority fences are not automatically logical occurrence identity.

## 5. Stable child-occurrence identity

### 5.1 Normative shape

A child occurrence MUST have semantic identity equivalent to:

```text
InteractionChildOccurrenceRef = (
    ParentSourceOccurrenceRef,
    InteractionDefinitionRef,
    AuthoritativeTargetDiscriminator,
    TypedEdgeOrCapabilityDiscriminator,
    OptionalCanonicalChildOrdinal,
    SemanticRevisionContext
)
```

For first-level fan-out:

```text
ParentSourceOccurrenceRef = RootSourceOccurrenceRef
```

For a nested cascade:

```text
ParentSourceOccurrenceRef = parent InteractionChildOccurrenceRef
```

A bounded cascade therefore forms a deterministic source-derived ancestry path:

```text
RootSourceOccurrenceRef
  / child-step-1
  / child-step-2
  / ...
```

No globally allocated `InteractionId`/UUID is required.

### 5.2 Why parent ancestry is required

Two distinct siblings under the same root can independently cascade into the same final target/edge. Identity truncated to only the ultimate root + final target/edge could merge those semantic children.

Including the stable parent child ref prevents that collision. If an owning mechanic intentionally defines two paths as one semantic idempotent join, that join MUST be explicit in its typed contract; accidental dedup-key truncation is forbidden.

### 5.3 `InteractionDefinitionRef`

MUST identify the stable authored/compiled interaction definition/object under the exact bound content revision.

MUST NOT use:

- memory address;
- transient loader slot;
- unordered registry position;
- source-code line number as accidental runtime identity.

### 5.4 `AuthoritativeTargetDiscriminator`

MUST derive from server-authoritative target/object resolution in the applicable World/Channel/Instance scope.

It may be a strong owner-defined object/entity identity or canonical typed world-object key.

MUST NOT use:

- client target hint;
- UI selection order;
- pointer/reference address;
- hash bucket;
- container discovery index;
- thread/worker completion order;
- unspecified database row order.

If an interaction family cannot name a stable authoritative target discriminator, that family is implementation-`BLOCKED`.

### 5.5 `TypedEdgeOrCapabilityDiscriminator`

MUST distinguish semantically different edges/capabilities on the same target, such as accepted typed `ON_ENTER`, `ON_LEAVE`, `ON_CONTACT`, `USE` or mechanic-specific versioned edge keys.

Free-form callback names/function pointers do not become authoritative identity.

### 5.6 `OptionalCanonicalChildOrdinal`

The ordinal MUST be absent by default.

It MAY be present only when all non-ordinal fields still describe more than one intentionally distinct semantic child.

When present:

1. multiplicity is explicit in authored/compiled semantics;
2. assignment follows explicit authored semantic order or a canonical typed comparator over stable semantic fields;
3. the same content revision reproduces the same ordinal;
4. runtime enumeration/hash-map/thread/worker/unspecified DB order never assigns it;
5. accidental duplicate definitions fail validation rather than receiving invented ordinals.

The ordinal is a semantic discriminator of last resort, not a patch over unstable iteration.

### 5.7 `SemanticRevisionContext`

The child MUST retain the applicable exact behavior-affecting context, including as needed:

- World Bundle/content artifact identity;
- interaction-definition revision;
- ruleset/world-policy revision;
- SIM profile revision;
- script artifact/WIT/execution-profile revision;
- delegated-owner semantic revision needed to interpret its outcome.

Retry/replay/recovery uses the same bound context or an explicitly accepted compatibility/reconciliation rule. A historical child MUST NOT be silently reinterpreted under the newest active content.

### 5.8 Logical identity versus current authority

Transient runtime ownership generation MUST NOT create a new logical child after failover.

```text
logical child identity = InteractionChildOccurrenceRef
current mutation authority = validate current AuthorityFenceEvidence
```

A stale generation/revision/fence completion fails before mutation. The current/replacement owner then reconciles the same child/foreign operation where one exists.

Immutable behavior-affecting revisions remain bound semantic identity context. Mutable revisions that are only optimistic/fencing preconditions remain fence evidence rather than fresh-occurrence identity.

### 5.9 Representation is deliberately unfrozen

This candidate does not choose:

- UUID allocation;
- digest/hash algorithm;
- binary representation;
- database key/schema;
- retention window/count;
- numeric protocol code/field values.

A later implementation MAY derive a compact digest/index from the canonical tuple/path, but tuple semantics remain authoritative and collisions MUST be handled safely. A diagnostic trace/correlation ID is never dedup authority.

## 6. Deterministic child plan

Before any child can commit or any foreign owner can accept work, the interaction owner MUST have a reproducible semantic child plan equivalent to:

```text
InteractionChildPlan {
    root_source_occurrence
    exact_semantic_revision_context
    ordered_child_occurrence_refs[]
    required_authority/precondition evidence
}
```

This is a semantic contract, not a storage schema.

### 6.1 Canonical order

Server-authoritative eligible targets/edges MUST be canonicalized using one accepted rule:

- explicit authored semantic order where order matters;
- stable typed comparator over authoritative semantic keys where the collection is set-like; or
- retained accepted FND-03 execution order only where the owning contract explicitly makes that order semantic/replay evidence.

Forbidden tie-breakers include hash-map/set traversal, pointer order, OS/thread/worker order and unspecified database row order.

### 6.2 Partial-progress recovery

Once any child has committed or a delegated owner has accepted work, recovery MUST reproduce the same accepted child set/order from retained authoritative evidence and/or exact immutable bound content/state.

Recovery MUST NOT:

- re-enumerate a newer mutable world and call it the same historical root;
- add/drop siblings because current state differs;
- renumber siblings from a different physical order;
- allocate a fresh UUID because original identity is inconvenient to reconstruct.

If the accepted plan cannot be reconstructed safely, fail closed into explicit reconciliation.

## 7. Child lifecycle and exactly-once rule

An implementation MAY represent state differently internally, but MUST preserve semantics equivalent to:

```text
UNSTARTED | PENDING | COMMITTED | REJECTED
```

- `UNSTARTED`: accepted child exists; no semantic commit/delegated acceptance yet; recovery may execute it once if current fences authorize.
- `PENDING`: final outcome unresolved; same child and same accepted foreign operation are reconciled.
- `COMMITTED`: semantic commit proven exactly once; replay/retry never reapplies it.
- `REJECTED`: logical child terminal; replay does not reevaluate it as fresh work.

Sibling refs MUST be distinct. Duplicate delivery of the same sibling MUST converge to one lifecycle/outcome.

Loss of a retained result payload MUST NOT re-enable execution.

## 8. Deterministic RNG

Every child-owned authoritative random decision MUST have semantic identity equivalent to:

```text
InteractionRngDecisionRef = (
    InteractionChildOccurrenceRef,
    RngPurposeKey,
    DeterministicDrawOrdinal
)
```

SIM owns the algorithm/profile/seed derivation.

GAME-INTERACTION requirements:

1. same child + purpose + draw ordinal is the same logical random decision;
2. retry/replay/recovery cannot reroll;
3. runtime failover/generation change cannot reroll;
4. stale/deferred completion cannot reroll;
5. draw ordinal follows semantic purpose order, not scheduling order;
6. unrelated siblings/purposes do not gain accidental draw-order coupling;
7. rejected work before authoritative acceptance does not consume gameplay RNG unless SIM/owner semantics explicitly commit such an occurrence;
8. security-sensitive seed/root evidence is not public correlation.

## 9. Public/cross-owner outcome state

A public/cross-component interaction outcome subject to retry/ambiguity MUST expose:

```text
InteractionOutcomeState = COMMITTED | PENDING | REJECTED
```

### 9.1 `COMMITTED`

The declared semantic commit point is proven complete exactly once.

For a named coupled workflow, only its final reconciliation owner/coordinator may establish whole-workflow `COMMITTED`. Participant-local success is insufficient.

### 9.2 `PENDING`

Final semantic outcome is unresolved and blind re-execution could duplicate effects/value.

While pending:

- retain the same `InteractionChildOccurrenceRef`;
- retain the same `OwnerOperationRef` where foreign work was accepted/may have been accepted and the owner can identify it;
- retain the same parent `CommandRef` for client-originated roots;
- reconcile/query/resume the same occurrence;
- do not issue a new `CommandRef` for the same semantic intent while the prior occurrence may still commit;
- transport reconnect/new `connection_generation` does not create a new semantic occurrence;
- a Foundation session recovery path does not erase durable/foreign operation history;
- retirement/final-outcome proof is required before a logically duplicate fresh attempt.

### 9.3 `REJECTED`

The original logical occurrence is terminal and no accepted work from it may later commit.

If policy permits a fresh attempt:

- client fresh attempt uses a **new `CommandRef`** under the current valid GameSession, or a new valid session only when Foundation requires it;
- non-client fresh attempt uses a new owner-defined root occurrence;
- old source replay returns the same rejection and never becomes fresh work.

## 10. Local mutation state under ambiguity

Where a pending cross-owner outcome can coexist with participant-local mutation, the safe envelope MUST expose an owner-approved state equivalent to:

```text
LocalMutationState = NOT_COMMITTED | COMMITTED | UNKNOWN
```

- `NOT_COMMITTED`: proven no local authoritative mutation from this occurrence committed;
- `COMMITTED`: local commit proven, but whole coupled workflow may still be pending;
- `UNKNOWN`: current recovery evidence cannot classify it safely.

Timeout, cancellation request, lost response or dependency outage MUST NOT be interpreted as `NOT_COMMITTED` without proof.

## 11. Mandatory safe correlation

A public/cross-component result MUST carry the applicable safe correlation needed for deterministic reconciliation:

- root source occurrence / externally legal parent `CommandRef` or event reference;
- `InteractionChildOccurrenceRef` or protocol-safe exact representation/reference;
- semantic scope (`WorldId`, `ChannelId`, `InstanceId` as applicable; never generic mutable `GLOBAL`);
- exact bound semantic revision reference;
- `InteractionOutcomeState`;
- stable Foundation machine category;
- contract-owned symbolic code;
- `OwnerOperationRef` when foreign work was accepted/may have been accepted and the owner exposes it;
- `LocalMutationState` when partial/local mutation ambiguity exists;
- optional redacted diagnostic/correlation reference that carries no authority.

MUST NOT expose credentials, secret RNG material, raw exceptions, stack traces or unrestricted internal persistence identifiers.

## 12. Retry authority vocabulary

GAME-INTERACTION distinguishes:

```text
RECONCILE_SAME_OCCURRENCE
NEW_OCCURRENCE_ALLOWED
OWNER_INTERVENTION_REQUIRED
NO_RETRY
```

- `RECONCILE_SAME_OCCURRENCE`: physical calls may repeat idempotently; source/child/OwnerOperationRef remains the same.
- `NEW_OCCURRENCE_ALLOWED`: prior occurrence is terminal; a fresh user/domain attempt gets a new root/new CommandRef.
- `OWNER_INTERVENTION_REQUIRED`: automated bounded reconciliation cannot establish a safe terminal state; the owning recovery/operations path must resolve it before a fresh semantic attempt.
- `NO_RETRY`: the owning terminal/security contract forbids interaction-level retry.

This is orthogonal to Foundation progression `RETRYABLE | TERMINAL | SECURITY_TERMINAL` and exists to remove ambiguity about **what** may be retried.

## 13. GAME-INTERACTION-owned outcome/error codes

The symbolic `GI_*` codes below are contract-owned by `GAME-INTERACTION-01` if this candidate is accepted. Numeric wire registration remains explicitly deferred to the future registered GAME-INTERACTION payload/schema path under FND-02.

### 13.1 Normative outcome matrix

| Code | Foundation category | Progression | Outcome | Retry authority | Same source / owner operation | New CommandRef/source | Local mutation | Caller terminal test | Final reconciliation owner |
|---|---|---|---|---|---|---|---|---|---|
| `GI_DEPENDENCY_UNAVAILABLE_REJECTED` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | `REJECTED` | `NEW_OCCURRENCE_ALLOWED` | old source replay-only; no foreign op exists | required for fresh attempt if still authorized | `NOT_COMMITTED` | this `REJECTED` proves old occurrence terminal | current interaction owner |
| `GI_DEPENDENCY_UNAVAILABLE_PENDING` | `DEPENDENCY_UNAVAILABLE` | `RETRYABLE` | `PENDING` | `RECONCILE_SAME_OCCURRENCE` | same child + same `OwnerOperationRef` where accepted/maybe accepted | forbidden for same intent while pending | `NOT_COMMITTED` / `COMMITTED` / `UNKNOWN` as owner evidence permits | only later owner proof yields terminal state | delegated owner or named workflow coordinator; interaction maps |
| `GI_TIMEOUT_REJECTED` | `TIMEOUT` | `RETRYABLE` | `REJECTED` | `NEW_OCCURRENCE_ALLOWED` | old source replay-only; no accepted foreign op | required for fresh attempt | `NOT_COMMITTED` | this `REJECTED` proves old occurrence terminal | current interaction owner |
| `GI_TIMEOUT_PENDING` | `TIMEOUT` | `RETRYABLE` | `PENDING` | `RECONCILE_SAME_OCCURRENCE` | same child + same owner operation | forbidden for same intent while pending | may be `COMMITTED`/`UNKNOWN`; timeout is not abort proof | owner commit/reject/cancel proof decides | delegated owner/workflow coordinator |
| `GI_CANCELLED_REJECTED` | `CANCELLED` | `TERMINAL` | `REJECTED` | `NEW_OCCURRENCE_ALLOWED` only for later distinct intent | same source replays cancellation result | new source required for later distinct intent | `NOT_COMMITTED`; all accepted work proven retired before semantic commit | documented cleanup/retirement proves terminal cancellation | interaction owner or workflow coordinator that proves retirement |
| `GI_CANCELLATION_RECONCILIATION_REQUIRED` | `INTERNAL_UNAVAILABLE` | `RETRYABLE` | `PENDING` | `RECONCILE_SAME_OCCURRENCE` | same child + same owner operation | forbidden for same intent while pending | `NOT_COMMITTED` / `COMMITTED` / `UNKNOWN` | a cancel request is not `CANCELLED`; terminal only after owner proof | delegated owner/workflow coordinator |
| `GI_RECONCILIATION_REQUIRED` | `INTERNAL_UNAVAILABLE` | `RETRYABLE` | `PENDING` | `RECONCILE_SAME_OCCURRENCE` or owner intervention after bounded automatic policy | same source/child/op | forbidden until terminal retirement/proof | `NOT_COMMITTED` / `COMMITTED` / `UNKNOWN` | terminal only after authoritative reconciliation | owner named by underlying operation/workflow |
| `GI_COUPLED_WORKFLOW_RECONCILIATION_REQUIRED` | `INTERNAL_UNAVAILABLE` | `RETRYABLE` | `PENDING` | `RECONCILE_SAME_OCCURRENCE` | same child + same workflow/participant operation refs | forbidden while same workflow intent pending | participant local may be `COMMITTED`/`UNKNOWN`; whole workflow still pending | only coordinator emits whole terminal classification | named workflow coordinator |

### 13.2 Dependency unavailable

`GI_DEPENDENCY_UNAVAILABLE_REJECTED` is legal only when the interaction owner proves:

- dependency failed before foreign acceptance;
- no local authoritative mutation committed;
- nothing from this occurrence can later commit.

Otherwise use `GI_DEPENDENCY_UNAVAILABLE_PENDING` and reconcile the same child/owner operation.

A dependency outage never means `safe to issue a new CommandRef` while prior acceptance/commit is ambiguous.

### 13.3 Timeout

Timeout describes expiration of a named deadline, not rollback.

- `GI_TIMEOUT_REJECTED`: only when timeout-before-acceptance/no-mutation is proven;
- `GI_TIMEOUT_PENDING`: when work was/may have been accepted or committed.

No general `timeout => fresh retry` rule is permitted.

### 13.4 Cancellation

Foundation defines `CANCELLED` as intentionally cancelled with documented cleanup state. Therefore GAME-INTERACTION MUST NOT return the `CANCELLED` category merely because a cancel request was sent.

- proven cleanup/retirement before semantic commit -> `GI_CANCELLED_REJECTED`, category `CANCELLED`, terminal old occurrence;
- commit proven to have won -> `COMMITTED`;
- cancel/commit ordering still ambiguous -> `GI_CANCELLATION_RECONCILIATION_REQUIRED`, category `INTERNAL_UNAVAILABLE`, state `PENDING`, same child/op.

This prevents a cancellation request from being mistaken for rollback proof.

### 13.5 Generic reconciliation

`GI_RECONCILIATION_REQUIRED` is the fallback only when no narrower truthful public category owns the current wait condition.

Automatic reconciliation MUST be bounded by the owning contract. If that contract exhausts automatic recovery without safe terminal proof, retry authority becomes `OWNER_INTERVENTION_REQUIRED`; this candidate does not invent numeric thresholds.

### 13.6 Coupled workflow recovery

Use `GI_COUPLED_WORKFLOW_RECONCILIATION_REQUIRED` only when a named coupled workflow remains unresolved and no more specific dependency/timeout/cancellation code is truthful.

- whole state stays `PENDING`;
- same workflow/participant operation refs are retained;
- participant-local commit never implies whole-workflow commit;
- new duplicate CommandRef/source occurrence is forbidden;
- final whole classification belongs to the named coordinator.

## 14. Stale generation/revision and delegated completion ambiguity

This section deliberately separates **stale delivery/apply failure** from **underlying operation outcome**.

### 14.1 Stale completion application

A completion/input produced under stale runtime generation/revision/fence MUST be rejected before it can mutate current authority.

Stable application-level code:

```text
GI_STALE_COMPLETION_REJECTED
category = STALE_GENERATION
progression = TERMINAL for this stale completion/application attempt
mutation_by_stale_attempt = NOT_COMMITTED
retry_authority_for_stale_message = NO_RETRY
```

The stale message itself is never replayed as current-authority mutation and never receives a new child identity.

Safe correlation carries the existing child ref, stale/current relation class and existing owner operation reference when safe; it MUST NOT expose secrets or use raw generation as a credential.

### 14.2 Underlying child may still be pending

`GI_STALE_COMPLETION_REJECTED` does **not** prove that the underlying delegated operation failed or rolled back.

If the foreign operation was accepted/may have been accepted and final outcome is unknown:

```text
underlying child state = PENDING
underlying code = GI_RECONCILIATION_REQUIRED
retry authority = RECONCILE_SAME_OCCURRENCE
same InteractionChildOccurrenceRef
same OwnerOperationRef
new CommandRef/source for same intent = FORBIDDEN
local/foreign mutation = COMMITTED or UNKNOWN until owner proves otherwise
final reconciliation owner = underlying delegated owner, mapped by current interaction owner
```

If the current owner already has authoritative terminal evidence, it surfaces that terminal result instead of converting stale delivery into ambiguity.

This split conforms to Foundation `STALE_GENERATION`'s no-mutation guarantee for the stale application while still representing an operation that may have committed elsewhere before its stale completion was delivered.

## 15. Foundation-owned security/session failures

GAME-INTERACTION MUST NOT reinterpret authentication/session/security failures as ordinary retryable interaction errors.

When FND-02/FND-04/FND-04C owns the narrow code:

- its stable code/category/progression remains authoritative;
- `SECURITY_TERMINAL` MUST NOT be downgraded to `RETRYABLE`;
- same-vs-new session/credential action follows Foundation;
- stale connection generation cannot submit or revive gameplay mutation;
- Foundation-safe correlation restrictions apply;
- GAME-INTERACTION does not mint a replacement `GI_*` session code.

A new GameSession does not erase previously accepted durable/delegated operation history; logically duplicate action remains blocked until any prior ambiguous operation is reconciled/retired.

## 16. Bounded internal-to-public mapping

Mapping MUST depend on semantic acceptance/commit evidence, not raw exception text.

Examples:

```text
internal service unavailable
  -> GI_DEPENDENCY_UNAVAILABLE_REJECTED
     OR GI_DEPENDENCY_UNAVAILABLE_PENDING

internal deadline expiry
  -> GI_TIMEOUT_REJECTED
     OR GI_TIMEOUT_PENDING

cancel request/ack implementation details
  -> GI_CANCELLED_REJECTED only with documented cleanup
     OR GI_CANCELLATION_RECONCILIATION_REQUIRED

stale worker completion
  -> GI_STALE_COMPLETION_REJECTED for stale application
     + GI_RECONCILIATION_REQUIRED for underlying child only if outcome remains ambiguous
```

Raw driver/network/DB/library exception strings MUST NOT become public API behavior.

Foreign-owner narrow errors whose retry/mutation/security semantics matter MUST remain owner-owned and be mapped according to the accepted adapter/workflow contract. Distinct `SECURITY_TERMINAL`, `TERMINAL` and same-attempt reconciliation semantics MUST NOT collapse into generic `TEMPORARY_FAILURE`.

## 17. CommandRef / session retry rules

### 17.1 Same `CommandRef`

Same `CommandRef` is used for:

- FND-02 duplicate replay/reconciliation of an already-reserved command;
- recovering the same eventual terminal command result;
- retaining the parent source identity of pending interaction children across eligible same-GameSession reconnect.

It never converts a terminal command into a fresh semantic attempt.

### 17.2 New `CommandRef`

A new `CommandRef` is required for a fresh client semantic attempt only after the prior occurrence is provably terminal and gameplay policy permits the new attempt.

Examples:

- terminal `REJECTED` + retry permitted -> new CommandRef;
- prior `COMMITTED` + later distinct player action -> new CommandRef;
- prior `PENDING` -> new CommandRef for the same semantic intent is forbidden.

### 17.3 New GameSession

Session admission/recovery belongs to FND-04. If old GameSession terminates and a new valid one is created, GAME-INTERACTION still MUST reconcile any durable/foreign operation from the old occurrence that could have committed before allowing a logically duplicate new action.

New command/session namespace is not proof that old external value/effect history disappeared.

## 18. Non-client source retry

For movement/timer/event/domain-owned roots:

- physical redelivery/recovery of the same source occurrence reuses the same child refs;
- a genuinely new committed movement/timer/event occurrence has a new root and therefore new child refs;
- recovery must not relabel a historical root merely because current world state or runtime generation changed;
- inability to reconstruct root identity fails closed/reconciles rather than allocating an ad hoc UUID.

## 19. Foreign-owner dependencies and explicit blockers

### 19.1 DUR-03 item/value

Owning contract: `docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`.

- DUR-03 TransactionId/OperationId/cause is the foreign owner identity;
- DUR-03 owns durable value commit/abort/ambiguity;
- GAME-INTERACTION preserves trigger/child correlation only;
- ambiguous durable result remains pending on the same DUR-03 transaction;
- stale runtime completion cannot duplicate committed value.

### 19.2 GAME-ABILITY effect

Owning contract: accepted GAME-ABILITY-01 effect/whole-gate contract.

At this successor baseline draft PR #268 remains unmerged/blocked and is therefore noncanonical.

**Implementation blocker:** any interaction adapter requiring final GAME-ABILITY effect operation identity, narrow effect code or formula/effect commit semantics is blocked until the Architecture Coordinator accepts the relevant GAME-ABILITY contract.

This successor does not freeze GAME-ABILITY formulas or error internals.

### 19.3 Movement/relocation/handoff

Owning contract: `UNKNOWN / NOT YET ACCEPTED` by explicit predecessor boundary.

**Implementation blocker:** any teleport/movement/relocation/handoff interaction is blocked until a named accepted owner contract defines operation identity, fences, completion, timeout/cancellation, stale completion and recovery.

This successor does not choose that owner.

### 19.4 Durable writable text

Owning contract: `UNKNOWN / NOT YET ACCEPTED`.

**Implementation blocker:** any authoritative durable writable-text interaction is blocked until a named accepted owner contract exists.

This successor does not choose that owner.

### 19.5 Client protocol registration

Owning contract: `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` plus a future registered GAME-INTERACTION typed gameplay payload/schema.

**Implementation blocker:** client-visible child-ref representation, outcome state, symbolic `GI_*` codes and correlation fields require accepted FND-02 gameplay payload/registry integration before runtime/client implementation acceptance.

This successor does not allocate numeric message/error IDs or modify FND-02.

## 20. Named coupled-workflow requirements

Any mechanic spanning multiple authoritative owners where partial success matters MUST have a named accepted workflow contract before implementation.

It MUST define:

- coordinator/final reconciliation owner;
- source occurrence and participant owner operation refs;
- participant authority boundaries;
- exact bound semantic revision context;
- generation/revision/fence checks;
- semantic commit point / accepted prepare-commit equivalent;
- participant/local mutation states;
- idempotency/duplicate behavior;
- timeout;
- cancellation;
- stale completion;
- crash/restart/failover recovery;
- compensation only where explicitly legal;
- bounded public error mapping;
- exact conditions for whole `COMMITTED`, `PENDING` and terminal `REJECTED`.

Generic interaction outcome state does not substitute for this workflow contract.

## 21. Final reconciliation owner matrix

| Case | Final authority | GAME-INTERACTION behavior |
|---|---|---|
| interaction-local child | current FND-03 authoritative runtime owner in exact semantic scope | validate fences, dedup child, emit one terminal outcome |
| DUR-03 item/value | DUR-03 transaction owner/coordinator | same transaction reconciliation; boundedly map result |
| GAME-ABILITY effect | accepted GAME-ABILITY effect owner | preserve trigger correlation; do not own formula/effect commit |
| movement/handoff | **BLOCKED** until named accepted owner | do not infer retry/completion semantics |
| named coupled workflow | accepted workflow coordinator | only coordinator declares whole semantic unit terminal |
| client command/session identity | FND-02/FND-04 | same-vs-new command/session follows Foundation |

## 22. Deterministic acceptance scenarios

### GI-SR-01 — one movement occurrence -> N contacts -> partial delivery/retry/recovery

**Given**

- committed authoritative movement occurrence `M`;
- exact semantic revision context `R`;
- authoritative contacts `A`, `B`, `C` discovered in arbitrary physical order;
- canonical semantics produce distinct ordered children:
  - `C1 = (M, DefA, A, EdgeEnter, -, R)`;
  - `C2 = (M, DefB, B, EdgeContact, -, R)`;
  - `C3 = (M, DefC, C, EdgeEnter, -, R)`;
- child RNG uses `(Ci, purpose, draw_ordinal)`.

**Progress**

1. `C1` commits.
2. `C2` delegates as owner operation `O2`, which is accepted, but final response is lost; `C2=PENDING`.
3. runtime fails before `C3` commits.
4. replacement owner recovers under newer ownership generation.
5. stale old-generation completion for `C2` arrives.

**Required**

1. same child plan under `R` is reconstructed;
2. `C1` is not applied again;
3. stale `C2` completion is rejected before mutation as `GI_STALE_COMPLETION_REJECTED`;
4. underlying `C2` remains the same child and reconciles the same `O2` under `GI_RECONCILIATION_REQUIRED` until owner truth is known;
5. `C3` executes at most once under its original child ref if current fences authorize;
6. siblings never collapse;
7. hash/container/worker order cannot change child identity/order;
8. retries/replay cannot reroll child RNG;
9. after reconciliation each child has exactly one terminal outcome;
10. replay under `R` reproduces the same semantic child plan/outcomes.

PASS requires all ten.

### GI-SR-01B — nested cascade collision

Two first-level children under `M` both cascade into the same final target/edge. Their nested child refs MUST remain distinct because their `ParentSourceOccurrenceRef` values differ. Truncating identity to `(M, final_target, final_edge)` fails acceptance.

### GI-SR-02 — dependency unavailable before acceptance

No foreign work accepted and no local mutation committed:

```text
GI_DEPENDENCY_UNAVAILABLE_REJECTED
state = REJECTED
local_mutation = NOT_COMMITTED
```

Old CommandRef remains terminal/replay-only. Fresh permitted client attempt uses a new CommandRef.

### GI-SR-03 — timeout after possible acceptance

Possible delegated acceptance at timeout:

```text
GI_TIMEOUT_PENDING
state = PENDING
same child
same OwnerOperationRef
new same-intent CommandRef = forbidden
```

Final delegated owner reconciles.

### GI-SR-04 — cancellation race

- commit proof -> `COMMITTED`;
- documented retirement before semantic commit -> `GI_CANCELLED_REJECTED` / terminal `REJECTED`;
- ordering ambiguous -> `GI_CANCELLATION_RECONCILIATION_REQUIRED` / `PENDING`.

No path guesses rollback or duplicates the operation.

### GI-SR-05 — stale delegated completion

Old-generation completion mutates nothing and receives `GI_STALE_COMPLETION_REJECTED`. If underlying operation outcome remains unknown, child remains `PENDING` with `GI_RECONCILIATION_REQUIRED`, same child and same owner operation. No new CommandRef/source/RNG stream is created.

### GI-SR-06 — coupled workflow partial participant success

One participant locally committed, another/final coordinator unresolved. Whole workflow remains `PENDING`; local state may say `COMMITTED`, but only the named coordinator emits whole terminal outcome. New duplicate CommandRef/source is forbidden.

### GI-SR-07 — terminal rejection then fresh attempt

After proven `GI_TIMEOUT_REJECTED`, if gameplay still permits another attempt, old CommandRef remains terminal/replay-only and the fresh action uses a new CommandRef/new child refs. New random decisions are legal because this is a genuinely new occurrence, not retry of the old one.

### GI-SR-08 — Foundation security-terminal

If FND-04C classifies the owning session/admission failure `SECURITY_TERMINAL`, GAME-INTERACTION does not wrap it as ordinary `GI_DEPENDENCY_UNAVAILABLE_*`, does not advise same-command/session retry, and follows Foundation recovery/reauthentication semantics.

## 23. Failure-mode invariants

Under crash/restart/retry/failover:

1. one semantic child has at most one authoritative commit;
2. distinct siblings/path siblings have distinct refs unless an explicit owner join rule says otherwise;
3. same child retry retains semantic revisions and RNG identity;
4. stale authority cannot mutate;
5. stale completion rejection does not imply underlying rollback;
6. timeout does not imply abort;
7. cancellation request does not imply `CANCELLED`;
8. `PENDING` forbids blind new semantic attempt;
9. `REJECTED` means nothing from that occurrence can later commit;
10. `COMMITTED` is never reapplied;
11. new GameSession does not erase durable/foreign reconciliation;
12. scripts remain proposal-only;
13. foreign commit authority remains foreign-owned;
14. missing owner/narrow error contract blocks implementation rather than caller guesswork.

## 24. Decision timing

These semantics MUST be decided before implementation because they define dedup equality, deterministic RNG occurrence identity, failover/recovery, caller retry, cross-owner adapter safety and public result meaning.

Deferring them until runtime/DDL/protocol coding risks persisting unstable identity or shipping retry behavior capable of duplicating value/effects.

## 25. Explicit non-decisions

`DECISIONS_NOT_TAKEN`:

- no new global gate ID;
- no generic global `InteractionId`/UUID;
- no generic global owner `OperationId`;
- no generic mutable `GLOBAL` interaction scope;
- no teleport/movement/handoff owner;
- no writable-text owner;
- no GAME-ABILITY formulas/effect internals;
- no numeric cascade/resource/retry limits;
- no Rust/runtime/client/server implementation;
- no DDL/migration/storage schema;
- no physical hash/digest/serialization choice;
- no numeric wire error IDs;
- no Platform/external-repository work;
- no production/deployment work;
- no coordinator-only global overlay mutation.

## 26. Cross-domain findings

`CROSS_DOMAIN_FINDINGS: NONE NEW`.

Inherited explicit blockers remain:

- GAME-ABILITY whole-gate integration is noncanonical while draft PR #268 remains unmerged/blocked;
- movement/handoff owner remains unresolved;
- durable writable-text owner remains unresolved.

## 27. Implementation readiness conditions

Even after architecture acceptance, runtime remains `NOT_STARTED` until a future authorized implementation task verifies the necessary dependencies.

Implementation is blocked until, as applicable:

- coordinator integrates/accepts GAME-INTERACTION including this successor;
- client-visible typed payload/error registration is accepted under FND-02;
- every used object family has a stable target discriminator;
- each delegated owner has stable operation identity and retry/commit semantics;
- each coupled workflow has a named coordinator/recovery/error contract;
- GAME-ABILITY/movement/writable-text blockers are resolved for mechanics that need them;
- executable tests prove the scenarios/invariants below.

## 28. Future executable evidence

A future implementation task MUST provide at minimum:

- property/golden tests showing child order independence from randomized container insertion;
- sibling uniqueness + same-sibling duplicate/replay idempotency tests;
- nested cascade path-collision tests;
- crash/failover tests at child lifecycle boundaries;
- stale-generation/revision completion tests;
- deterministic RNG replay/no-reroll tests;
- dependency unavailable pre/post acceptance tests;
- timeout pre/post acceptance tests;
- cancellation race tests;
- FND-02 duplicate CommandRef/reconnect tests;
- DUR-03 ambiguous transaction integration where item/value is involved;
- coupled-workflow participant partial-success recovery;
- public error mapping/safe correlation tests;
- Foundation security-terminal no-downgrade tests.

Numeric limits/runtime mechanics remain owner-task work.

## 29. Governance terminal condition

The successor worker MUST stop after task/analysis/candidate delivery, draft PR, exact-head full-diff self-review and exact-head ordinary repository CI evaluation.

The worker MUST NOT:

- mark PR ready;
- trigger Codex/OpenAI/owner-funded independent review;
- merge/auto-merge;
- close issue #274 or predecessor #262;
- close/archive PR #269;
- archive task/release ownership;
- mutate coordinator-only global architecture surfaces.

Required handoff:

`INTEGRATION_READY — DRAFT PR — COORDINATOR ACTION REQUIRED`

`NEXT_ACTION: ARCHITECTURE_COORDINATOR_AUDIT`

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`  
`IMPLEMENTATION_AUTHORITY: NONE`
