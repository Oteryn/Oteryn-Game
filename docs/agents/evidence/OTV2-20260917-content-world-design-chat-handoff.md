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

This handoff is a navigation/status record. The primary dossier contains the consolidated technical content.

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

## Current recommended next slice

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
