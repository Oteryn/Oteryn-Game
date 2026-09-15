# VSL-MOVE-01 — Minimal Movement, Collision and Visibility Contract Candidate

- Date: 2026-08-16
- Gate: `VSL-MOVE-01`
- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Scope: first real-boundary movement/collision/visibility vertical slice only
- Runtime/client/protocol/content/DDL/Platform/production authority: **NONE**
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Problem

The accepted foundation already defines runtime ownership, command ordering, content revisions and client reconciliation, but the first movement implementation still needs one explicit domain contract for:

- who owns authoritative actor position and same-scope relocation;
- what one movement occurrence means;
- how static collision and dynamic occupancy are evaluated;
- how movement-triggered interactions and teleports compose without creating two position writers;
- how visibility/interest state is derived and reconciled to the client;
- how retry, reconnect, stale work and revision changes remain safe.

Without this gate an implementation could accidentally make client coordinates, a physics/path worker, GAME-INTERACTION, or a renderer-side projection a second movement authority.

## 2. Accepted constraints

This candidate consumes without replacing:

- FND-02 `CommandRef`, per-session order, connection-generation fencing, server sequence, state-domain revision and snapshot/delta/resync semantics;
- FND-03 one current `ChannelRuntime` / `InstanceRuntime` owner and non-interleaved authoritative resolution boundary;
- FND-04 GameSession/CharacterLease/admission/recovery authority;
- DUR-04 immutable exact World Bundle/content/map/ruleset revision binding;
- SIM deterministic ordering, numeric semantics and revision-bound retry;
- GAME-INTERACTION child-occurrence/reconciliation semantics;
- GAME-AI proposal-only pathfinding/intent semantics;
- GAME-CHANNEL same-Channel versus Channel-switch distinction;
- ALPHA-CLIENT non-authoritative client projection and semantic-input boundary.

## 3. Authoritative movement owner

For the first slice, **authoritative local position and same-scope relocation are owned by the current FND-03 runtime owner**:

```text
public Channel scope -> current ChannelRuntime
instance scope       -> current InstanceRuntime
```

`VSL-MOVE-01` names a logical **Movement domain role inside that current owner**. This is not a new process/service/global authority and does not change ADR-0009.

Only the current runtime owner may commit:

- actor authoritative position/floor;
- same-scope local relocation;
- dynamic occupancy effects caused by that position change;
- the authoritative spatial revision used by downstream interest/reconciliation.

The client, pathfinding worker, GAME-AI, GAME-INTERACTION, renderer, persistence callback and World Bundle loader may supply intent/facts/proposals but cannot directly commit actor position.

## 4. Three movement classes

The contract distinguishes:

### `LOCAL_STEP`

A normal movement occurrence that remains within the same current `ChannelRuntime` or `InstanceRuntime` authority scope.

### `LOCAL_RELOCATION`

A movement-owner committed relocation within the same current authority scope, for example a simple authored teleport/link or a validated forced relocation. The source may be a player command, accepted interaction child, AI/gameplay occurrence or system-owned occurrence.

### `SCOPE_HANDOFF`

Movement that changes authoritative runtime scope, including Channel↔Instance or different-Channel authority transitions.

`SCOPE_HANDOFF` is **not** implemented as a local teleport and is outside the first movement slice. It uses accepted FND/Channel/handoff authority transition semantics. A source runtime must not keep mutating the actor after the committed handoff fence.

This separation prevents a convenient teleport implementation from becoming a hidden cross-owner migration protocol.

## 5. Movement occurrence identity

VSL-MOVE does not introduce a mandatory globally allocated `MovementId`.

One logical movement occurrence is identified by an accepted authoritative source occurrence plus an explicit semantic movement discriminator and exact behavior-affecting revision context, for example:

```text
MovementOccurrenceRef = (
  SourceOccurrenceRef,
  MovementSemanticKind,
  ActorSemanticRef,
  SemanticRevisionContext,
  OptionalCanonicalSubOccurrence
)
```

