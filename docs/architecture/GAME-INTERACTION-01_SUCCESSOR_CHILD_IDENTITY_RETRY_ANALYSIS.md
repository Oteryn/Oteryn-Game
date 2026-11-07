# GAME-INTERACTION-01 successor analysis — child occurrence identity and retry semantics

- Status: `PROPOSED / NONCANONICAL`
- Gate: `GAME-INTERACTION-01` (unchanged)
- Successor issue/task/PR: #274 / `OTV2-20260815-game-interaction-successor-r1` / draft #277
- Predecessor issue/PR: #262 / draft #269
- Predecessor reviewed head: `71253a8d5805ed37ec451e40e2c7200c38031a52`
- Base: `main@cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e`
- Mode: `PAPER-ONLY / CONTRACT`
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`
- Implementation authority: `NONE`

## 1. Why this is a successor

The predecessor task on #269 records `repair_cycles_for_current_gate: 3`. Its final coordinator reconciliation found two remaining material gaps only after that budget was exhausted. A further predecessor edit would therefore be an impermissible fourth repair cycle.

This successor keeps the same gate ID, `GAME-INTERACTION-01`, but uses a new issue/task/branch/draft PR and new owned paths. Predecessor #262/#269 remain read-only evidence and their lifecycle is not changed here.

Scope is limited to:

1. stable fan-out/cascade child-occurrence identity; and
2. complete cross-owner/client error, retry, ambiguity and reconciliation semantics.

## 2. Verified architecture constraints

### PROVEN — FND-02 already owns client-command identity

`docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` defines:

```text
CommandRef = (GameSessionId, CommandId)
```

Once a CommandId is reserved, a duplicate does not execute again. Pending/terminal command identity survives eligible same-GameSession reconnect. A fresh semantic command uses a new CommandRef only after the prior occurrence is terminal and a fresh attempt is permitted.

**Consequence:** GAME-INTERACTION must not create a competing global command/interaction UUID.

### PROVEN — SIM requires stable semantic RNG identity

`SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` requires retry-stable gameplay randomness derived from stable semantic occurrence + purpose, and forbids NodeId/thread/pointer/unordered-container position/transient generation as RNG identity.

**Consequence:** fan-out children need stable sibling identity before deterministic RNG can be correct.

### PROVEN — DUR-03 demonstrates same-attempt reconciliation

`DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` keeps one logical transaction across retry/ambiguous completion, rejects stale-generation completion from mutating current runtime, and reconciles committed/aborted/ambiguous outcome rather than issuing blind second value mutation.

**Consequence:** cross-owner interaction recovery must preserve the same foreign owner operation identity.

### PROVEN — Foundation error shape is stricter than a category label

`docs/contracts/FOUNDATION_ERROR_VOCABULARY.md` requires every public/cross-component error to define:

- stable category and contract-owned code;
- `RETRYABLE`, `TERMINAL` or `SECURITY_TERMINAL`;
- same/new command/session or owner-intervention rule;
- safe correlation;
- idempotency/partial-mutation outcome;
- bounded internal-to-public mapping.

Foundation also defines:

- `CANCELLED` as intentionally cancelled with documented cleanup state;
- `STALE_GENERATION` as a stale fence with no mutation committed by the stale application.

Those definitions matter for the ambiguity design below.

## 3. Finding 1 — root-only dedup is insufficient

One committed movement occurrence can contact several objects/edges:

```text
movement M
  -> plate A / ON_ENTER
  -> hazard B / ON_CONTACT
  -> trigger C / ON_ENTER
