# Prompt lifecycle CI routing evidence

- protected baseline: `bd35d7fd0ca5c13a32f59d011ec8b9571d69e897`
- branch: `ci/prompt-lifecycle-neutral-lane`
- classifier change: exact-path neutral treatment for `docs/agents/PROMPT_LIFECYCLE.json`
- fail-closed control: `docs/agents/PROJECT_LANES.json` remains FULL in regression coverage
- repair PR itself must run FULL because `tools/repository/**` changes are control-plane inputs
