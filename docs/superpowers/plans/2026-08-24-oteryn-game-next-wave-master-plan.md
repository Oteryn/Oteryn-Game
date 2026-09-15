# Oteryn Game Next-Wave Master Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move Oteryn Game from the merged Foundation/Domain/Content/QA foundation to the first real authoritative Movement and Combat vertical slices without allowing parallel workers to invent unresolved architecture, resource limits, protocol authority, persistence topology, or E2E evidence.

**Architecture:** GitHub remains the technical source of truth and the implementation coordinator is the only lane that releases write authority. Preparation decisions are separated from executable implementation, shared paths/registries/stable IDs stay serialized, generic engines run in parallel only after their explicit gates are satisfied, and Movement/Combat remain serial integration gates.

**Tech Stack:** Rust 1.94 workspace, `apps/game-server`, native Rust client, PostgreSQL target per ADR-0004, FND-02/FND-03/FND-04, DUR-01/02/03/04, SIM-DETERMINISM, GAME-* contracts, ADR-0007 QA E2E, GitHub Actions `game-gate`, Superpowers planning/TDD/debugging/verification workflows.

**Spec:** `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`

**Operational truth:** `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` plus live GitHub Issue/PR/CI state outrank this plan when they disagree.

## Global Constraints

- Canonical repository: `Oteryn/Oteryn-Game`.
- Authoring snapshot: `main@dc22e0da8efcc6f4458416191261063b295af5b4` after QA shell PR #98.
- This document is an orchestration-level master plan, not a lane implementation plan. No runtime implementation worker executes code directly from this document; every concrete implementation lane first gets a lane-specific child Superpowers plan after its exact allocation/topology is known.
- No worker writes without a merged coordinator allocation naming exact owned paths, branch, task, base SHA and exclusions.
- One substantial task = one branch = one PR. No direct pushes to `main`.
- Public registries, stable IDs, shared workspace paths, Cargo topology, architecture policy and workflows are serialized coordinator mutations even when code workers otherwise run in parallel.
- Exact numeric hard maxima required by accepted contracts are never inferred from generic limits and never selected independently by implementation workers.
- Production/protected/live-data/Platform/external-repository authority is absent unless separately granted.
- Reference `UNKNOWN / CONFLICT / PENDING` behavior remains fail-closed or explicitly non-shipping fixture behavior.
- Tier 1 requires real production server/protocol/persistence boundaries; Tier 2 requires the native client boundary. Mock/synthetic/direct-domain success never becomes terminal Tier 1/2 proof.
- Mandatory whole-diff self-review applies to every delivery. Genuinely independent exact-head review is required whenever root governance requires it, especially protocol/session/admission/fencing, persistence, item/loot/value and multichannel changes.
- Completion claims require fresh exact-head verification, required CI, zero unresolved review threads, expected-head merge, post-merge verification, task archive and ownership/lease release.

---

## 1. Proven Starting State

| Area | State at authoring snapshot | Evidence / consequence |
| --- | --- | --- |
| Bootstrap | DONE | PR #10 + closeout #11 |
| Simulation core | DONE | PR #14, merge `66619daf5837f31f7c54676e9f8351ed4ae220b0` |
| Foundation | DONE technically | PR #59; post-merge independent audit PASS; historical mandatory pre-merge exact-head review remains `NOT_PROVEN` |
| Domain | DONE | PR #56 + lifecycle closeout |
| Content evidence seam | DONE for non-production evidence | PR #58 plus P0 repair #87 and closeout #89; production Content remains separately owner-gated by Issue #54 |
| QA evidence shell | MERGED | PR #98, exact tested head `8c736d4c3aff0e91694748a254df1a20b3dcf176`; 17/17 focused tests PASS; real gameplay Tier 1/2 still `NOT_EVALUATED` |
| Durability | NOT IMPLEMENTED | architecture accepted; topology/allocation gate remains |
| Ability | NOT IMPLEMENTED | architecture accepted; resource hard-max gate remains |
| Interaction | NOT IMPLEMENTED | architecture accepted; resource hard-max gate remains; hard prerequisite for Movement |
| AI | NOT IMPLEMENTED | architecture accepted; resource hard-max gate remains; not a hard prerequisite for first Movement/Combat DAG |
| Production gameplay listener/client-entry seam | ABSENT | direct blocker for Client and real Tier 1 server journey |
| Native gameplay client | BLOCKED | waits for compatible merged production server seam |
| Movement | BLOCKED | waits for Interaction + Client + real QA boundary readiness |
| Combat | BLOCKED | waits for Movement + Ability + Interaction + Durability + Client + QA |

**Known preparation Issues:** #93 resource hard maxima, #94 Durability topology, #95 Content Format Spike, #96 production gameplay server seam, #97 programme-status reconciliation.

