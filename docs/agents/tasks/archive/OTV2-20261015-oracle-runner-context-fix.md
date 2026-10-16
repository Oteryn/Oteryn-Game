# OTV2-20261015-oracle-runner-context-fix

```yaml
task_id: OTV2-20261015-oracle-runner-context-fix
title: Make the Oracle workflow runner context executable
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/fix-oracle-game-runner-context-20261015
issue: 2217
pr: 2222
base_sha: fc03ef48f9e0604bba0c1e9b484e3ffa0be50a46
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex Oracle migration task
created_at: 2026-10-15T23:26:00Z
updated_at: 2026-10-15T23:26:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/oracle-operations.yml
  - docs/agents/tasks/active/OTV2-20261015-oracle-runner-context-fix.md
  - docs/agents/tasks/archive/OTV2-20261015-oracle-runner-context-fix.md
public_contracts: []
depends_on: [Oteryn/Oteryn-Platform#1639]
blocks: []
cross_repository_coordination_id: Oracle migration 20261015
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome and acceptance

Governing Issue #2217 authorizes the Game Oracle runner/workflow migration.
GitHub's actual dispatch parser rejects runner.name at job-env scope, HTTP422
line35. Bind the same identity in step env, where runner context is available.
The existing expected-name comparison, ARM64/UID checks, negative Docker probe,
protected environment and component-bound forced host command remain mandatory.
Actionlint reproduces the original error and accepts the corrected sibling files.

This bounded source repair ends with its delivery PR. Physical protected-workflow
acceptance and native entry stay explicitly open in the existing waiting Oracle
deployment packet under #2217, with no canonical terminal PR dispatch target.
Server protocol repair #2219 and client PR1942 have disjoint ownership.

## High-risk authority/recovery qualification

NOT_APPLICABLE: expression-context placement repairs GitHub workflow parsing;
no authority, credential scope, environment release, command, protection or
runtime transition changes. The already reviewed root adapter and its regression
suites from Platform1644 are retained. No native gameplay acceptance is claimed.

## Validation

- actionlint 1.7.7 .github/workflows/oracle-operations.yml: pass (original fails runner context availability)
- python deploy/oracle/test_runner_boundary.py: pass
- python tools/repository/validate_repository_policy.py: pass
- python -m unittest discover -s tools/repository -p 'test_*.py': pass (88 tests)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (124 tests)
- Exact candidate CI and advisory review: pending freeze.

## Closeout

Source delivery: #2222. This final authoring commit archives the bounded
parser repair; exact frozen-head CI and advisory review remain required for its
integration. Resolve merge commit/result from the protected PR readback. Physical
rollout stays open under #2217; no production or native entry completion is claimed.
Source branch disposition: auto-delete after protected integration.
