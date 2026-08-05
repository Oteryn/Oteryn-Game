# OTV2-20260805-global-architecture-continuation-prompt

```yaml
task_id: OTV2-20260805-global-architecture-continuation-prompt
title: Persist the global architecture continuation prompt and decision register
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/global-architecture-continuation-prompt-20260805
pr: 11
base_sha: 98b7193f285f64268bee16969a0d2d1c5b026132
head_sha: pending-final-validation-checkpoint
owner: chatgpt-github-agent
created_at: 2026-08-05T12:04:00+02:00
updated_at: 2026-08-05T12:18:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/prompts/README.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/README.md
  - docs/agents/tasks/active/OTV2-20260805-global-architecture-continuation-prompt.md
public_contracts:
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
depends_on:
  - ADR-0001 through ADR-0005
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
blocks: []
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Provide a self-contained repository execution prompt that lets a replacement agent continue the full Oteryn v2 architecture decision programme without chat history, while preserving one durable global register of unresolved project domains and one exact next action.

The delivered state:

- preserves all accepted ADR invariants;
- includes the global unresolved decision horizon, not only map/editor topics;
- prioritizes decisions by workspace, durable-gameplay, vertical-slice, alpha, expansion and deferred stages;
- authorizes autonomous technical decisions within repository and safety boundaries;
- requires durable task/branch/PR/validation/audit/merge/archive lifecycle;
- routes the canonical foundation checkpoint to the prompt;
- keeps Workspace and Dependency Contract as the immediate next package.

## Architecture and source of truth

### PROVEN

- ADR-0001 through ADR-0005 are accepted on `main` at the task base.
- The canonical active foundation task predated ADR-0005 in parts of its baseline and had no durable coordinator-prompt reference.
- `FOUNDATION_DECISION_BACKLOG.md` contained the ordered foundation gates but did not enumerate the complete later global product decision horizon.
- Repository prompt standards require exact authority, sources, ownership, acceptance, validation, autonomy, handover and stop conditions.

### DERIVED

- A dedicated coordinator prompt plus a staged global decision register reduces chat dependency and prevents later systems such as item transactions, houses, social systems, operations and product milestones from being forgotten or prematurely frozen.

## Acceptance criteria

- [x] Self-contained global coordinator prompt exists under `docs/agents/prompts/`.
- [x] Prompt includes role/mode, write allowlist, source order, accepted baseline, decision order, lifecycle, validation, audit, stop conditions and handover.
- [x] Prompt requires autonomous execution without plan-only completion and limits owner escalation to irreducible decisions.
- [x] Global decision register captures foundation, gameplay, client/world/content, operations and product domains.
- [x] Register classifies subjects by stage and preserves deferred extension points.
- [x] Foundation backlog links the global register and coordinator prompt.
- [x] Canonical active foundation task is reconciled with ADR-0005 and names the prompt.
- [x] Agent governance index and prompt index route agents to reusable prompts.
- [x] Full changed-file audit has zero open material findings.
- [x] Prompt evaluation verdict is `PASS` under `PROMPT_EVAL_STANDARD.md`.
- [ ] Exact-head `Agent governance` passes on the final checkpoint head.
- [ ] PR is squash-merged and this task is archived with ownership released.

## Excluded scope

- no architecture package beyond recording the accepted/global decision horizon;
- no Workspace and Dependency Contract implementation in this delivery;
- no Rust workspace, protocol, server, client or Studio implementation;
- no external-repository writes;
- no production, database, asset or release mutation;
- no change to accepted ADR decisions.

## Implementation / findings

- Added `docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md`.
- Added `docs/agents/prompts/README.md`.
- Added `docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`.
- Reconciled the foundation backlog with the global register, item anti-duplication gate, client migration gate and exact next action.
- Reconciled the canonical active foundation checkpoint with ADR-0005, the global register and the reusable prompt.
- Updated the agent governance index to route future agents to reusable prompts.
- No external repository, runtime code, production state or proprietary asset was changed.

## Prompt evaluation

Verdict: `PASS`.

Gates reviewed against `PROMPT_EVAL_STANDARD.md`:

- **Authority:** one exact writable repository; external and production exclusions explicit.
- **Resolution:** canonical task, backlog, global register, prompt and live-state source order are named.
- **Ownership:** package lifecycle requires exclusive paths/contracts; this delivery declares all seven changed paths.
- **Architecture:** ADR-0001 through ADR-0005 invariants are preserved.
- **Completeness:** outcome, decision order, package minimums, validation, audit and completion are explicit.
- **Evidence:** primary sources, exact revisions and `PROVEN/DERIVED/UNKNOWN/CONFLICT` are required.
- **Validation:** focused checks, full diff, independent audit, exact-head CI and review hygiene are explicit.
- **Autonomy:** plan-only stopping is prohibited; owner escalation is bounded to irreducible choices.
- **Handover:** exact durable checkpoint fields and one `next_action` are mandatory.
- **Safety:** secrets, production, assets, destructive operations and cross-repository writes are protected.

## Validation

### Focused

- method: prompt evaluation against `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md` and `PROMPT_EVAL_STANDARD.md`
- reviewed head: `7021d0a0333ea49a38e9e716e87e22e118e62bbc`
- result: `PASS`

### Component/integration

- result: `NOT_APPLICABLE` — documentation/prompt coordination only

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior changes

### Exact-head CI

- head: pending this final validation checkpoint commit
- workflow/run: `Agent governance`, pending automatic run
- result: pending

## Independent audit

- reviewed head: `7021d0a0333ea49a38e9e716e87e22e118e62bbc`
- method/auditor: adversarial full-diff architecture, prompt, ownership and handover review
- changed paths: exactly seven intended paths
- material findings: none open
- verdict: `PASS`

Audit checks included:

- no accepted ADR is weakened or silently superseded;
- the prompt cannot authorize writes outside `blakinio/Oteryn-v2`;
- the prompt does not authorize workspace/runtime implementation;
- immediate and later decision stages are distinguished;
- item/currency anti-duplication is no longer implicit inside a generic persistence heading;
- later global systems are preserved without premature final design;
- the active foundation task, backlog, register and prompt agree on the exact next action;
- no chat-only dependency remains;
- no external repository, production operation or proprietary asset mutation occurred.

One ownership-list omission for `docs/agents/prompts/README.md` was found during review and resolved in this checkpoint.

## PR and closeout

- PR: #11
- changed-file review: seven intended documentation paths
- unresolved review threads: none observed before final checkpoint
- related/superseded PRs: none
- merge commit/result: pending exact-head governance
- ownership release: pending archive after merge

## Context checkpoint

```yaml
last_progress: The global prompt, decision register and foundation routing passed prompt evaluation and full-diff audit; the final checkpoint commit now requires exact-head Agent governance.
status: validating
branch: docs/global-architecture-continuation-prompt-20260805
head_sha: pending-final-validation-checkpoint
pr: 11
ci_check_generation: null
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Verify Agent governance on the final checkpoint commit, then mark PR #11 ready and squash-merge it.
```
