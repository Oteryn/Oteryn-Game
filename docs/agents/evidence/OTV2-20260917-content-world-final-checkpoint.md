# OTV2 Content / World final checkpoint — 2026-09-17

```yaml
status: RETAINED_EVIDENCE
classification: PROPOSED_NONCANONICAL
repository: Oteryn/Oteryn-Game
pr: 641
branch: agent/content-world-design-dossier-20260917
protected_main_at_checkpoint: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
pre_checkpoint_head: 7d2b966365f03b0684c2badf142644649f28c70e
implementation_authority: NONE
worker_release: NONE
format_acceptance: NONE
resource_registry_mutation_authority: NONE
production_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
```

## Purpose

Retain the completed owner-requested content/world architecture investigation and its final bounded local-object contract review on PR #641. This file is a recovery/status checkpoint, not a new architecture authority, implementation allocation, merge request or replacement for live GitHub state.

Read the retained package in this order:

1. `docs/agents/evidence/OTV2-20260917-content-world-design-dossier.md` — consolidated content/world design and corrections from the original investigation;
2. `docs/agents/evidence/OTV2-20260917-content-world-execution-design.md` — source-backed execution/state audit and dependency-aware next-build sequence;
3. `docs/agents/evidence/OTV2-20260917-content-world-local-transition-contract-candidate.md` — revision-2 minimum local-object transition candidate after bounded author review;
4. `docs/agents/evidence/OTV2-20260917-content-world-design-chat-handoff.md` — retained conversation/navigation through Phase H;
5. this checkpoint — exact closeout state and next boundary.

## Work retained

The investigation established and preserved these design directions without mutating runtime:

- native Oteryn source/project -> typed canonical semantic model -> deterministic compiler -> separate server-authoritative/client-safe artifacts -> staging -> explicit activation;
- stable namespaced definition identity separate from authored placement identity, runtime instance identity, reward/materialization occurrence and command/transaction identity;
- authored static world/content separate from runtime mutable state and durable player/value state;
- typed capability families instead of arbitrary authoritative attribute bags;
- ordered semantic map placements with separate visual, collision, interaction and gameplay footprints;
- source authoring shards, compiled bundle chunks and runtime spatial sectors treated as independent partition domains;
- explicit stateful-object transitions instead of assuming sprite/type swaps are the canonical state machine;
- client hints/presentation never treated as gameplay authority;
- Interaction occurrence/proposal/reconciliation machinery reused rather than replaced by another generic callback/receipt framework;
- GAME-ITEM and DUR-03 ownership preserved for durable item/value legality, custody and conservation;
- GAME-NPC-SERVICE architecture from protected PR #500 treated as already decided architecture, while real trade settlement remains downstream;
- Crystal/OTBM/XML/Lua/legacy data used only as migration/reference evidence, never as code to transliterate or canonical runtime authority;
- reimport specified as a semantic three-way comparison between previous imported baseline, new source and local Oteryn corrections;
- incomplete unrelated catalogue content allowed outside a selected executable closure while every selected dependency remains fail-closed;
- source evidence, target parity, redistribution permission and runtime capability kept as separate admission axes.

## Final bounded local-object result

The broad ownership question was narrowed materially.

Protected architecture already places the public mutable world/map overlay under the existing `ChannelRuntime` / `InstanceRuntime` current writer. The remaining first-child gap is therefore **the typed local-object operation and its composition with existing Foundation/Movement/Content boundaries**, not a new service or new world-state owner.

The retained revision-2 candidate recommends one ordinary, non-value-bearing, scope-local object with:

- exact immutable definition + revision + Content generation;
- stable authored PlacementKey and complete footprint;
- finite named states and explicitly allowed transitions;
- current runtime scope/owner/incarnation validation;
- one non-interleaved owner commit for logical object state plus all of that object's spatial/presentation contributions;
- Movement consuming the resulting current spatial view without Content callbacks mutating actor position;
- replay returning the original outcome without re-executing a transition;
- late CommandResult never overwriting newer world state;
- snapshot/result/delta reconciliation using existing Foundation sequencing/barrier rules;
- first bounded activation policy rejecting a different Content generation while that live scope remains active, with staging still independent.

Opening one object removes only that object's spatial contribution. It must not clear another wall/object/policy contribution merely because a combined cached cell flag becomes false in a naive implementation.

## Corrections discovered during final review

The final author review corrected or made explicit four important boundaries:

1. **Protocol registry:** on inspected protected main, `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` has empty `command_types` and `state_domains`. Generic Foundation envelopes exist, but there is no already-registered spatial/world gameplay domain. A real client/server child therefore requires owning typed command/result and state delta/snapshot registration. This evidence PR allocates no numeric IDs.
2. **Whole-session command order:** FND-02 ordering applies across targets and command families. Admission ordering alone is insufficient; a consumer must establish its execution/commit turn before publishing the authoritative effect or terminal result. Post-mutation failure of retirement/order validation is too late.
3. **Semantic duplicate comparison:** where duplicate/conflict classification needs the request payload, compare normalized typed intent plus the originally selected binding, not raw protobuf byte equality. An expired retained response is not permission to execute an old CommandRef again.
4. **Snapshot egress:** both StateDelta and CommandResult produced above an active snapshot target sequence must respect the existing bounded server-side snapshot barrier before transmission. Constructing a barrier object without composing it into real egress does not prove this property.

These are inherited accepted Foundation constraints made explicit for the proposed child; they are not new global Server Seam gates and are not claims of discovered production vulnerabilities.

