# Oteryn Game Autonomous Codex Review Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the owner from routine Codex review handoffs by giving allocated lane leads bounded standing authority to request fresh independent exact-head Codex reviews through GitHub and own the repair/re-review loop.

**Architecture:** Keep the existing Terra/Sol/Work control-plane architecture unchanged. Add one machine-readable Codex review policy, make root and docs-agent governance explicitly recognize its standing authorization, and require active control planes to treat the matrix as a deterministic merge gate rather than a technical judgment.

**Tech Stack:** GitHub PR review surfaces, Markdown/YAML/JSON governance, existing Oteryn agent prompts and exact-head GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-27-oteryn-game-autonomous-codex-review-loop-design.md`

## Global Constraints

- Repository is `Oteryn/Oteryn-Game`; live protected `main` and GitHub lifecycle state outrank cached chat.
- The policy is ineffective until merged to protected `main`.
- Standing authorization covers only independent read-only exact-head review/audit and non-mutating test/reproduce/fuzz/static-analysis used for that review.
- All other owner-funded Codex/OpenAI/API usage remains per-invocation owner-authorized.
- Codex reviewer task/session must be fresh and materially non-authoring.
- Any material head change invalidates prior qualifying review.
- Canonical review transport is the GitHub PR when native Codex review capability is proven available; never fabricate invocation.
- Terra retains zero technical discretion.
- No runtime, Cargo/workspace, protocol/schema/registry, production, secret, live-data or external-repository mutation.
- This authority-policy change requires genuinely independent exact-head review before merge.

---

### Task 1: Add deterministic Codex review policy

**Files:**
- Create: `docs/agents/CODEX_REVIEW_POLICY.json`

**Produces:** standing-authorization scope, risk matrix, independence, lane-loop and fallback semantics.

- [x] Define covered/prohibited operations.
- [x] Define `CODEX_REQUIRED`, optional and non-required risk classes.
- [x] Define GitHub PR transport and preferred `@codex review` trigger with capability proof.
- [x] Define fresh non-authoring reviewer and exact-head invalidation rules.
- [x] Define lane-owned repair/re-review loop and Terra zero-discretion behavior.

### Task 2: Update normative authorization rules

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/agents/AGENTS.md`
- Modify: `docs/agents/OWNER_FUNDED_AI_POLICY.md`

**Produces:** a canonical exception to per-invocation approval only for the exact bounded review policy.

- [ ] Root governance recognizes `CODEX_REVIEW_POLICY.json` as standing owner authorization after merge.
- [ ] Root governance requires allocated lane leads to apply the matrix and own request/repair/re-review.
- [ ] Nearer docs-agent rule no longer blocks a covered standing-authorized review trigger.
- [ ] Owner-funded policy keeps every non-covered AI invocation deny-by-default.

### Task 3: Record durable task lifecycle

**Files:**
- Create: `docs/agents/tasks/active/OTV2-20260827-autonomous-codex-review-loop.md`

**Produces:** Issue #229 branch/ownership/acceptance/validation checkpoint.

- [ ] Record exact admission main, branch, Issue, owned paths and excluded scope.
- [ ] Record exact one next action and final evidence model.

### Task 4: Qualify governance package

**Files:** all files in this plan.

- [ ] Open one draft PR referencing Issue #229.
- [ ] Verify diff contains governance/docs only.
- [ ] Run exact-head Agent governance, Architecture semantic audit, Merge authority audit and Merge gate.
- [ ] Perform whole-diff author self-review on unchanged exact head.
- [ ] Obtain genuinely independent non-authoring exact-head review.
- [ ] Require zero unresolved review threads and current-main readback before expected-head squash merge.

### Task 5: Post-merge activation

**Files:** no new product paths.

- [ ] Verify protected-main readback contains the standing authorization.
- [ ] Close/archive Issue #229 task lifecycle through a separate bounded closeout if repository policy requires it.
- [ ] Only after canonical merge may active lane leads use the standing authorization without per-run owner confirmation.
