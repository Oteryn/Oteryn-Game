# Oteryn Game Sol-Lead + Selective-Codex Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt the owner-approved execution model in which Work remains the serialized GitHub control plane, difficult delivery lanes are led by separate GPT-5.6 Sol Extra High sessions, and Codex is used only as selective implementation/debug/test/build/repository assistance.

**Architecture:** Preserve the existing Oteryn Game runtime architecture and implementation DAG. First reconcile the current Work lifecycle truth, then narrow the existing Work coordinator profile to control-plane duties, package bounded Sol lane-lead prompts plus one shared selective-Codex handoff contract, and finally publish a durable wave scheduler/owner launch sheet. Adoption is split into independently reviewable governance packages so no package can silently widen authority.

**Tech Stack:** GitHub Issues/branches/PRs/Actions, Markdown/YAML/JSON governance files, repository prompt lifecycle/evaluation tooling, GPT-5.6 Sol Extra High lane sessions, optional selective Codex execution, Rust 1.94 programme context without runtime mutation in this governance plan.

**Spec:** `docs/superpowers/specs/2026-08-26-oteryn-game-sol-lead-selective-codex-execution-design.md`

## Global Constraints

- Canonical repository is `Oteryn/Oteryn-Game`; live protected `main`, Issues, task records, PRs, exact heads, checks and reviews outrank cached chat or stale prose.
- Owner-approved execution hierarchy is `Owner -> Supervising Architect -> Work Control Plane -> Sol Lane Leads -> optional selective Codex`, with `Oteryn: work auditor` independent/read-only.
- `CODEX_USE: SELECTIVE_IMPLEMENTATION_ASSISTANCE` is the only accepted Codex posture in this model.
- Codex is optional and may assist `IMPLEMENT / DEBUG / TEST / BUILD / REPO_EXECUTION`; it is not the default programme brain or end-to-end lane owner.
- Never claim Codex ran without durable evidence of a real supported handoff/execution.
- Default to one heavy Codex implementation lane at a time; a second requires proven path/shared-surface independence and a concrete throughput benefit.
- Up to five useful Sol lane chats may be active, but normally no more than two or three repository-mutating leads at once.
- Root/app Cargo manifests, `Cargo.lock`, workspace boundaries, composition roots, stable registries/IDs, shared contracts/ADRs, workflows and governance surfaces remain serialized by the Work control plane.
- Sol aliases grant no write authority by themselves; every mutating lead must resolve and bind to a current exact merged allocation.
- Preserve the critical dependency chain `Durability -> Server Seam -> Client/QA -> Movement -> Combat` unless newer accepted authority proves a different order.
- Do not rewrite historical evidence to imply a pre-merge review or lifecycle event that did not occur.
- No package in this plan authorizes gameplay/runtime, production/protected-environment, secret, live account/session/data or external-repository mutation.
- Fresh planning snapshot after owner spec approval: protected `main@2e8c1605535167e91fe0c7775e520f599e3d89ea`, where the serialized Durability shared Cargo/CI lease allocation is already merged. Treat this SHA only as plan provenance; every package resolves live state again.

---

### Task 1: Reconcile transition truth before role migration

**Package:** A — transition reconciliation

**Files:**
- Modify only through an exact new reconciliation allocation after fresh audit: `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`
- Move/update exact affected task records under `docs/agents/tasks/active/` and `docs/agents/tasks/archive/` only when live GitHub proves the corresponding terminal state.
- Record post-merge Ability review evidence in an immutable PR/review/audit channel; do not falsify the historical Ability task packet.
- Update the Work coordinator task checkpoint only through its authorized control-plane lifecycle.

**Interfaces:**
- Consumes: final `Oteryn: work auditor` findings, live PR #171/#172/#178 evidence, Issue #162/#165/#166/#167/#174 state, current protected main, current Durability shared-lease state.
- Produces: one truthful transition baseline with no stale merged-lane ownership and one exact Durability next action.

- [ ] **Step 1: Freeze fresh audit and GitHub evidence**

Resolve current protected main and independently verify the final audit disposition for Ability independent review evidence and lifecycle metadata. Record every material claim as `PROVEN / DERIVED / UNKNOWN / CONFLICT`.

- [ ] **Step 2: Reconcile Ability review truth**

