# Oteryn v2 — Implementation Executor DAG

- Programme ID: `OTV2-NATIVE-IMPLEMENTATION`
- Status: `RELEASE_CANDIDATE`; becomes canonical/released only when PR #314 lawfully merges to `main`.
- Canonical repository: `blakinio/Oteryn-v2`.
- Reconciled architecture base: `main@3ed4ca602f389d5a8549e0fc19dcc688a7b7a78c`.
- External repositories: read-only unless separately owner-authorized.
- Production/protected-environment/live-data authority: **NONE**.
- Reference parity authority: **NONE**.
- Direct worker write authority without a live coordinator allocation: **NONE**.

## 1. Purpose

This programme converts the accepted Oteryn-v2 architecture into a bounded implementation sequence without allowing workers to make unresolved architecture decisions inside code.

The implementation is coordinator-led and dependency-driven. Workers do not choose final crate/service topology before the bootstrap cutover establishes real repository paths and machine-enforced ownership.

Merging this document releases a **coordination programme**, not implementation work by itself. No worker starts, writes or receives an allocation merely because this DAG exists on `main`.

## 2. Global invariants

Every lane MUST preserve:

- native Rust client/server stack and one `protocol-oteryn` gameplay protocol;
- server-authoritative legality, ordering, movement, combat, items and value;
- FND-02 command/sequence/revision/resync semantics;
- FND-03 one logical authoritative mutation owner per runtime scope;
- FND-04 admission/GameSession/CharacterLease/fencing semantics;
- DUR-01/02/03 durability, idempotency, anti-duplication and crash-recovery semantics;
- DUR-04 content/compiler/bundle/script authority and final-format evidence gate;
- SIM deterministic numeric/RNG/order/replay requirements;
- accepted GAME-CHANNEL / GAME-CHAR / GAME-ITEM semantics;
- accepted GAME-ABILITY / GAME-INTERACTION / GAME-AI / ALPHA-CLIENT / ANL contracts;
- accepted/lifecycle-closed Stage-C movement/combat/content architecture;
- QA-E2E real-boundary proof rules;
- `PROVEN / DERIVED / UNKNOWN / CONFLICT` evidence discipline;
- current root governance, including its AI/review authorization boundaries;
- no direct Codex/owner-funded AI invocation by a repository agent without exact authorization for that use;
- no production, Platform write, protected secrets, live account/session/data or external-repository mutation without separate explicit authority.

Reference-unknown behavior MUST remain fail-closed or use an explicitly test-only non-shipping fixture profile. A fixture is never Reference parity evidence.

## 3. Why bootstrap is serial

At reconciliation, `main` remains intentionally `pre-native-protocol`:

- only `apps/client` exists as an application;
- `workspace-boundaries.toml` enumerates exactly 19 current members;
- `tools/architecture-check` enforces the current pre-native shape;
- current policy forbids package-name fragments including `protocol-oteryn`, `transport`, `game-session`, `game-server`, `persistence`;
- Rust CI contains client-only negative closure checks;
- FND-02 registry intentionally has empty gameplay `command_types`, `state_domains` and capabilities;
- game-event registry intentionally has no domain event types.

Therefore the first implementation PR MUST be an atomic, reviewed transition from the pre-native policy to a real immediate-consumer implementation shape. Creating speculative empty crates first is forbidden.

These are verified live facts at the #314 reconciliation base, not a promise that they remain forever. The implementation coordinator must re-read live `main` before allocation and adapt only through accepted architecture/governance.

## 4. Canonical DAG

