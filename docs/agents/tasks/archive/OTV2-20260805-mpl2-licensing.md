# OTV2-20260805-mpl2-licensing

```yaml
task_id: OTV2-20260805-mpl2-licensing
title: Establish MPL-2.0 licensing and protected asset boundaries
mode: GOVERNANCE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: governance/mpl-2.0-licensing-20260805
pr: 33
base_sha: cb35e11a22754ad0946b5676090863598950c474
head_sha: 99f9b726d717473e08398c3363f6e7ed68d68731
merge_sha: 746a82168358760936caa1aaafd70cafe89be1b4
owner: released
created_at: 2026-08-05T16:51:00+02:00
updated_at: 2026-08-05T17:10:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
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

Oteryn-v2 source code and technical documentation are published under the Mozilla Public License 2.0, while project branding and qualifying creative game assets remain outside the software license grant unless separately licensed.

## Architecture and source of truth

- `PROVEN`: PR #33 introduced the canonical root MPL-2.0 text and was squash-merged as `746a82168358760936caa1aaafd70cafe89be1b4`.
- `PROVEN`: the machine-readable repository policy records SPDX identifier `MPL-2.0`, the scope policy, asset notice, trademark notice and secondary-license compatibility choice.
- `PROVEN`: the retained governance validator pins the exact canonical `LICENSE` Git blob.
- `DERIVED`: the resulting file-level weak-copyleft boundary permits modular reuse while retaining MPL obligations for distributed covered-file modifications.

## Acceptance criteria

- [x] Root `LICENSE` contains the unmodified canonical MPL-2.0 text.
- [x] The licensing scope clearly covers code, scripts, configuration, tests, schemas and technical documentation unless overridden by a file-specific notice.
- [x] Creative game assets and project branding are explicitly outside the MPL grant unless separately licensed.
- [x] Contribution guidance establishes inbound licensing under MPL-2.0 and requires third-party provenance.
- [x] Repository policy validation requires and verifies the licensing contract.
- [x] Focused validators and exact-head GitHub checks passed.
- [x] Independent audit found no open material ambiguity or contradiction.
- [x] Implementation PR was squash-merged and task ownership was released.

## Excluded scope

- No CLA, copyright assignment, dual-commercial licensing programme or registered-trademark claim.
- No relicensing of third-party material.
- No changes to runtime architecture, protocol, persistence or product implementation.
- No cross-repository licensing change.

## Implementation / findings

- Added the canonical MPL-2.0 license text at repository root.
- Defined a precise default scope for source code, scripts, configuration, schemas, tests and technical documentation.
- Reserved art, audio, maps, narrative and other creative assets repository-wide unless separately licensed.
- Separated Oteryn names and branding from the software license without claiming registration.
- Documented inbound-equals-outbound contribution licensing and clarified that MPL acceptance without a CLA does not grant proprietary relicensing rights.
- Extended machine-readable repository policy and validation to preserve the licensing boundary.
- Pinned the exact canonical `LICENSE` Git blob in the retained validator.

## Validation

### Focused

- command/run: `Agent governance / validate`, final workflow run `31018678784`, exact head `99f9b726d717473e08398c3363f6e7ed68d68731`
- result: `PASS`; agent governance, repository policy and PR metadata validation succeeded.

### Component/integration

- command/run: Dependency review `31018678867`; CodeQL `31018677643`, exact head `99f9b726d717473e08398c3363f6e7ed68d68731`
- result: `PASS`

### E2E

- scenario: `NOT_APPLICABLE` — licensing governance has no executable runtime outcome.
- result: `NOT_APPLICABLE`

### Exact-head CI

- head: `99f9b726d717473e08398c3363f6e7ed68d68731`
- workflow/runs: Agent governance `31018678784`; Dependency review `31018678867`; CodeQL `31018677643`
- result: all `PASS` before squash merge.

## Independent audit

- exact head: `99f9b726d717473e08398c3363f6e7ed68d68731`
- method/auditor: independent structured review of the complete ten-file diff, canonical license text, software/asset/branding boundaries, inbound contribution terms, third-party provenance rules, machine-readable policy and validator assertions
- material findings:
  - clarified that Exhibit B is present in the standard MPL text while Oteryn-v2 does not attach or apply the separate incompatibility notice;
  - made the creative-asset reservation explicitly repository-wide;
  - strengthened validation from fragment checks to an exact canonical Git-blob pin;
  - no open material findings remain.
- verdict: `PASS`

## PR and closeout

- changed-file review: complete; exactly the declared licensing, governance, documentation, validation and task paths changed
- unresolved review threads: none
- related/superseded PRs: none
- merge commit/result: PR #33 squash-merged as `746a82168358760936caa1aaafd70cafe89be1b4`
- ownership release: complete; no owned paths remain

## Context checkpoint

```yaml
last_progress: MPL-2.0 licensing governance merged through PR #33 and the completed task was archived.
status: completed
branch: docs/archive-mpl2-licensing-20260805
head_sha: 746a82168358760936caa1aaafd70cafe89be1b4
pr: 33
ci_check_generation: completed-implementation
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: none; task complete
```