If repository risk policy or the completed auditor finding still requires independent review for Ability #171, perform a genuinely independent review of the exact merged Ability tree/final PR head and record it explicitly as `POST_MERGE_RECONCILIATION`. Never rewrite it as a pre-merge gate.

- [ ] **Step 3: Reconcile merged Wave A lifecycle records**

For Ability, Interaction and AI, replace stale active/allocation metadata only with live PR/final-head/merge/Issue evidence. Archive/release ownership only where the repository closeout policy is satisfied.

- [ ] **Step 4: Reconcile Durability and coordinator checkpoint**

Record the current Durability state from live GitHub. At the plan snapshot the shared Cargo/CI lease allocation has merged on `main@2e8c1605535167e91fe0c7775e520f599e3d89ea`; use newer truth if available and do not retain the obsolete `allocation authority is unmerged` blocker.

- [ ] **Step 5: Validate Package A**

Run/read the exact-head governance/repository-policy checks required by changed paths, review the whole diff, verify zero ownership overlap, require any independent review mandated by final risk classification, and merge only with expected exact head.

**Package A exit gate:** current Work/audit state is truthful enough that a new execution profile cannot hide historical findings or duplicate active ownership.

---

### Task 2: Specialize the existing Work coordinator into control-plane duty

**Package:** B — Work control-plane specialization

**Files:**
- Modify: `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`
- Modify: `docs/agents/prompts/README.md`
- Modify: `docs/agents/PROMPT_LIFECYCLE.json`
- Create/update one exact governance task packet for Package B.

**Interfaces:**
- Consumes: Package A truthful baseline, existing `OTV2_IMPLEMENTATION_COORDINATOR`, existing Work audit contract, accepted execution design.
- Produces: canonical `Oteryn: work coordinator` behavior that coordinates rather than substantively implementing high-complexity lanes.

- [ ] **Step 1: Preserve alias and narrow behavior**

Keep short invocation `Oteryn: work coordinator`. Revise the prompt so its primary mode is `CONTROL_PLANE_COORDINATION`: live reconciliation, exact allocations, shared leases, dependency order, integration qualification, merge/closeout and architecture escalation.

- [ ] **Step 2: Add explicit Sol-lead delegation rule**

When a current lane is assigned a canonical Sol lead, Work must not duplicate substantive implementation in its own session. It allocates the lane, validates the returned evidence packet against GitHub, serializes shared turns and integrates only qualified exact heads.

- [ ] **Step 3: Preserve existing authority ceilings**

State explicitly that this specialization does not grant new runtime, merge-authority, production, cross-repository or architecture-decision powers. Shared-path mutation still needs the exact existing coordinator lease/allocation mechanism.

- [ ] **Step 4: Add fail-closed lane-state vocabulary**

Work must understand `READ_ONLY_PREPARATION`, `WAITING_ALLOCATION`, `READY_TO_IMPLEMENT`, `IMPLEMENTING`, `SHARED_LEASE_REQUIRED`, `CODEX_HANDOFF_REQUIRED`, `WAITING_EXTERNAL`, `WAITING_ARCHITECTURE`, `READY_FOR_INTEGRATION`, `REVIEW_RECONCILIATION_REQUIRED`, `COMPLETED_RELEASED` and preserve `UNKNOWN/CONFLICT` as mutation blockers.

- [ ] **Step 5: Evaluate and validate Package B**

Evaluate the revised Work prompt against all ten `PROMPT_EVAL_STANDARD.md` gates. Because this is a coordinator-behavior governance change, obtain a genuinely independent exact-head review if repository policy classifies the change as authority-sensitive, then require exact-head governance/semantic/merge-authority/merge-gate success.

**Package B exit gate:** Work remains the same owner-facing alias but acts as a serialized control plane and no longer competes with allocated Sol leads for deep implementation ownership.

---

### Task 3: Package the shared Sol lead and selective Codex contracts

**Package:** C — Sol execution prompt family

**Files:**
- Create: `docs/agents/SOL_SELECTIVE_CODEX_EXECUTION.md`
- Create: `docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md`
- Conditionally create only if final Package A evidence still requires them: `docs/agents/prompts/OTV2_SOL_ABILITY_RECONCILIATION.md`, `docs/agents/prompts/OTV2_SOL_LIFECYCLE_RECONCILER.md`
- Modify: `docs/agents/prompts/README.md`
- Modify: `docs/agents/PROMPT_LIFECYCLE.json`
- Create/update one exact governance task packet for Package C.

