# Oteryn Game Next-Wave Master Plan Hardening

> This document is a normative execution hardening amendment to `docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md`. It narrows and clarifies execution only; it does not expand architecture, runtime, registry, production, Platform, external-repository or owner authority. Live coordinator allocations and live GitHub Issue/PR/CI state remain operationally authoritative.

**Goal:** Close two execution ambiguities found after PR #99 merged: make the production gameplay Server Seam a first-class worker lane with a required child plan, and make Movement resource-limit closure an explicit pre-allocation gate. Also define safe multiagent work packages that can be prepared now and released only when their own gates are satisfied.

**Base:** `main@0ab7c7d08b8e532af1633b7aa80a800b1935cf1a`.

## 1. Server Seam execution hardening

The preparation work in Issue #96 remains decision/allocation-only. Runtime implementation is a separate implementation lane:

```text
lane_id: OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
worker_alias: Oteryn: impl server seam
prompt: docs/agents/prompts/OTV2_IMPL_GAMEPLAY_SERVER_SEAM.md
required_child_plan: docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md
```

Earliest lawful launch condition:

- [ ] Issue #96 decision packet is accepted.
- [ ] A coordinator allocation PR is merged with exact branch, base SHA, owned paths, exclusions and any shared-path lease.
- [ ] Required transport/resource limits are accepted and no gameplay command/state/event ID is invented by this lane.
- [ ] The child Superpowers implementation plan exists before runtime code.
- [ ] Independent exact-head review is pre-declared for protocol/session/admission/fencing changes.

The Server Seam may run in parallel with Durability, Ability, Interaction and AI only when its exact primary paths are disjoint and any shared composition/Cargo/registry mutation is serialized by the coordinator.

After Server Seam merges and is exact-head verified:

```text
Server Seam -> QA Tier 1 expansion
Server Seam -> Native Client allocation
```

It does not imply Movement or Combat availability.

## 2. Movement resource-limit closure hard gate

Issue #93 must not be treated as complete for Movement merely because it inventoried Movement dimensions. Before `OTV2-IMPL-MOVE` receives write authority, the coordinator must perform an explicit Movement resource-limit closure step.

Required closure inventory includes at least:

```text
movement inputs/work per authoritative cycle
movement command payload dimensions not already owned by FND-02
occupancy/spatial query result cardinality and work
same-scope relocation chain depth/work
visibility/interest set cardinality and work
snapshot/delta extensions attributable to Movement
auxiliary path/spatial proposal cardinality/work consumed by the slice
```

Each exercised dimension must end in exactly one terminal state:

```text
REGISTERED_EXACT
CONTRACT_EXACT_UNREGISTERED -> registered before allocation
NOT_APPLICABLE_TO_FIRST_SLICE -> explicitly excluded fail-closed
```

`EVIDENCE_CANDIDATE` and `OWNER_DECISION_REQUIRED` are non-terminal for an exercised Movement dimension and block Movement allocation.

The coordinator may close these values through the existing #93 lifecycle if still active, or through a narrowly scoped follow-up decision/registry task. No new arbitrary number may be selected inside Movement worker code.

## 3. Durability numeric fallback gate

Issue #94 must surface every DUR-03 amplification/count/depth/work dimension exercised by the first Durability increment. If any required finite bound is still unresolved after #94, the coordinator must create or extend an owner-decision/registry gate before `Oteryn: impl durability` starts.

An unresolved required Durability maximum is a blocker, not a note in the topology packet and not permission for the worker to choose a value.

## 4. Preparation-wave semantics

Completion of the whole Preparation Wave is not a global prerequisite for releasing every independent implementation lane.

The coordinator may release a lane immediately after that lane's own Definition of Ready is satisfied:

```text
#93 accepted applicable bounds -> Ability / Interaction / AI may be allocated
#94 topology + applicable bounds -> Durability may be allocated
#96 accepted allocation -> Server Seam may be allocated
```

#95 Content Format Spike and #97 status reconciliation remain important preparation work but are not first-gameplay critical-path blockers unless live governance or a concrete dependency changes that fact.

## 5. Correct critical path

```text
#93 ──> Ability ────────────────────────────────┐
   ├─> Interaction ────────────────┐            │
   └─> AI                          │            │
                                   │            │
#94 ──> Durability ─────────────────────────────┤
                                                │
#96 ──> Server Seam ──> Client ──> QA Tier 2 ──┼─> Movement ─> Combat
                    └────────> QA Tier 1        │
                                   ^            │
Movement hard-max closure ─────────┘            │
                                                │
#95 Content Format Spike = parallel/off first critical path
#97 Status reconciliation = parallel/off first critical path
```

Movement remains serial after Interaction + Client + real-boundary QA capability + applicable Movement limits. Combat remains serial after Movement + Ability + Interaction + Durability + Client + QA.

## 6. Multiagent work packages

The next wave can be prepared for multiple agents now, but preparation is distinct from write release. Every package below is individually gated and receives exact allocation only when lawful.

