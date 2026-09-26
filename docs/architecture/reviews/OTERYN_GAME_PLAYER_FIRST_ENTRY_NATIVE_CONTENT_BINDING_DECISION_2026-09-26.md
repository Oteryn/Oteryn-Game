# Player first-entry authority and native Content binding

- Decision: `PLAYER_FIRST_ENTRY_NATIVE_CONTENT_BINDING_V1`
- Date: 2026-09-26
- Repository: Oteryn/Oteryn-Game
- Source: #930; allocation/control plane: #162
- Admission main: 91fb3135a8a67d012bfe048b230f3501780d244b
- Owner direction: **ACCEPTED / LIVE**, [#930 comment 5845304217](https://github.com/Oteryn/Oteryn-Game/issues/930#issuecomment-5845304217)
- Document status at authoring: **CANDIDATE**, pending exact-head validation, independent review and protected integration.
- Implementation/source at the admission baseline: no qualified native artifact or production activation issuer was proven.
- Live deployment authority: NONE; actor registry delta: []

This decision records the owner-accepted direction. Protected integration supplies bounded source and first-entry architecture only; it does not establish qualified artifact bytes, current activation, implementation completion, a writer lease or live deployment authority.

## Problem and verified baseline

At admission main `91fb3135a8a67d012bfe048b230f3501780d244b`, durable fresh
admission and committed Channel actor slots exist, but the accessible initial-position
method is test-only and uses synthetic markers. First-production compiler/staging APIs
exist; the only `AuthorizedContentGeneration` implementation is test-only. No qualified
native entry-room artifact, canonical runtime WorldId binding or current activation
proof is supplied by that baseline. Declared readiness revision strings are not active
Content evidence. These are bounded source observations, not a claim about private
external releases.

## Selected route

Use an authored native entry-room v1 under unchanged FIRST_PRODUCTION_CONTENT_PROFILE/v1
solely in PREPRODUCTION_FIRST_SLICE. A Game-owned first-entry source selects one
qualified cell from its exact active generation. Character supplies current
owner/world/lifecycle; Content supplies immutable geometry and provenance; only
the independently current ChannelRuntime writes position.

Eligibility is FIRST_ADMISSION_OF_UNPOSITIONED_ACTOR. An existing positioned actor
cannot be reset. Same-GameSession reconnect and recovery of a present actor preserve
its position. Respawn, broader fresh re-entry and Character-location persistence
remain separate accepted requirements before their activation.

## Options and trade-offs

| Route | Benefit | Cost and disposition |
| --- | --- | --- |
| A: qualified native Content start, Game entry issuer, Channel write | Produces the first controlled actor through existing ownership and profile boundaries. | Requires genuine source qualification and an explicit current activation operator. Selected by the owner. |
| B: persisted Character location as the initial-position source | Can support later durable re-entry or respawn. | Requires accepted location persistence, recovery and migration semantics absent from this bounded prerequisite. Deferred. |

Route A minimizes time to a real step/return/blocked-cell proof while preserving
progress on reconnect. Incorrect source activation or stale authority would expose
players to position resets, inconsistent collision or unauthorized control; the
current-pin and no-write rules below are mandatory. Operators must qualify an actual
release and establish quiescence. This decision does not promise restart continuity.

## Authored source

| Cell label | Native local coordinates | Classification |
| --- | --- | --- |
| start | (0, 0, 0) | Walkable; sole first-entry location |
| east | (1, 0, 0) | Walkable |
| north | (0, -1, 0) | Blocked |

The coordinates are authored map choices, not default/fallback positions or resource
maxima. Exact logical keys must satisfy the unchanged owning profile. No Reference
equivalence is asserted. Resolve a genuine current Game-owned WorldId assignment
and an explicit native frame; do not invent their readiness.

The existing profile requires >=3 cells and a complete bounded Region/Area/Terrain,
relocation, behavior, presentation, creature/spawn, formula/effect/ability and
item/loot/XP/RNG graph. Every required record needs a real source declaration and
applicable accepted policy/formula reference. This decision selects no new formula
or activation of those unrelated gameplay pipelines.

Required outputs are actual source-manifest bytes/licensing/digest, qualified
package/exact Content Lock, revision/profile/compiler/canonicalization context,
and deterministic server/client bytes and digests. None exists merely because a
fixture compiled. Test sources and placeholder provenance cannot be promoted.

## Activation issuer and current pin

A separate typed Game-owned activation issuer outside Content authority is restricted
to PREPRODUCTION_FIRST_SLICE. It binds exact package/generation, server/client
digests, World/revision context, applicability, decision identity, monotonic activation
sequence, expected-current or explicit empty-start, and current control/Node binding.
The accepted sequence must be strictly newer than its applicable current floor;
missing continuity is refusal, not permission to reset that floor.

Use the accepted trusted operator-file mechanics where suitable, with a distinct
typed purpose. LaunchAuthorizationFile alone has no Content activation purpose.
No new signing topology, CDN or general authority framework is selected.

Hold new admission, establish real quiescence, recheck authorization/expected-current,
then atomically activate one complete validated pair before authoritative Channel
creation. The Channel pins that exact active generation. Staging or cached receipts
are not a current activation pin. Existing live-scope hot-reload prohibition applies.

## Position write and retry

Order: proven durable fresh COMMIT -> exact runtime actor-slot COMMIT -> independent
first-entry revalidation -> one Channel-owned position initialization -> input eligibility.

The typed issued location binds the distinct `WorldId` and `ChannelId`, runtime scope
ownership generation, native coordinate frame, map revision and exact active Content
generation. Content attests that the selected cell exists and is qualified walkable;
Character independently attests current owner/world/lifecycle. Neither Content nor
transport acquires Channel position-write authority.

Immediately before writing, resolve current Character owner/world/lifecycle/revision,
GameSession/connection generation, CharacterLease generation, runtime scope assignment
and ownership generation, exact actor identity/local generation, committed-unpositioned
eligibility, active Content pin, map/frame and qualified walkable start.

Immutable receipts describe expected bindings; they do not reconstruct current
authority. Async completions re-enter current checks before position/control grants.

Exact retry reconciles the same completed initialization without another write.
Changed/stale/missing/recycled/reassigned/wrong-scope bindings receive no write.
An already positioned actor cannot receive replacement initialization. Movement
after initialization is not undone by retry. A recycled local ID inherits no prior
disposition. Retain only the bounded binding/disposition needed for that generation.

A post-COMMIT failure does not prove durable noncommit or terminality and cannot
fabricate rollback or premature slot release. Socket close alone is not control loss.

## Restart

After restart, Content is not-ready with no active generation. Reactivation requires
current exact authorization and a valid monotonic floor. Actor continuation requires
independently fenced placement/presence/session continuity evidence.

If any of that evidence is missing or ambiguous, refuse/remain not-ready. Do not guess
a position, recreate a present actor, reset activation ordering, renew session/grace
or protection deadlines. This slice claims no uninterrupted failover persistence.

## Resources and separate gates

Protected #912 remains binding: actor registry delta=[]; 131072 is only an explicit
preproduction bound, not a production default/capacity/readiness claim.

Reuse current first-production graph/cell/span/key/manifest/provenance/artifact and
generation-pair limits including Amendments01-03. MOVE-RL-03=1 applies only to its
isolated decision. No new hard maximum is selected here.

Classify actual added variable-size initialization/authorization retention and lookup
resources before execution; reuse an applicable accepted limit or retain an explicit
qualification prerequisite. #139 whole-cycle/retained input/fairness acceptance,
#642 wire/schema/stable IDs, measured liveness, sealed current Recovery, ControlLoss
and positive ClientResume remain separate gates. Full Reference and live deployment
are not accepted through this direction.

## Decision test

- Decide now: YES. First controlled actor and positive #822 proof lack an entry issuer
  and active Content source.
- Downstream work blocked: the first true playable-control owner cycle, its sealed
  ControlLossEpoch prerequisite and positive ClientResume evidence for #822.
- What becomes harder later: native first-entry bindings will need explicit migration
  if durable re-entry/respawn replaces them. Avoid client-authored start, login reset, fixture promotion,
  native/Reference conflation, receipt-derived current authority and invented restart.
- Supersession evidence: accepted durable re-entry/respawn/runtime continuity or proven
  insufficiency of this bounded native profile, retaining all authority/fencing rules.
- Unselected: broad import, general spawn search, public IDs, bundle format, production
  capacity/deployment, cross-scope relocation and Character-position schema.

## Qualification and integration

Freeze one complete bounded architecture candidate, verify owned delta, pass applicable
governance/architecture/Merge Gate, independent exact-head architecture/security review,
material finding dispositions, governed MQ, real merge_group game-gate and protected
readback before treating the document as protected execution authority.

This docs-only delivery changes no executable consumer boundary; runtime E2E is NOT_APPLICABLE to the document candidate. Its future implementation must prove actual authored source/digest/package/lock, deterministic
ordinary-release bytes, independent loaded geometry/collision, start and step/return/
blocked refusal, wrong package/digest/world/map/frame/generation refusal, actual hold/
quiescence/expected-current/sequence, restart not-ready, first initialization/idempotent
retry and independently changed Character/session/lease/scope/actor/Content no-write.
Use AuthorityInvariant × ConsumerBoundary × MutationOperator; no record-derived
negative current-authority oracle.

Use `.github/workflows/agent-governance.yml`, `.github/workflows/architecture-semantic-audit.yml` and `.github/workflows/merge-gate.yml` for this document candidate. Future implementation reuses existing rust.yml, content_first_production tests, node-boot-qualification.yml
with tools/qualification/node_boot/run.sh, gameplay-server-seam.yml with
WP5_QUALIFICATION=seam bash tools/qualification/wp5_s3b/run.sh, and protected gates.
Prior route success is route evidence, not proof of this new candidate/reconnect.

Expected ownership: Content production/activation/source, Node operator/config/serve,
small Game entry module, runtime_actor_carrier and necessary gameplay composition/tests.
The sole #162 control plane must assign exact paths and prove authoring/validation/
integration capability before release. No listed path is implicitly leased.

## Authority and supersession

Owner acceptance is live and applies to this direction. Candidate-specific validation
and independent review apply after freeze; the architecture becomes protected only
through the normal qualification, owner-performed Merge Queue admission, successful
merge_group aggregate game-gate and protected-main readback. Until then the document
is a candidate. After that readback, a fresh #162 implementation allocation may consume
this architecture within its own paths and prerequisites.

This changes only #930 first-entry/source direction. It does not supersede Character/FND
fences, ChannelRuntime's sole position ownership, first-production profile safety,
D1 first-production/Reference profile separation, #912 actor registry prohibition,
FND-04B continuity, #139/#642/liveness gates or live-environment authority boundaries.
Nothing here creates automatic merge, production or cross-repository authority.

## Admission-baseline disposition

The facts below describe implementation at the named admission baseline. Current PR,
check and protected-main evidence governs document delivery; these baseline observations
are not a permanent claim that this decision can never become accepted.

```text
OWNER_DIRECTION=ACCEPTED_LIVE
DOCUMENT_DELIVERY=FOLLOW_EXACT_HEAD_PR_REVIEW_MQ_AND_PROTECTED_READBACK
QUALIFIED_NATIVE_SOURCE=NOT_YET_PROVEN
CURRENT_ACTIVATION_ISSUER=NOT_YET_IMPLEMENTED
FIRST_CONTROLLED_ACTOR=NOT_YET_QUALIFIED
ACTOR_REGISTRY_DELTA=[]
SERVER_SEAM_822=NOT_COMPLETED
```

## Implementation placement

A later exclusive allocation may select the minimum necessary parts of Content
production/activation/source, Node operator/config/serve, a small Game entry module,
`foundation/runtime_actor_carrier.rs`, Movement and gameplay composition/tests.
These are prospective boundaries, not current leases. No public codec, schema,
registry, runtime or source-data mutation is authorized by this docs-only task.

## Source references

- [Owner acceptance #930](https://github.com/Oteryn/Oteryn-Game/issues/930#issuecomment-5845304217)
- [Control-plane readback #162](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5845309266)
- docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md
- docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_01.md
- docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_02.md
- docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_03.md
- docs/architecture/VSL-MOVE-01_MINIMAL_MOVEMENT_VISIBILITY_CONTRACT_CANDIDATE.md
- docs/architecture/reviews/OTERYN_GAME_RUNTIME_ACTOR_FIRST_SLICE_BOUND_APPLICATION_DECISION_2026-09-25.md
- docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
- docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md
- docs/architecture/OTERYN_PRODUCT_PROFILE_REFERENCE_TARGET_RECONCILIATION_2026-09-09.md
- docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md