Examples of `SourceOccurrenceRef`:

- client-originated movement -> FND-02 `CommandRef`;
- GAME-INTERACTION-triggered relocation -> accepted `InteractionChildOccurrenceRef`;
- AI/environment/system movement -> stable owner-defined event/timer/operation occurrence.

A transient runtime generation, worker ID, pointer, packet or wall-clock timestamp is not logical movement identity.

If one source intentionally causes multiple distinct movements, the owning mechanic must provide a canonical semantic child/sub-occurrence discriminator. Physical enumeration order must never invent identity.

## 6. Revision binding

Every movement occurrence binds the exact semantics needed to reproduce legality and result, including as applicable:

- World Bundle/content revision;
- map/navigation/collision revision;
- ruleset/world-policy revision;
- SIM determinism profile revision;
- applicable zone/interaction definition revision;
- actor movement-capability revision.

Retry/recovery of the same occurrence cannot silently re-evaluate against an incompatible newer map/ruleset revision. It completes under a compatible bound context or fails/reconciles explicitly.

## 7. Authoritative movement resolution

A movement occurrence is resolved within one non-interleaved FND-03 owner input through semantics equivalent to:

```text
current owner + ownership generation
+ source occurrence
+ exact bound revisions
+ current authoritative actor spatial state
+ normalized semantic movement intent/proposal
-> bounded deterministic legality evaluation
-> one owner-local spatial commit OR typed rejection
-> derived post-commit interaction/interest facts
```

Required order:

1. validate current semantic scope and ownership generation;
2. validate current GameSession/CharacterLease/principal where source requires it;
3. validate source occurrence/idempotency/order;
4. bind exact behavior-affecting revisions;
5. normalize movement request into one bounded semantic candidate operation;
6. evaluate static collision/spatial metadata from the bound immutable World Bundle;
7. evaluate current dynamic occupancy/actor/state constraints from the authoritative runtime state;
8. apply applicable accepted zone/movement capability rules;
9. deterministically accept one exact destination or reject;
10. commit authoritative local spatial mutation exactly once;
11. derive post-commit interest/interaction observations and typed outcome.

A movement rejected before commit changes no authoritative position.

## 8. Client request boundary

The client emits semantic movement intent only.

The client MUST NOT author:

- accepted destination coordinates;
- collision truth;
- floor/teleport legality;
- movement speed/cooldown eligibility;
- current position revision;
- visibility membership;
- authoritative interpolation/path state.

For a directional/step command, the server derives the exact candidate from authoritative current position and bound movement rules.

A future click-to-move/path intent may contain a bounded goal/hint but cannot turn a client-generated route into authority. Route/path semantics require their own accepted implementation profile.

## 9. Static and dynamic legality

Movement legality is the composition of typed facts, not one mutable collision callback.

### Static facts

From the exact bound World Bundle, as applicable:

- tile/cell existence and coordinate bounds;
- floor/vertical-link semantics;
- static blocking/terrain capability metadata;
- authored pure local relocation links;
- zone/policy references;
- spatial indexes needed for deterministic queries.

### Dynamic facts

From the current runtime owner, as applicable:

- actor occupancy/blocking;
- current dynamic object/overlay state;
- current target actor state;
- current accepted interaction/world-state facts;
- current ownership/fencing state.

No mutable worker-owned navigation cache can override current authoritative facts.

## 10. Interaction boundary

GAME-INTERACTION owns interaction child identity/retry/reconciliation; VSL-MOVE owns final local position commit.

The first slice supports two explicit interaction patterns.

### 10.1 Post-movement trigger

A movement commits first, then deterministic authored trigger eligibility may create an `InteractionChildOccurrenceRef` derived from the committed movement occurrence.

Examples: pressure/contact/entry triggers whose semantics do not need to retroactively make the movement invalid.

