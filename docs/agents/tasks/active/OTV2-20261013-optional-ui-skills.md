# OTV2-20261013-optional-ui-skills

```yaml
task_id: OTV2-20261013-optional-ui-skills
title: Task-authorized optional UI skills
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/optional-ui-skills-20261013
pr: null
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
- [ ] Focused validation and exact-head CI.
- [ ] Governed integration; no activation/enforcement guarantee from static checks.

## Validation

Pending local governance checks, upstream SHA-256 verification and whitespace review. Product E2E: NOT_APPLICABLE because no runtime UI or gameplay behavior changes.

## Review

Self-review required. Independent AI review not selected: bounded optional-aid documentation narrows selection and changes no autonomous write/integration authority, runtime security, CI or trust boundary. Actual discovery/model behavior NOT_EVALUATED; representative cases are in the skill catalog.

## Next action

Validate and publish the authoring branch, then complete the packet before freeze.
