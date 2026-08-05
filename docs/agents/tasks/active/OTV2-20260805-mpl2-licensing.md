# OTV2-20260805-mpl2-licensing

```yaml
task_id: OTV2-20260805-mpl2-licensing
title: Establish MPL-2.0 licensing and protected asset boundaries
mode: GOVERNANCE
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: governance/mpl-2.0-licensing-20260805
pr: 33
base_sha: cb35e11a22754ad0946b5676090863598950c474
head_sha: 931af15cc44eb274acece1958df76553111ae589
owner: GPT-5.6 Thinking
created_at: 2026-08-05T16:51:00+02:00
updated_at: 2026-08-05T17:06:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - LICENSE
  - LICENSE-ASSETS.md
  - TRADEMARKS.md
  - README.md
  - CONTRIBUTING.md
  - docs/repository/LICENSING.md
  - docs/repository/GITHUB_GOVERNANCE.md
  - .github/repository-policy.json
  - tools/repository/validate_repository_policy.py
  - docs/agents/tasks/active/OTV2-20260805-mpl2-licensing.md
  - docs/agents/tasks/archive/OTV2-20260805-mpl2-licensing.md
public_contracts:
  - LICENSE
  - LICENSE-ASSETS.md
  - TRADEMARKS.md
  - docs/repository/LICENSING.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish Oteryn-v2 source code and technical documentation under the Mozilla Public License 2.0 while explicitly reserving project branding and creative game assets unless a file-specific license grants additional rights.

## Architecture and source of truth

- `PROVEN`: the repository was public and contained no root license file at task start.
- `PROVEN`: no other active task owned the paths declared by this task.
- `PROVEN`: Mozilla identifies MPL 2.0 as the current MPL and publishes the canonical license text and SPDX identifier `MPL-2.0`.
- `DERIVED`: a file-level weak copyleft license provides a suitable balance between reusable modules and requiring distributed modifications to covered files to remain available.

## Acceptance criteria

- [x] Root `LICENSE` contains the unmodified canonical MPL-2.0 text.
- [x] The licensing scope clearly covers code, scripts, configuration, tests, schemas and technical documentation unless overridden by a file-specific notice.
- [x] Creative game assets and project branding are explicitly outside the MPL grant unless separately licensed.
- [x] Contribution guidance establishes inbound licensing under MPL-2.0 and requires third-party provenance.
- [x] Repository policy validation requires and verifies the licensing contract.
- [x] Focused validators and exact-head GitHub checks pass on the audited implementation head.
- [x] Independent audit finds no open material ambiguity or contradiction.

## Excluded scope

- No CLA, copyright assignment, dual-commercial licensing programme or registered-trademark claim.
- No relicensing of third-party material.
- No changes to runtime architecture, protocol, persistence or product implementation.
- No cross-repository licensing change.

## Implementation / findings

- Added the canonical MPL-2.0 license text at repository root.
- Added a precise default scope for source code, scripts, configuration, schemas, tests and technical documentation.
- Reserved art, audio, maps, narrative and other creative assets repository-wide unless separately licensed.
- Separated Oteryn names and branding from the software license without claiming registration.
- Documented inbound-equals-outbound contribution licensing and clarified that MPL acceptance without a CLA does not grant proprietary relicensing rights.
- Extended machine-readable repository policy and validation to preserve the licensing boundary.
- Pinned the exact canonical `LICENSE` Git blob in the retained validator so an accidental or unauthorized text modification fails governance CI.

## Validation

### Focused

- command/run: `Agent governance / validate`, workflow run `31018453233`, head `931af15cc44eb274acece1958df76553111ae589`
- result: `PASS`; agent governance, repository policy and PR metadata validation succeeded.

### Component/integration

- command/run: Dependency review `31018454819`; CodeQL `31018454807`, head `931af15cc44eb274acece1958df76553111ae589`
- result: `PASS`

### E2E

- scenario: `NOT_APPLICABLE` — licensing governance has no executable runtime outcome.
- result: `NOT_APPLICABLE`

### Exact-head CI

- audited implementation head: `931af15cc44eb274acece1958df76553111ae589`
- workflow/runs: Agent governance `31018453233`; Dependency review `31018454819`; CodeQL `31018454807`
- result: all `PASS`; the readiness checkpoint commit requires its own unchanged-head gate before merge.

## Independent audit

- exact head: `931af15cc44eb274acece1958df76553111ae589`
- method/auditor: independent structured review of the complete PR diff, the canonical license text, software/asset/branding boundaries, inbound contribution terms, third-party provenance rules, machine-readable policy and validator assertions
- material findings:
  - fixed ambiguous wording that could suggest Exhibit B was absent from the standard MPL text; the policy now correctly says the separate incompatibility notice is not attached or applied;
  - fixed possible directory-scope ambiguity by making the creative-asset reservation explicitly repository-wide;
  - strengthened the validator from fragment-only checks to an exact canonical Git-blob pin;
  - no open material findings remain.
- verdict: `PASS`

## PR and closeout

- changed-file review: complete; exactly the ten declared licensing, governance, documentation, validation and task paths changed
- unresolved review threads: none at readiness checkpoint
- related/superseded PRs: none found
- merge commit/result: pending final unchanged-head checks
- ownership release: pending merge and archive

## Context checkpoint

```yaml
last_progress: Completed independent audit and all checks on implementation head 931af15cc44eb274acece1958df76553111ae589; no open material finding remains.
status: ready
branch: governance/mpl-2.0-licensing-20260805
head_sha: 931af15cc44eb274acece1958df76553111ae589
pr: 33
ci_check_generation: readiness-checkpoint
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: 2026-08-05T17:06:00+02:00
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Merge PR #33 only after all required checks pass on the unchanged readiness-checkpoint head.
```
