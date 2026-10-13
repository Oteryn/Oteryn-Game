# OTV2-20261013-optional-ui-skills

```yaml
task_id: OTV2-20261013-optional-ui-skills
title: Task-authorized optional UI skills
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/optional-ui-skills-20261013
pr: 2081
issue: 2080
base_sha: 6337f2aaf95a6582f03a056e5b91ace5367d2f74
head_sha: null
owner: Codex current chat
created_at: 2026-10-13T17:53:59.601261Z
updated_at: 2026-10-13T17:53:59.601261Z
execution_policy: continuous_progress
owned_paths: [AGENTS.md, .agents/skills/**, docs/agents/tasks/active/OTV2-20261013-optional-ui-skills.md, docs/agents/tasks/archive/OTV2-20261013-optional-ui-skills.md]
public_contracts: []
depends_on: []
blocks: []
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome

Optional project-local design/performance aids require a user grant for the current task; after a broad grant the agent selects the smallest useful set. Required checks remain unaffected.

## Acceptance criteria

- [x] Task-scoped grant, continuation, revocation and worker handoff rules.
- [x] Native-client wrapper, pinned Anthropic source and Apache-2.0 license.
- [x] Oteryn-owned performance procedure; no unlicensed Rux source copied.
- [x] Focused local validation; exact-head CI remains a live PR prerequisite.
- [x] Final payload prepared for governed integration; archive becomes effective only if PR #2081 merges. No activation/enforcement guarantee from static checks.

## Validation

- `python tools/agents/validate_governance.py` — pass.
- `python -m unittest discover -s tools/agents/tests` — pass, 60 tests.
- Upstream SHA-256 provenance and license verification — pass.
- `git diff --check` — pass.
- Product E2E: NOT_APPLICABLE because no runtime UI or gameplay behavior changes.
- Actual client discovery and behavioral trials: NOT_EVALUATED (0 trials).
- Exact final SHA, CI, queue evidence and merge result belong in live PR #2081 after freeze; no self-referential head is recorded here.

## Review

Self-review required. Independent AI review not selected: bounded optional-aid documentation narrows selection and changes no autonomous write/integration authority, runtime security, CI or trust boundary. Actual discovery/model behavior NOT_EVALUATED; representative cases are in the skill catalog.

## PR and closeout

PR: https://github.com/Oteryn/Oteryn-Game/pull/2081 . Complete changed-file self-review: pass. Exact-head checks/queue pending in live PR at authoring time. Merge commit/result: squash merge of #2081, conditional on governed terminal integration. Source branch disposition: auto_delete_after_merge if configured; verify before releasing ownership. This packet is archived in the final authoring delta per archive/README.md; it reaches main only on merge.
