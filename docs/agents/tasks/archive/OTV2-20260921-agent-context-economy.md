> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #712 merged as `8cf06e58ca8b0148e67bf41b9eec9d620136800d`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260921-agent-context-economy

```yaml
task_id: OTV2-20260921-agent-context-economy
title: Bound agent context acquisition
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/agent-context-economy-711
issue: 711
pr: 712
base_sha: a0f05f19d0caf6e44d704f59d383218b5663af73
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-21T11:32:55Z
updated_at: 2026-09-21T11:32:55Z
execution_policy: continuous_progress
owned_paths:
  - AGENTS.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/prompts/README.md
  - docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md
  - docs/agents/prompts/OTV2_SOL_SERVER_SEAM_LEAD.md
  - docs/agents/prompts/OTV2_SOL_CLIENT_QA_LEAD.md
  - docs/agents/prompts/OTV2_SOL_MOVEMENT_LEAD.md
  - docs/agents/prompts/OTV2_SOL_COMBAT_LEAD.md
  - docs/agents/prompts/OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md
  - docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md
  - tools/agents/validate_governance.py
  - docs/agents/tasks/active/OTV2-20260921-agent-context-economy.md
public_contracts:
  - Game agent context-routing and reusable-prompt startup contract
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Reduce recurring agent startup/context cost without weakening authority, lifecycle, validation, review or accepted product semantics. Preserve all historical evidence and make it on-demand rather than hot-path context.

## Architecture and source of truth

- PROVEN: root `AGENTS.md` and `docs/agents/CONTEXT_ROUTING.md` already require routed/minimum context.
- PROVEN: bound META 3.1 prompting policy defines reusable prompts as task-specific deltas and says normal tasks need not fetch the complete META bundle.
- PROVEN: Issue #162 has 728 comments and is a high-cost long-lived coordinator surface; complete-history reads are unnecessary for ordinary current-state decisions.
- PROVEN: the owner-facing operator runbook is currently named in five technical Sol worker mandatory-startup blocks.
- DERIVED: targeted live-state reads preserve correctness while avoiding repeated historical reconstruction.

## High-risk authority/recovery qualification

NOT_APPLICABLE — governance/prompt context routing only; no production mutation, authority-bearing session, persistence recovery, PREPARE/COMMIT or protected runtime operation is changed.

## Acceptance criteria

- [x] Root and context routing explicitly require targeted current-state reads and prohibit default bulk timeline/registry/programme reconstruction.
- [x] Work and Implementation coordinator startup contracts use targeted current-state reads.
- [x] Technical Sol worker startup no longer loads the owner-facing operator runbook.
- [x] WP3-v2 coordinator refreshes only gate-relevant locators instead of a fixed programme bundle.
- [x] Ordinary alias reuse no longer requires prompt evaluation.
- [x] Governance validation detects regression of these hot-path rules.
- [ ] Exact-head repository CI passes.

## Excluded scope

No deletion/rewrite of #162 history, allocation history, evidence, accepted architecture/contracts or prompt aliases. No runtime/gameplay/protocol/persistence/production/external-repository mutation.

## Implementation / findings

The change keeps provenance intact and changes only what must be loaded eagerly. Historical material remains available when a concrete claim requires it.

First exact-head Agent governance run reached the META adoption tests after the governance validator itself passed. It found an unnecessary lifecycle metadata bump for the Work coordinator (`1.9` -> `1.10`) against an existing pinned contract test. The repair reuses the protected lifecycle registry unchanged; prompt startup semantics are changed without rewriting unrelated lifecycle metadata.

## Validation

### Focused

- command/run: Agent governance validator on exact PR head
- result: pending

### Component/integration

- command/run: META inherited prompt-policy + repository policy CI
- result: pending

### E2E

- scenario: NOT_APPLICABLE — prompt/governance context routing only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: none before publication
- verdict: candidate ready for exact-head CI

## Independent review

- required: NO — governance/prompt routing cleanup does not alter runtime, production authority or product contracts; repository CI remains required.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: first exact-head governance run isolated unnecessary lifecycle-version bump; repair reuses protected registry unchanged
status: validating
branch: agent/agent-context-economy-711
head_sha: null
pr: 712
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish one exact candidate, open PR, and require exact-head governance/META/repository validation
```