```text
OTV2-IMPL-COORD
  |
  v
OTV2-IMPL-BOOTSTRAP                                    SERIAL GATE
  |
  +--> OTV2-IMPL-FOUNDATION                            protocol/runtime/admission
  +--> OTV2-IMPL-SIM                                   deterministic numeric/RNG/time/replay core
  +--> OTV2-IMPL-DOMAIN                                Character/Item semantic core
  +--> OTV2-IMPL-CONTENT                               minimum VSL compiler/loader
  +--> OTV2-IMPL-QA                                    evidence platform shell as seams appear

FOUNDATION + DOMAIN
  +--> OTV2-IMPL-DURABILITY                            persistence/idempotency/value substrate

FOUNDATION + SIM + DOMAIN + CONTENT
  +--> OTV2-IMPL-ABILITY                               one typed effect engine
  +--> OTV2-IMPL-INTERACTION                           retry-safe trigger/workflow engine
  +--> OTV2-IMPL-AI                                    bounded proposal-only AI/spawn/path engine

FOUNDATION + compatible client protocol seam
  +--> OTV2-IMPL-CLIENT                                native gameplay client integration

FOUNDATION + SIM + DOMAIN + CONTENT + INTERACTION + CLIENT + QA
  +--> OTV2-IMPL-MOVE                                  first authoritative movement slice

MOVE + FOUNDATION + SIM + DOMAIN + CONTENT + ABILITY + INTERACTION + DURABILITY + CLIENT + QA
  +--> OTV2-IMPL-COMBAT                                death/loot/XP/pickup slice

FOUNDATION + DOMAIN + DURABILITY
  +--> OTV2-IMPL-CHANNEL                               later channel switching/product-policy lane

CONTENT semantic/compiler seam
  +--> OTV2-CONTENT-FORMAT-SPIKE                       evidence-only

concrete producer event registrations exist
  +--> OTV2-IMPL-ANALYTICS                             later read-only analytics lane
```

The coordinator MAY overlap lanes only after bootstrap has merged and an exact path/allocation record proves their owned paths do not overlap. Public registry/workspace-policy/stable-ID mutations are serialized even when code workers otherwise run in parallel.

Movement and Combat are serial integration gates for the first vertical-slice proof. Combat MUST consume a merged, integration-ready Movement slice plus already-merged generic engines rather than implementing SIM/Ability/Interaction/AI/Domain/DUR/Movement architecture incidentally inside the Combat VSL.

## 5. `OTV2-IMPL-BOOTSTRAP`

Create the first real server-side implementation shape and update repository policy/tooling atomically so accepted implementation has immediate consumers and CI can validate it.

The coordinator allocates exact paths. The lane normally includes the minimum necessary set from:

- root Cargo workspace membership/dependencies;
- `workspace-boundaries.toml`;
- `tools/architecture-check` policy model/tests;
- Rust/merge CI assumptions affected by moving beyond pre-native state;
- minimum real server application/composition root and immediate-consumer seams required by accepted FND contracts;
- nearest `AGENTS.md` files for newly high-risk ownership areas;
- focused bootstrap tests and migration/readiness documentation.

No gameplay mechanics, PostgreSQL production schema, broad client gameplay enablement, final content format, fake protocol adapter or placeholder crate with no real consumer.

Completion requires code and machine policy to agree on one exact merged workspace shape with build/test/Clippy/supply-chain CI green and Canary still absent.

## 6. `OTV2-IMPL-FOUNDATION`

Implement the minimum FND-ID/FND-02/FND-03/FND-04 stack necessary for a real local game-server endpoint and native client/server test path:

- typed identifiers;
- FND-02 foundation schema/codegen/codec/framing/TLS profile and accepted resource limits;
- CommandRef, connection generation, server sequence, state revision, snapshot/delta/resync foundation;
- authoritative runtime owner/lane/lifecycle scaffolding;
- admission/GameSession/CharacterLease/reconnect fencing;
- typed foundation errors and failure scenarios;
- no gameplay command/state IDs invented by this lane.

Protocol/admission/session changes are high risk and require genuinely independent final review plus negative/replay/fencing evidence.

## 7. `OTV2-IMPL-SIM`

Implement protocol/persistence/UI-neutral deterministic simulation machinery consumed by gameplay lanes:

- simulation profile revision;
- checked exact/fixed-scale numeric helpers and named rounding semantics;
- purpose-isolated retry-stable gameplay RNG;
- semantic time normalization;
- stable deterministic ordering/comparators;
- canonical state/hash/replay-support seams.

SIM does not own gameplay formulas or product values.

## 8. `OTV2-IMPL-DOMAIN`

Implement protocol/persistence-neutral semantic domain core required by the first slice:

- Character identity/lifecycle/build/progression primitives needed by VSL;
- Item definition/instance legality and typed semantic location/custody vocabulary;
- exact revision/profile/content interpretation seams;
- typed errors/transitions;
- explicit VSL fixtures where Reference facts remain unknown.

GAME-CHAR/ITEM semantics remain distinct from physical DB mechanics, wire representation and UI.

## 9. `OTV2-IMPL-DURABILITY`

Implement accepted profile-neutral durability needed by first native runtime/VSL work:

