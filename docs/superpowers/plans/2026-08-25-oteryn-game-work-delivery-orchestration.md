# Oteryn Game Work Delivery Orchestration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Coordinate the post-blocker Oteryn Game implementation from current protected `main` through a truthful Server Seam + native Client + Movement + Combat vertical slice using path-isolated subagents and fail-closed architecture escalation.

**Architecture:** ChatGPT Work is the bounded execution coordinator and subagent dispatcher. A separate owner-designated Supervising Architect owns material architecture decisions; Work persists `ARCHITECTURE_ESCALATION_REQUIRED` packets instead of letting workers invent missing architecture. Path-disjoint implementation lanes may run concurrently, while Cargo/shared composition/registries/stable IDs and Movement/Combat integration gates stay serialized.

**Tech Stack:** Rust 1.94 workspace, `apps/game-server`, native Rust client, SQLx 0.9.0 for the accepted first Durability topology, PostgreSQL target per accepted architecture, `protocol-oteryn`, GitHub Issues/PRs/Actions, repository exact-head `game-gate`, Superpowers subagent/TDD/debugging/verification workflows.

**Spec:** `docs/architecture/reviews/OTERYN_GAME_POST_BLOCKER_WORK_ORCHESTRATION_2026-08-25.md`

## Global Constraints

- Canonical repository is `Oteryn/Oteryn-Game`; GitHub live state outranks this plan.
- Execute this plan through alias `Oteryn: work coordinator` only after `OTV2_WORK_DELIVERY_COORDINATOR` is canonical on protected `main`.
- The Work coordinator is a stricter execution profile of `OTV2_IMPLEMENTATION_COORDINATOR`; it has no new production, architecture or cross-repository authority.
- Every substantial mutating child uses one Issue/task, one branch/worktree, one exact path allocation and one PR.
- No worker broadens its own paths, contract authority or resource values.
- Shared Cargo/workspace/composition/registry/stable-ID/workflow/governance surfaces are serialized.
- Maximum useful concurrency is five substantial implementation workers, and only when exact path/dependency isolation is proven.
- Physical Tier 1/Tier 2 cannot be replaced by synthetic/direct-domain evidence.
- Movement remains blocked on exact child plan + #139 closure + Interaction + compatible Client + real QA readiness.
- Combat remains blocked on merged Movement plus integration-ready Ability, Interaction, Durability, Client and QA; AI is not a symmetry blocker unless live accepted architecture makes it one.
- Any material architecture/API/schema/security/persistence/resource/cross-repository conflict uses `ARCHITECTURE_ESCALATION_REQUIRED`; affected work stops fail-closed.
- No production deployment, protected secret, live account/session/data, Platform/Atlas/META write, Reference parity or permanent Content-format decision is authorized.

---

### Task 1: Start the Work coordinator from live protected main

