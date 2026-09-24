# OTV2 Content / World execution design — continuation of PR #641

```yaml
status: RETAINED_ANALYSIS
revision: 1
date: 2026-09-17
repository: Oteryn/Oteryn-Game
branch: agent/content-world-design-dossier-20260917
inspected_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
predecessor_pr_head: 13e48009cefaa7c7f0ffc7b07390cd5b4cc4eca5
architecture_status: PROPOSED_NONCANONICAL
implementation_authority: NONE
format_acceptance: NONE
registry_mutation_authority: NONE
production_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
```

This is the execution/state continuation of the [original dossier](OTV2-20260917-content-world-design-dossier.md), not a replacement for its retained history. The [chat handoff](OTV2-20260917-content-world-design-chat-handoff.md) remains the entry point. The owner explicitly requested continued audit, analysis and architecture work on the existing PR. No new programme, implementation allocation or public contract is created.

**Result:** retain the native Content architecture, reuse the implemented Interaction kernel, and specify the missing definition-to-owner bindings. Do not implement an independent generic callback engine, turn the bootstrap profile into Reference by increasing counts, or require full Studio/quests/economy before a bounded source and world-object proof.

Evidence labels below mean `PROVEN` for directly inspected source/metadata, `DERIVED` for conclusions from it, and `UNKNOWN` where required evidence is missing. Proposed behavior is explicitly `RECOMMENDATION`; it is not current runtime behavior.

## 1. Exact evidence basis

All relative repository-source links in this section were inspected at `b44fefe08f6aaf1b2c1c23dedd92bab0de87146e`. Their presence on that commit is source evidence, not an execution or deployment result.

| Ref | Inspected source | Verified boundary |
|---|---|---|
| S1 | [`content/production.rs`](../../../apps/game-server/src/content/production.rs), `FirstProductionCell`, `FirstProductionCreature`, `FirstProductionItem`, `FirstProductionContentSource`, `ProductionArtifactMetadata` | Bootstrap cell has terrain/collision but no ordered placement collection; creature has behavior/presentation/policy but no explicit combat/reward links; item has presentation/materializable but no typed GAME-ITEM capability link. Exact revision/provenance/profile and server/client generation matching already exist. |
| S2 | [`content/model.rs`](../../../apps/game-server/src/content/model.rs), `EffectFamily` | The inspected Content enum contains `Damage` only. This is not evidence that the separate Ability execution kernel cannot heal. |
| S3 | [`interaction/identity.rs`](../../../apps/game-server/src/interaction/identity.rs), `RootSourceOccurrenceRef`, `ChildOccurrenceRef`, `SemanticRevisionContext`, `AuthorityFenceEvidence`, `RngDecisionRef` | Child identity and authority fences are separate. Child identity includes parent, definition, target, edge, ordinal and selected content/ruleset/simulation revisions. |
| S4 | [`interaction/dispatch.rs`](../../../apps/game-server/src/interaction/dispatch.rs), `ForeignOwner`, `ProposalAdapter`, `InteractionDispatcher`; [`interaction/lifecycle.rs`](../../../apps/game-server/src/interaction/lifecycle.rs), `ChildLifecycleBook` | Existing proposal/reconciliation boundary; foreign-owner vocabulary is `Foundation / Ability / Durability`. Lifecycle storage in this module is an in-memory `BTreeMap`; ambiguous/cancellation-requested outcomes remain pending. This alone proves neither PostgreSQL retention nor an end-to-end world-object adapter. |
| S5 | [ADR-0005](../../architecture/ADR-0005-native-world-format-and-oteryn-studio.md) and [GAME-ITEM-01](../../architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md) | Native source/model/bundle separation; namespaced identity; static placement is not automatically a durable item; GAME-ITEM owns legality, DUR-03 owns value/location transactions. |
| S6 | [NPC boundary](../../architecture/OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md), PR #500 | PR #500 is merged, merge `3508441b33cf627bf30336aacb9cd7fba95bde9a`; its document is on inspected main. Logical GAME-NPC-SERVICE lives inside the current runtime owner. Dialogue/catalogue and real value transfer are separate slices. |
| S7 | [Full-world producer README](../../../tools/game-atlas-fullworld-source/README.md) and [corridor census README](../../../tools/reference-world-corridor-census/README.md) | Existing Game-owned migration projection/census API; visible ordering and pinned coordinate transform are already defined. Nested container contents are not visible map-stack presentations. Census output is migration/OTS hypothesis evidence, not Global parity or production maxima. |
| S8 | Live #504 semantic-delta discussion and #162 checkpoint `5712770013` | Successor design is separate from implementation release. Checkpoint records preproduction actor carrier #573 as integrated, but #508 implementation and the Movement-readable composed resolver are not released/proven there. It records #513 as an evidence-allocation proposal, not a production item-value proof. |

Reproducible source blob anchors: S1 `14a86249556758330471e6cee9e6ff6a90220b79`; S3 `e506ebe235b6db3dcab58e90c5957dbbf0253942`; S4 dispatch `7a6c2400f60befc5a75562e2bda881f8d141f81b`, lifecycle `bb048b5b31f050496c2f98d1a3c3897879c7a23f`; S6 `60a3fa7af16020d7b2922c39c6630808653e8bca`; S7 producer README `88984b12e594a362f93b742547dbc80383542088`, census README `35a3b6d91b85ed7669d45542d561120a00d9f6b3`.

