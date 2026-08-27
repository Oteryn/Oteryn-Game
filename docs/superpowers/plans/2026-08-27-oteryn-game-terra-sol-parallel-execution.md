# Oteryn Game Terra-Control-Plane + Sol Parallel Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package the owner-requested execution architecture in which ChatGPT Work on Terra High is a deterministic no-technical-discretion control plane, while GPT-5.6 Sol Extra High lane leads perform deep implementation reasoning and a Sol Supervising Architect resolves material architecture decisions.

**Architecture:** Preserve the current Oteryn Game runtime architecture and implementation DAG. Add a new Terra-specific control-plane profile rather than mutating the currently reusable Work coordinator prompt in place, package five current critical-path Sol lead profiles plus one post-VSL expansion profile, register them, and publish a deterministic wave scheduler. The package changes execution/governance only and remains non-canonical until exact-head governance plus genuinely independent review passes.

**Tech Stack:** GitHub Issues/branches/PRs/Actions, Markdown/YAML/JSON agent governance, repository prompt lifecycle/evaluation tooling, ChatGPT Work/Terra High, GPT-5.6 Sol Extra High.

**Spec:** `docs/superpowers/specs/2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md`

## Global Constraints

- Canonical repository: `Oteryn/Oteryn-Game`.
- Admission protected main: `4c395ece416c3c56aed5607653a0730c52dcb3fd`; every execution step resolves fresh live state.
- Governance Issue: #213.
- No runtime/product/Cargo/workspace/protocol/schema/registry/production/external-repository mutation in this package.
- Terra has zero technical/architecture discretion; it applies only explicit deterministic state/merge predicates.
- Sol lane leads may write only under current exact merged allocation and exact owned paths.
- Material public API/schema/persistence/trust/resource/cross-lane decisions escalate to the Sol Supervising Architect.
- Product/scope/authority decisions outside existing architecture escalate to the owner.
- Shared Cargo/composition/registries/contracts/workflows/governance remain serialized.
- Existing `Oteryn: work coordinator` remains unchanged by this package; the new Terra profile is additive until separately superseded.
- Existing active worker branches/PRs, including Durability #167/#212, remain untouched.
- Because the package redistributes integration responsibility and narrows Work technical discretion, exact-head independent review is required before canonical merge.

---

### Task 1: Register the governance delivery lifecycle

**Files:**
- Create: `docs/agents/tasks/active/OTV2-20260827-terra-sol-parallel-execution.md`
- Existing Issue: #213

**Interfaces:**
- Consumes: root `AGENTS.md`, `docs/agents/AGENTS.md`, current protected main, Issue #213.
- Produces: one exact governance task/branch/owned-path record for this package.

- [ ] **Step 1: Bind exact admission state**

Record Issue #213, branch `docs/terra-sol-parallel-agent-architecture-20260827`, admission main `4c395ece416c3c56aed5607653a0730c52dcb3fd`, exact owned governance paths and no product authority.

- [ ] **Step 2: Record acceptance and independent-review requirement**

Acceptance must name the Terra no-discretion rule, prompt family, scheduler, README/lifecycle registration, prompt evaluation and independent exact-head review.

- [ ] **Step 3: Keep closeout fields non-self-referential**

Do not commit a fake final SHA into the file. Final head/CI/review evidence belongs in immutable PR/check evidence once the head is frozen.

---

### Task 2: Package the Terra control plane and architecture escalation role

**Files:**
- Create: `docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md`
- Create: `docs/agents/prompts/OTV2_SOL_SUPERVISING_ARCHITECT.md`

**Interfaces:**
- Terra consumes: canonical DAG, live allocations, Sol evidence packets, CI/review state.
- Terra produces: deterministic dispatch/integration/wait/escalation actions only.
- Architect consumes: durable `ARCHITECTURE_ESCALATION_REQUIRED` packets.
- Architect produces: durable bounded architecture resolution or `OWNER_DECISION_REQUIRED`.