**Files:**
- Read: `AGENTS.md`
- Read: `docs/agents/AGENTS.md`
- Read: `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`
- Read: `docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md`
- Read: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`
- Read: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Read: `docs/architecture/reviews/OTERYN_GAME_POST_BLOCKER_WORK_ORCHESTRATION_2026-08-25.md`
- Create through GitHub lifecycle: one coordinator Issue titled `coord: deliver post-blocker gameplay vertical slice`
- Create under its exact allocation: one active coordinator task packet under `docs/agents/tasks/active/`

**Interfaces:**
- Consumes: protected live `main`, current Issues/PRs/checks and all accepted lane contracts.
- Produces: one durable coordinator identity, exact `admission_main_sha`, current readiness matrix and one next action.

- [ ] **Step 1: Resolve live repository identity and state**

Use GitHub, not cached chat/local state, to record protected `main`, all open user PRs, open implementation Issues, current active task ownership and exact branch/head identities.

Expected classification includes `PROVEN / DERIVED / UNKNOWN / CONFLICT` for every material readiness claim.

- [ ] **Step 2: Reconcile completed work before creating workers**

Verify at minimum:

```text
#91 completed
PR #98 merged as dc22e0da8efcc6f4458416191261063b295af5b4
QA shell exists
physical gameplay Tier 1/Tier 2 = NOT_EVALUATED
#93/#115/#116/#123 completed
PR #144 merged
PR #151 merged
#139 non-current until Movement prerequisites
```

If live main contains a different newer terminal result, use the newer evidence and record drift; do not restart terminal work.

- [ ] **Step 3: Create/resume the coordinator task**

The task must record exact base/head, owned coordinator-only paths, no implicit runtime ownership, dependencies, execution budget and one `next_action`.

- [ ] **Step 4: Validate coordinator readiness**

Run/read the repository governance checks applicable to the coordinator-only allocation and perform a whole-allocation self-review before releasing workers.

---

### Task 2: Produce exact Wave A allocations and child plans

**Files:**
- Modify only through coordinator authority: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Create: one child Superpowers plan/task packet per lane that is actually ready
- Consume reusable worker prompts:
  - `docs/agents/prompts/OTV2_IMPL_GAME_INTERACTION.md`
  - `docs/agents/prompts/OTV2_IMPL_GAME_ABILITY.md`
  - `docs/agents/prompts/OTV2_IMPL_GAME_AI.md`
  - `docs/agents/prompts/OTV2_IMPL_DURABILITY.md`
  - `docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md`

**Interfaces:**
- Consumes: accepted #140/#144 resource evidence, Durability #94/#122 topology, Server Seam #96 + closed #115/#116, current source tree.
- Produces: independently mergeable exact-path allocations with no shared writer overlap.

- [ ] **Step 1: Evaluate each candidate independently**

For each lane, write one Definition-of-Ready result:

```text
READY_TO_ALLOCATE
WAITING_DEPENDENCY
ARCHITECTURE_ESCALATION_REQUIRED
BLOCKED_OWNER_AUTHORITY
```

Do not force all five lanes into the same state.

- [ ] **Step 2: Map primary and shared paths**

For each `READY_TO_ALLOCATE` lane, derive exact production/test/task paths from current code and its reusable worker prompt. List every requested shared/Cargo/registry path separately as a serialized coordinator lease.

If two lanes require the same primary semantic owner rather than merely the same composition file, stop those lanes and use architecture escalation.

- [ ] **Step 3: Write one child plan per ready lane**

Each child plan must define concrete TDD/focused/component/E2E/review steps for its exact source paths. The orchestration plan does not substitute for those child implementation plans.

- [ ] **Step 4: Merge allocation authority before dispatch**

No worker writes from an unmerged allocation. Run exact-head governance/review/CI required for the allocation PR, merge it, read back protected `main`, then create the worker branch from the recorded exact base.

---

### Task 3: Dispatch path-disjoint Wave A workers

**Files:**
- Worker-owned exact paths only; determined by merged Task 2 allocations.
- Shared mutation paths stay coordinator-owned and serialized.

**Interfaces:**
- Consumes: merged allocations and child plans.
- Produces: implementation PRs for the ready subset of Interaction, Ability, AI, Durability and Server Seam.

- [ ] **Step 1: Dispatch one self-contained subagent per independent lane**

Each dispatch includes exact repository/base/Issue/task/branch/owned paths/prerequisites/contracts/exclusions/validation and the required return packet from `OTV2_WORK_DELIVERY_COORDINATOR.md`.

- [ ] **Step 2: Keep workers isolated**

Never share a writable branch/worktree. A worker encountering an unowned shared path reports it rather than editing it.

- [ ] **Step 3: Triage worker blockers**

Classify returns:

```text
IMPLEMENTATION_REPAIR
SHARED_LEASE_REQUIRED
WAITING_EXTERNAL
BLOCKED_OWNER_AUTHORITY
ARCHITECTURE_ESCALATION_REQUIRED
READY_FOR_INTEGRATION
```

Only the last state enters integration immediately.

- [ ] **Step 4: Continue independent lanes during a blocked lane**

An architecture/external blocker pauses only dependent/semantically affected lanes. Record the dependency reason before continuing others.

---

### Task 4: Integrate the production Server Seam and prove real Tier 1

**Files:**
- Consume exact Server Seam child plan/allocation.
- Test evidence: existing `apps/game-server/tests/**` shell plus new exact allocated physical-journey support where the QA allocation permits it.

**Interfaces:**
- Consumes: Foundation/FND-04 verifier, accepted listener limits, Server Seam implementation PR.
- Produces: merged production transport/admission/client-entry seam and truthful server-side physical Tier 1 evidence.

- [ ] **Step 1: Review Server Seam authority and diff**

Reject any listener-local second protocol/session authority, production secret/port choice, invented gameplay IDs or weakened fencing/backpressure behavior.

- [ ] **Step 2: Run protocol/session negative validation**

Require malformed/oversized/unknown-message, admission/reconnect generation fencing, backpressure/drain and replay/idempotency evidence specified by the child plan and accepted contracts.

- [ ] **Step 3: Obtain required independent exact-head review**

Protocol/session/admission/fencing changes require the qualifying independent review mandated by repository policy. Do not substitute a semantic-audit job that says `NOT_APPLICABLE`.

- [ ] **Step 4: Merge Server Seam through exact-head gates**

Require current-main integration refresh, full diff review, exact-head CI including `game-gate`, zero unresolved threads and expected-head squash merge.

- [ ] **Step 5: Execute/record real applicable Tier 1**

Use the production server/protocol boundary for the accepted connect/admit/reconnect/resync journey. If a required physical dependency is still absent, record `NOT_EVALUATED`/`BLOCKED` rather than PASS.

---

### Task 5: Allocate and integrate the compatible native Client, then prove Tier 2

**Files:**
- Consume: `docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md`
- Create exact Client child plan/task/allocation from current main after Server Seam merge.
- Client production/test paths are frozen by that allocation, not guessed by this master plan.

**Interfaces:**
- Consumes: merged compatible Server Seam and Foundation protocol contract.
- Produces: native client entry boundary compatible with the merged server and truthful Tier 2 evidence.

- [ ] **Step 1: Recompute Client Definition of Ready after Server Seam merge**

A historical preparation statement is insufficient; bind exact current protocol/server merge SHAs.

- [ ] **Step 2: Create and merge exact Client allocation/child plan**

Serialize any shared stable protocol or workspace mutation. Escalate if the client requires changing accepted wire/session architecture rather than consuming it.

- [ ] **Step 3: Dispatch the Client worker and integrate its PR**

Use focused/client integration tests, protocol golden/negative evidence where applicable, full diff review, required independent review and exact-head repository gates.

- [ ] **Step 4: Execute truthful Tier 2 through the native Client**

Tier 2 must cross the actual native-client boundary and record presentation/client evidence required by ADR-0007. Shared production codecs are not the sole wire oracle.

---

### Task 6: Activate and close the exact Movement resource gate #139

**Files:**
- Modify only through serialized decision/allocation lifecycle: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` if exact new Movement rows are accepted.
- Create/update exact #139 evidence/decision task packet and Movement child plan.
- Do not write Movement runtime yet.

**Interfaces:**
- Consumes: merged Interaction, compatible Client, real QA readiness, accepted Movement contracts and existing inherited FND resources.
- Produces: exact first Movement slice plus `REGISTERED_EXACT` or explicit fail-closed exclusion for every exercised Movement-only row.

- [ ] **Step 1: Freeze the exact first Movement child slice**

Name the operations/queries/relocation/interest/snapshot/path behavior actually exercised. Do not decide unused future Movement dimensions.

- [ ] **Step 2: Re-evaluate all #139 rows against that exact slice**

For every exercised row require evidence-backed finite hard maximum or explicit fail-closed exclusion. Preserve exact inherited FND rows only where #139 already proves identity.

- [ ] **Step 3: Escalate missing material resource decisions**

If evidence cannot choose an accepted reversible maximum within current authority, emit `ARCHITECTURE_ESCALATION_REQUIRED`; the Movement worker never chooses the number.

- [ ] **Step 4: Serialize registry mutation and close #139**

After accepted decisions, perform the separate registry writer lifecycle, run max/max+1/overflow/retry/replay obligations, exact-head gates and terminal #139 closeout.

---

### Task 7: Implement and integrate Movement as the serial gameplay gate

**Files:**
- Consume: `docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md`
- Create exact Movement allocation/child plan only after Task 6 is terminal.
- One Movement implementation branch only.

**Interfaces:**
- Consumes: Foundation + SIM + Domain + Content seam + Interaction + Client + QA + closed #139.
- Produces: first authoritative Movement integration slice and its physical E2E evidence.

- [ ] **Step 1: Prove the Movement architecture gate checkpoint**

Record exact prerequisite merge SHAs, resource rows, QA state, ownership and zero unresolved architecture escalation affecting Movement.

- [ ] **Step 2: Merge exact Movement allocation before runtime writes**

No implementation worker starts from the architecture checkpoint alone.

- [ ] **Step 3: Dispatch one Movement worker**

Follow its child TDD plan; require deterministic SIM semantics, authoritative ownership, collision/interaction boundaries, resource rejection and replay/reconnect correctness as applicable.

- [ ] **Step 4: Integrate through full validation and physical journey evidence**

Run focused/component/integration/E2E, whole diff, required independent review, exact-head CI and expected-head merge. Recompute Combat readiness only after post-merge readback.

---

### Task 8: Implement and integrate Combat as the next serial gameplay gate

**Files:**
- Consume: `docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md`
- Create exact Combat allocation/child plan from current main after Movement merge.
- One Combat implementation branch only.

**Interfaces:**
- Consumes: merged Movement plus integration-ready Ability, Interaction, Durability, Client, QA and existing Foundation/SIM/Domain/Content predecessors.
- Produces: first authoritative Combat slice with durability/value/fencing/replay and physical E2E evidence required by its exact plan.

- [ ] **Step 1: Prove the Combat architecture gate checkpoint**

Verify exact prerequisite SHAs and resource/contract state. Do not block solely for AI symmetry unless live accepted architecture requires AI for the exact Combat slice.

- [ ] **Step 2: Escalate any unresolved value/persistence/ability semantic gap**

Combat must not invent item/value/persistence/resource semantics to progress.

- [ ] **Step 3: Allocate, dispatch and integrate one Combat worker**

Use the exact child plan and all repository validation/review gates. Persistence/item/value changes receive the mandated genuinely independent exact-head review.

- [ ] **Step 4: Record physical E2E and post-merge state**

The journey must cross the real server/client boundaries appropriate to the slice and preserve authoritative failure evidence.

---

### Task 9: Terminal programme closeout

**Files:**
- Modify: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Archive: every completed coordinator/worker task record used by this wave
- Update only directly affected maintained programme status documents

**Interfaces:**
- Consumes: protected main after all slice merges, terminal Issue/PR/CI/review evidence.
- Produces: no stale active locks/leases, one truthful terminal status and a next-wave readiness handoff.

- [ ] **Step 1: Verify all task/branch/PR/lease terminal states**

No active task may remain merely because its closeout metadata was forgotten.

- [ ] **Step 2: Verify no architecture escalation was silently bypassed**

Every escalation affecting merged code must have a durable recorded resolution; unresolved escalations remain explicit blockers and prevent a false programme-complete claim.

- [ ] **Step 3: Reconcile maintained status from protected main**

Distinguish implemented, physically proven, blocked/not evaluated, production-ready and Reference-parity states. Do not conflate them.

- [ ] **Step 4: Run final governance/architecture/repository validation**

Run the current repository-selected exact checks, whole closeout diff review, exact-head CI and expected-head merge for the closeout PR.

- [ ] **Step 5: Report the bounded terminal outcome**

The valid success claim is the exact implemented vertical slice and evidence achieved. Do not claim production deployment, production readiness or Reference parity without their separate gates.