Retry/replay of the movement cannot create a second logical interaction child.

### 10.2 Pure local relocation edge

A static, immutable same-scope relocation link whose effect is only local spatial relocation MAY be interpreted as part of the Movement domain's bound spatial topology when its exact definition/revision is in the immutable World Bundle and it needs no foreign mutable side effect.

The movement owner validates both source and destination and commits one local relocation outcome.

### 10.3 Stateful relocation/workflow

A door, switch, script or interaction whose state mutation must happen before relocation is **not** silently folded into collision logic.

It requires an accepted named GAME-INTERACTION workflow and produces a movement proposal/occurrence only after the owning interaction state reaches the required accepted outcome. The first slice does not invent cross-owner atomicity for this class.

## 11. Local relocation chain safety

Repeated authored local relocations/teleport edges cannot recurse without a hard bound.

Before executable acceptance, the Resource Limits Registry must define a finite local-relocation/trigger descendant depth/work ceiling and deterministic terminal behavior.

On exhaustion the occurrence fails/reconciles according to the owning mechanic; it never loops until success or silently truncates a required semantic chain.

## 12. Movement timing and speed

This contract does **not** freeze Global Tibia speed/step-delay/diagonal/terrain formula values.

Authoritative eligibility for a movement occurrence must consume an explicit versioned movement timing/ruleset policy when such timing is exercised. SIM owns numeric/rounding semantics.

For structural VSL/E2E proof while Reference rules are still `UNKNOWN/PENDING`, tests MAY use an explicit **non-shipping VSL fixture movement profile** with deterministic bounded values. Such a fixture:

- is test/evidence only;
- is not a Reference rule;
- cannot ship as Reference behavior;
- cannot be promoted to product policy without owner/evidence acceptance.

## 13. Spatial revision and visibility/interest authority

Authoritative visibility is derived from committed server spatial/world state, not maintained as a peer client truth.

The runtime owner must expose a versioned spatial/visibility state domain compatible with FND-02's state-revision model. Exact wire field/message registration remains FND-02 gameplay payload work.

A committed movement may change:

- actor position;
- entities/tiles entering or leaving the observing character's interest set;
- visible state of already interested entities;
- zone/presentation facts that are part of the authoritative observation projection.

Interest computation must be bounded and deterministic for the bound revision/profile. Hash/set iteration, worker completion order or database row order cannot decide membership/order.

## 14. Client reconciliation

Movement/visibility delivery uses FND-02 unchanged:

- post-admission frames carry the current non-zero connection generation;
- authoritative messages participate in server sequencing as registered;
- a state delta applies only to the expected base/state revision;
- stale-generation frames never mutate the active client projection;
- duplicate/old server sequence is never applied twice;
- sequence/revision gap suspends affected state application and triggers bounded resync;
- incomplete replacement snapshot cannot partially become the active baseline.

The client may interpolate/present locally but must reconcile to the server projection and never advance authoritative state revisions itself.

## 15. Visibility details deliberately deferred

This contract freezes the authority/reconciliation shape, not exact Reference visibility geometry.

Still evidence/profile-owned:

- exact view rectangle/range;
- floor visibility rules;
- line-of-sight/occlusion formulas;
- diagonal/edge visibility details;
- invisibility/stealth/spectator policies;
- exact delta packing strategy.

The VSL fixture profile may use bounded explicit test values without claiming Reference parity.

## 16. Failure and retry semantics

| Condition | Required result |
|---|---|
| duplicate client CommandRef | FND-02 replay/pending reconciliation; no second movement commit |
| stale connection generation | reject/discard before authority |
| stale runtime ownership generation | no mutation; current owner/recovery reconciles |
| source position/revision mismatch | typed rejection/resync; never teleport client to guessed state |
| static collision blocked | terminal movement rejection for that occurrence |
| dynamic occupancy changed before commit | current-owner legality decides; no stale worker result authority |
| incompatible content/map/ruleset revision | fail/reconcile explicitly; no newest-revision reinterpretation |
| local relocation chain exceeds bound | deterministic bounded failure; no recursive loop |
| visibility result exceeds registered bound | bounded snapshot/resync/degradation policy; no unbounded allocation |
| client state revision gap | resync; no speculative authoritative delta |
| owner fenced after async proposal | old proposal discarded/revalidated by current owner |