```

Using only `M` as dedup identity merges siblings. Generating a fresh UUID per delivery makes retry/recovery unable to prove sibling sameness. Appending a hash-map/vector index makes identity nondeterministic across replay.

Nested cascades add another collision: two distinct first-level children may both reach the same final target/edge. An identity containing only ultimate root + final target/edge can merge those paths.

## 4. Recommended identity model

Use a source-derived composite child reference:

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

For a cascaded child:

```text
ParentSourceOccurrenceRef = parent InteractionChildOccurrenceRef
```

This creates a bounded deterministic ancestry path, not a global UUID namespace.

### 4.1 Interaction definition/object identity

Must be stable authored/compiled semantic identity under the exact bound content revision. It must not be a pointer, loader slot, hash bucket or source-line accident.

### 4.2 Authoritative target discriminator

Must be resolved by the server and stable in the exact World/Channel/Instance scope. A client hint, UI ordering, pointer, transient container index or thread discovery order cannot be authoritative identity.

If the owner cannot supply a stable target discriminator, that interaction family is not implementation-ready.

### 4.3 Typed edge/capability discriminator

Same target may have semantically different `ON_ENTER`, `ON_LEAVE`, `ON_CONTACT`, `USE`, etc. Those edges need stable typed keys so they do not dedup-collapse.

### 4.4 Optional child ordinal

Absent by default. Allowed only for intentionally distinct semantic multiplicity that remains otherwise identical.

When used it must derive from explicit authored order or a canonical comparator over stable semantic fields. It must never be assigned from runtime/container/thread/DB discovery order. Accidental duplicates should fail content validation rather than receive invented ordinals.

### 4.5 Exact semantic revision context

Bind the exact behavior-affecting revision/artifact set required by the occurrence, such as World Bundle/content, interaction-definition, ruleset/world-policy, SIM and proposal-only script profile revisions where applicable.

Retry/replay remains bound to the same context or follows an explicit compatibility/reconciliation rule. It does not silently reinterpret an old child under newest content.

## 5. Critical separation — logical identity versus authority fences

Runtime ownership generation and mutable state/domain revisions remain fail-closed fences, but transient generation must not create a fresh logical child after failover.

```text
same logical child + newer valid owner generation
!= new gameplay occurrence
```

Correct model:

```text
semantic identity = InteractionChildOccurrenceRef
current mutation permission = current generation/revision/fence validation
```

This preserves both exactly-once semantics and stale-owner safety.

## 6. Deterministic child plan

Stable child keys are insufficient if recovery re-enumerates a different mutable world. Before any child can commit or any foreign owner can accept work, the owner needs a reproducible authoritative child plan equivalent to:

```text
root occurrence
+ exact semantic revision context
+ canonical ordered child refs
+ required precondition/fence evidence
```

Canonical order comes from authored semantic order, stable typed comparator, or explicitly retained accepted execution order when the owning contract makes it semantic. Hash-map/set/pointer/thread/worker/unspecified DB order is forbidden.

After partial progress, recovery must use the same accepted child set/order. It cannot add/drop/renumber siblings based on newer mutable state.

## 7. Child lifecycle

Semantic states equivalent to:

```text
UNSTARTED | PENDING | COMMITTED | REJECTED
```

are sufficient for architecture reasoning:

- `UNSTARTED` -> accepted child has not committed/delegated; may execute once when current fences permit;
- `PENDING` -> final state unresolved; reconcile same child and same owner operation;
- `COMMITTED` -> exactly-once semantic commit proven; never reapply;
- `REJECTED` -> old occurrence terminal; never reevaluate as the same occurrence.

Physical ledger/storage representation remains deliberately undecided.

## 8. RNG identity

For child-owned random decisions:

```text
RngDecisionRef = (
    InteractionChildOccurrenceRef,
    RngPurposeKey,
    DeterministicDrawOrdinal
)
```

SIM still owns the actual RNG algorithm/profile. The key requirement is semantic stability: retry, recovery, stale delivery and runtime ownership move cannot reroll the same child/purpose/draw.

## 9. Finding 2 — public operation state

Caller/cross-owner semantics need an explicit state:

```text
COMMITTED | PENDING | REJECTED
```

### `COMMITTED`

Declared semantic commit point is proven exactly once. In a coupled workflow, participant-local commit is not enough; only the named coordinator/final owner can prove whole-workflow commit.

### `PENDING`

Final outcome is unresolved. Caller must reconcile the same child and same foreign operation reference. A fresh same-intent CommandRef/source is forbidden while the old occurrence may still commit.

### `REJECTED`

The original occurrence is terminal and nothing accepted from it can later commit. A fresh permitted attempt is a new root occurrence; for a client this is a new CommandRef.

## 10. Partial/local mutation disclosure

For cross-owner pending outcomes where one participant may already have mutated, expose an owner-approved state equivalent to:

```text
NOT_COMMITTED | COMMITTED | UNKNOWN
```

Timeout, lost response, dependency outage or cancel request are never sufficient evidence for `NOT_COMMITTED` by themselves.

## 11. Retry authority model

Distinguish four actions:

```text
RECONCILE_SAME_OCCURRENCE
NEW_OCCURRENCE_ALLOWED
OWNER_INTERVENTION_REQUIRED
NO_RETRY
```

This makes `RETRYABLE` precise: it says whether the retry is same-attempt reconciliation, genuinely new action after a terminal outcome, or an owner recovery process.

## 12. Error semantics by required case

### 12.1 `DEPENDENCY_UNAVAILABLE`

**Before any foreign acceptance/local commit can be proven:**

```text
GI_DEPENDENCY_UNAVAILABLE_REJECTED
category = DEPENDENCY_UNAVAILABLE
progression = RETRYABLE
state = REJECTED
retry = NEW_OCCURRENCE_ALLOWED
local mutation = NOT_COMMITTED
```

Old CommandRef/source only replays the terminal rejection. Fresh attempt requires a new source/new CommandRef.

**After or ambiguously around acceptance:**

```text
GI_DEPENDENCY_UNAVAILABLE_PENDING
category = DEPENDENCY_UNAVAILABLE
progression = RETRYABLE
state = PENDING
retry = RECONCILE_SAME_OCCURRENCE
```

Same child + same OwnerOperationRef. No new same-intent CommandRef. Delegated owner/workflow coordinator owns final reconciliation.

### 12.2 `TIMEOUT`

Timeout is deadline observation, not rollback proof.

- proven timeout before acceptance/mutation -> `GI_TIMEOUT_REJECTED`, terminal old occurrence, fresh attempt gets new source;
- possible acceptance/commit -> `GI_TIMEOUT_PENDING`, same child/op reconciliation, no new same-intent source.

### 12.3 `CANCELLED`

Foundation says `CANCELLED` requires documented cleanup state. Therefore a cancel request must not be labelled `CANCELLED` before cleanup/retirement is proven.

- cleanup/retirement before semantic commit proven -> `GI_CANCELLED_REJECTED`, category `CANCELLED`, `TERMINAL`, `REJECTED`;
- commit won -> `COMMITTED`;
- ordering/ack still ambiguous -> `GI_CANCELLATION_RECONCILIATION_REQUIRED`, category `INTERNAL_UNAVAILABLE`, `RETRYABLE`, `PENDING`, same child/op.

### 12.4 Stale/delegated completion ambiguity

Two truths must be separated:

1. **stale completion application** — `GI_STALE_COMPLETION_REJECTED`, category `STALE_GENERATION`, terminal for that stale message/application, no mutation by stale application, no retry of stale message;
2. **underlying foreign operation** — if still ambiguous, child stays `PENDING` under `GI_RECONCILIATION_REQUIRED`, same child + same OwnerOperationRef, no new CommandRef/source, local/foreign mutation may be `COMMITTED` or `UNKNOWN` until owner truth is known.

This avoids violating Foundation's no-mutation semantics for `STALE_GENERATION` while refusing to pretend that stale delivery means the foreign operation rolled back.

### 12.5 Coupled workflow pending/recovery

If no narrower category is truthful:

```text
GI_COUPLED_WORKFLOW_RECONCILIATION_REQUIRED
category = INTERNAL_UNAVAILABLE
progression = RETRYABLE
state = PENDING
retry = RECONCILE_SAME_OCCURRENCE
```

The same workflow/participant operation refs are retained; participant local commit does not imply whole commit; only the named coordinator owns final classification.

## 13. Correlation requirements

As applicable and safe, a public/cross-owner result needs:

```text
root source occurrence / legal parent CommandRef
InteractionChildOccurrenceRef
World/Channel/Instance semantic scope
bound semantic revision reference
COMMITTED | PENDING | REJECTED
stable Foundation category
contract-owned symbolic code
OwnerOperationRef? when foreign work exists
LocalMutationState? when partial mutation is relevant
redacted diagnostic correlation? (never authority)
```

Credentials, secret RNG material, raw exceptions, stack traces and unstable implementation text remain internal.

## 14. Bounded internal-to-public mapping

Internal exception classes do not define caller behavior. Mapping uses semantic acceptance/commit evidence:

```text
service unavailable
  -> dependency REJECTED or PENDING