The raw legacy corpus, asset digests, actual official-client observations and full production call graph were not requalified in this continuation. README-recorded input digests are retained provenance, not a new successful corpus run. Open manifest PR #640 is a separate owned candidate and is not substituted for protected Reference authority.

## 2. Bounded audit findings and exact triggers

These are not eight newly invented blockers for Server Seam. Each finding is attached to the child that would exercise it; source collection and non-executable analysis remain possible.

| Finding | Classification | Consequence / exact trigger |
|---|---|---|
| CW-01 — semantic successor, not larger bootstrap | PROVEN, S1/S2 | Before Reference lowering: add only the approved semantic delta under #504. Larger cardinalities alone cannot encode ordered objects, Heal, item capability links or creature rewards. Preserve old profile interpretation. |
| CW-02 — presentation projection is not a lossless gameplay IR | PROVEN boundary, DERIVED integration risk, S7 | Before claiming a gameplay importer: account for nested contents, sidecars, bindings and unsupported fields upstream of the lossy projection. Do not infer that every map presentation is a materializable item. |
| CW-03 — Interaction already provides the common occurrence machinery | PROVEN, S3/S4 | Reuse it when a future adapter is allocated. Another global event ID, receipt manager or script dispatcher would need evidence that these contracts cannot serve the selected child. |
| CW-04 — world-object mutation binding needs an explicit owner seam | PROVEN inspected enum/API; composed readiness UNKNOWN, S4 | Before executing authored door/lever transitions: identify the semantic owner, its current-writer entry point and qualified adapter. An enum member named Foundation is not permission to route arbitrary world/economy mutation through it. |
| CW-05 — retained in-memory lifecycle is not restart durability | PROVEN, S4 | Before any reward or durable mutation: compose the owning DUR-03 receipt/transaction/reconciliation path and test actual failures. Do not call `ChildLifecycleBook` a durable journal. |
| CW-06 — old NPC architecture dependency wording is stale | PROVEN, S6 | Do not keep #500 architectural protection as an unresolved condition. Target values, capabilities, implementation allocation and actual execution remain independently gated. |
| CW-07 — several delegated children do not imply atomic business effects | DERIVED, S4/S5/S6 | Before admitting a recipe with coupled irreversible effects, require the owning transaction contract or exclude that recipe. Generic Interaction sequencing is not a distributed transaction. |
| CW-08 — a successful import cannot certify target truth | PROVEN distinction, S7/S8; exact target fields UNKNOWN | Before exercising target-sensitive Reference content, bind each required field to accepted target evidence. Do not block unrelated candidate ingestion or silently treat UNKNOWN as false/zero. |

## 3. Minimal architecture: bind existing layers rather than add a framework

`RECOMMENDATION` — use this flow, with each arrow implemented only by its separately allocated owner:

```text
pinned import/source snapshot
  -> typed candidate definitions + explicit unresolved dispositions
  -> selected release roots and semantic dependency closure
  -> supported owner-capability bindings + field evidence + resource admission
  -> existing Content compiler / staging / exact generation activation
  -> current ChannelRuntime or InstanceRuntime
       -> Interaction occurrence/plan when composition is needed
       -> exact owning domain validation and mutation
       -> authoritative outcome + versioned client observation
```

The following table names semantic responsibilities; it does not introduce Rust structs, protocol IDs or new services.

| Representative object/action | Authored input | Runtime authority | Persistence boundary |
|---|---|---|---|
| Wall / static obstacle | Typed terrain/placement and spatial facts | Current world/spatial view consumed by Movement | No item-value instance merely because a sprite exists |
| Plain door / local lever | Placement, finite state vocabulary, explicit transition and footprint rules | Proposed narrow world-object/spatial role inside the existing current runtime writer; Interaction may plan a transition | Runtime-only or durable state must be declared for the selected mechanic; no automatic persistence of every door |
| Move through an opening | Current traversability, zone rules and movement intent | Movement/current actor owner, not a Content callback | Existing movement/recovery contract; opening and crossing are different operations |
| Open an inventory container | GAME-ITEM container definition and current instance reference | GAME-ITEM/container access owner | Opening a view does not transfer custody; changes to contents use DUR-03 |
| Pick up/materialize an item | Explicit materialization eligibility and resolved ItemType revision | Owning item/value producer and DUR-03 | One authoritative instance/location transition and retained outcome |
| Creature death / reward | Explicit combat, corpse, loot and XP references | Existing combat/reward owners; Interaction may coordinate proposals | Selection is distinct from item/XP settlement; no reward-on-load shortcut |
| NPC talk / catalogue | NPC/dialogue/service definitions and exact revisions | Protected GAME-NPC-SERVICE role; AI remains actor-local behavior | First widget slice has no value mutation or promised durable conversation continuity |
| NPC buy/sell | Catalogue/price revision plus selected intent | GAME-NPC-SERVICE proposes; GAME-ITEM and DUR-03 validate/settle | Later `NPC_SINGLE_TRADE_COMMIT_V1`; client catalogue is not purchase authority |
| Travel / cross-scope transition | Typed transition/capability reference | Existing travel/admission/current-owner contracts | Not a same-owner local relocation disguised by distant coordinates |