## 17. Resource-limit dimensions

Before implementation acceptance, concrete hard ceilings and boundary tests must exist for applicable:

1. reserved/pending movement commands per GameSession (including inherited FND-02 limit);
2. movement inputs per owner work cycle where externally amplifiable;
3. static/dynamic spatial candidates examined per movement decision;
4. occupancy/query result count;
5. post-movement interaction descendants;
6. local relocation/teleport chain depth/work;
7. interest enter/leave/update entity count per delta;
8. visibility/spatial query candidate/result count;
9. snapshot entity/count/bytes and chunk counts inherited/extended from FND-02;
10. queued/in-flight auxiliary path/spatial proposals where used;
11. diagnostic evidence volume for adversarial movement spam.

Missing required bounds block executable acceptance; this paper contract invents no numbers.

## 18. Minimum first-slice scenarios

A conforming executable slice must prove at least:

1. admitted client -> semantic movement command -> authoritative local step -> client observes updated position;
2. static collision rejection leaves position unchanged;
3. dynamic occupancy change rejects/re-evaluates stale proposal safely;
4. duplicate CommandRef never moves twice;
5. stale connection/runtime generation cannot move an actor;
6. committed movement creates one deterministic post-movement interaction child where configured;
7. pure same-scope relocation/teleport commits one authoritative result without scope handoff;
8. state-revision or server-sequence gap triggers resync rather than guessed application;
9. replacement snapshot atomically restores client observation after reconnect/resync;
10. movement under intentionally shuffled backing collection order yields the same normalized result;
11. map/content revision incompatibility fails closed;
12. no client/UI/render state can directly commit server position.

Tier 1 must traverse the production protocol/server path. Tier 2 must exercise the native client's normal semantic input/projection path. A direct runtime mutation test is useful component evidence but not terminal product-boundary proof.

## 19. Explicit non-decisions

`DECISIONS_NOT_TAKEN`:

- exact Global movement speed/step timing/diagonal formulas;
- exact Reference visibility/LOS/floor rules;
- client prediction/rollback algorithm;
- click-to-move path algorithm;
- pathfinding library;
- renderer interpolation/camera technology;
- physical Rust types/module boundaries;
- numeric resource limits;
- cross-Channel/Instance handoff protocol beyond accepted parent authority;
- stateful door/scripted relocation workflow details;
- wire message IDs/field encoding;
- database schema;
- production capacity/scaling.

## 20. Decision timing

- **Must decide now?** `YES` for movement owner, occurrence identity, commit/retry/revision boundary, local relocation classification, interaction ownership and visibility/reconciliation authority.
- **Concrete downstream blocked:** movement runtime/client vertical slice, environment-trigger integration, safe AI movement adoption and protocol state projection.
- **Harder later:** client coordinates, navigation workers or interaction callbacks could become competing writers; changing movement identity/retry after content/protocol exists would create duplicate/teleport/replay bugs.
- **Superseding evidence:** representative movement/content cannot be expressed without pathological workflows; measured performance proves the owner-local model unacceptable; a later accepted cross-scope movement/handoff contract requires a stronger equivalent abstraction.
- **Deliberately not decided:** all algorithmic/formula/technology/numeric choices listed above.

## 21. Recommendation

`RECOMMENDATION: ACCEPT` this minimum movement/visibility architecture for the first vertical slice.

Acceptance would authorize **architecture only**. Implementation remains separately owner-authorized and Reference behavior remains evidence-gated.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`