deadline
  -> timeout REJECTED or PENDING

cancel request
  -> CANCELLED only with cleanup proof
     otherwise cancellation reconciliation PENDING

stale completion
  -> stale completion application REJECTED
     + underlying child PENDING only if final operation outcome remains ambiguous
```

Foreign narrow codes remain foreign-owned when their retry/security/mutation meaning matters.

## 15. Foundation security/session cases

FND-02/FND-04/FND-04C own session, admission, generation and security-terminal semantics.

GAME-INTERACTION must not:

- downgrade `SECURITY_TERMINAL` to ordinary retry;
- invent replacement session codes;
- advise same command/session when Foundation requires a new authenticated path;
- treat a new GameSession as proof that old durable/foreign operations disappeared.

## 16. New CommandRef decision rule

For client-originated interaction:

- same CommandRef -> duplicate replay/reconciliation of the same reserved command;
- `PENDING` -> same parent CommandRef; new same-intent CommandRef forbidden;
- terminal `REJECTED` + fresh attempt permitted -> new CommandRef;
- prior `COMMITTED` + later distinct user action -> new CommandRef;
- session replacement follows Foundation, but durable/delegated ambiguity from the old command still requires reconciliation.

For movement/timer/event roots, physical redelivery of the same source occurrence reuses the same child refs; only a genuinely new authoritative root occurrence creates new children.

## 17. Foreign-owner dependencies

### DUR-03

DUR-03 owns TransactionId/OperationId, item/value linearization and durable reconciliation. GAME-INTERACTION only correlates the trigger child and boundedly maps result.

### GAME-ABILITY

GAME-ABILITY owns effect legality/formulas/effect operation semantics. Draft PR #268 is unmerged/blocked at this baseline, so final narrow effect operation/code integration remains implementation-blocked until coordinator acceptance of the owning GAME-ABILITY contract.

### Movement/relocation/handoff

Final owner contract remains unresolved by explicit scope. Relocation mechanics remain implementation-blocked until a named accepted contract defines operation identity, completion, timeout/cancel/stale and recovery.

### Durable writable text

Owner remains unresolved. Durable writable-text mechanics remain blocked until a named accepted owner contract exists.

### Client payload/error registration

FND-02 + future registered GAME-INTERACTION gameplay payload/schema must own the client-visible encoding/numeric registration. This successor defines semantics but does not modify FND-02 or allocate numeric IDs.

## 18. Named coupled workflows

Each coupled mechanic must explicitly define coordinator, participant operation refs, authority/fences, exact revisions, commit point, idempotency, timeout, cancellation, stale completion, crash recovery, legal compensation and bounded public mapping.

Generic `InteractionOutcomeState` is not a generic distributed transaction mechanism.

## 19. Reconciliation owner matrix

| Interaction family | Final reconciliation authority | Interaction responsibility |
|---|---|---|
| local interaction state | current FND-03 runtime owner | child dedup, fence validation, one terminal outcome |
| DUR-03 item/value | DUR-03 owner/coordinator | preserve child/source correlation; bounded map |
| GAME-ABILITY effect | accepted GAME-ABILITY owner | preserve trigger; do not own formulas/effect commit |
| movement/handoff | **BLOCKED** pending named owner | do not guess owner/retry semantics |
| named coupled workflow | workflow coordinator | do not advertise whole commit from participant local success |
| client CommandRef/session | FND-02/FND-04 | preserve same-vs-new command/session rules |

## 20. Deterministic acceptance scenarios

### GI-SR-01 — one movement occurrence, N contacts, partial delivery/retry/recovery

Given movement `M`, semantic context `R`, contacts `A/B/C` discovered in arbitrary physical order, canonical semantics derive distinct ordered `C1/C2/C3`.

1. `C1` commits.
2. `C2` is accepted as foreign operation `O2`; response is lost -> `PENDING`.
3. runtime fails before `C3` commits.
4. replacement owner recovers under newer ownership generation.
5. stale completion for `C2` arrives.

Required:

- same child plan under `R`;
- `C1` never reapplied;
- stale completion rejected before mutation;
- underlying `C2` reconciles same `O2` and never rerolls;
- `C3` executes at most once if current fences permit;
- sibling identity/order independent of hash/container/worker order;
- each child RNG decision remains `(child,purpose,draw)` stable;
- exactly one terminal outcome per child after reconciliation;
- replay reconstructs same plan/outcomes.

### GI-SR-01B — nested cascade path collision

Two different first-level children reach the same final target/edge. Their nested refs remain distinct because parent child refs differ. Identity truncated to ultimate root + final target/edge fails acceptance.

### GI-SR-02 — dependency pre-acceptance

Return terminal `GI_DEPENDENCY_UNAVAILABLE_REJECTED`; old CommandRef replay-only; fresh permitted attempt new CommandRef.

### GI-SR-03 — timeout post/ambiguous acceptance

Return `GI_TIMEOUT_PENDING`; same child + same OwnerOperationRef; new same-intent CommandRef forbidden until terminal proof.

### GI-SR-04 — cancellation race

Commit proof -> `COMMITTED`; cleanup-before-commit proof -> `GI_CANCELLED_REJECTED`; ambiguous ordering -> `GI_CANCELLATION_RECONCILIATION_REQUIRED/PENDING`.

### GI-SR-05 — stale completion

Stale message -> `GI_STALE_COMPLETION_REJECTED`, no mutation. Underlying ambiguous operation -> `GI_RECONCILIATION_REQUIRED/PENDING`, same child/op, no fresh source/RNG.

### GI-SR-06 — coupled partial success

Participant local mutation may be committed but whole state remains `PENDING`; only named coordinator may produce whole terminal state.

## 21. Decision timing

These semantics must be decided before implementation because they block safe dedup, deterministic RNG, failover recovery, caller retry and cross-owner adapter contracts.

Deferring them risks persisted unstable identities or ambiguous APIs capable of duplicate effects/value.

Physical hash/serialization/storage, numeric resource limits, runtime topology, formulas and foreign-owner internals remain deliberately undecided.

## 22. Preserved predecessor invariants

No change to:

- server target authority;
- typed state machines;
- explicit scope/lifetime;
- GAME-ITEM/DUR-03 value authority;
- GAME-ABILITY effect authority;
- movement/handoff delegation;
- named coupled workflows;
- bounded cascades;
- DUR-04 proposal-only scripts;
- deterministic revisions/order/RNG;
- prohibition on generic mutable global interaction scope.

## 23. Cross-domain findings

`NONE NEW`.

Inherited blockers remain explicit:

- GAME-ABILITY whole-gate integration is noncanonical while #268 remains unmerged/blocked;
- movement/handoff owner is unresolved;
- durable writable-text owner is unresolved.

## 24. Decisions not taken

- no new global gate ID;
- no generic global InteractionId/UUID;
- no generic global OperationId;
- no movement/handoff owner;
- no writable-text owner;
- no GAME-ABILITY formulas;
- no numeric limits;
- no runtime/DDL/storage schema;
- no Platform/production changes;
- no coordinator-only global overlay edits.

## 25. Recommended disposition

**RECOMMENDATION:** coordinator acceptance should require that the integrated GAME-INTERACTION contract preserves:

1. recursive source-derived child/path identity;
2. exact semantic revision binding plus separate current authority fences;
3. deterministic canonical child plan/order and RNG identity;
4. explicit `COMMITTED/PENDING/REJECTED` caller state;
5. same-occurrence reconciliation for ambiguity and new source only after terminal proof;
6. strict Foundation `CANCELLED` and `STALE_GENERATION` meanings;
7. named foreign reconciliation owners and explicit blockers instead of guessed codes.

`NEXT_ACTION: ARCHITECTURE_COORDINATOR_AUDIT`