### Package A — preparation agents, parallel now

These tasks can be researched/prepared concurrently because they are decision/evidence/status work and do not share runtime mutation authority:

| Work package | Source | Output | Runtime write authority |
| --- | --- | --- | --- |
| Resource hard maxima | #93 | decision packet + accepted serialized registry update where permitted | none until later allocations |
| Durability topology | #94 | topology/DB/migration decision + exact allocation proposal | none until later allocation |
| Content format evidence | #95 | bounded spike/evidence dossier | evidence-only |
| Production Server Seam preparation | #96 | exact design/allocation packet | none until later allocation |
| Programme status reconciliation | #97 | verified status prose | docs-only |

Recommended maximum: five parallel preparation workers plus one coordinator, subject to current allocation/path checks.

### Package B — implementation agents after individual gates

The coordinator may prepare branches/task templates/prompts in advance, but write authority activates separately:

| Alias | Gate | Parallelism |
| --- | --- | --- |
| `Oteryn: impl durability` | #94 accepted + exact allocation + applicable DUR bounds | Ability, Interaction, AI, Server Seam if paths/leases disjoint |
| `Oteryn: impl ability` | #93 applicable limits terminal + exact allocation | Durability, Interaction, AI, Server Seam if disjoint |
| `Oteryn: impl interaction` | #93 applicable limits terminal + exact allocation | Durability, Ability, AI, Server Seam if disjoint |
| `Oteryn: impl ai` | #93 applicable limits terminal + exact allocation | Durability, Ability, Interaction, Server Seam if disjoint |
| `Oteryn: impl server seam` | #96 accepted + exact allocation + child plan | D/A/I/AI when shared-path mutations are serialized |

Recommended maximum useful implementation concurrency remains five substantial workers plus the coordinator.

### Package C — integration agents, staged

```text
Server Seam merged -> QA Tier 1 + Client
Client merged -> QA Tier 2
Interaction + Client + QA capability + Movement limits terminal -> Movement
Movement + Ability + Interaction + Durability + Client + QA -> Combat
```

Client, Movement and Combat must not consume unmerged sibling output as if it were a dependency.

## 7. Coordinator requirements before dispatching a multiagent wave

Before dispatching any worker set, the coordinator must publish/merge one allocation state that proves:

- [ ] exact current `main` SHA;
- [ ] one task ID/branch/base SHA per worker;
- [ ] exact owned paths and exclusions;
- [ ] no primary-path overlap;
- [ ] shared Cargo/workspace/composition/registry leases serialized;
- [ ] predecessor SHAs are on `main`;
- [ ] every exercised external/content-controlled resource dimension is terminally bounded or excluded fail-closed;
- [ ] required child plan path for each implementation lane;
- [ ] review class and independent-review requirement before first write;
- [ ] merge order and what later workers may consume only after merge.

Prepared prompts/branches without this merged allocation are not write authority.

## 8. Updated launch matrix amendment

Add this row to the master launch matrix:

| Alias | Earliest lawful launch condition | Can run in parallel with |
| --- | --- | --- |
| `Oteryn: impl server seam` | #96 accepted + exact implementation allocation + required child plan | Durability, Ability, Interaction, AI when exact paths are disjoint and shared mutations are serialized |

Amend `Oteryn: impl movement` launch condition to require explicit terminal Movement resource-limit closure, not inventory alone.

## 9. Updated coordinator sequence

Use this sequence while overlapping independent work:

1. Re-read current `main`, live allocations, Issues and open PRs.
2. Run/finish #93, #94, #95, #96 and #97 in parallel where allocations permit.
3. Release Ability/Interaction/AI as soon as their #93 dimensions are terminal and exact allocations merge.
4. Release Durability as soon as #94 topology plus applicable numeric gates are terminal and exact allocation merges.
5. Release `Oteryn: impl server seam` immediately after #96 acceptance/allocation; do not wait for all generic engines if paths are disjoint.
6. After Server Seam merge, release QA Tier 1 and Client as separately allocated work.
7. Close every applicable Movement hard maximum before Movement allocation.
8. After Client and Interaction merge and QA can exercise Tier 1/Tier 2, release Movement.
9. Release Combat only after merged Movement plus Ability/Interaction/Durability and remaining canonical prerequisites.
10. Keep Channel, Analytics and permanent Content-format adoption behind their later explicit gates.

## 10. Definition of Done for this amendment

This hardening is complete only when:

- [ ] the Server Seam executor prompt exists and is registered in the reusable prompt index;
- [ ] Movement hard-max closure is explicit and fail-closed;
- [ ] Durability unresolved numeric dimensions cannot silently escape #94;
- [ ] multiagent packages preserve one-writer/shared-path serialization and merged-predecessor rules;
- [ ] no new runtime/registry/production authority is introduced by this documentation change;
- [ ] repository governance, diff checks and exact-head CI pass before merge.