**Interfaces:**
- Consumes: specialized Work control plane, current reusable implementation worker prompts/plans, accepted runtime contracts, selective Codex policy.
- Produces: bounded Sol lead aliases with a common handoff/evidence contract and no standalone write authority.

- [ ] **Step 1: Write the shared selective Codex contract**

Define the exact policy label `CODEX_USE: SELECTIVE_IMPLEMENTATION_ASSISTANCE`, preferred uses, prohibited default uses, evidence requirements, token-conservation posture, direct-handoff-unavailable behavior, and the minimum bounded handoff packet containing repository, Issue, task, branch, exact SHA, owned/forbidden paths, one objective, commands and exact return evidence.

- [ ] **Step 2: Create `Oteryn: sol durability lead`**

Bind the lead to the canonical Durability topology, Issue/allocation/current shared lease, SQLx/PostgreSQL/fencing review requirements and exact owned-path rules. It may mutate only after current merged allocation authority and must return `SHARED_LEASE_REQUIRED` for unowned shared surfaces.

- [ ] **Step 3: Create `Oteryn: sol server seam lead`**

Default to `READ_ONLY_PREPARATION` until merged Durability adapter readiness is proven. When allocated, bind to protocol/session/admission/reconnect/fencing negative validation, real Tier 1 evidence and mandatory independent exact-head review where policy requires it.

- [ ] **Step 4: Create `Oteryn: sol client qa lead`**

Default to read-only preparation until a compatible merged Server Seam exists. When allocated, own the exact native-client slice plus truthful Tier 1/Tier 2 coordination without calling synthetic/direct-domain evidence physical E2E.

- [ ] **Step 5: Create `Oteryn: sol movement lead`**

Require current #139/resource-gate readiness, merged Interaction and compatible Client/QA prerequisites. The lead may prepare read-only before the gate, but it must not invent unregistered Movement limits or implement runtime before terminal resource closure and exact allocation.

- [ ] **Step 6: Create `Oteryn: sol combat lead`**

Require merged Movement plus current Ability/Interaction/Durability/Client/QA prerequisites. Any unresolved persistence/value/item/resource semantic gap must become `ARCHITECTURE_ESCALATION_REQUIRED` before mutation.

- [ ] **Step 7: Add transition-only aliases only when proven necessary**

Create Ability reconciliation and lifecycle reconciler prompts only if Package A leaves a still-actionable bounded task. Do not preserve temporary aliases after the need has already been terminally resolved.

- [ ] **Step 8: Standardize the Sol return packet**

Every lead returns exact `lane / issue / task_id / admission_main_sha / integration_main_sha / branch / pr / final_head_sha / changed_paths / shared_lease_used / codex_usage / validation / e2e / self_review / independent_review / architecture_escalation / unresolved_findings / recommended_control_plane_action`. Work independently verifies it.

- [ ] **Step 9: Evaluate and validate Package C**

Run the prompt evaluator against every new prompt. Any material `FAIL` blocks registration. Review the whole prompt family for contradictory ownership or duplicated authority, then require exact-head governance/semantic/merge-authority/merge-gate success and the independent review required by final governance risk classification.

**Package C exit gate:** all required Sol aliases are canonical, reusable and powerless to mutate without current exact merged allocation.

---

### Task 4: Publish the wave scheduler and owner launch sheet

**Package:** D — scheduler, launch instructions and adoption closeout

**Files:**
- Create: `docs/agents/programs/OTERYN_V2_SOL_EXECUTION_WAVE_SCHEDULER.md`
- Modify narrowly: `docs/agents/prompts/README.md`
- Modify narrowly: `docs/agents/PROMPT_LIFECYCLE.json` only if Package C did not already complete final registration metadata.
- Update/archive the execution-model adoption task records after terminal merge/release.
- Update only directly affected maintained programme-status/control-plane documents.

**Interfaces:**
- Consumes: Packages A-C on protected main, current Work/auditor state and the actual current critical-path lane.
- Produces: one canonical answer to "what do I start now?" and deterministic promotion rules for later waves.