**Important drift:** the live-allocation prose at the authoring snapshot still describes QA as branch-only even though PR #98 is merged. Issue #97 exists specifically to reconcile maintained status overlays without changing architecture.

---

## 2. Critical Path and Parallel Work

```text
                     +--> #94 Durability topology --> Durability --------+
                     |                                                  |
#93 hard maxima -----+--> Ability --------------------------------------+--> Combat
                     |                                                  |
                     +--> Interaction ------------------+               |
                                                       |               |
#96 prep -> Server Seam impl ------> Native Client ------+--> Movement --+
                                                       |       ^
QA shell #98 ------------------------------------------+-------|
                                                               |
                                  real Tier 1/2 QA -------------+

#93 has two lawful release moments: the applicable Ability/Interaction/AI subset may close early, while every Movement-exercised dimension must close before Movement allocation. AI is not a hard dependency of first Movement/Combat.
#96 preparation is followed by a dedicated allocated Server Seam implementation worker; that worker may overlap path-disjoint generic engines.
#95 Content Format Spike is evidence-only and not on the first gameplay critical path.
#97 status reconciliation is documentation hygiene and may run in parallel with preparation; neither #95 nor #97 is a global barrier to a path-disjoint lane whose own gate is already satisfied.
```

### Parallelism rule

The coordinator may run the preparation work for #93, #94, #95, #96 and #97 concurrently because they produce decision/evidence/status artifacts rather than overlapping runtime mutations. Executable Ability/Interaction/AI/Durability/Server-Seam work starts only after each lane's own decision/allocation gate is merged. Preparation-wave completeness is tracked separately from lane release: unfinished #95 or #97 must not delay a path-disjoint implementation lane that already satisfies its own Definition of Ready.

### Maximum useful implementation concurrency

Use at most five substantial implementation workers at once. The recommended generic-engine wave is four primary workers — Durability, Ability, Interaction and AI — while the coordinator remains separate. A fifth worker may implement the production server seam only if its exact path allocation is disjoint from those four and no serialized shared-path/registry mutation is concurrently owned.

---

### Task 1: Reconcile Maintained Programme Status (#97)

