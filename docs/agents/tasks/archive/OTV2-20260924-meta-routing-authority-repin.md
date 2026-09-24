> Lifecycle closeout: **ARCHIVED AFTER CANONICAL PR MERGE**. Canonical PR #841 merged as `b003913926b7de941d0ed6b32b438eeb0fe96c04`. This packet is no longer active; any nonterminal wording below is retained only as historical provenance.

# OTV2-20260924-meta-routing-authority-repin

```yaml
task_id: OTV2-20260924-meta-routing-authority-repin
title: Repin Game to final META integration-routing authority
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/meta-routing-repin-835
pr: 841
base_sha: f812f6dc5586f0120a6e832fe58b7edd5c6ea474
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: coordination-agent
created_at: 2026-09-24T13:40:00Z
updated_at: 2026-09-24T14:10:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - tools/agents/tests/test_meta_agent_policy_adoption.py
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md
  - docs/agents/tasks/active/OTV2-20260924-meta-routing-authority-repin.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn#224
blocks: []
cross_repository_coordination_id: META-INTEGRATION-ROUTING-REPIN
external_repositories:
  - Oteryn/Oteryn
```

## Outcome

Repin Game's immutable META 3.1 provider binding from its prior protected ancestor to final protected META `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, which contains the reviewed organization direct/delegated protected-integration routing guard.

## Architecture and source of truth

- PROVEN: Oteryn/Oteryn PR #224 protected-integrated at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb` with successful real `merge_group` META CI.
- PROVEN: Game already contains the provider-local direct/delegated routing consumption introduced by protected PR #817.
- DERIVED: this task needs the immutable authority repin, exact-authority regression and only the minimal wording repairs required for three reusable prompts to defer route selection to the bound META router; it must not re-copy META routing policy into Game.
- PROVEN: fresh validation of the repinned candidate identified exactly three reusable prompt consumers whose route wording was rejected by the new central policy; those three prompt paths are therefore explicitly owned by this repair.
- UNKNOWN: final exact Game candidate SHA until PR creation/freeze.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this governance-only task performs no runtime, durable, production or authority-bearing Game mutation. Protected integration remains independently fenced by Game Merge Queue and `game-gate`.

## Acceptance criteria

- [ ] Binding authority commit equals `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- [ ] Game provider-adoption regression expects the same exact authority.
- [ ] Bound META provider-policy validation accepts all changed reusable prompt consumers without provider-local route selection.
- [ ] Exact-head Agent Governance / required Game CI are green.
- [ ] Applicable independent review has zero unresolved material findings.
- [ ] Integration occurs through protected Merge Queue with real `merge_group` `game-gate` and protected-main readback.

## Excluded scope

No gameplay/runtime/protocol/persistence/Cargo/vendor/workflow/ruleset/protection/production/secret mutation and no reusable-prompt changes beyond the three explicitly owned integration-wording repairs. No direct merge, generic auto-merge, bypass, force/reset/rebase or provider-local Merge Queue bridge.

## Validation

### Focused

- command/run: pending hosted exact-head governance
- result: pending

### Component/integration

- command/run: NOT_APPLICABLE — immutable governance coordinate only
- result: NOT_APPLICABLE

### E2E

- scenario: NOT_APPLICABLE — no product behavior changes
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: governance-only
- result: pending

## Self-review

- exact head: pending
- method/reviewer: coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — immutable organization policy authority movement
- exact head: pending
- method/auditor: Codex review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #817 is prior routing-consumer repair, not superseded
- protected auto-merge: not used as a substitute for bound routing
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: final META authority authenticated; bounded Game repin authored
status: validating
branch: governance/meta-routing-repin-835
head_sha: null
pr: 841
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: freeze the successor exact head and consume fresh exact-head CI/review evidence
```
