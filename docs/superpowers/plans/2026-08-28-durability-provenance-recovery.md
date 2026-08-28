# Durability Provenance Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recover the #167 Durability implementation onto a clean, reviewable successor history without rewriting or retroactively authorizing the corrupted PR #212 branch history.

**Architecture:** The active Work control plane first merges a docs-only recovery allocation from exact protected `main`. Only after that allocation is canonical may a successor Durability branch be created from the recorded protected-main base and reconstruct the current ownership-shaped #212 candidate by copying only explicitly allocated files as read-only source material. PR #212 remains immutable historical evidence and cannot qualify or merge.

**Tech Stack:** GitHub Issues/PRs/Actions, repository agent governance, Rust 1.94, SQLx 0.9.0, PostgreSQL 17 test harness, Superpowers execution/review workflows.

**Spec:** `Oteryn/Oteryn-Game#240`

## Global Constraints

- Canonical repository: `Oteryn/Oteryn-Game` only.
- Recovery admission main: `7c2da078596a7d2e27c3066ff74ac69b8b7f9af6` unless protected `main` moves before allocation merge; any move requires non-force refresh and requalification of the allocation PR.
- Do not force-push, rewind, rebase, replace, delete or merge `impl/game-durability-journal` / PR #212.
- Do not ratify `cd808d396018832b632be26911105a36f0cb7a20`, `73e17f418c63ec038f5aa7ef8f0888ac74b75aa2`, or later unauthorized writes as having been authorized when performed.
- Historical #212 code/tests/reviews are read-only input/evidence, not qualification evidence for the successor.
- Successor write authority is limited to the exact runtime/test paths listed in Task 2 plus its own successor task packet.
- No Cargo, `Cargo.lock`, Foundation, Server Seam, workflow, registry, governance, production, secret, live-data or external-repository authority transfers to the successor worker.
- Required exact-head CI and required independent review must be produced on the successor head; historical #212 CI/reviews cannot satisfy those gates.
- The only automatically admissible historical source snapshot is exact PR #212 commit `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`. Any later #212 head is untrusted for reconstruction unless the active control plane separately records a new exact source-admission decision before copying any bytes from it.

---

### Task 1: Canonicalize the recovery allocation

**Files:**
- Create: `docs/agents/tasks/active/OTV2-20260828-durability-provenance-recovery.md`
- Create: `docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md`
- Create: `docs/superpowers/plans/2026-08-28-durability-provenance-recovery.md`

**Interfaces:**
- Consumes: Issue #240, parent coordinator #162, protected `main`, PR #212 immutable evidence.
- Produces: one exact successor branch name, exact owned-path allowlist, exact recovery sequencing and one worker task that remains non-mutating until this allocation PR merges.

- [ ] **Step 1: Publish only the docs-only recovery allocation from protected main**

Create the allocation branch from the exact protected-main SHA and write only the three files listed above.

- [ ] **Step 2: Verify allocation scope**

Require the PR diff to contain only the three coordinator/recovery documentation paths and no runtime, Cargo, workflow, registry or existing worker-owned file.

- [ ] **Step 3: Run allocation qualification**

Require repository governance validation, whole-diff self-review, exact-head repository checks, current-main reconciliation and any independent review required by protected-main policy for `docs/agents/**` changes.

- [ ] **Step 4: Merge with expected head and read back protected main**

Do not release the successor worker until the allocation merge SHA is proven on protected `main`.

---

### Task 2: Reconstruct the Durability candidate on clean history