**Must decide before the plain-door runtime child:** which existing world/spatial owner exposes the local transition operation. The proposed logical role is not an independent scheduler, database writer or microservice. If no protected contract owns that seam, obtain a bounded architecture amendment before the adapter; do not grow `ForeignOwner` or public APIs under this retained-evidence PR.

## 4. Binding and identity: five different things

`RECOMMENDATION` — keep the following identities distinct and reuse accepted Foundation/domain representations wherever they exist.

1. **Definition binding:** typed family + stable namespaced key + exact compatible definition revision. File path, display name and compact bundle index are not identity.
2. **Authored placement:** stable source/world placement identity. A move can preserve it; a copy is a new placement. Definition revision or shard filename is not the placement key.
3. **Live target:** current accepted runtime scope + resolved placement/entity or existing ItemInstanceId + its current incarnation. Do not invent a new durable UUID for a static wall.
4. **Semantic occurrence:** existing `RootSourceOccurrenceRef` and `ChildOccurrenceRef`, with the revisions selected for that occurrence. An actual new source event is different from retrying an earlier one.
5. **Current authority:** existing session/owner/state/domain fences. This context decides whether an attempt may apply now; it is not a mechanism for minting a new logical reward.

`WorldId` is not a map filename or an authored world key. `ChannelId` and `InstanceId` remain distinct scope concepts. This document deliberately does not redefine `RuntimeScopeRef` or any public identity encoding.

A future Content-to-owner binding must resolve a **supported, versioned semantic capability**, not merely contain a non-empty string such as `can_open`. Its validation must identify the compatible owner rule, input/state requirements, effect composition class and client-safe projection. Missing support in the selected runtime profile is a linking/admission failure, not a no-op.

S3 already includes content/ruleset/simulation context in child identity. Therefore a retry must retain the originally selected context. Replanning an old source event against the newly active content and then treating the resulting child as fresh would defeat that identity boundary. Current ownership fencing remains separate and must still be checked by the owning operation/reconciliation contract.

## 5. Stateful object operation and client observation

`RECOMMENDATION` — distinguish command acknowledgement from current world state.

```text
bounded authenticated input
 -> bind current actor/session and resolve the source occurrence
 -> resolve an existing operation/replay, or admit one new intent
 -> resolve exact definition + current target incarnation/state
 -> owner validates range, permissions, revision and affected spatial facts
 -> one local authoritative state/index update, or a typed delegated proposal
 -> original operation outcome + separately versioned world observation
```

For a plain door, prefer an explicit transition or desired-state intent over an unqualified `toggle`. **Desired state alone is not replay safety:** A opens, B closes, then retrying A's `EnsureOpen` must not reopen the door. The retry returns A's earlier result without a new mutation. The same source occurrence with a different payload is a conflict, not a new operation.

A duplicate acknowledgement of A's success must also not make the client render an old open state after B's close. Observations need the owning generation/incarnation and ordered state revision; stale deltas are discarded or trigger bounded resynchronization. The operation result and the newest state can legitimately differ: A succeeded earlier and the door is closed now.

Within one current writer, evaluate occupancy and commit the whole affected spatial delta without interleaving another movement mutation. A multi-cell object's state and collision/index effects cannot be published cell by cell. Client presentation follows the committed result, never authorizes it.

The synthetic witness below chooses **reject closing an occupied door** only as a fixture policy. Actual Reference behavior on occupied tiles, items in a doorway, pushing, decay, floor/height and PZ interaction is `UNKNOWN` until the selected rule is evidenced. Do not invent displacement behavior or declare this fixture policy Global parity.

A plain door does not require a generic quest ledger. Conversely, a durable quest door cannot be treated as runtime-only simply because the first fixture was runtime-only. Restart policy is part of the selected mechanic.

## 6. Composition classes prevent half-success recipes

`RECOMMENDATION` — classify an authored action before allowing it to execute:

| Composition class | Allowed first shape | Rejection boundary |
|---|---|---|
| Local atomic state transition | One current owner validates and commits object state plus its spatial projection | No mutation of another domain's durable item/currency/progression state |
| Single foreign-owner transaction | One existing delegated operation owns every conservation-sensitive effect in that transaction | No success acknowledgement from proposal acceptance or transport timeout |
| Outcome-driven presentation | Dialogue/UI/animation reacts to a committed/rejected foreign result | A visual update cannot be treated as the economic commit |
| Coupled effects spanning owners | Deferred unless an accepted owner contract defines the complete commit/recovery semantics | Generic sequential children, guessed compensation, or automatic rollback are insufficient |

Examples: opening an ordinary door can be local. Consuming a valuable key and granting a once-only reward is not safely expressed as two unrelated callbacks. Opening a trader widget is not a purchase. A failed second child cannot be assumed to undo a committed first child.

This does not require a new transaction coordinator, broker or general saga framework. Prefer the existing owning transaction boundary. If it cannot express the chosen action, exclude that action from the first executable profile and name the exact later owner decision.

## 7. Materialization, retry and restart

`RECOMMENDATION` — retain the existing separation:

```text
eligible source occurrence
 -> deterministic selection under pinned rules/content/RNG context
 -> owning materialization/transfer operation
 -> durable committed/rejected outcome or unresolved pending state
 -> gameplay/client projection
```

A conceptual reward discriminator includes the applicable WorldId, **declared eligibility scope**, source occurrence and reward slot. Its exact representation remains with the accepted owning contracts. Do not implement this conceptual tuple as a competing receipt store.

