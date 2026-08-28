# Oteryn Game Autonomous Codex Review Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the owner from routine Codex review handoffs by giving allocated lane leads bounded standing authority to request fresh independent exact-head Codex reviews through GitHub and own the repair/re-review loop.

**Architecture:** Keep the existing Terra/Sol/Work control-plane architecture unchanged. Add one machine-readable Codex review policy, make root and docs-agent governance explicitly recognize its standing authorization, register that normative policy in the governance contract, and make the deterministic governance validator fail closed on authority/routing/gate drift. Active control planes treat the validated matrix as a deterministic merge gate rather than a technical judgment.

**Tech Stack:** GitHub PR review surfaces, Markdown/YAML/JSON governance, stdlib Python governance validation, existing Oteryn agent prompts and exact-head GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-27-oteryn-game-autonomous-codex-review-loop-design.md`

## Global Constraints

- Repository is `Oteryn/Oteryn-Game`; live protected `main` and GitHub lifecycle state outrank cached chat.
- The policy is ineffective until merged to protected `main`.
- Standing authorization covers only independent read-only exact-head review/audit and non-mutating test/reproduce/fuzz/static-analysis used for that review.
- All other owner-funded Codex/OpenAI/API usage remains per-invocation owner-authorized.
- Codex reviewer task/session must be fresh and materially non-authoring.
- Any material head change invalidates prior qualifying review.
- Canonical review transport is the GitHub PR when native Codex review capability is proven available; never fabricate invocation.
- Lane-lead risk self-tags may only increase review rigor; OPTIONAL/NOT_REQUIRED routing requires canonical independently attributable evidence or an explicit mechanical scope rule.
- A qualifying review requires exact-head successful review evidence and zero unresolved blocking findings/required threads.
- Terra retains zero technical discretion.
- No runtime, Cargo/workspace, protocol/schema/registry, production, secret, live-data or external-repository mutation.
- This authority-policy change requires genuinely independent exact-head review before merge.

---

### Task 1: Add deterministic Codex review policy and fail-closed validation

**Files:**
- Create: `docs/agents/CODEX_REVIEW_POLICY.json`
- Modify: `docs/agents/GOVERNANCE_CONTRACT.json`
- Modify: `tools/agents/validate_governance.py`

**Produces:** standing-authorization scope, independently provable risk routing, independence/gate semantics, and deterministic validation that prevents later silent weakening/deletion.

- [x] Define covered/prohibited operations.
- [x] Define `CODEX_REQUIRED`, optional and non-required risk classes.
- [x] Define GitHub PR transport and preferred `@codex review` trigger with capability proof.
- [x] Define fresh non-authoring reviewer and exact-head invalidation rules.
- [x] Define lane-owned repair/re-review loop and Terra zero-discretion behavior.
- [x] Make lane-lead self-tags escalation-only and require canonical proof for any downgrade.
- [x] Require explicit successful exact-head evidence plus zero unresolved blocking findings/required threads.
- [x] Register the policy in `GOVERNANCE_CONTRACT.json.required_documents`.
- [x] Load and fail-closed validate the policy's authority, risk routing, independence, prohibitions and gate invariants in `validate_governance.py`.

### Task 2: Update normative authorization rules

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/agents/AGENTS.md`
- Modify: `docs/agents/OWNER_FUNDED_AI_POLICY.md`

**Produces:** a canonical exception to per-invocation approval only for the exact bounded review policy.

- [x] Root governance recognizes `CODEX_REVIEW_POLICY.json` as standing owner authorization after merge.
- [x] Root governance requires allocated lane leads to apply the matrix and own request/repair/re-review.
- [x] Nearer docs-agent rule no longer blocks a covered standing-authorized review trigger.
- [x] Owner-funded policy keeps every non-covered AI invocation deny-by-default.
- [x] Prose mirrors fail-closed downgrade validation and exact-head success semantics from the machine contract.

### Task 3: Record durable task lifecycle

**Files:**
- Create: `docs/agents/tasks/active/OTV2-20260827-autonomous-codex-review-loop.md`

**Produces:** Issue #229 branch/ownership/acceptance/validation checkpoint.

- [x] Record exact admission main, branch, Issue, all nine owned paths and excluded scope.
- [x] Record the independent Codex P1 repair history without treating historical review as final qualification.
- [x] Record exactly one next action and final evidence model.

### Task 4: Qualify governance package

**Files:** all nine files in this plan/delivery scope.

- [x] Open one draft PR referencing Issue #229.
- [x] Verify diff contains governance/docs/tooling only.
- [ ] Run exact-head Agent governance, Architecture semantic audit, Merge authority audit and Merge gate on final unchanged head.
- [ ] Perform whole-diff author self-review on final unchanged head.
- [ ] Obtain genuinely independent non-authoring exact-head re-review after all P1 repairs.
- [ ] Require zero unresolved review threads and current-main readback before expected-head squash merge.

### Task 5: Post-merge activation

**Files:** no new product paths.

- [ ] Verify protected-main readback contains the standing authorization and validator registration.
- [ ] Close/archive Issue #229 task lifecycle through a separate bounded closeout if repository policy requires it.
- [ ] Only after canonical merge may active lane leads use the standing authorization without per-run owner confirmation.