- [ ] **Step 1: Encode Terra hard no-discretion boundary**

The prompt must explicitly forbid product/runtime edits, technical design selection, API/schema/resource/ownership decisions and review-finding adjudication.

- [ ] **Step 2: Encode deterministic release predicates**

Require proven current main, Issue/task, merged allocation, owned paths, terminal prerequisites, zero overlap and zero unresolved architecture/policy conflict before dispatching mutation.

- [ ] **Step 3: Encode deterministic integration predicates**

Require `READY_FOR_INTEGRATION`, unchanged exact head, allocation-conformant paths, required tests/E2E, exact-head CI, independent review where required and zero unresolved threads before merge execution.

- [ ] **Step 4: Encode decision routing**

Use `LANE_DECISION_REQUIRED`, `ARCHITECTURE_ESCALATION_REQUIRED`, `OWNER_DECISION_REQUIRED` and `POLICY_CONFLICT` exactly.

- [ ] **Step 5: Encode Architect authority**

The architect may resolve material architecture within existing owner-approved authority, but must not turn architecture resolution into implicit runtime write authority. Owner-level product/scope/authority choices return `OWNER_DECISION_REQUIRED`.

---

### Task 3: Package the five current Sol implementation leads

**Files:**
- Create: `docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md`
- Create: `docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md`

**Interfaces:**
- Each lead consumes its existing `OTV2_IMPL_*` prompt, active Issue/task/allocation and live main.
- Each lead produces one exact evidence packet ending in `READY_FOR_INTEGRATION`, wait, lane decision or architecture escalation.

- [ ] **Step 1: Standardize all lead startup rules**

Every lead resolves live GitHub, reads root/nearest instructions, verifies exact allocation before writes, owns one branch/PR and refuses sibling/unowned/shared-path assumptions.

- [ ] **Step 2: Standardize local decision authority**

Allow only path-local implementation choices preserving accepted contracts/resource limits/ownership. Material choices escalate.

- [ ] **Step 3: Bind Durability Lead**

Resolve Issue #167 and any live successor/PR. Preserve existing branch history; never restart merely because main advanced. Consume current Foundation reconnect boundary and SQLx/PostgreSQL contracts. Persistence/fencing changes retain high-risk independent review requirements.

- [ ] **Step 4: Bind Server Seam Lead**

Default to `READ_ONLY_PREPARATION` until the durable adapter prerequisite is terminal. When allocated, own production listener/client-entry integration, malformed/oversized/unknown input, admission/reconnect fencing, backpressure/drain and truthful Tier 1 evidence.

- [ ] **Step 5: Bind Client/QA Lead**

Default to read-only until compatible Server Seam is merged. When allocated, own exact native-client integration plus truthful Tier 1/Tier 2 orchestration without substituting synthetic/direct-domain evidence for physical boundaries.

- [ ] **Step 6: Bind Movement Lead**

Require current #139 gate, compatible Client/QA, Interaction and exact Movement allocation. The lead must not choose unregistered resource maxima.

- [ ] **Step 7: Bind Combat Lead**

Require merged Movement plus current Ability/Interaction/Durability/Client/QA readiness. Persistence/value/item/resource gaps outside accepted authority escalate before mutation.

- [ ] **Step 8: Standardize evidence return**

All five prompts must return exact Issue/task/base/head/PR/changed paths/shared lease/tests/E2E/self-review/independent-review/escalation/unresolved findings/recommended control-plane action.

---

### Task 4: Package post-VSL expansion and the wave scheduler

**Files:**
- Create: `docs/agents/prompts/OTV2_SOL_POST_VSL_EXPANSION.md`
- Create: `docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md`

**Interfaces:**
- Expansion lead consumes: terminal VSL result plus all current accepted remaining Game backlog/architecture.
- Expansion lead produces: exact future-wave decomposition and child lifecycle proposals, not runtime implementation.
- Scheduler consumes: live GitHub state and canonical prompt/transition rules.
- Scheduler produces: owner launch sheet and deterministic promotion gates.