Eligibility can be world-, channel-, instance- or character-scoped only when the owning mechanic declares it. Runtime channel locality must not multiply a world-once claim; equally, two independently eligible per-channel source occurrences must not be incorrectly collapsed. Bundle reload, streaming, renderer cache eviction, cosmetic changes and changed runtime ownership are not new eligibility events.

The selected result and binding survive ambiguous acknowledgement. Timeout does not prove rejection; cancellation request does not prove retirement. Reconcile the original operation rather than rerolling, assigning a new child or refunding/minting speculatively. This is consistent with S4's pending-state semantics, but real durable composition still needs separate proof.

For durable effects, retained outcome/conservation evidence must survive the relevant restart/replay horizon. Expiring an in-memory response cache cannot permit an old claim to become new. A future bounded implementation must define either retained deduplication evidence or an authoritative rejection fence for requests outside its admissible horizon. Do not choose indefinite unbounded memory as the solution.

For runtime-only object state, restarting may use an explicitly accepted reset policy; old session/incarnation work must then be rejected rather than applied to the replacement object. This does not authorize resetting durable rewards or invent a Reference restart policy.

## 8. The importer boundary needs more than Atlas tiles

`RECOMMENDATION` — reuse the existing pinned Game-owned parser/producer lineage, but distinguish its products:

```text
accepted legacy input snapshot
 -> existing bounded decoder / source records
      -> migration/source-semantic candidates with explicit loss accounting
      -> existing public/presentation projection where appropriate
```

The second output cannot reconstruct information intentionally absent from the first. S7 explicitly excludes nested container children from visible map-stack presentations and does not claim NPC/quest/spawn inference. A full-world visual export therefore cannot, by itself, provide a lossless gameplay source model.

For each selected imported construct, retain an input locator and disposition: direct transfer, explicit normalization, derived candidate, native rewrite required, deliberately excluded, unresolved or conflict. Include relevant sidecars and binding tables in the locked input set; a map digest alone is not the whole content source.

Do not add another OTBM parser or copy Crystal gameplay code. Extend or expose the narrow existing Game-owned source boundary only after an exact allocation. Existing digest-scoped producers continue rejecting unaccepted input revisions; this proposal does not turn them into unrestricted importers.

Reconciliation must account for counts and important semantic fields before/after conversion, including deliberately omitted data. A bounded/truncated diagnostic must report the truncation and remaining count; truncated output is not evidence of zero unresolved records. Source evidence, distribution permission, target parity and runtime capability are separate admission axes.

## 9. Selected closure and honest spatial boundaries

`RECOMMENDATION` — compile only explicit roots plus their required semantic dependencies. Unselected incomplete quests or catalogue entries can remain in the authoring workspace without blocking an independently complete release subset.

For the selected subset, require:

- every required definition/placement/rule reference resolves to the expected typed family and exact revision;
- every exercised critical field has an acceptable value/evidence binding, not just an entity-level source label;
- the runtime profile supports each selected owner capability;
- the required spatial cells, footprints and exercised transition endpoints are complete;
- the applicable finite resource envelope is admitted before production use.

**Unknown is not empty.** Missing shard, unloaded region, unobserved collision and intentional void need distinct outcomes. A reachable missing cell cannot silently become walkable empty ground. A visual footprint crossing a selected window needs complete covered data or an explicit non-executable boundary, not clipped success.

Classify dependency edges. A strong definition edge needs linking; a same-owner spatial step needs complete spatial authority; a cross-scope travel reference is resolved by its transition/admission contract, not an instruction to import the entire destination world recursively. Exercises must include the endpoint evidence actually needed by the selected journey.

A graph reference cycle is not automatically an infinite execution loop. Conversely, a terminating linker does not prove bounded trigger execution. Bound alias/prefab expansion, trigger work, selected-child fan-out and ancestry under their actual owner contracts; do not forbid legitimate finite open/close state cycles merely because the state graph is cyclic.

## 10. Revisions, fingerprints and activation

`RECOMMENDATION` — distinguish four questions without changing the existing generation identity:

| Question | Required interpretation |
|---|---|
| Are authored gameplay semantics identical? | Compare the versioned typed semantic representation, preserving meaningful array/placement order. This is not by itself activation permission. |
| Is presentation identical? | Compare the relevant client-safe presentation dependency closure. Cosmetic difference is not a new reward occurrence. |
| Is qualification still valid? | Bind source/field evidence, target cut, declarations and capability/resource qualification. Evidence changes can require requalification even when a numeric value is unchanged. |
| Is this the exact accepted artifact pair? | Retain the complete Content generation, profile, provenance and artifact digest checks. Do not bypass them because a narrower fingerprint matches. |

Freeze source membership/bytes for a build through an immutable snapshot. Do not hash a manifest, read changing files afterwards and claim one coherent generation. Importer/mapping/canonicalization versions belong in reproducibility evidence.

RFC 8259 describes unordered object members and ordered arrays and warns about duplicate-name interoperability. A future strict source parser should reject duplicates and unsupported critical fields rather than rely on a library's last-key-wins behavior. RFC 8785 is a possible canonicalization reference, not automatic selection: it specifies array-order preservation, UTF-16 property sorting, number serialization and unmodified Unicode strings. A generic `sort_keys` call is not a full JCS implementation. Exact number/unit representation and canonicalization version must be settled with the chosen bounded source codec, not inferred from a JSON filename.

