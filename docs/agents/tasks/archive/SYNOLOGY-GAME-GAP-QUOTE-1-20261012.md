# SYNOLOGY-GAME-GAP-QUOTE-1-20261012

```yaml
task_id: SYNOLOGY-GAME-GAP-QUOTE-1
title: "SYNOLOGY-GAME-GAP-QUOTE-1: strip the quote from node ids parsed in registration_gap"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/supervisor-gap-quote-20261012
issue: null
pr: 2023
head_sha: "exact head in the PR"
final_head_sha: "exact head in the PR"
owner: claude-code implementation writer
created_at: 2026-10-12
updated_at: 2026-10-12
owned_paths:
  - deploy/synology-game/supervisor.sh
  - deploy/synology-game/test-supervisor-gap.sh
  - .github/workflows/synology-game-deploy.yml
  - docs/agents/tasks/archive/SYNOLOGY-GAME-GAP-QUOTE-1-20261012.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Real node.log lines carry `detail="node_id=<id>"` for `registering`, so the id extracted by `registration_gap` kept a trailing `"` and never matched the `registered` id: a false gap blocked the Synology deploy. All `node_id=` extractions in `supervisor.sh` (gap, current-node-id) and the `previous=` extraction in `synology-game-deploy.yml` now capture only `[0-9a-f-]*`. The registered match is anchored on a non-id character or end of line. A true gap still prints the id and exits 1.

`test-supervisor-gap.sh` gained fixtures in the real quoted format: registering+registered ok, registering only gap, second registering without registered gap, registered with the id as last field ok.

## Excluded scope

No gate workflow, node binary or log format change.

## Validation

bash deploy/synology-game/test-supervisor-gap.sh: pass
shellcheck, actionlint: not available in the environment

## Review and closeout

Review is decided by the control plane.