**Files:**
- Create/modify on successor only: `apps/game-server/build.rs`
- Create/modify on successor only: `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- Create/modify on successor only: `apps/game-server/src/bin/oteryn-game-migrate.rs`
- Create/modify on successor only: `apps/game-server/src/durability/admission_journal.rs`
- Create/modify on successor only: `apps/game-server/src/durability/db.rs`
- Create/modify on successor only: `apps/game-server/src/durability/mod.rs`
- Create/modify on successor only: `apps/game-server/src/durability/schema.rs`
- Create/modify on successor only: `apps/game-server/tests/durability_postgres.rs`
- Create/modify on successor only: `apps/game-server/tests/support/postgres.rs`
- Modify on successor only: `docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md`

**Interfaces:**
- Consumes: allocation merge SHA from Task 1 and read-only file contents from exact historical PR #212 source snapshot `fb30fba2a888835dfc7cbde27f940b79d7bfe05d` only.
- Produces: `impl/game-durability-journal-recovery-240`, one new Draft PR, clean ancestry rooted at the allocation-recorded protected main, and an ownership-shaped candidate.

- [ ] **Step 1: Create the successor branch from the allocation-recorded protected main**

Branch name: `impl/game-durability-journal-recovery-240`.

Do not branch from PR #212 and do not cherry-pick commits from its corrupted ancestry.

- [ ] **Step 2: Reconstruct only the exact allocated file snapshots**

Copy file contents, not commits, from exact source snapshot `fb30fba2a888835dfc7cbde27f940b79d7bfe05d` into the successor allowlist. Any needed path outside the allowlist is `SHARED_LEASE_REQUIRED` and blocks that mutation. Do not consume bytes from any later #212 head without a new durable control-plane source-admission decision.

- [ ] **Step 3: Re-establish TDD evidence**

Re-run the real PostgreSQL regression suite on the successor. The successor must independently prove the previously discovered reconnect-lifecycle cases, including committed PREPARE replay after process restart, exact incumbent binding on expired PREPARED replay, and fail-closed fast-reconnect generation reconstruction.

- [ ] **Step 4: Verify ownership-shaped diff**

Compare successor head to its recorded protected-main base and require no changed path outside the nine runtime/test files plus the successor task packet.

---

### Task 3: Qualify the successor independently

**Files:**
- No new production paths beyond Task 2.
- Evidence only in GitHub PR/review/check state after final candidate freeze.

**Interfaces:**
- Consumes: clean successor PR/head and Task 2 focused evidence.
- Produces: exact-head qualification state suitable for coordinator integration consideration.

- [ ] **Step 1: Run focused and component validation**

Require Rust formatting, strict Clippy for the affected targets, complete applicable game-server tests and isolated PostgreSQL 17 Durability E2E.

- [ ] **Step 2: Freeze the candidate head**

After implementation/task metadata are complete, record the exact final SHA in immutable PR/task evidence. Do not move the head for status-only bookkeeping.

- [ ] **Step 3: Obtain fresh independent exact-head review**

Apply current protected-main `CODEX_REVIEW_POLICY.json`. The allocated Durability lane lead owns any required covered `@codex review` request; the Work coordinator does not trigger it on the lane's behalf.

- [ ] **Step 4: Run exact-head repository CI**

Require the full applicable Merge gate/Rust/Linux/Windows/supply-chain/governance/architecture checks on the same unchanged successor head.

- [ ] **Step 5: Reconcile all blocking review threads**

Zero unresolved P0/P1/P2 findings and zero required unresolved review threads before `READY_FOR_INTEGRATION`.

---

### Task 4: Retire the corrupted delivery and integrate the successor

**Files:**
- Historical PR #212/branch: metadata/state only; no branch mutation.
- Successor task archive/status changes only after successful merge.

**Interfaces:**
- Consumes: clean exact-head successor qualification.
- Produces: one canonical Durability merge, preserved incident evidence, released ownership and downstream Server Seam re-evaluation.

- [ ] **Step 1: Close PR #212 as superseded historical evidence**

Close without merge only after the successor PR exists and links #212/#240. Preserve the original branch and all review/CI history.

- [ ] **Step 2: Integrate the successor with expected-head protection**

Reconcile against current protected main without force/rewind, rerun invalidated evidence if the head moves, then squash merge only when every gate is proven.

- [ ] **Step 3: Post-merge readback and closeout**

Verify protected-main merge SHA, archive/release the successor task, close #240 completed, reconcile #167, and recompute Server Seam readiness from fresh live state.
