# OTV2-20260907-wp1-merge-group-pin-rotation

```yaml
task_id: OTV2-20260907-wp1-merge-group-pin-rotation
title: Preapprove exact WP1 Merge Queue credibility gate blob
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 364
parent_coordinator_issue: 162
ci_programme_issue: 308
authority_comment: 5574626339
admission_main_sha: 1b41d485cc4bf126d2a9e5fe9717cc8530ece3d5
final_head_sha: cdd17fd999ac7299ce6d66435ef869a0b7a69afb
merge_sha: 3328d329f222057594c758ea42ef600d175e07c4
owner: null
future_gate_blob: 539a726b7d39cabe785892f70ea30d1944189d91
prior_gate_blob: e3291fe8fca8fc70166d5652b43d5a26fa0d762
owned_paths: []
blocks: []
external_repositories: []
updated_at: 2026-09-07
```

## Terminal outcome

The protected merge-authority audit now preapproves exactly future
`merge-group-gate.yml` blob `539a726b7d39cabe785892f70ea30d1944189d91`.
PR #398 integrated through normal protected Merge Queue as
`3328d329f222057594c758ea42ef600d175e07c4`; protected main readback matched the
accepted audit source and exact pin.

This task did not activate the gate itself. It changed only the protected audit
pin/fragments plus its lifecycle record. The gate remains on PR #396 until the
separate activation candidate receives protected-base audit PASS, exact-head
review/checks, normal FULL Merge Queue and protected readback.

## Qualification evidence

Exact pin candidate head: `cdd17fd999ac7299ce6d66435ef869a0b7a69afb`.
Independent Codex review reported no major issues. Canonical PR checks passed:
Agent governance `34153552463`, Architecture semantic audit `34153552312`,
required Merge gate `34153552373`.

The non-required protected-base self-audit `34153552208` rejected the candidate
only because it modified `.github/workflows/merge-authority-audit.yml` itself.
That self-modification refusal was intentionally preserved and was never
relabeled PASS. Coordinator authorization #308 comment `5574626339` plus the
independent exact-head review supplied the legitimate protected rotation path.
Full native Merge Queue `34154306504` qualified the integration candidate before
protected merge.

Protected readback of `.github/workflows/merge-authority-audit.yml` at
`3328d329f222057594c758ea42ef600d175e07c4` shows:
`EXPECTED_MERGE_GROUP_GATE_BLOB = 539a726b7d39cabe785892f70ea30d1944189d91`.
The audit still uses protected-base `pull_request_target`, inert candidate reads,
same-repository/exact-head/main-base identity, exact one-blob approval, literal
runtime construction of future `${{ ... }}` expressions, pinned actions and
forbids writes, dispatch and `continue-on-error` behavior.

## Released custody

This archive releases the special `.github/workflows/merge-authority-audit.yml`
pin-rotation lease. It grants no new audit mutation and no ruleset, required
status, Merge Queue, workflow-permission, product/runtime/Cargo/registry,
production or external-repository authority.

Only the separately allocated WP1 gate lane #396 retains its bounded source/test
custody. WP2/#361, WP3/#356, WP4/#335 and Server Seam #247 remain separately
controlled.

Runtime E2E is NOT_APPLICABLE to this archival record. Normal repository checks,
Merge Queue and protected readback remain required for this archive candidate
before the task is considered terminally released.