- [ ] **Step 1: Make post-VSL expansion read-only by default**

It may inventory/decompose remaining work into World/Content, NPC/AI, Player Systems/Economy, Native Client/Renderer and Tooling/Ops families only where live accepted authority supports them. It must not create speculative runtime writes.

- [ ] **Step 2: Encode current V0-V4 launch sequence**

Record Durability -> Server Seam -> Client/QA -> #139/Movement -> Combat and read-only preparation opportunities.

- [ ] **Step 3: Encode concurrency limits**

One Terra control plane, one independent auditor, up to five Sol chats, normally at most two mutating Sol leads; a third requires proven total path/shared-surface independence plus a documented throughput reason.

- [ ] **Step 4: Encode promotion rules**

No manual inference: every transition is triggered by terminal dependency evidence plus fresh allocation.

- [ ] **Step 5: Encode owner launch-sheet fields**

Each row: alias, requested model/effort, mode (`READ_ONLY`/`MUTATING`), exact prerequisite, live state, next alias unlocked.

---

### Task 5: Register prompts and aliases

**Files:**
- Modify: `docs/agents/prompts/README.md`
- Modify: `docs/agents/PROMPT_LIFECYCLE.json`

**Interfaces:**
- Consumes: all new prompt files.
- Produces: canonical short-invocation discoverability after merge.

- [ ] **Step 1: Register control aliases**

Add `Oteryn: terra game coordinator` and `Oteryn: sol supervising architect`, explicitly leaving existing `Oteryn: work coordinator` reusable and unsuperseded.

- [ ] **Step 2: Register current Sol lead aliases**

Add Durability, Server Seam, Client/QA, Movement and Combat aliases.

- [ ] **Step 3: Register post-VSL expansion alias**

Add `Oteryn: sol post-vsl expansion` as architecture/planning-only by default.

- [ ] **Step 4: Add lifecycle entries**

Each entry must be `reusable`, version `1.0`, state exact authority/ownership scope and state that alias existence grants no write authority without live allocation.

---

### Task 6: Validate, independently review and publish the PR

**Files:**
- Review the complete Issue #213 branch diff.

**Interfaces:**
- Consumes: Tasks 1-5 exact branch head.
- Produces: a reviewable governance PR; merge remains blocked until required evidence passes.

- [ ] **Step 1: Validate prompt structure**

Evaluate every new prompt against `docs/agents/PROMPT_EVAL_STANDARD.md`: Authority, Resolution, Ownership, Architecture, Completeness, Evidence, Validation, Autonomy, Handover, Safety. Any material fail blocks readiness.

- [ ] **Step 2: Run governance validation**

Require `python tools/agents/validate_governance.py` through an available trusted execution path or exact-head GitHub Actions. Runtime/E2E is `NOT_APPLICABLE` because this branch changes governance/prompt documents only.

- [ ] **Step 3: Self-review complete branch**

Check no prompt grants implicit write authority, no Terra technical discretion survives, no lane can grab shared surfaces, no alias is presented as current runtime allocation and no historical evidence is rewritten.

- [ ] **Step 4: Obtain genuinely independent exact-head review**

Because the package changes execution/merge authority boundaries, an independent reviewer must review the exact final head. The authoring session cannot count as independent review.

- [ ] **Step 5: Open PR and keep it unmerged until gates pass**

PR must reference #213, state governance-only scope, list aliases, document that existing worker branches are untouched, and require exact-head governance/architecture/merge-authority/merge-gate plus zero unresolved threads.

- [ ] **Step 6: Merge only after current-main reconciliation**

If protected main advances, preserve branch history, normally merge-up current main if required by policy, rerun invalidated checks/review, then squash/expected-head merge only when all deterministic predicates are satisfied.