- [ ] **Step 1: Encode Transition Wave T0 from live state**

At execution time resolve which of these are still useful: Durability Lead, Ability reconciliation, lifecycle reconciliation, Server Seam read-only preparation and Client/QA read-only preparation. Do not start already-terminal transition roles.

- [ ] **Step 2: Encode dependency-triggered promotions**

Record explicit promotion gates:

```text
Durability merged -> Server Seam may become mutating after fresh allocation
Server Seam merged -> Client/QA may become mutating; Movement may only evaluate current resource-gate readiness
Client/QA ready + #139 terminal -> Movement may become mutating after exact allocation
Movement merged -> Combat may become mutating after fresh readiness/allocation
```

- [ ] **Step 3: Encode concurrency and Codex budgeting**

Scheduler allows up to five useful Sol chats, normally at most two or three mutating leads and one heavy Codex implementation lane. A second Codex-heavy lane requires Work to record proven independence and the throughput reason.

- [ ] **Step 4: Define the owner launch-sheet format**

The maintained launch sheet must list only: alias, requested model/effort, `READ_ONLY` or `MUTATING`, exact prerequisite/status, whether Codex is currently permitted/needed, and the next alias unlocked by terminal merge. No owner should need to reconstruct the DAG manually.

- [ ] **Step 5: Define architecture escalation handoff**

Every lane uses `ARCHITECTURE_ESCALATION_REQUIRED` with exact Issue/task/head/blocked decision/evidence/affected paths. The owner routes that packet to the Supervising Architect; Work must not solve the architecture gap itself.

- [ ] **Step 6: Validate terminal adoption state**

Verify Work control-plane prompt, Sol prompt family, scheduler, README/lifecycle entries, active task ownership and audit state from protected main. Run current exact-head governance/architecture/repository checks, whole-diff review and any required independent review before the final scheduler/closeout merge.

**Package D exit gate:** the owner can launch the correct current Sol chats from aliases alone, Work serializes all shared state, and no stale adoption task/lease remains.

---

### Task 5: Start the first real Sol execution wave only after canonical adoption

**Files:**
- No fixed product paths in this governance plan; each future lane consumes its current merged allocation and existing lane-specific implementation plan.
- Work updates the live allocation/control-plane state through the normal coordinator lifecycle.

**Interfaces:**
- Consumes: terminal Packages A-D and current protected main.
- Produces: bounded lane execution under the new model, not additional execution-governance design.

- [ ] **Step 1: Resolve current launch sheet**

The owner starts only aliases marked runnable by the canonical scheduler on current main.

- [ ] **Step 2: Keep read-only preparation parallel to the critical mutating lane**

Use spare Sol chats for downstream dependency/API/test preparation, not speculative unallocated runtime implementation.

- [ ] **Step 3: Spend Codex only where execution materially benefits**

The active Sol lead decides whether a bounded implementation/debug/test/build/repository task benefits from Codex. Unsupported direct invocation becomes `CODEX_HANDOFF_REQUIRED` or safe Sol-only continuation; it never becomes fabricated evidence.

- [ ] **Step 4: Return every completed lane to Work for independent GitHub verification**

Work verifies exact head, changed paths, checks, reviews, E2E classification, shared-lease use and unresolved findings before integration/closeout.

- [ ] **Step 5: Re-run the independent Work auditor at material programme checkpoints**

At minimum audit after execution-model adoption and again before terminal programme closeout; additional audits are appropriate after high-risk Server Seam/Movement/Combat integration or any material control-plane finding.

## Plan self-review

- Spec coverage: all role hierarchy, concurrency, selective Codex, shared leases, wave scheduling, transition truth, escalation and owner launch-sheet requirements map to Tasks 1-5.
- Placeholder scan: no `TBD`, `TODO`, `implement later` or unspecified authority placeholders are permitted.
- Dependency consistency: Package A precedes Work specialization; Package B precedes Sol integration behavior; Package C precedes scheduler publication; Package D precedes first canonical Sol execution wave.
- Scope consistency: runtime/product architecture remains unchanged; future product writes are allocation-gated and occur outside this governance adoption plan.
- Current-state drift: Durability shared Cargo/CI lease allocation merged after the design snapshot; the plan records that as fresh provenance and requires every package to resolve newer live truth.