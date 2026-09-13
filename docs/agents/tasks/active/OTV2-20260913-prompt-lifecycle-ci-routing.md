# OTV2-20260913-prompt-lifecycle-ci-routing

```yaml
task_id: OTV2-20260913-prompt-lifecycle-ci-routing
title: Keep PROMPT_LIFECYCLE governance changes off Windows runtime lanes
mode: REPAIR
status: active
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/prompt-lifecycle-neutral-lane
base_sha: bd35d7fd0ca5c13a32f59d011ec8b9571d69e897
head_sha: cf51345b54ede9cad6024d57a1eb0a4b3fa71c26
owner: chatgpt-session
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260913-prompt-lifecycle-ci-routing.md
```

## Outcome

Changes to the exact governance registry `docs/agents/PROMPT_LIFECYCLE.json` remain on mandatory governance/repository checks without selecting Rust or Windows runtime lanes. Other structured governance JSON remains fail-closed to FULL.

## Validation

- exact baseline: `main@bd35d7fd0ca5c13a32f59d011ec8b9571d69e897`;
- classifier allowlist is exact-path only;
- regression requires `docs/agents/PROJECT_LANES.json` to remain FULL;
- this repair PR itself is expected to select FULL because it modifies `tools/repository/**`.
