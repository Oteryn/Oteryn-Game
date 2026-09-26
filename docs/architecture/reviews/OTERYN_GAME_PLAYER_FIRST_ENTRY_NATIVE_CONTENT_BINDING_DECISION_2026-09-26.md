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
and qualify the source-bound native frame below; neither readiness nor a frame supplied
independently of the qualified source may be inferred.

The existing profile requires >=3 cells and a complete bounded Region/Area/Terrain,
relocation, behavior, presentation, creature/spawn, formula/effect/ability and
item/loot/XP/RNG graph. Every required record needs a real source declaration and
applicable accepted policy/formula reference. This decision selects no new formula
or activation of those unrelated gameplay pipelines.

Required outputs are actual source-manifest bytes/licensing/digest, qualified
package/exact Content Lock, revision/profile/compiler/canonicalization context,
and deterministic server/client bytes and digests. None exists merely because a
fixture compiled. Test sources and placeholder provenance cannot be promoted.

### Qualified native frame prerequisite

Use the accepted `oteryn-world-spatial-v1` semantic revision 1. Its native tile axes,
floor ordering and authored envelope rules are binding; this decision creates no new
coordinate profile or public frame identity. The actual native source/manifest must
declare its authored frame identity, axes, origin, finite bounds and declared floor,
consistent with that contract and the exact World/map revision and three source cells.
Floor 0 and origin (0, 0) are authored choices, not implicit surface/default meanings.

Qualification must prove the declaration from the genuine immutable source/manifest
bytes. Recompute their existing source-manifest digest and verify its binding through
the existing package provenance, Content Lock and exact staged generation. Retain a
typed qualified binding connecting that declaration to exact World/map/profile/package
context, manifest/lock provenance and the final server/client artifact digest pair.
This is an evidence chain; source-manifest bytes need not contain their own digest or
the downstream artifact digests. Copied GenerationIdentity fields, an operator's frame
assertion or a wrapper containing an independently supplied frame do not qualify it.
Raw source-manifest payloads remain excluded from the runtime artifact/profile under
Amendment 02. Source/frame qualification is producer/release evidence; activation and
restart may consume only the bounded binding established by that genuine proof under
the Game-owned boundary. This selects no runtime source parser or provenance fetch.

At the admission baseline, FirstProductionContentSource and GenerationIdentity have
no coordinate-frame identity field. Global spatial semantics and engineering frame
markers alone therefore do not establish that a named frame belongs to this generation.
The required source-to-frame qualification is NOT_YET_PROVEN.

No first-production artifact manifest/format, source parser, profile capability or
public identity expansion is selected here. If the real source/manifest cannot encode
and prove this binding within accepted source-representation and owning-profile
boundaries, implementation remains
fail-closed pending a separately accepted bounded amendment. No position write may
precede that evidence; wrapping absent provenance cannot satisfy the prerequisite.

## Activation issuer and current pin

A separate typed Game-owned activation issuer outside Content authority is restricted
to PREPRODUCTION_FIRST_SLICE. It binds exact package/generation, server/client
digests, World/revision context, applicability, decision identity, monotonic activation
sequence, expected-current or explicit empty-start, and current control/Node binding.
It must also validate and carry the same source-qualified native frame binding; exact
artifact identity alone cannot substitute for that proof.
The accepted sequence must be strictly newer than its applicable current floor;
missing continuity is refusal, not permission to reset that floor.

Use the accepted trusted operator-file mechanics where suitable, with a distinct
typed purpose. LaunchAuthorizationFile alone has no Content activation purpose.
No new signing topology, CDN or general authority framework is selected.

Hold new admission, establish real quiescence, recheck authorization/expected-current,
then atomically activate one complete validated pair before authoritative Channel
creation. The Channel pins that exact active generation. Staging or cached receipts
are not a current activation pin. Existing live-scope hot-reload prohibition applies.
The committed activation and Channel pin must preserve the same qualified frame
binding together with the generation. A caller-supplied frame cannot be attached or
replaced independently, even when World/map/artifact digests and coordinates match.

## Position write and retry

Order: proven durable fresh COMMIT -> exact runtime actor-slot COMMIT -> independent
first-entry revalidation -> one Channel-owned position initialization -> input eligibility.

The typed issued location binds the distinct `WorldId` and `ChannelId`, runtime scope
ownership generation, native coordinate frame, map revision and exact active Content
generation. Content attests that the selected cell exists and is qualified walkable;
Character independently attests current owner/world/lifecycle. Neither Content nor
transport acquires Channel position-write authority.
The location's frame is obtained from that source-qualified, currently pinned binding,
not from an independent request/operator argument. Current pin equality includes the
qualified frame binding before initialization or later Movement eligibility.

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
Retry also binds the same qualified frame; it cannot reconcile a missing or substituted
frame by copying the expected identity from its prior receipt.

A post-COMMIT failure does not prove durable noncommit or terminality and cannot
fabricate rollback or premature slot release. Socket close alone is not control loss.

## Restart

After restart, Content is not-ready with no active generation. Reactivation requires
current exact authorization and a valid monotonic floor. Actor continuation requires
independently fenced placement/presence/session continuity evidence.
Restore/requalify the native frame from its genuine immutable source/manifest evidence
and exact artifact generation before current activation and pinning. A persisted frame
label or generation receipt without that source binding is insufficient.

If any of that evidence is missing or ambiguous, refuse/remain not-ready. Do not guess
a position, recreate a present actor, reset activation ordering, renew session/grace
or protection deadlines. This slice claims no uninterrupted failover persistence.

## Resources and separate gates

Protected #912 remains binding: actor registry delta=[]; 131072 is only an explicit
preproduction bound, not a production default/capacity/readiness claim.

Reuse current first-production graph/cell/span/key/manifest/provenance/artifact and
generation-pair limits including Amendments01-03. MOVE-RL-03=1 applies only to its
isolated decision. No new hard maximum is selected here.

Classify actual added variable-size qualified-frame/initialization/authorization retention and lookup
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

Frame qualification/evidence must cover the whole source -> generation -> activation ->
Channel pin -> issued location chain and its retry/restart paths. Include independent
single-invariant cases with valid unrelated bindings: same coordinates and artifact
generation but substituted frame; missing frame declaration; wrong declared spatial
contract/revision or authored frame semantics; source-manifest bytes not matching the
generation's bound digest; activation/pin/location frame disagreement; and retry or
restart lacking the original source-qualified binding. Every case must refuse before
position/control write, preserve prior state and avoid authority/deadline renewal.
Positive evidence must prove the actual qualified native frame through loaded cells
and real step/return/blocked-cell behavior, rather than equality of copied markers.

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
QUALIFIED_NATIVE_FRAME_BINDING=NOT_YET_PROVEN
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
- docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md
- docs/architecture/OTERYN_CANONICAL_WORLDID_CONTENT_BOUNDARY_2026-09-09.md
