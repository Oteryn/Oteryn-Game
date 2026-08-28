# Durability PR #212 Provenance Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reconstruct a qualified Durability candidate on a clean branch while preserving, but not ratifying, the compromised PR #212 history.

**Architecture:** A coordinator-only allocation first transfers the exact #167 paths to one successor worker. The successor starts from the allocation merge SHA, treats PR #212 as read-only evidence, reconstructs only authorized path content without importing its ancestry, then performs a complete new TDD/validation/review lifecycle.

**Tech Stack:** Rust 1.94 workspace, SQLx 0.9.0, PostgreSQL 17 isolated E2E, GitHub Issues/PRs/Actions, native GitHub Codex review.

**Spec:** `docs/agents/tasks/active/OTV2-20260828-recover-durability-pr212-provenance.md`

## Global Constraints

- The successor is lawful only after the recovery allocation merges to protected `main`.
- Preserve `impl/game-durability-journal` and Draft PR #212 unchanged as forensic evidence; never force-push, reset, rebase, delete, qualify or merge them.
- Do not merge/cherry-pick/import the compromised ancestry; copy/reimplement only the specified owned-path content on the clean successor branch.
- No Cargo, lockfile, Foundation, Server Seam, registry, workflow, production or external-repository mutation is authorized.
- Historical tests, CI and reviews may guide investigation but cannot qualify the successor head.

---

### Task 1: Freeze and verify the evidence boundary

**Files:**

- Read: `AGENTS.md`
- Read: `docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md`
- Read: `docs/agents/GITHUB_ONLY_EXECUTION.md`
- Read: `docs/agents/tasks/active/OTV2-20260828-recover-durability-pr212-provenance.md`
- Read-only evidence: PR #212; Issue #240 comment `5453015299`; commits `8f6f1de4…`, `cd808d…`, `73e17f…`, `a4d1d…`, `fb30f…`

**Interfaces:**

- Consumes: protected allocation merge SHA and immutable historical evidence.
- Produces: a fresh verified recovery admission record and no mutation of the original branch.

- [ ] **Step 1: Verify the allocation is canonical**

  Resolve GitHub `main`, Issue #240, Issue #167 and this task. Confirm that the allocation PR is merged, `main` equals its recorded merge SHA, and the old PR #212 branch/head is retained as evidence-only.

- [ ] **Step 2: Record the source boundary**

  In the successor task/PR record `8f6f1de4…`, `cd808d…`, `73e17f…`, the prior paused `a4d1…` evidence and the Issue #240-bound `fb30f…` source manifest. State explicitly that no later task claims these commits were authorized when made.

- [ ] **Step 3: Verify the old candidate remains untouched**

  Compare the observed PR #212 head and branch history to the frozen evidence record. If either moved, stop qualification and return the exact drift to the control plane; do not repair or rewrite it.

### Task 2: Create the clean successor candidate

**Files:**

- Modify: only the eleven `owned_paths_after_allocation_merge` paths in the recovery task.
- Create/update: the successor task record on its allocated path.

**Interfaces:**

- Consumes: protected allocation merge SHA and read-only source blobs from PR #212.
- Produces: `recovery/durability-212-owned-successor` with no compromised ancestor.

- [ ] **Step 1: Create one isolated successor worktree and branch**

  Create `recovery/durability-212-owned-successor` directly from the allocation merge SHA. Verify its merge base is that SHA and that it has no `cd808d…`/`73e17f…` ancestor.

- [ ] **Step 2: Port only authorized path content**

  Use only the Issue #240-bound `fb30f…` blob identities as read-only input. Reconstruct only the exact allocated Durability module, migration, binary, build and PostgreSQL test paths; do not bring in the original commit ancestry or any shared file.

- [ ] **Step 3: Check allocation conformance**

  Run `git diff --name-only <allocation-merge-sha>...HEAD` and compare every path to the recovery task allowlist. Any extra path is `SHARED_LEASE_REQUIRED` or out of scope, not a reason to edit it.

### Task 3: Re-establish Durability behavior with TDD

**Files:**

- Modify/Test: `apps/game-server/src/durability/admission_journal.rs`
- Test: `apps/game-server/tests/durability_postgres.rs`

**Interfaces:**

- Consumes: Foundation V1 reconnect boundary and the accepted durable journal contracts.
- Produces: a clean candidate with fresh regression evidence for every recovered semantic behavior.

- [ ] **Step 1: Re-run or recreate focused RED evidence**

  Before each recovered behavioral repair, make the selected Durability test fail on the clean successor for the exact missing invariant. Historical red/green commits only locate cases; they do not replace a fresh observation.

- [ ] **Step 2: Prove retained reservation fencing through COMMIT**

  Add a real isolated PostgreSQL contention regression for the current Codex P2 acceptance predicate: after a PREPARED validation, concurrent removal or reassignment of the exact retained transport reservation cannot leave COMMIT able to publish ACTIVE authority from incomplete evidence. The lane lead selects the path-local SQL fencing mechanism; changing Foundation contracts or transaction semantics beyond the allocation is an escalation.

- [ ] **Step 3: Implement the smallest permitted repair**

  Modify only the allocated Durability paths so the RED test passes while preserving exact attempt, session, lease, scope, controller-generation, transport-ref, deadline, nonce and FND-02 fences.

- [ ] **Step 4: Run focused GREEN evidence**

  Run the affected PostgreSQL test names plus the full Durability PostgreSQL harness. Record the exact command, result and isolated database version in the successor task/PR.

### Task 4: Freeze and qualify the successor

**Files:**

- Review: all changed paths on the successor branch.

**Interfaces:**

- Consumes: a clean allocated candidate.
- Produces: `READY_FOR_INTEGRATION` only when all exact-head gates prove it.

- [ ] **Step 1: Run current required validation**

  Run the task-required migration fresh/compatibility/checksum/ahead/behind/dirty/interruption/runtime-DDL-denial evidence, durability fencing/replay/outage/recovery evidence, formatting, strict Clippy, package/workspace tests and exact-head GitHub workflows including `game-gate`.

- [ ] **Step 2: Perform whole-diff and allocation review**

  Verify branch ancestry, exact changed paths, migration contents, dependency diff, zero unrelated changes and no unresolved required review threads.

- [ ] **Step 3: Obtain a fresh native Codex review**

  The allocated Sol Durability lead freezes the exact successor head and uses the covered native GitHub Codex review route. Repair only successor findings, then re-run invalidated checks and request a fresh review for every material head change.

- [ ] **Step 4: Return the integration handoff**

  Return the exact successor head, clean review state, CI/E2E evidence, allocation conformance and one deterministic coordinator action. Do not merge under the lane lead profile.

### Task 5: Retire the compromised candidate only after successor admission

**Files:**

- Update only through the active control-plane recovery closeout paths.

**Interfaces:**

- Consumes: an open successor PR with evidence preserved.
- Produces: PR #212 closed as superseded while its branch/history remain retained evidence.

- [ ] **Step 1: Verify the successor PR exists**

  Confirm the successor task/PR/branch are durable and link all frozen source identities before changing PR #212 lifecycle state.

- [ ] **Step 2: Close PR #212 as superseded**

  The control plane, not the successor lane lead, closes PR #212 with a durable explanation that it is retained forensic evidence and not a rejected technical result. Do not delete or rewrite its branch.

- [ ] **Step 3: Continue ordinary exact-head integration**

  The successor is integrated only after the normal review, CI, merge fencing, post-merge task archive and ownership-release gates are met.