## What is deliberately not frozen

This package does **not** select or accept:

- permanent source text/container technology;
- final source shard or bundle chunk dimensions;
- runtime spatial-sector dimensions;
- final compiled bundle serialization/compression;
- production numeric maxima for the Reference successor;
- a complete quest model/runtime/persistence implementation;
- broad NPC/economy/market/bank/depot semantics;
- full scripting/WIT runtime;
- durable ordinary-door state;
- occupied-door Global behavior;
- generalized live Content hot reload/rebind;
- a production public object resolver from the private/preproduction actor carrier;
- implementation paths, protocol IDs or worker ownership.

## Evidence and validation retained

Historical prototype evidence from the original conversation reported **63 tests passed**; those tests were explicitly classified as prototype validation and were not reconstructed/rerun in the later continuation.

The execution-design witness was extracted/rerun on Python 3.13.5: **26 tests passed**, source SHA-256:

`9809f28dd0b34e4c2b7c42dadcdeda856d35f8587b487240de9c889d94f5034d`

The local-transition witness was extracted/rerun on Python 3.13.5: **21 tests passed**, including **4096 length-four serialized synthetic traces**, source SHA-256:

`e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa`

Those models deliberately test replay, conflicting payloads, stale authority, occupied-close fixture behavior, collision-contribution composition, sequence/revision gaps and bounded capacity. They assume typed/trusted fixture inputs, serialized calls and indivisible model publication. They do **not** prove production Rust composition, parallel execution, PostgreSQL durability, actual protocol payload registration, native-client behavior, source-parser correctness, production resource capacity or Global Reference parity.

A final scratch counterexample demonstrated why isolated Owner and CommandIngress sketches cannot substitute for the real composition proof: command 2 can mutate the isolated model before command 1 has retired, while later terminal-order validation rejects command 2. The owning implementation must prevent that publication order; the counterexample is not a claim about current production code.

Latest pre-checkpoint hosted state for exact head `7d2b966365f03b0684c2badf142644649f28c70e` at readback:

- Agent governance `35238876610`: `SUCCESS`;
- Architecture semantic audit workflow `35238876609`: `SUCCESS` as workflow execution only;
- Merge gate `35238876656`: `IN_PROGRESS` at that readback.

The preceding head `70ac6659341a6d204ef6dbbfe990ae70950da141` had full `game-gate` success, but that does not qualify later commits. Earlier semantic-audit logs reported `profile=NOT_APPLICABLE`, so workflow success must not be described as independent semantic acceptance of these evidence documents.

## Readiness conclusion

Broad design discovery for the first local-object case is complete enough to stop expanding the architecture speculatively.

The next useful step is **not another general audit**. It is an owning decision followed by exactly one scoped implementation proof selected by #162/control plane:

### A. Smallest synthetic in-process Rust composition proof

Use if real wire prerequisites are not yet protected/ready. Exercise actual existing Foundation ordering/replay primitives plus the allocated typed object operation and spatial composition. Use explicit finite test-only limits and synthetic values. Do not claim playable-client execution, production capacity or Reference parity.

### B. Real client-server local-object proof

Requires, in addition to A:

- owning gameplay command/result and state-domain registration;
- qualified current-owner/object lookup;
- selected Content successor/source binding path;
- actual server egress/snapshot barrier composition;
- client delta/result/snapshot reconciliation tests.

### C. Reference-behavior claim

Requires B plus accepted target evidence for every exercised target-sensitive rule. A synthetic technical proof must not be promoted to Reference parity by naming or packaging.

The first implementation child should test at minimum:

- earlier pending GameSession command versus later command targeting another object — later authoritative effect/result must not commit first;
- failure before local owner publication preserves the original admitted command lineage;
- response retention expiry cannot make an old CommandRef executable again;
- stale session/current-owner/incarnation/content binding fails closed;
- opening removes only that object's collision contribution;
- movement-first vs close-first produce a valid serialized outcome with no actor occupying newly blocked space;
- registered/typed wire payload rejection for unsupported/unknown forms when the wire child exists;
- snapshot in progress while both a result and a world delta are produced — neither bypasses the target-sequence barrier.

No DUR-03 durable settlement is made an artificial dependency of this plain non-value-bearing object. Durable keys, rewards or other value-bearing effects require their owning transaction path when selected later.

## Control-plane handoff already published

An informational minimum-implementation decision packet was published to #162 after the bounded author review. It does not release a worker, assign paths, allocate protocol IDs or reactivate blocked descendants. The receiving coordinator must refresh live state, custody, exact dependencies, integration capability and authorization before any material worker release.

## Terminal status of this retained evidence work

```text
BROAD_DESIGN_DISCOVERY: COMPLETE_FOR_FIRST_LOCAL_OBJECT_CASE
LOCAL_TRANSITION_CONTRACT: PROPOSED_NONCANONICAL_REVISION_2
AUTHOR_REVIEW: COMPLETED_WITH_CORRECTION
INDEPENDENT_ARCHITECTURE_ACCEPTANCE: NOT_CLAIMED
IMPLEMENTATION: NOT_STARTED_BY_THIS_PR
PRODUCTION_PROOF: NOT_CLAIMED
REFERENCE_PARITY_PROOF: NOT_CLAIMED
WORKER_RELEASE: NONE
NEXT_AUTHORITY: #162 / owning architecture-control path
```

Live GitHub Issue/PR/check state and protected accepted contracts supersede this checkpoint whenever they move. Do not infer implementation or merge authority from this retained evidence file.