- durable ID representation;
- migration/test substrate where explicitly implementation-authorized;
- Character/session persistence adapters required by current consumers;
- DUR-03 stable TransactionId/OperationId/idempotency/reconciliation/anti-dup semantics;
- typed item/value durability and runtime↔durable prepare/commit/reconcile seam;
- outbox/audit coupling where mandatory;
- crash/restart/concurrency/rollback evidence.

No market/bank/depot/entitlement breadth. High-risk persistence/value changes require genuinely independent review.

## 10. `OTV2-IMPL-CONTENT`

Implement the minimum typed canonical semantic graph and deterministic compiler/loader seam required by Stage-C:

- stable content keys/revisions/provenance;
- bounded synthetic/VSL content fixture builder/source;
- deterministic server-authoritative and allowlisted client-safe projections;
- non-production `VSL_BUNDLE_EVIDENCE_PROFILE` or equivalent;
- corruption/oversize/incompatibility rejection;
- staged all-or-nothing activation;
- fixture world/collision/relocation/creature/spawn/ability/loot/XP/item/presentation data required by VSL.

The lane MUST NOT select permanent World Project/Bundle encoding.

## 11. `OTV2-IMPL-ABILITY`

Implement the owner-accepted one typed authoritative ability/effect engine:

- revision-bound occurrence lineage;
- legality/target/cast/channel/commit semantics;
- owner-scoped commit groups;
- cooldown/charge/condition state;
- typed damage/heal/effect composition;
- bounded future/repeated work and continuation/recovery;
- deterministic reaction/proc descendants;
- proposal-only client/content/script/AI adapters.

Exact Reference formulas remain evidence-gated.

## 12. `OTV2-IMPL-INTERACTION`

Implement bounded successor-child/trigger/retry/reconciliation workflows:

- stable recursive child identity;
- deterministic ordering/RNG;
- truthful pending/committed/rejected outcomes;
- bounded recursion/work;
- typed trigger dispatch;
- explicit adapters to Movement/Ability/Item/DUR owners;
- no generic distributed transaction or foreign-state direct mutation.

## 13. `OTV2-IMPL-AI`

Implement deterministic bounded AI/spawn/path proposals:

- finite versioned AI state;
- bounded deterministic perception/decision;
- proposal-only pathfinding with stale-result rejection;
- spawn lifecycle/retry/provenance;
- typed intents routed through Movement/Ability;
- no direct value/position/effect authority.

## 14. `OTV2-IMPL-CLIENT`

Implement native gameplay integration only against real merged Foundation/domain seams:

- production protocol consumer;
- admission/session/reconnect;
- semantic input to typed command intent;
- authoritative result/state reconciliation;
- client-safe content revisions;
- settings/privacy/diagnostics;
- no client-authoritative gameplay;
- fail-closed capability until all requirements are compatible.

Tier 2 evidence is mandatory for supported client journeys.

## 15. `OTV2-IMPL-QA`

Implement the smallest real-boundary QA-E2E platform required to prove Foundation and VSL work:

- deterministic scenario/evidence model;
- exact artifact/revision/seed/topology/fault evidence;
- Tier 1 server/protocol/persistence journeys;
- Tier 2 instrumented native-client journeys;
- cleanup and failure evidence;
- no mock success accepted as terminal proof;
- no test adapter in production artifacts.

QA may evolve incrementally as real production seams merge.

## 16. `OTV2-IMPL-MOVE`

Implement the accepted Stage-C movement slice by consuming Foundation/SIM/Domain/Content/Interaction/Client/QA:

- owner-local authoritative step and same-scope relocation;
- static exact-revision collision/spatial legality;
- dynamic current-runtime legality;
- stable occurrence lineage;
- post-movement Interaction children;
- bounded deterministic visibility/interest state domain;
- owning FND-02 command/result/state registration;
- client intent/reconciliation;
- deterministic component tests plus real Tier 1/Tier 2 evidence.

Cross-scope handoff remains separate.

## 17. `OTV2-IMPL-COMBAT`

Implement the accepted first creature combat/death/value slice by consuming merged Movement and generic engines:

- merged `OTV2-IMPL-MOVE` is a hard first-slice prerequisite;
- GAME-ABILITY is the only effect pipeline;
- one stable death occurrence per creature lifecycle generation;
- deterministic SIM loot selection with exact content revisions;
- DUR-03 durable loot materialization/reconciliation;
- separate idempotent single-principal Character XP settlement;
- Interaction + Item + DUR pickup;
- owning protocol registrations and authoritative client projection;
- crash/lost-response/retry/no-dup tests plus Tier 1/Tier 2 evidence.