For the first runtime composition, prefer an explicit safe-point switch or rejection while incompatible/pending work exists over a generalized live-patching system. Previously admitted pending work retains its selected revisions until reconciled, or activation is held. Retiring a placement with live/durable state requires an accepted disposition/migration; it cannot simply vanish and respawn with fresh entitlement.

LKG rollback of content bytes is not rollback of already committed database effects. Old artifacts may be needed to interpret retained work. The owning activation/migration contract must either preserve compatible interpretation or reject the switch. This continuation does not enable migration-bearing artifacts in first-production v1.

Primary standards checked for the narrow JSON claims above: [RFC 8259, sections 4–6](https://www.rfc-editor.org/rfc/rfc8259) and [RFC 8785, sections 3.1–3.2](https://www.rfc-editor.org/rfc/rfc8785). Neither standard selects Oteryn's source format or gameplay schema.

## 11. Reimport is a semantic three-way operation

`RECOMMENDATION` — compare last imported baseline, newly imported candidate and local authored correction by stable semantic identity/field, not file offset.

| Change | Intended result |
|---|---|
| Upstream changes; local equals old baseline | Adopt the upstream candidate, retaining new provenance/evidence state |
| Local correction; upstream unchanged | Preserve the correction |
| Both sides make the same semantic change | Coalesce without duplicating identity |
| Both sides change the same field differently | Explicit conflict; no silent winner |
| Upstream deletes; local edited or live/durable state references it | Conflict or owner-approved retirement/migration; not silent deletion |
| File/shard movement only | Preserve placement identity and semantic bindings |
| Actual copy of an object | Allocate distinct authored placement identity through the accepted authoring process |
| Ambiguous legacy matching / composite grouping | Retain ambiguity; no coordinate/name-only identity guess |

Represent missing, deleted, unknown and not-applicable separately in the real schema. The witness uses `None` as a deletion sentinel in one synthetic test only, not as the proposed content encoding.

A source snapshot update does not automatically supersede the dated Global target. Newly observed values require their own target-continuity or declared-difference disposition. Keep accepted corrections and original evidence history reconstructible.

## 12. Resource decisions: finite boundaries without arbitrary widening

`RECOMMENDATION` — inventory actual new dimensions before choosing successor maxima. Do not inherit the bootstrap's `32x32`, one-floor or single-item cardinalities as the product world design, and do not add arbitrary headroom to them.

| Dimension | Required measurement / failure witness |
|---|---|
| Parsing and linking | Input/decoded bytes, nesting, records, reference edges, string sizes, diagnostics and work; reject before oversized allocation where required |
| Ordered world placements | Per-cell and aggregate counts/bytes; canonical order; footprint expansion and cross-window coverage |
| Rules and interactions | Supported rule/state vocabulary, expansion depth, child fan-out, lookup work and retained lifecycle/receipt horizon |
| Activation | Candidate + current + any necessarily retained generation working set; incompatible/pending switch behavior |
| Server/client projections | Separate bytes/record counts and dependency closure; server-only data must not leak through the client-safe projection |
| Source reimport | Candidate/change/conflict volume and bounded comparison/report generation; no silent truncation of semantic loss |

For each production dimension record the semantic reason, owner, unit, finite maximum, check location, max/max+1/overflow witness and failure behavior. Source fixtures may use explicitly non-production finite test limits. Representative performance tuning is separate from the correctness requirement to reject unsafe/unbounded work; neither a single fixture nor a historical bootstrap number supplies a new production capacity claim.

## 13. Decision timing and alternatives

These are recommendations for a later owning decision, not acceptance through this document.

| Decision | Must decide now? / blocked child | Real alternatives and recommendation | Cost of deferral / supersession evidence / deliberately undecided |
|---|---|---|---|
| Typed source-to-owner capability binding | YES before executable successor lowering | Explicit checked binding versus untyped callback names; choose the checked binding using existing domain ownership | Untyped coupling would become a compatibility burden. Reopen on a proven unsupported mechanic; leave encoding and exact Rust API open. |
| Narrow stateful-world mutation seam | YES before the first door/lever runtime adapter, not before importing candidates | Local role in current runtime writer versus an independent service; choose the local role | A second writer creates unnecessary coordination. Reopen only on accepted scope/ownership requirements; leave public adapter changes to the owner contract. |
| Occurrence versus current authority | YES before replayable side effects | Reuse S3/S4 versus a second identity/receipt system; choose reuse | Incorrect retries can repeat state/value effects. Supersede only with exact contract incompatibility evidence; durable physical schema remains DUR-owned. |
| Coupled effect class | YES before a recipe consumes/grants durable value | One owning transaction versus unqualified multi-child sequence; choose the owning transaction or exclude recipe | Partial value success is not repairable by presentation rollback. Reopen after a protected composition contract; no general saga selected. |
| First activation policy | YES before stateful runtime activation | Safe-point switch/reject incompatible work versus generalized hot reload; prefer the bounded first shape | Unsafe switches reinterpret retained state. Reopen after actual product need and migration/failure evidence; no arbitrary live-migration system now. |
| Permanent project/bundle codec, shard size, renderer layout | NO for this retained design or initial source witness | Keep bounded candidates; compare realistic encodings with measured corpus later | Freeze only when downstream representation requires it. Reopen from size/load/edit/compatibility evidence; no final JSON5/JSONL/FlatBuffers/chunk/GPU decision. |
| Full quests, broad NPC economy, generalized scripting and full Studio | NO for the plain source/world-object child | Preserve extension points and defer, rather than expanding the critical path | Revisit when the selected playable milestone exercises them; no schema/runtime allocation now. |

Player impact considered: old acknowledgements must not visually undo newer state; stale operations cannot duplicate rewards; unknown spatial data must not allow movement through an invented gap. Producer impact considered: existing compiler/importer/Interaction reuse, bounded adapter scope and explicit test oracles are preferred to maintaining parallel frameworks.

## 14. Implementation-shaped sequence, without worker release

**P0 — source inventory and bindings.** Reuse pinned Game-owned extraction inputs, select representative records and retain typed candidates/dispositions. Provide one explainable chain from source field to proposed capability. This can proceed as authorized evidence work without waiting for a playable Server Seam; it does not make arbitrary new source revisions admissible to current digest-scoped tools.

**P1 — bounded source/linker decision and witness.** Specify minimum strong references, ordered placements, declaration/evidence binding and supported capabilities needed for the selected corridor. Use a non-production synthetic door case to test semantics, not to invent an observed Global door. Keep final file/container/shard choices open unless that child actually requires them.

**P2 — explicit #504 successor definition/lowering.** After its exact dependencies and resource decisions are protected, lower only the selected semantic delta through existing Content safety boundaries. Preserve bootstrap interpretation. NPC authored service candidates can reuse protected #500; transaction-capable trade remains later.

**P3 — one real current-owner world interaction.** The released child must name the world-object mutation seam, Movement-readable actor/current-owner/static-world dependencies, client command/observation boundary and exact tests. Reuse the #573 preproduction carrier only within its actual authority; do not claim that its integration already delivers #508/Movement composition. No full quest/economy dependency is added to a plain door unnecessarily.

**P4 — one durable value operation.** Separately compose the qualified item/DUR-03 path with source occurrence, selection, commit, duplicate/ambiguous response and real restart tests. Only this stage may claim durable materialization proof. It is not replaced by the model below.

#162 remains allocation/integration authority. No instruction here releases P0–P4 workers, expands an existing path lease, writes Reference manifest #640, changes protocol IDs, edits a resource registry, or mutates runtime/production.

## 15. Acceptance witnesses for owning implementation children

The following are **test requirements, not executed production test results**. Each belongs only to a child exercising that behavior.

| Case | Required oracle |
|---|---|
| Source enumeration reordered | Same canonical semantic result where enumeration is non-semantic; no identity churn |
| Ordered placement list reversed | Different semantic result where order matters; do not globally sort it away |
| Missing required reference / unsupported capability / unknown exercised field | Selected release rejected with exact locator/reason; unrelated unselected incomplete catalogue does not block |
| Duplicate root with changed payload/revisions | Conflict; no second mutation, RNG draw or new child |
| A opens, B closes, then A is retried | A's historical result reconciles; current closed state/revision remains authoritative |
| Old success acknowledgement arrives after newer state | Client does not overwrite newer entity generation/revision |
| Movement races a multi-cell close | One owner-ordered valid outcome; no partial spatial-index state; selected occupied-close policy independently evidenced |
| Owner/session/incarnation changes | Stale mutation rejected; unresolved original work reconciled by the accepted owner mechanism, not assigned fresh eligibility |
| Commit succeeds but response is lost | Exactly the original durable outcome recovered; no second mint or speculative refund |
| Crash at pre-commit, ambiguous commit and post-commit/pre-observation boundaries | Real PostgreSQL/owner recovery preserves conservation, identity and pending/terminal truth |
| Content activation while old work is pending | Pinned interpretation retained or switch held/rejected; no migration-by-retry |
| Cosmetic rebuild / unload-reload / channel reassignment | No unintended source/reward reset; explicitly per-channel eligibility remains distinct |
| Reimport deletion against local correction or retained state | Conflict or approved retirement/migration; no silent loss |
| Max/max+1 and arithmetic/expansion overflow | Bounded failure without partial activation or leaked/unowned work |

## 16. Validation performed and limits

The new Appendix A is a self-contained **design witness**, executed in an isolated scratch directory. Result: **26 tests passed**. Two tests intentionally demonstrate broken-design counterexamples: desired-state-only replay reopens a later-closed door, and losing a reward receipt permits duplication in the model. Passing those tests means the counterexample was reproduced, not that the broken behavior is acceptable.

Other witnesses check occurrence memoization, changed-payload/content conflicts, stale revision rejection, a synthetic occupied-close rule, pending ambiguity, retained model outcomes, declared eligibility scope, selected-reference closure, ordering and three-way reimport. All inputs are synthetic. A model copy is not a database restart. A sequential in-memory step is not proof of transactional atomicity, concurrency, fencing, persistence or resource bounds.

The original dossier's **63 tests** are historical reported prototype evidence. Their source and execution were not reconstructed in this continuation; do not add 63 and 26 into a production validation total.

Documentation publication is an API-native documentation edit on the existing branch; no local Git commit is selected and reconstructed for publication. Local scratch checks do not replace the repository governance validator. Direct GitHub transport from this container is unavailable, so a full local checkout/governance run is not claimed. Exact-head hosted checks and independent review, where required by current policy, remain separate. A successful Architecture Semantic Audit workflow is not automatically a semantic-profile PASS. This document does not claim `READY_FOR_INTEGRATION` or Merge Queue success.

## 17. Exact remaining unknowns and their effects

- **Protected world-object owner/adapter contract:** needed before authored stateful-world execution; not needed merely to retain candidate definitions. S4 does not supply that binding by itself.
- **Accepted target corridor fields and continuity:** needed for Global-sensitive collision, transition, occupied-door and NPC/value behavior. Migration candidates and synthetic examples do not close those fields.
- **Actual successor source corpus and resource decision:** needed before production maxima and profile implementation. This continuation does not measure the raw corpus or select capacities.
- **Real composed runtime/client and DUR-03 execution evidence:** needed for interaction, conservation and restart claims. Source inspection and the Appendix do not establish it.
- **Exact-head CI/review of the published documentation generation:** read the live PR/checks rather than carrying forward predecessor-head results.

The next architectural deliverable should therefore be a small, owner-reviewed **source-to-capability and local state-transition decision** for the selected first child, not another general file-format catalogue. Candidate ingestion remains separable from executable promotion.

## Appendix A — executable design witnesses

Extract the single Python block between `DESIGN_WITNESS_BEGIN` and `DESIGN_WITNESS_END` into a temporary file and run it with Python 3.11 or newer. It uses only the standard library. This block is an explanatory executable specification, not a supported compiler, server module, migration tool or CI substitute.

<!-- DESIGN_WITNESS_BEGIN -->
```python
"""Executable design witnesses, not Oteryn runtime or PostgreSQL tests.

Inputs below stand for already authenticated, owner-admitted occurrences.
All state is in memory; atomicity, durability, fencing and bounded storage are
assumptions of the model, NOT properties this script verifies in production.
Identifiers, quantities and capacities are synthetic fixture values only.
"""
from copy import deepcopy
from dataclasses import dataclass
import json
import unittest


@dataclass(frozen=True)
class Command:
    root: str
    selected_content: str
    desired_open: bool
    expected_revision: int


class DoorWitness:
    def __init__(self):
        self.open = False
        self.revision = 0
        self.memo = {}

    def apply(self, command, occupied=False):
        previous = self.memo.get(command.root)
        if previous is not None:
            return previous[1] if previous[0] == command else "CONFLICT"
        if command.expected_revision != self.revision:
            result = "STALE"
        elif not command.desired_open and occupied:
            result = "OCCUPIED"
        else:
            self.revision += int(self.open != command.desired_open)
            self.open = command.desired_open
            result = "COMMITTED"
        self.memo[command.root] = (command, result)
        return result


def claim_key(world, eligibility_scope, source_occurrence, reward_slot):
    return (world, eligibility_scope, source_occurrence, reward_slot)


class RewardWitness:
    def __init__(self):
        self.receipts = {}
        self.value = 0

    def commit(self, key, pinned_payload):
        previous = self.receipts.get(key)
        if previous is not None:
            return "REPLAY" if previous == pinned_payload else "CONFLICT"
        # One indivisible MODEL step; not an implementation of a DB transaction.
        self.receipts[key] = pinned_payload
        self.value += 1
        return "COMMITTED"


def pending_after(outcome):
    return outcome in {"AMBIGUOUS", "CANCELLATION_REQUESTED"}


def selected_closure(graph, roots, admitted):
    seen, todo = set(), list(roots)
    while todo:
        key = todo.pop()
        if key in seen:
            continue
        if key not in graph or key not in admitted:
            raise ValueError(key)
        seen.add(key)
        todo.extend(graph[key])
    return frozenset(seen)


def merge_field(base, upstream, local):
    if upstream == local:
        return upstream
    if local == base:
        return upstream
    if upstream == base:
        return local
    raise ValueError("REIMPORT_CONFLICT")


def ascii_fixture_encoding(value):
    # Restricted ASCII/integer fixtures only. This is NOT an RFC 8785 codec.
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


class DesignWitnesses(unittest.TestCase):
    def test_desired_state_alone_does_not_prevent_aba(self):
        opened = True       # original EnsureOpen
        opened = False      # another accepted Close
        opened = True       # blind replay of EnsureOpen: WRONG for a replay
        self.assertTrue(opened)

    def test_occurrence_replay_preserves_later_close(self):
        door = DoorWitness()
        first = Command("a", "g1", True, 0)
        door.apply(first)
        door.apply(Command("b", "g1", False, 1))
        self.assertEqual(door.apply(first), "COMMITTED")
        self.assertEqual((door.open, door.revision), (False, 2))

    def test_same_root_changed_intent_conflicts(self):
        door = DoorWitness()
        door.apply(Command("a", "g1", True, 0))
        self.assertEqual(door.apply(Command("a", "g1", False, 1)), "CONFLICT")
        self.assertEqual((door.open, door.revision), (True, 1))

    def test_retry_must_not_reselect_content(self):
        door = DoorWitness()
        door.apply(Command("a", "g1", True, 0))
        self.assertEqual(door.apply(Command("a", "g2", True, 0)), "CONFLICT")

    def test_stale_new_operation_does_not_mutate(self):
        door = DoorWitness()
        door.apply(Command("a", "g1", True, 0))
        self.assertEqual(door.apply(Command("b", "g1", False, 0)), "STALE")
        self.assertEqual((door.open, door.revision), (True, 1))

    def test_fixture_occupied_close_is_rejected(self):
        door = DoorWitness()
        door.apply(Command("a", "g1", True, 0))
        self.assertEqual(door.apply(Command("b", "g1", False, 1), True), "OCCUPIED")
        self.assertEqual((door.open, door.revision), (True, 1))

    def test_rejected_occurrence_cannot_be_reinterpreted_later(self):
        door = DoorWitness()
        door.apply(Command("a", "g1", True, 0))
        close = Command("b", "g1", False, 1)
        door.apply(close, True)
        self.assertEqual(door.apply(close, False), "OCCUPIED")
        self.assertTrue(door.open)

    def test_no_change_success_is_still_memoized(self):
        door = DoorWitness()
        first = Command("a", "g1", False, 0)
        door.apply(first)
        door.apply(Command("b", "g1", True, 0))
        self.assertEqual(door.apply(first), "COMMITTED")
        self.assertTrue(door.open)

    def test_ambiguous_does_not_mean_rejected(self):
        self.assertTrue(pending_after("AMBIGUOUS"))

    def test_cancellation_request_does_not_prove_retirement(self):
        self.assertTrue(pending_after("CANCELLATION_REQUESTED"))
        self.assertFalse(pending_after("CANCELLED_WITH_RETIREMENT_PROOF"))

    def test_lost_reply_does_not_repeat_model_reward(self):
        ledger = RewardWitness()
        key = claim_key("w", "world-once", "source-a", "reward-a")
        ledger.commit(key, ("item-a", "selected-g1"))
        self.assertEqual(ledger.commit(key, ("item-a", "selected-g1")), "REPLAY")
        self.assertEqual(ledger.value, 1)

    def test_conflicting_reward_payload_is_not_a_retry(self):
        ledger = RewardWitness()
        key = claim_key("w", "world-once", "source-a", "reward-a")
        ledger.commit(key, ("item-a", "selected-g1"))
        self.assertEqual(ledger.commit(key, ("item-b", "selected-g2")), "CONFLICT")
        self.assertEqual(ledger.value, 1)

    def test_retained_model_receipt_survives_model_copy(self):
        ledger = RewardWitness()
        key = claim_key("w", "world-once", "source-a", "reward-a")
        ledger.commit(key, ("item-a", "selected-g1"))
        restored = deepcopy(ledger)
        self.assertEqual(restored.commit(key, ("item-a", "selected-g1")), "REPLAY")
        self.assertEqual(restored.value, 1)

    def test_losing_receipt_is_a_duplication_counterexample(self):
        ledger = RewardWitness()
        key = claim_key("w", "world-once", "source-a", "reward-a")
        ledger.commit(key, ("item-a", "selected-g1"))
        ledger.receipts.clear()  # deliberate broken-design counterexample
        ledger.commit(key, ("item-a", "selected-g1"))
        self.assertEqual(ledger.value, 2)

    def test_world_scoped_source_is_not_multiplied_by_channel(self):
        keys = {claim_key("w", "world-once", "source-a", "reward-a")
                for _channel in ("channel-a", "channel-b")}
        self.assertEqual(len(keys), 1)

    def test_declared_per_channel_eligibility_remains_distinct(self):
        keys = {claim_key("w", channel, "source-a", "reward-a")
                for channel in ("channel-a", "channel-b")}
        self.assertEqual(len(keys), 2)

    def test_new_source_occurrence_is_not_suppressed(self):
        self.assertNotEqual(claim_key("w", "world-once", "a", "slot"),
                            claim_key("w", "world-once", "b", "slot"))

    def test_unselected_unknown_definition_does_not_block_closure(self):
        graph = {"door": ("appearance",), "appearance": (), "future": ("missing",)}
        self.assertEqual(selected_closure(graph, ["door"], {"door", "appearance"}),
                         frozenset({"door", "appearance"}))

    def test_selected_missing_reference_fails(self):
        with self.assertRaises(ValueError):
            selected_closure({"door": ("missing",)}, ["door"], {"door"})

    def test_selected_unadmitted_definition_fails(self):
        with self.assertRaises(ValueError):
            selected_closure({"door": ()}, ["door"], set())

    def test_reference_cycle_terminates_without_implying_execution_safety(self):
        self.assertEqual(selected_closure({"a": ("b",), "b": ("a",)}, ["a"], {"a", "b"}),
                         frozenset({"a", "b"}))

    def test_map_member_order_is_not_semantic_order(self):
        self.assertEqual(ascii_fixture_encoding({"a": 1, "b": 2}),
                         ascii_fixture_encoding({"b": 2, "a": 1}))

    def test_placement_array_order_must_be_preserved(self):
        self.assertNotEqual(ascii_fixture_encoding(["ground", "object"]),
                            ascii_fixture_encoding(["object", "ground"]))

    def test_unchanged_local_field_accepts_upstream_change(self):
        self.assertEqual(merge_field("old", "new", "old"), "new")

    def test_local_correction_survives_unchanged_upstream(self):
        self.assertEqual(merge_field("old", "old", "local"), "local")

    def test_upstream_deletion_against_local_edit_conflicts(self):
        # None denotes deletion only in this fixture, never UNKNOWN semantics.
        with self.assertRaises(ValueError):
            merge_field("old", None, "local")


if __name__ == "__main__":
    unittest.main(verbosity=2)
```
<!-- DESIGN_WITNESS_END -->
