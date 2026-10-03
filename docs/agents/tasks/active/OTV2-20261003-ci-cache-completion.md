# OTV2-20261003-ci-cache-completion

```yaml
task_id: OTV2-20261003-ci-cache-completion
title: Measure actual Rust cache performance with an isolated manual pilot
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/ci-cache-completion-20261003
pr: null
issue: 1622
base_sha: d75ba6d6bc0c02f6c13b4ff860c9bf3a439930e7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex CI repair task, sole publisher under direct owner instruction
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/rust-cache-pilot.yml
  - tools/repository/benchmark_rust_cache.py
  - tools/repository/test_benchmark_rust_cache.py
  - docs/agents/evidence/OTV2-20261003-rust-cache-pilot.md
  - docs/agents/tasks/active/OTV2-20261003-ci-cache-completion.md
  - docs/agents/tasks/archive/OTV2-20261003-ci-cache-completion.md
public_contracts: []
depends_on: [OTV2-20261003-ci-stack-applicability]
blocks: []
external_repositories: []
```

## Outcome and evidence

AUTHORING: controlled compilation experiment, without changing canonical gates.
Local actual server library repetitions pass; honest measured scope/versions and
numeric results are in the evidence document. Hosted workspace result is pending.
No usable common cache backend was found in accepted code or agent environment.
GitHub repository/organization secrets and variables are UNKNOWN: this connector
receives HTTP403 for their metadata; secret values were never requested.

## Qualification and lifecycle

Focused isolation/locked-command/sample-grouping regressions pass; repository
policy accepts the pinned manual workflow. Native full CI and required independent
provider review remain pending until final remote candidate freeze. Only the unique
active CP dispatches funded review and integration after live exact-head dedupe.
Parent #1634 merged with successful native Merge Queue; accepted-main composition
is complete. Archive this record in final authoring after assigning the actual PR.
High-risk production authority/recovery: NOT_APPLICABLE; no runtime mutation or
production credential is operated. The pilot is protected-main only with read-only
contents permission, exact accepted source binding, private local cache/tempdir and
no cache save or qualification bypass. Archive this task in final authoring commit
with actualPR beforefreeze. Exact SHA/freeze and terminal MQ proof remain outsideGit.