**Files:**
- Modify: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`
- Modify only if verification proves necessary: directly related status/index overlays named by Issue #97
- Do not modify: accepted architecture contracts, registries, runtime code, Cargo/workspace files

**Interfaces:**
- Consumes: merged `main`, live allocations, PR #98, Foundation audit history
- Produces: maintained status prose that distinguishes `ACCEPTED`, `IMPLEMENTED`, `PROVEN`, `BLOCKED` and `NOT_EVALUATED`

- [ ] **Step 1: Freeze the exact status base**

Run:

```bash
git fetch origin --prune
git rev-parse origin/main
```

Expected: record one exact `main` SHA in the task and use GitHub PR/CI evidence for every changed status statement.

- [ ] **Step 2: Correct only verified drift**

Required truth after reconciliation:

```text
Bootstrap = implemented
bounded SIM core = implemented
Foundation = implemented; post-merge audit PASS; historical pre-merge review NOT_PROVEN
Domain = implemented
Content evidence seam = implemented/non-production; repair #87 terminal
QA evidence shell = merged; gameplay Tier 1/2 NOT_EVALUATED
Durability/Ability/Interaction/AI/Client/Movement/Combat = not implemented on main
production gameplay server seam = absent
production Content = separately owner-gated
```

- [ ] **Step 3: Validate documentation governance**

Run:

```bash
python tools/agents/validate_governance.py
git diff --check
```

Expected: both PASS.

- [ ] **Step 4: Review and merge**

Require whole-diff self-review, exact-head repository checks including `game-gate`, zero unresolved threads, squash merge and post-merge readback. Independent implementation review is not required unless the diff unexpectedly changes an architecture/runtime authority statement rather than status prose.

---

### Task 2: Decide Wave-2 Gameplay Resource Hard Maxima (#93)

**Files:**
- Create: `docs/architecture/reviews/OTERYN_GAME_WAVE2_RESOURCE_LIMITS_DECISION_PACKET_2026-08-24.md`
- Modify only after owner/coordinator acceptance: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- Create/update the exact coordinator task record allocated for #93

**Interfaces:**
- Consumes: accepted GAME-ABILITY-01, GAME-INTERACTION-01, GAME-AI-01, VSL-MOVE-01 and current `RESOURCE_LIMITS_REGISTRY.json`
- Produces: one classified inventory of required dimensions and, only after accepted evidence/owner decision, exact registered maxima with units/failure behavior/boundary tests

- [ ] **Step 1: Inventory every required dimension**

Classify each dimension exactly as:

```text
REGISTERED_EXACT
CONTRACT_EXACT_UNREGISTERED
EVIDENCE_CANDIDATE
OWNER_DECISION_REQUIRED
NOT_APPLICABLE_TO_FIRST_SLICE
```

Ability minimum inventory: target candidates/resolved targets, geometry/query work, dynamic retarget depth, Effect Plan cardinality/size, calculation stages, multi-hit/target work, channel/periodic/future work, conditions, reactions/descendants/root work and cross-domain proposals.

Interaction minimum inventory: cascade depth, fan-out, total descendant work, outstanding delegated owner operations, reconciliation/retry work and content-controlled trigger/child work.

AI minimum inventory: authored/evaluation work, candidate/perception/memory work, pending timers/operations, path queue/search/route limits, repath/retry work, spawn work and postponed occupancy retries.

Movement inventory for staged closure before Movement allocation: movement work per cycle, occupancy/query results, relocation depth/work, visibility/interest counts, snapshot extensions and auxiliary path/spatial proposals. Inventory may be prepared with the first #93 decision packet, but every Movement dimension exercised by the first slice must later be accepted/registered or explicitly excluded fail-closed before the coordinator may allocate Movement.

- [ ] **Step 2: Reject accidental product policy**

For every missing exact value, record:

```text
unit
owning contract
amplification/control source
failure category
allocation impact
client visibility
boundary tests
evidence candidate or owner-decision requirement
```

Do not copy generic FND frame/count ceilings into gameplay semantic work limits unless the owning contract explicitly says they are the same resource.

- [ ] **Step 3: Register only accepted values**

If exact values receive accepted evidence/owner approval, update `RESOURCE_LIMITS_REGISTRY.json` in a separate serialized coordinator mutation. Validate JSON, registry policy, governance and boundary-test obligations before any engine allocation consumes the new entries.

- [ ] **Step 4: Release or narrow generic-engine lanes**

Ability, Interaction and AI may receive implementation allocations as soon as every resource dimension exercised by their own first slice is either registered with an accepted hard maximum or explicitly excluded fail-closed from that slice. They do not wait for unrelated Movement-only dimensions to close.

- [ ] **Step 5: Close the Movement resource gate before Movement allocation**

Before the coordinator allocates `Oteryn: impl movement`, re-read the #93 inventory against the exact proposed Movement child plan. Every Movement-exercised count/depth/work/size dimension must then be `REGISTERED_EXACT` or explicitly excluded fail-closed from that slice. If accepted evidence or an owner decision is still missing, keep Movement blocked and continue the #93 decision/serialized-registry lifecycle rather than letting the Movement worker choose a number.

---

### Task 3: Decide First Durability Implementation Topology (#94)

**Files:**
- Create: `docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md`
- Create/update the exact coordinator task record allocated for #94
- No runtime/Cargo/migration write before the later implementation allocation

**Interfaces:**
- Consumes: ADR-0004, DUR-01, DUR-02, DUR-03, FND-03/FND-04, GAME-CHAR, GAME-ITEM, ANL-01, current workspace/Cargo topology
- Produces: exact first-increment paths, DB/migration technology decision, migration ledger ownership, test DB strategy and shared-path lease requirements

- [ ] **Step 1: Evaluate the bounded default topology**

Evaluate this first, rather than creating a speculative new crate:

```text
apps/game-server/src/durability/**
+ one game-owned migration ledger
+ isolated non-production PostgreSQL test infrastructure
+ explicit Cargo/lockfile/shared-path lease only where needed
```

A dedicated crate is justified only by a demonstrated immediate-consumer boundary that the game-server module cannot satisfy safely.

- [ ] **Step 2: Select implementation technology with compatibility evidence**

The packet must compare the viable Rust PostgreSQL/migration candidates against Rust 1.94 compatibility, migration immutability, compile/supply-chain impact, async runtime integration, test isolation and maintenance/security posture. The packet selects one implementation path; implementation workers do not select it later by convenience.

- [ ] **Step 3: Freeze correctness boundaries**

The packet must define:

```text
one authoritative game migration history
dedicated migration execution; no production startup auto-DDL
fail-closed schema/migration compatibility check
isolated test database lifecycle
async PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE flow
no synchronous DB/network blocking inside FND-03 writer lane
stable TransactionId/OperationId semantics
ambiguous commit reconciliation
atomic durable audit/outbox where required
```

- [ ] **Step 4: Produce the exact implementation allocation proposal**

List every runtime, migration, test, Cargo/lockfile and shared path needed by the first increment. Mark DUR-03 resource dimensions that remain blocked by hard-max decisions. Any such missing bound is an explicit blocker: route it through #93 or a separately owner-approved numeric decision/serialized-registry task before Durability implementation allocation. Only a later merged coordinator allocation converts this topology decision into write authority.

---

### Task 4: Run the Content Physical-Format Evidence Spike (#95)

**Files:**
- Create under allocation: `tools/content-format-spike/**`
- Create: `docs/architecture/reviews/OTERYN_GAME_CONTENT_FORMAT_SPIKE_2026-08-24.md`
- Create/update the exact spike task record
- Do not modify permanent production Content format/activation authority

**Interfaces:**
- Consumes: ADR-0005, DUR-04, merged Content semantic/compiler seam, `docs/migration/CRYSTAL_WORLD_CONTENT_MIGRATION_DESIGN_CHECKPOINT.md`
- Produces: repeatable evidence comparing 2-3 bounded physical representation candidates and one explicit owner decision dossier

- [ ] **Step 1: Allocate the evidence-only spike**

Use alias only after the exact coordinator allocation merges:

```text
Oteryn: content format spike
```

- [ ] **Step 2: Compare candidates on the same deterministic fixtures**

Required evidence includes deterministic serialization, diffability, partial/atomic authoring, parser/decompression limits, streaming locality, patchability, compatibility/versioning, corruption recovery, size/load/memory measurements, tooling ergonomics and server-only/client-safe projection separation.

- [ ] **Step 3: Prove non-decision semantics**

The report must state and preserve:

```text
SPIKE_RESULT != OWNER_FORMAT_DECISION
```

No `.omap/.owb` permanent naming, production loader adoption or ADR-0005 final encoding change may occur merely because one prototype benchmarks best.

- [ ] **Step 4: Merge only bounded tooling/evidence**

Run dependency/supply-chain review for prototype libraries, malformed/corruption tests, deterministic fixture checks, whole-diff self-review and exact-head CI. Obtain independent review if the spike creates a material parser/decompression/download/signing trust boundary.

---

### Task 5: Prepare and Allocate the Production Gameplay Server Seam (#96)

**Files:**
- Create: `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`
- Create/update the exact coordinator task/allocation record for #96
- Create before runtime code, after the exact implementation allocation is known: `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md`
- Prompt: `docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md`
- Runtime paths are fixed by that allocation; no worker chooses them ad hoc

**Interfaces:**
- Consumes: merged Foundation framing/codec/runtime/admission/reconnect implementation, FND-02/03/04, current `apps/game-server`
- Produces: one production listener/client-entry seam that preserves Foundation ownership and exposes a real Tier-1 server boundary without falsely claiming Movement/Combat availability

- [ ] **Step 1: Freeze exact transport/listener/composition paths**

The design/allocation packet names exact server paths and any Cargo/shared-path mutations. No gameplay command/state ID is invented by this task.

- [ ] **Step 2: Define the real boundary journey**

The minimum physical server journey is:

```text
connect -> frame/decode -> admission -> GameSession -> reconnect/resume -> resync/fail-closed gameplay entry
```

Unsupported gameplay remains explicitly unavailable until owning command/state registrations exist.

- [ ] **Step 3: Create the child plan and launch only after allocation**

After the #96 decision and exact `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` allocation merge, create the lane-specific child plan from the now-known runtime/test/Cargo paths. Then invoke:

```text
Oteryn: impl server seam
```

The worker must verify that exact merged allocation and child plan before any write. Use TDD for malformed/oversized/unknown-message, stale generation, reconnect/fencing and authority-before-mutation negatives. Protocol/session/admission/fencing changes require genuinely independent exact-head review.

- [ ] **Step 4: Unlock QA Tier 1 and Client readiness**

After merge, `Oteryn: impl qa` may receive a new allocation to prove real Tier-1 Foundation journeys. `Oteryn: impl client` becomes eligible for allocation only after the compatible production seam is merged and verified.

---

### Task 6: Implement First Durability Increment

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-durability-first-increment.md`
- Runtime/migration/test/Cargo files: exactly those accepted by #94 and granted by the merged coordinator allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_DURABILITY.md`

**Interfaces:**
- Consumes: merged #94 topology decision, applicable accepted resource limits, Foundation/Domain/DUR contracts
- Produces: first profile-neutral durable substrate required by current server/VSL consumers

- [ ] **Step 1: Require Definition of Ready**

Do not invoke the worker until #94 is accepted, exact paths/branch/base SHA are allocated, shared-path ownership is clear and every exercised DUR-03 amplification dimension has an accepted finite bound or is explicitly excluded from the first increment.

- [ ] **Step 2: Launch the allocated worker**

```text
Oteryn: impl durability
```

- [ ] **Step 3: Execute the child plan with TDD**

Required evidence includes migration compatibility/interruption, DB concurrency anomalies, same-TransactionId retries, ambiguous-commit reconciliation, crash after DB commit before runtime completion, stale ownership-generation rejection, outbox/audit atomicity where exercised and isolated PostgreSQL E2E. A mock DB is not terminal persistence proof.

- [ ] **Step 4: Independently review and merge**

Persistence/item/value changes require genuinely independent exact-head review, full workspace CI, database-focused tests, `game-gate`, expected-head merge, post-merge verification, task archive and ownership release.

---

### Task 7: Implement Ability Engine

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-ability-first-slice.md`
- Runtime/test files: exact paths granted by the merged Ability allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_GAME_ABILITY.md`

**Interfaces:**
- Consumes: merged Foundation/SIM/Domain/Content and applicable #93 hard maxima
- Produces: one typed authoritative ability/effect engine consumed later by Combat

- [ ] **Step 1: Require all exercised resource limits**

Targeting, Effect Plan, future/repeated work, conditions, reactions and cross-domain proposal dimensions used by the first slice must be registered or explicitly excluded fail-closed.

- [ ] **Step 2: Launch in parallel with other generic engines**

```text
Oteryn: impl ability
```

- [ ] **Step 3: Preserve ownership boundaries**

Ability owns effect legality/staging/commit semantics but does not directly own item/value conservation, movement, AI choice or client authority. Reference formulas remain evidence-gated.

- [ ] **Step 4: Merge only exact-head validated work**

Require TDD, deterministic SIM/replay tests, bounded-work boundary tests, whole-diff self-review, any independent review triggered by the exercised protected semantics, exact-head CI and lifecycle closeout.

---

### Task 8: Implement Interaction Engine

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-interaction-first-slice.md`
- Runtime/test files: exact paths granted by the merged Interaction allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_GAME_INTERACTION.md`

**Interfaces:**
- Consumes: merged Foundation/SIM/Domain/Content and applicable #93 hard maxima
- Produces: retry-safe deterministic trigger/child/workflow engine; hard prerequisite for Movement

- [ ] **Step 1: Require cascade/reconciliation limits**

The allocated slice must have accepted finite bounds for every exercised cascade, fan-out, total descendant, delegated operation and reconciliation/retry dimension.

- [ ] **Step 2: Launch as a priority generic-engine worker**

```text
Oteryn: impl interaction
```

- [ ] **Step 3: Prove identity and retry semantics**

Tests must cover sibling/nested child identity, duplicate delivery, pending vs committed vs rejected, timeout before/after foreign acceptance, cancellation races, stale generation/revision completion and no blind new semantic attempt while outcome is ambiguous.

- [ ] **Step 4: Merge before Movement allocation**

Movement cannot be allocated until the Interaction implementation consumed by it is merged and integration-ready, not merely present on a sibling branch.

---

### Task 9: Implement AI / Spawn / Path Proposal Engine

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-ai-first-slice.md`
- Runtime/test files: exact paths granted by the merged AI allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_GAME_AI.md`

**Interfaces:**
- Consumes: merged Foundation/SIM/Domain/Content and applicable #93 hard maxima
- Produces: bounded deterministic AI/spawn/path proposals routed to authoritative owners

- [ ] **Step 1: Require finite AI/path/spawn budgets**

Evaluation work, candidate/memory sets, pending operations, path queues/search/route results, repath/retry and spawn occupancy retries used by the slice must be bounded by accepted registry entries.

- [ ] **Step 2: Launch in parallel**

```text
Oteryn: impl ai
```

- [ ] **Step 3: Preserve proposal-only semantics**

AI/path workers never directly commit position, effects, durable value or foreign-owner state. Stale results are rejected by current authority/revision evidence.

- [ ] **Step 4: Merge independently of the first Movement critical path**

AI should progress in parallel, but lack of AI completion does not block the first Movement or Combat allocation under the canonical DAG unless a concrete first-slice mechanic explicitly adds it as a dependency through a reviewed coordinator decision.

---

### Task 10: Implement Native Gameplay Client

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-native-gameplay-client.md`
- Client/runtime/test files: exact paths granted by the merged Client allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md`

**Interfaces:**
- Consumes: merged compatible production server seam plus Foundation/domain client-safe contracts
- Produces: native protocol/session/reconciliation client path required by Tier 2 and Movement

- [ ] **Step 1: Verify the real server seam**

Do not allocate Client from architecture documents alone. The production listener/client-entry seam must be merged, exact-head validated and compatible with the client path.

- [ ] **Step 2: Launch**

```text
Oteryn: impl client
```

- [ ] **Step 3: Keep the client non-authoritative**

Semantic input becomes typed intent; server results/revisions remain authoritative. Prediction/presentation never becomes movement/effect/value authority, and unsupported capabilities fail closed.

- [ ] **Step 4: Require Tier-2-ready instrumentation**

The merged Client must expose enough deterministic evidence to let QA prove the native-client boundary without introducing production test adapters.

---

### Task 11: Expand QA from Shell to Real Tier 1 / Tier 2 Journeys

**Files:**
- Create before new QA work: `docs/superpowers/plans/2026-08-24-oteryn-real-boundary-e2e.md`
- Test/evidence files: exact paths granted by a new QA allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_QA_E2E.md`

**Interfaces:**
- Consumes: merged QA shell #98, production server seam, then native Client
- Produces: truthful physical Tier 1 and Tier 2 evidence required by Movement/Combat

- [ ] **Step 1: Prove Tier 1 after server seam merge**

Use a new QA allocation and alias:

```text
Oteryn: impl qa
```

Prove real connect/admit/reconnect/resync/fail-closed server journeys through the production boundary.

- [ ] **Step 2: Prove Tier 2 after Client merge**

Exercise the native client's normal semantic input/projection path with exact build/protocol/content/revision/seed/clock/topology evidence. Synthetic client or direct runtime mutation is rejected by the merged shell.

- [ ] **Step 3: Preserve failed evidence**

`FAIL`, `BLOCKED`, `UNSTABLE` and `NOT_EVALUATED` results remain historical evidence. Reruns append attempts; they do not rewrite prior failures as if they never occurred.

---

### Task 12: Implement First Authoritative Movement Slice

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-movement-vsl.md`
- Runtime/protocol/client/test files: exact paths granted by the Movement allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md`

**Interfaces:**
- Consumes: Foundation + SIM + Domain + Content + Interaction + Client + QA, all merged/integration-ready
- Produces: first authoritative real-boundary movement/collision/visibility vertical slice

- [ ] **Step 1: Enforce Movement Definition of Ready**

Required before allocation:

```text
Foundation merged
SIM merged
Domain merged
Content evidence seam merged/repaired
Interaction merged
production server seam merged
Client merged
QA Tier-1/Tier-2 path capable of testing the slice
applicable Movement resource hard maxima registered
serialized FND-02 command/state registration authority available to coordinator
```

- [ ] **Step 2: Launch**

```text
Oteryn: impl movement
```

- [ ] **Step 3: Implement the complete vertical path**

```text
native client input
-> production protocol
-> admission/session
-> authoritative movement owner
-> static + dynamic legality
-> committed position/revision
-> deterministic visibility/interest projection
-> protocol state/result
-> native client reconciliation/render path
```

No client coordinate, path worker or Interaction callback becomes a second movement writer.

- [ ] **Step 4: Prove real Tier 1 and Tier 2**

Component tests are necessary but insufficient. Merge authority requires real physical server and native-client journeys with exact evidence and all applicable boundary/resource tests.

---

### Task 13: Implement First Combat / Death / Loot / XP / Pickup Slice

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-combat-vsl.md`
- Runtime/protocol/client/persistence/test files: exact paths granted by the Combat allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md`

**Interfaces:**
- Consumes: Movement + Foundation + SIM + Domain + Content + Ability + Interaction + Durability + Client + QA, all merged/integration-ready
- Produces: first authoritative combat-to-durable-value vertical slice

- [ ] **Step 1: Enforce Combat Definition of Ready**

Combat does not implement missing prerequisites inside itself. Movement, Ability, Interaction and Durability must already be merged; Client/QA real-boundary infrastructure must be working.

- [ ] **Step 2: Launch**

```text
Oteryn: impl combat
```

- [ ] **Step 3: Prove one stable semantic chain**

```text
target
-> ability/effect
-> damage
-> one stable death occurrence
-> deterministic loot selection under exact content/SIM revisions
-> durable materialization/reconciliation
-> idempotent Character XP settlement
-> Interaction + Item + DUR pickup
-> reconnect/crash/lost-response recovery
-> native client authoritative reconciliation
```

- [ ] **Step 4: Prove no duplication/value ambiguity**

Independent exact-head review is mandatory for exercised loot/item/value/persistence invariants. Tests must cover duplicate requests, timeout/ambiguous commit, crash after durable commit, stale runtime projection, fresh output IDs, same-TransactionId retry and no second loot/XP settlement.

- [ ] **Step 5: Require Tier 1 and Tier 2 terminal evidence**

Do not declare the first gameplay VSL proven from component/mock tests. The full combat chain must traverse the production server path and native client path with evidence accepted by ADR-0007.

---

### Task 14: Later Channel Implementation

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-channel-first-slice.md`
- Runtime/protocol/persistence/test files: exact paths granted by the Channel allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_GAME_CHANNEL.md`

**Interfaces:**
- Consumes: Foundation + Domain + Durability and accepted numeric product prerequisites
- Produces: channel selection/switch/anti-hopping semantics without production orchestration ownership

- [ ] **Step 1: Defer until genuinely needed**

Channel is not on the first Movement/Combat critical path. Do not pull it forward merely because the prompt exists.

- [ ] **Step 2: Launch only after numeric/product readiness**

```text
Oteryn: impl channel
```

Multichannel/session/fencing semantics require independent exact-head review where root policy applies.

---

### Task 15: Later Analytics Implementation

**Files:**
- Create before code: `docs/superpowers/plans/2026-08-24-oteryn-analytics-first-producers.md`
- Analytics/event-consumer/test files: exact paths granted by the Analytics allocation
- Prompt: `docs/agents/prompts/OTV2_IMPL_ANALYTICS.md`

**Interfaces:**
- Consumes: concrete merged producer event registrations
- Produces: read-only ingestion/quality/invariant/reporting without gameplay mutation authority

- [ ] **Step 1: Wait for real producers**

Do not start full analytics while concrete producer event families remain absent.

- [ ] **Step 2: Launch after producer readiness**

```text
Oteryn: impl analytics
```

Analytics cannot invent producer schemas, mutate gameplay or turn incomplete telemetry into player-sanction authority.

---

## 3. Agent Launch Matrix

| Alias | Earliest lawful launch condition | Can run in parallel with |
| --- | --- | --- |
| `Oteryn: implementation coordinator` | Always; one active coordinator authority | all workers; coordinator owns allocation/integration, not their primary paths |
| `Oteryn: content format spike` | #95 exact allocation merged | #93, #94, #96, #97 and later generic engines if paths remain disjoint |
| `Oteryn: impl durability` | #94 accepted + exact implementation allocation + applicable bounds | Ability, Interaction, AI; server seam if no shared-path conflict |
| `Oteryn: impl ability` | #93 applicable limits satisfied + exact allocation | Durability, Interaction, AI |
| `Oteryn: impl interaction` | #93 applicable limits satisfied + exact allocation | Durability, Ability, AI |
| `Oteryn: impl ai` | #93 applicable limits satisfied + exact allocation | Durability, Ability, Interaction |
| `Oteryn: impl server seam` | #96 decision accepted + exact `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` allocation + child plan | Durability, Ability, Interaction, AI when owned paths/shared leases are disjoint |
| `Oteryn: impl client` | production server seam #96 implementation merged/verified + exact Client allocation | QA Tier-1 expansion and non-overlapping generic work |
| `Oteryn: impl qa` | new exact QA allocation; Tier 1 after server seam, Tier 2 after Client | Client/integration as coordinated; never fake unavailable evidence |
| `Oteryn: impl movement` | Interaction + Client + QA + all canonical Movement predecessors integration-ready | no competing mutation of its shared protocol/registry/composition paths |
| `Oteryn: impl combat` | Movement + Ability + Interaction + Durability + Client + QA integration-ready | later non-overlapping AI/content evidence only; value/shared-path mutations remain serialized |
| `Oteryn: impl channel` | Foundation + Domain + Durability + numeric product readiness + allocation | later only; not on first VSL critical path |
| `Oteryn: impl analytics` | concrete producer event registrations merged + allocation | read-only analytics lane after producers exist |

---

## 4. Definition of Ready for Any Implementation Worker

A worker is genuinely `READY` only when all applicable statements are true:

- [ ] Required predecessor delivery SHAs are on `main`, not merely on sibling branches.
- [ ] Coordinator allocation PR is merged and names exact task ID, branch, base SHA and owned paths.
- [ ] No active task/PR claims overlapping primary paths or shared mutation leases.
- [ ] Required contracts are accepted and implementation-authorized for the claimed slice.
- [ ] Every exercised externally/content-controlled count/depth/work/size has an accepted finite hard maximum or is explicitly excluded fail-closed.
- [ ] Required stable ID/registry ranges are allocated by the owning coordinator; the worker does not invent them.
- [ ] Shared Cargo/workspace/composition paths have a current one-writer lease if needed.
- [ ] Test plan states which evidence is component, Tier 1, Tier 2 or not applicable before first write.
- [ ] High-risk independent-review requirement is recorded before the worker starts.
- [ ] Branch starts from the exact merged allocation base and remote head is verified before local mutation.

If any statement is false, the lane remains preparation/read-only rather than starting implementation optimistically.

---

## 5. Definition of Done for Any Delivery

`DONE` requires this exact lifecycle:

```text
final exact head
-> focused tests / required real-boundary evidence
-> mandatory whole-diff self-review
-> genuinely independent exact-head review where required
-> exact-head GitHub CI including game-gate
-> zero unresolved review threads / material P0-P2 findings
-> current-main compare and conflict/overlap check
-> expected-head squash merge
-> verify merged main SHA and intended tree
-> verify Issue/task terminal state
-> archive task
-> release owned paths/shared lease
-> update coordinator/live status without rewriting historical evidence
```

A local commit, green local tests, a draft PR, a reviewer result on an older head, or a synthetic/mock journey is not `DONE`.

---

## 6. Coordinator Merge and Shared-Path Policy

The coordinator serializes at least:

```text
apps/game-server/src/lib.rs
apps/game-server/Cargo.toml
Cargo.toml
Cargo.lock
workspace-boundaries.toml
docs/contracts/* registries
stable protocol/event/state IDs
architecture policy/tooling
.github/workflows/**
```

Parallel workers may prepare primary-path commits independently, but a worker requiring a shared path must wait for a coordinator lease or receive an explicitly bounded integration turn. Do not let multiple workers repeatedly merge/rebase shared files as an informal coordination mechanism.

Before each merge, the coordinator re-reads current `main`, PR head, changed paths, active allocations and unresolved threads. Concurrent disjoint merges may be incorporated non-destructively; overlapping authority changes require serialization.

---

## 7. Wave Exit Criteria

### Preparation Wave completeness (not a global lane-release barrier)

This checklist records whether all preparation work is complete. It is not a prerequisite bundle for every implementation lane: #95/#97 may remain in progress while a path-disjoint lane starts after its own Definition of Ready is satisfied.

- [ ] #97 maintained status reflects merged QA #98 and current implementation truth.
- [ ] #93 produces accepted applicable hard-max decisions/registrations for first Ability/Interaction/AI slices, or explicitly narrows those slices fail-closed; its Movement subset remains open/continued until every Movement-exercised dimension closes before Movement allocation.
- [ ] #94 freezes first Durability topology and exact implementation allocation proposal.
- [ ] #95 produces bounded format evidence without a permanent-format decision.
- [ ] #96 freezes the production server-seam implementation boundary and exact allocation proposal.

### Generic Engine Wave exit

- [ ] Durability merged and lifecycle-released.
- [ ] Ability merged and lifecycle-released.
- [ ] Interaction merged and lifecycle-released.
- [ ] AI may still be finishing without blocking Movement unless a reviewed concrete dependency was added.
- [ ] Production server seam merged and Tier-1-capable.

### Client / QA Wave exit

- [ ] Native Client merged against the production server seam.
- [ ] QA proves required real Tier 1 Foundation/server journeys.
- [ ] QA proves required native-client Tier 2 journey capability.

### Movement Wave exit

- [ ] First authoritative Movement slice merged.
- [ ] Real Tier 1 and Tier 2 Movement evidence PASS on exact revisions.
- [ ] Resource and malformed/stale/replay boundary tests PASS.

### Combat Wave exit

- [ ] First Combat/death/loot/XP/pickup slice merged.
- [ ] Durable no-dup/crash/retry evidence PASS.
- [ ] Real Tier 1 and Tier 2 Combat journey PASS.
- [ ] No Reference parity promotion occurred from fixture-only data.

---

## 8. Plan Self-Review Checklist

Before using this master plan to release a wave, the coordinator must recheck:

- [ ] `main` has not invalidated the snapshot or dependency assumptions.
- [ ] Live allocations and open PRs do not already supersede a planned step.
- [ ] Every child implementation gets its own lane-specific Superpowers plan with exact files/tests after topology/allocation is known; this includes the production Server Seam integration task.
- [ ] No step asks an implementation worker to make an unresolved owner/architecture/numeric decision.
- [ ] #93 is revalidated against the exact Movement child plan and every Movement-exercised dimension is closed before Movement allocation.
- [ ] Movement and Combat remain serial integration gates.
- [ ] QA evidence classifications remain truthful.
- [ ] Production Content, permanent World format, Channel/Analytics and Reference parity remain outside the first gameplay critical path unless separately accepted and allocated.

## 9. First Coordinator Actions After This Plan Merges

Execute in this order while allowing the explicitly disjoint preparation work to overlap:

1. Re-read current `main`, live allocations, active tasks/leases and open PRs.
2. Drive the path-disjoint #93/#94/#95/#96/#97 preparation work concurrently where exact allocations permit; do not treat unfinished #95/#97 as a global barrier.
3. As soon as the applicable #93 subset is accepted/registered or fail-closed-excluded, release Ability / Interaction / AI independently with exact non-overlapping allocations.
4. Close #94 and create the exact Durability allocation; route any unresolved exercised DUR-03 hard maximum through #93 or a separately owner-approved numeric/registry decision before releasing Durability.
5. After #96 is accepted, create the exact `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` allocation and child plan, then launch `Oteryn: impl server seam`. Run it alongside path-disjoint generic engines when shared leases allow.
6. Run #95 Content Format Spike as evidence-only work and #97 status reconciliation whenever their own allocations permit, without holding already-ready unrelated runtime lanes.
7. Merge and verify the Server Seam, then allocate Client and a fresh QA Tier-1 expansion; after Client merges, prove the required Tier-2 native-client capability.
8. Before Movement allocation, re-read #93 against the exact Movement child plan and close every Movement-exercised resource dimension; then require all canonical Movement predecessors on `main`.
9. Allocate/merge Movement as the first serial gameplay integration gate with real Tier 1/Tier 2 evidence.
10. Allocate Combat only after Movement plus Ability/Interaction/Durability and the remaining canonical prerequisites are merged/integration-ready.
11. Defer Channel, Analytics and permanent Content-format adoption to their explicit gates rather than allowing first-VSL scope creep.

This master plan is an execution map, not a grant of write authority. Every executable step still requires the live coordinator allocation mandated by repository governance.
