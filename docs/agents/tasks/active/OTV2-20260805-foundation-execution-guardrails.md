# OTV2-20260805-foundation-execution-guardrails

```yaml
task_id: OTV2-20260805-foundation-execution-guardrails
title: Refine foundation decision and implementation gates
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/workspace-dependency-contract-20260805
pr: 13
base_sha: bd87792c92e26835d44c633a6064808f487a58a2
head_sha: pending-final-validation-checkpoint
owner: chatgpt-github-agent
created_at: 2026-08-05T12:42:00+02:00
updated_at: 2026-08-05T12:54:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-preimplementation-contracts.md
  - docs/agents/tasks/active/OTV2-20260805-foundation-execution-guardrails.md
public_contracts:
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
depends_on:
  - ADR-0001 through ADR-0005
blocks:
  - execution of FND-01 with ambiguous implementation gates
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn
  - blakinio/otclient
```

## Outcome

Make the foundation programme safer and more executable by preserving architecture-first decisions while allowing compile-time feedback after `FND-01`, preventing premature crate proliferation, assigning stable gate identifiers and separating the non-owning programme checkpoint from individual contract packages.

## Architecture and source of truth

### PROVEN

- ADR-0001 through ADR-0005 are accepted on the trusted base.
- The prior backlog required all four foundation contracts before creation of the root Cargo workspace.
- The prior candidate workspace listed many possible crates but did not accept an initial graph.
- The canonical foundation task had no owner or owned paths and functions as an umbrella checkpoint.
- The prior global coordinator prompt defaulted to execution and did not explicitly define a read-only analysis invocation.
- PR #13 is the only open PR in `blakinio/Oteryn-v2` for this package.

### DERIVED

- Requiring protocol, runtime and admission contracts before any compilable workspace delays useful dependency and interface feedback.
- Treating the complete capability horizon as an initial workspace risks empty crates and premature abstraction.
- Stable gate IDs reduce cross-document drift.

## Acceptance criteria

- [x] Only `FND-01` blocks root workspace bootstrap.
- [x] A separately authorized minimal compilable bootstrap is allowed after `FND-01`, without authorizing production protocol, runtime, admission or durable gameplay implementation.
- [x] Bounded technical spikes are explicitly non-canonical, reversible and evidence-producing.
- [x] The candidate crate list is classified as a capability horizon, and initial members require an immediate consumer and observable acceptance.
- [x] Stable gate IDs are consistent across backlog, register, programme task and coordinator prompt.
- [x] The programme task is explicitly non-owning; every decision package requires its own task, branch and PR.
- [x] The coordinator prompt supports explicit read-only analysis without creating repository state.
- [x] Architecture dependency rules must become executable CI checks after workspace bootstrap.
- [x] Full changed-file audit has zero open material findings.
- [ ] Exact-head governance CI passes.
- [ ] PR is ready, squash-merged and this package task is archived with ownership released.

## Excluded scope

- no change to ADR-0001 through ADR-0005;
- no root Cargo workspace or Rust code;
- no protocol, runtime, admission or persistence implementation;
- no external-repository writes;
- no production, database, asset or deployment mutation.

## Implementation / findings

- Added progressive gates: pre-`FND-01`, post-`FND-01` bootstrap and layer-specific implementation gates.
- Made `FND-01` the only contract blocking canonical root-workspace bootstrap.
- Kept `FND-02`, `FND-03`, `FND-04`, `DUR-01` through `DUR-04`, `VSL-01` and `VSL-02` as explicit gates for their respective behavior.
- Reclassified the large crate list as a capability horizon and prohibited empty placeholder crates.
- Required an immediate consumer and observable acceptance for every initial workspace member.
- Required retained machine checks for dependency directions and forbidden edges after bootstrap.
- Added stable IDs across the backlog, global register, programme checkpoint and coordinator prompt.
- Made the canonical foundation task a non-owning programme checkpoint.
- Added explicit `ANALYZE_ONLY` behavior to the coordinator prompt.
- No accepted ADR, runtime code, external repository, production state or proprietary asset was changed.

## Validation

### Focused

- command/run: repository governance/document validation through the `Agent governance` workflow
- result: pending exact final head

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture/governance documentation only
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- head: pending final checkpoint commit
- workflow/run: `Agent governance`, pending
- result: pending

## Independent audit

- exact head reviewed: efb58be928783c2b7c527c051f5ffb7f436f208e
- method/auditor: adversarial full-diff architecture review by the delivery coordinator
- material findings: none open
- non-material observations:
  - the existing branch name predates the narrowed guardrail package but is dedicated and conflict-free;
  - stable IDs for later expansion subjects are coordination identifiers, not accepted contracts;
  - compile-only spikes remain explicitly non-canonical and cannot bypass layer gates.
- verdict: `PASS`

## PR and closeout

- changed-file review: five declared paths only; no out-of-scope repository changes
- unresolved review threads: none observed before final validation
- related/superseded PRs: no other open PRs found at task start
- merge commit/result: pending
- ownership release: pending archive after merge

## Context checkpoint

```yaml
last_progress: Progressive gates, stable IDs, capability-horizon rules, non-owning programme coordination and ANALYZE_ONLY behavior are implemented and independently audited in PR #13.
status: validating
branch: docs/workspace-dependency-contract-20260805
head_sha: pending-final-validation-checkpoint
pr: 13
ci_check_generation: pending-final-head
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Run and verify exact-head Agent governance for the final checkpoint commit, then complete PR review and merge gates.
```