Fixture formula/rate values remain non-shipping until Reference evidence is promoted. Independent review is mandatory for exercised durable loot/value invariants.

## 18. `OTV2-IMPL-CHANNEL`

Later, after Foundation/Domain/Durability, implement accepted game-domain Channel product semantics:

- typed ChannelRef/product identity;
- recommendation/explicit-target semantics;
- optional bounded pre-admission queue semantics where allocated;
- reconnect vs voluntary switch classification;
- hard switch locks;
- durable Character+World anti-hopping guard transitions;
- destination admission/guard consistency.

PERF owns numeric capacity; OPS owns production orchestration/hysteresis. Exact switch cooldown stays blocked until accepted numeric product evidence exists. High-risk session/channel semantics require independent review.

## 19. Evidence lane — `OTV2-CONTENT-FORMAT-SPIKE`

Compare bounded physical representation candidates against accepted DUR-04/ADR-0005 criteria: deterministic output, diffability, partial/atomic authoring, bounded parsing/decompression, streaming locality, patchability, compatibility, corruption recovery and tooling ergonomics.

Keep fixtures legally safe and produce a decision dossier. The spike MUST NOT silently make its winner the permanent format.

## 20. Later lane — `OTV2-IMPL-ANALYTICS`

Do not start full analytics while concrete domain producer event families are absent.

After producers register typed events, implement read-only ANL-02/03 ingestion/quality/invariant/reporting. Analytics may not mutate gameplay, sanction players or invent producer schemas. Missing/partial evidence remains fail-closed.

## 21. Coordinator responsibilities

`OTV2-IMPL-COORD` MUST:

1. read live main and this programme before every allocation wave;
2. maintain one canonical implementation allocation/status record with exact branches, PRs, owned paths, dependencies and merge order;
3. serialize public-contract/registry/workspace-policy/stable-ID mutations;
4. prevent two workers from owning the same path or stable ID range concurrently;
5. merge only exact-head validated PRs under repository policy;
6. require high-risk independent reviews where root `AGENTS.md` requires them;
7. run real-boundary E2E before marking a VSL implementation `PROVEN`;
8. never convert architecture acceptance into Reference parity or production enablement;
9. keep `PROD-ENTITLEMENTS-01` excluded until separately accepted;
10. stop on a genuinely unresolved owner/authority decision rather than letting a worker improvise.

## 22. Suggested invocation order

After this programme is released by a merged prompt-package delivery, the user should normally start only:

```text
Oteryn: implementation coordinator
```

Coordinator sequence:

```text
1. Bootstrap serially.
2. Allocate Foundation + SIM + Domain + Content + QA where paths permit.
3. Allocate Durability after Foundation/Domain seams are stable enough.
4. Allocate Ability + Interaction + AI after Foundation/SIM/Domain/Content.
5. Allocate Client after compatible production Foundation seam exists.
6. Allocate Movement after its prerequisites are integration-ready and merge it as the first gameplay integration gate.
7. Allocate Combat only after Movement plus Ability/Interaction/Durability and its other prerequisites are integration-ready.
8. Allocate Channel later when multichannel product implementation is needed and numeric prerequisites permit.
9. Run Content Format Spike as evidence; run Analytics only after producer events exist.
```

Direct worker aliases exist for recovery/manual allocation but MUST verify an active coordinator allocation naming their exact lane and paths before writing.

## 23. Release semantics

A lawful merge of PR #314 changes programme availability from prepared to released. It authorizes only reuse of the coordinator prompt/programme under the repository's normal task/allocation governance.

It does **not**:

- create an implementation allocation;
- start Bootstrap or any worker;
- authorize direct worker writes;
- authorize production, protected-environment, live-data, Platform or external-repository mutation;
- authorize owner-funded AI use;
- select the permanent content format;
- establish Reference parity;
- accept `PROD-ENTITLEMENTS-01`.

After merge, the safe normal entry remains an explicit user invocation of `Oteryn: implementation coordinator` (or an equivalent explicit request to start that programme). Direct worker aliases remain allocation-gated.

## 24. Completion truth

`architecture accepted` != `implementation complete` != `Reference parity` != `production ready`.

A lane is complete only when its owned implementation, focused tests, required integration/E2E, independent review where applicable, exact-head CI, merge, task archive and ownership release are all terminally complete.
