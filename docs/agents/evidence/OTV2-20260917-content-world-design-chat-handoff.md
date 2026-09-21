# OTV2 Content / World design chat handoff — 2026-09-17

```yaml
status: RETAINED_EVIDENCE
repository: Oteryn/Oteryn-Game
branch: agent/content-world-design-dossier-20260917
base_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
primary_dossier: docs/agents/evidence/OTV2-20260917-content-world-design-dossier.md
related_issue: 504
implementation_authority: NONE
production_authority: NONE
```

## Purpose

Retain the substantive owner conversation that evolved the Oteryn content/world design from an initial source-format proposal through interaction/state/durability corrections and a CrystalServer reference audit.

This handoff is a navigation/status record. The primary dossier contains the consolidated technical content. The latest bounded contract candidate is linked in [Phase H](#phase-h--bounded-local-transition-contract-candidate). Phase G retains the preceding execution/state audit.

## Conversation evolution retained

### Phase A — start content migration now

The owner challenged the idea that content must wait for later Server Seam work and asked why existing maps/items/NPCs/quests could not already be transferred into Oteryn.

The resulting conclusion was:

- content extraction/import can and should begin now;
- the missing piece is a scalable source/authoring/import layer feeding the canonical semantic model;
- large catalogues should not be handwritten as Rust structs;
- `ref` work is conflict/behavior qualification, not a manual prerequisite for every simple static field;
- legacy/reference sources can seed candidates with provenance while disputed/gameplay-sensitive fields remain evidence-gated.

### Phase B — design files, attributes and map structure

The first design iteration proposed:

- editable source project separated from compiled runtime bundle;
- stable namespaced keys;
- typed item/creature/NPC/quest definitions;
- chunked semantic map rather than `OTBM 2`;
- explicit ordered placements and distinct visual/collision/interaction footprints;
- source definitions separated from live/durable runtime state;
- deterministic compiler producing client-safe and server-authoritative projections.

### Phase C — higher-reasoning correction

A deeper review corrected several premature choices:

- strict JSON became the initial source candidate instead of automatically selecting JSON5;
- `32x32` became an experiment candidate rather than a frozen permanent chunk size;
- map/source identity was separated from file names and sharding;
- reimport was defined as a three-way comparison rather than overwrite;
- field-level evidence must bind the actual value, not merely the entity name;
- release content became a closed transitive subset rather than requiring the entire candidate catalogue to be complete;
- quest/reward and item/materialization identity semantics were strengthened;
- source/binary/runtime partitioning was separated.

The synthetic demonstrator reached 63 passing tests after corrections. These tests were explicitly classified as prototype validation rather than production proof.

### Phase D — client and interaction semantics

The owner asked how a client/character knows whether an object can be opened, closed, moved or blocks movement.

The retained answer is:

- visuals/sprite IDs never determine authority;
- typed object/placement semantics describe capabilities;
- server-side movement/interaction/item owners resolve legality;
- client receives only safe presentation/state/hints;
- wall, door, container and pickup semantics are distinct;
- effective collision is derived from definitions + current state + dynamic occupancy/policy, not duplicated manually on every tile;
- stateful objects need stable identity and explicit state transitions;
- retries must not accidentally toggle state twice.

### Phase E — CrystalServer reference audit

CrystalServer was inspected as reference/migration evidence only.

The audit found useful coverage for:

- appearance-derived spatial/item properties;
- item parser field inventory;
- real-world movement/pathfinding/height/hangable edge cases;
- action/move-event binding conflicts and shadowing diagnostics;
- map inputs spread across OTBM, sidecars, startup tables and scripts;
- separate runtime spatial sectoring;
- dynamic world-change source data;
- real spawn/creature/NPC field breadth;
- quest storage patterns useful for reverse engineering but unsuitable as canonical Oteryn semantics.

Owner clarification was explicit: **do not copy Crystal code**. Crystal is help/source/reference only; Oteryn is its own Rust design and implementation.

### Phase F — final synthesis

The final synthesis identified the next fundamental problem as execution/state ownership rather than more folder/schema brainstorming.

The next production-shaped proof must connect:

```text
authored definition
 -> typed capability
 -> concrete placement/instance
 -> current runtime state
 -> authoritative validation/mutation owner
 -> client observation/reconciliation
 -> retry/restart/reimport semantics
```

## Existing repository evidence created during the conversation

Two prior #504 comments were published before this retained branch:

- comment `5714569200`: v0.2 continuation, prototype gap corrections and source/reimport design;
- comment `5714818029`: CrystalServer audit and v0.3 implications.

The dossier consolidates those findings plus the later interaction/runtime clarification and final synthesis so repository readers do not need the chat history to reconstruct the design.

## Validation evidence

The conversation used synthetic, non-production design fixtures. The last reported prototype result was:

```text
63 tests passed
```

Material corrected prototype cases included:

- strict lexical end-of-input handling;
- unknown in-memory cell fields;
- empty shards outside world bounds;
- duplicate/conflicting candidate field paths;
- negative-coordinate shard arithmetic;
- half-open boundary cases;
- broader semantic fingerprinting.

Not proven by that result:

- Rust production parser/compiler integration;
- real client/server interaction flow;
- PostgreSQL item/quest durability;
- production bundle technology;
- full-world performance/resource bounds;
- renderer correctness;
- Global Reference parity.

## Original recommended next slice — refined by Phase G

1. source-contract minimum for representative world objects/interactions;
2. pinned legacy-source manifest using existing Game-owned extraction logic where possible;
3. bounded parser/linker/validator with zero-silent-loss and binding-conflict diagnostics;
4. one real corridor with wall/door/transition, ordinary creature, one item and one NPC/service;
5. explicit Reference successor lowering through existing compiler/staging/activation boundaries;
6. real movement + stateful interaction + client reconciliation;
7. durable single-item materialization/transfer with retry/reconnect/restart qualification;
8. broader catalogue ingestion in parallel, but fail-closed promotion of only complete/evidenced subsets.

## Authority boundary

This retained record does not create a new programme or bypass #162 allocation/control-plane authority. It does not select final JSON/JSONL/FlatBuffers/chunk dimensions, mutate `FIRST_PRODUCTION_CONTENT_PROFILE/v1`, register production limits, modify runtime, or authorize deployment/merge.

```text
IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_HANDOFF
FORMAT_ACCEPTANCE: NONE
REGISTRY_MUTATION_AUTHORITY: NONE
PRODUCTION_AUTHORITY: NONE
MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY
```

## Phase G — PR #641 execution/state architecture continuation

Phase-G continuation: [Content / World execution design](OTV2-20260917-content-world-execution-design.md), introduced by commit `c6e4c16d55e864980d0fcddf0c4fda2430cfbce9`. Read it together with the original dossier; the preceding conversation is retained history, not a fresh runtime qualification.

The owner requested continued audit, analysis and architecture work on the existing PR. The continuation inspected protected `main@b44fefe08f6aaf1b2c1c23dedd92bab0de87146e` and predecessor PR head `13e48009cefaa7c7f0ffc7b07390cd5b4cc4eca5`, retaining the same branch and documentation-only scope.

### Source-backed additions and corrections

- Eight bounded audit findings now identify exact source symbols, evidence status and the specific downstream trigger; they are not new global Server Seam blockers.
- The production Content profile lacks ordered cell placements, the GAME-ITEM capability bridge and explicit creature reward/combat bindings. This is a semantic successor problem, not merely insufficient numeric limits.
- Existing Interaction occurrence, proposal and reconciliation machinery is reusable. Its in-memory lifecycle book does not establish durable restart behavior or a composed world-object mutation adapter.
- PR #500 is already merged. GAME-NPC-SERVICE architecture is protected; dialogue/catalogue can be separated from later real trade settlement. Runtime and target-evidence dependencies remain separate.
- Existing Game/Atlas visual projection is not a lossless gameplay IR. Reuse the Game-owned input/decoder lineage while accounting for nested contents, sidecars and binding semantics, rather than adding a parallel OTBM parser or copying Crystal code.
- Desired-state operations alone do not prevent stale replay: A opens, B closes, and replaying A must not reopen the door or overwrite newer client state.
- Coupled irreversible effects need an owning transaction/composition contract; several generic children do not imply atomicity. Cosmetic rebuild, reload and owner changes do not create reward eligibility.
- Source snapshots, selected dependency closure, semantic/presentation/evidence distinctions, activation safety and deletion-aware reimport now have explicit proposed behavior and acceptance witnesses.

### Reproducible validation of this continuation

The addendum embeds the full independent synthetic witness source. It was extracted and run on Python `3.13.5`: **26 tests passed**, including two intentionally reproduced broken-design counterexamples. Witness SHA-256: `9809f28dd0b34e4c2b7c42dadcdeda856d35f8587b487240de9c889d94f5034d`.

Local checks also validated the metadata authority markers, section sequence, code fences, relative links against the connector-inspected path inventory, and added-file whitespace. These are scratch/document checks, not the repository governance validator.

The original **63 tests** remain historical reported evidence and were not rerun. Do not combine them with the 26 witnesses as production coverage. No Rust runtime, real client/server, raw corpus, PostgreSQL, concurrency, production resource or Global-parity qualification was performed by this continuation. Exact-head hosted CI/review must be read from the live PR; predecessor checks do not qualify new commits.

### Current next boundary

The earlier eight-step recommendation is refined into separate, dependency-aware children in addendum section 14: source inventory/bindings, bounded source/linker decision, explicit #504 successor lowering, one current-owner world interaction, then one durable value operation. A plain door must not acquire full quest/economy/Studio dependencies unnecessarily; candidate ingestion stays separate from executable promotion.

The next architectural decision is the minimum **typed source-to-capability binding and local state-transition owner seam**. Production execution still requires its protected owner contract, exact allocation, accepted target fields, admitted resource envelope and real composed tests. This evidence continuation changes none of those authority boundaries and releases no worker.


## Phase H — bounded local-transition contract candidate

The next concrete candidate is [Content / World local-transition contract](OTV2-20260917-content-world-local-transition-contract-candidate.md), introduced in commit `d1e578ca85fa42f97f5d5f4ea26f51e2622058da` on the same PR/branch. It remains `PROPOSED_NONCANONICAL`; no runtime, public API, registry, production or merge authority is granted.

Fresh protected-main inspection narrows Phase G's owner question: the multichannel scope matrix already assigns the public map runtime overlay to ChannelRuntime, and accepted VSL-MOVE keeps position/occupancy in the current ChannelRuntime/InstanceRuntime. The missing deliverable is the typed local-object operation and its composition, not a new owner service. Existing Foundation `CommandIngress` also already distinguishes pending work, retained results and expired outcomes; a second global replay/receipt subsystem is unnecessary.

The candidate defines minimum typed definition/placement/state/transition bindings, an owner-local non-interleaved commit, removal of only the changing object's spatial contribution, current-authority validation before replay, existing FND result/delta/snapshot reconciliation and a bounded first activation policy. It recommends no different Content-generation activation while that scope is live; staging remains separate. Durable keys, rewards, quest state, house ACL, multi-owner effects and hot-reload machinery are not prerequisites for the plain-object child.

Validation on Python 3.13.5: **21 new synthetic model tests PASS**, including **4096 length-four serialized traces**; four deliberately broken model variants were rejected. The predecessor **26-test** model was extracted from its retained document and rerun successfully. The historical 63-test prototype remains unverified here. The new model source is embedded in the candidate, SHA-256 `e1e6cb4713b84586df2293205daa9b7667b52f26c3f8adfff80c1d93e49aadaa`.

These results do not prove production Rust, wire compatibility, concurrency, PostgreSQL, source-parser correctness, resource capacity or Global parity. Current Context assignment, snapshot admission and atomic publication are explicit model assumptions. The full local checkout remains unavailable because container DNS could not resolve github.com; publication uses API-native documentation editing and exact-head hosted checks remain separate.

Phase H supplies the recommendation Phase G left open. The next product-facing boundary is owning review and a fresh exact #162 allocation for the smallest source-to-owner composition, with actual symbols, target evidence, resources and real runtime/client checks. No worker is released by this handoff and no prior pending task is silently reclassified as complete.
