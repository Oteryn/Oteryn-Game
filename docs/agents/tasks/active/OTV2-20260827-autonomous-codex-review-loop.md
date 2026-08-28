# OTV2-20260827-autonomous-codex-review-loop

```yaml
task_id: OTV2-20260827-autonomous-codex-review-loop
title: Authorize autonomous GitHub Codex review loop
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/autonomous-codex-review-loop-20260827
pr: 230
base_sha: 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: governance-authoring-session
created_at: 2026-08-27T21:55:30Z
updated_at: 2026-08-28T05:28:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/OWNER_FUNDED_AI_POLICY.md
  - docs/agents/CODEX_REVIEW_POLICY.json
  - tools/agents/validate_governance.py
  - docs/agents/tasks/active/OTV2-20260827-autonomous-codex-review-loop.md
  - docs/superpowers/specs/2026-08-27-oteryn-game-autonomous-codex-review-loop-design.md
  - docs/superpowers/plans/2026-08-27-oteryn-game-autonomous-codex-review-loop.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

An allocated lane lead can autonomously request and consume a fresh independent exact-head Codex review through the canonical GitHub PR without requiring the owner to relay prompts or approve every covered review invocation. The lane lead retains repair/re-review ownership until required review passes, while Terra/Work mechanically verify the resulting exact-head evidence.

## Architecture and source of truth

- `PROVEN`: protected admission main is `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`.
- `PROVEN`: pre-change root `AGENTS.md` required explicit owner authorization for every owner-funded AI invocation.
- `PROVEN`: pre-change `docs/agents/OWNER_FUNDED_AI_POLICY.md` forbade standing authorization.
- `PROVEN`: native Codex GitHub review is now configured for this repository and produced independent exact-head findings on PR #230.
- `DERIVED`: without a standing authorization, the owner remains a manual relay in every Codex review loop.
- `PROVEN`: Issue #229 records explicit owner scope for this governance change.
- `PROVEN`: PR #230 contains the bounded standing-authorization implementation and remains non-canonical until merged.
- `PROVEN`: the first real Codex review exposed three P1 governance gaps: worker-controlled risk downgrade, missing explicit successful-review gate semantics, and missing deterministic validation/registration of the normative Codex policy.

## Acceptance criteria

- [x] Machine-readable review policy defines exact standing-authorized and prohibited operations.
- [x] Risk matrix defines deterministic `CODEX_REQUIRED` classes.
- [x] Lane-lead self-tags cannot reduce required review; downgrade requires mechanically provable canonical authority.
- [x] Fresh non-authoring reviewer and exact-head re-review rules are explicit.
- [x] Successful review requires exact-head success evidence plus zero unresolved blocking findings/required threads.
- [x] Root governance recognizes the standing authorization only after protected-main merge.
- [x] Nearer docs-agent policy does not conflict with covered review invocations.
- [x] Owner-funded AI policy preserves per-invocation approval for every non-covered AI use.
- [x] Lane lead owns candidate -> Codex review -> repair -> fresh review loop without owner relay.
- [x] Terra/Work inherit deterministic risk/review routing from root policy and do not adjudicate technical findings.
- [x] `CODEX_REVIEW_POLICY.json` is registered as a required governance document.
- [x] `validate_governance.py` loads the policy and fail-closed validates its authority, routing, independence, prohibition and gate invariants.
- [ ] Exact-head governance/architecture/merge-authority/merge-gate checks pass on the final unchanged head.
- [ ] Whole-diff author self-review has no blocking finding on the final unchanged head.
- [ ] Genuinely independent non-authoring exact-head re-review has no P0/P1 finding.
- [ ] Zero unresolved review threads before merge.

## Excluded scope

No runtime, Cargo/workspace, protocol/schema/registry, production/protected environment, secret, live-data, external-repository or branch-protection mutation. No Codex invocation is authorized by the unmerged branch itself. No implementation or merge authority is granted to a Codex reviewer.

## Implementation / findings

Issue #229 and PR #230 are active. The final governance package now spans nine governance/docs/tooling paths after independent Codex review correctly required deterministic registration and validation of the new normative policy. The policy itself remains bounded to review authority; the validator additions only fail closed on later authority drift. Final exact head, CI, self-review and independent-review evidence must remain external GitHub evidence; do not add a self-referential evidence-only commit after candidate freeze.

## Validation

### Focused

- command/run: exact-head Agent governance / repository governance validator
- result: pending final head

### Component/integration

- command/run: Architecture semantic audit / Merge authority audit / Merge gate
- result: pending final head

### E2E

- scenario: NOT_APPLICABLE — governance/docs only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: external PR evidence
- trigger source: pull_request
- workflow/run/job: pending final head
- runner assignment: GitHub Actions
- classification: governance authority-policy change
- result: pending final head

## Self-review

- exact head: external PR review evidence
- method/reviewer: governance-authoring-session
- material findings: pending final head
- verdict: pending final head

## Independent review

- required: YES — owner-funded AI authority-policy expansion/change
- exact head: external PR review evidence
- method/auditor: genuinely independent non-authoring Codex reviewer
- prior reviewed head: `f7bac3c0d4541629b2a90325a4bbc12c1752e499` — historical after P1 repairs
- prior material findings: three P1, repaired in later heads
- final verdict: pending fresh exact-head re-review

## PR and closeout

- changed-file review: nine governance/docs/tooling paths after Codex-required validator integration
- unresolved review threads: pending fresh re-review/final readback
- related/superseded PRs: #214 execution architecture; #223 auditor evidence-write authority
- protected auto-merge: disabled until independent exact-head review
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: three Codex P1 findings repaired; normative policy registered and fail-closed validator added
status: validating
branch: docs/autonomous-codex-review-loop-20260827
head_sha: external_pr_evidence
pr: 230
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
ci_trigger_source: pull_request
ci_check_generation: current_pr_head
ci_checks_for_current_head: pending
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_actions
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: fresh_codex_re_review_authorization_after_final_head_qualification
blocker: final_exact_head_qualification_and_independent_re_review_pending
next_action: qualify the final unchanged PR #230 head with exact-head CI and whole-diff self-review
```
