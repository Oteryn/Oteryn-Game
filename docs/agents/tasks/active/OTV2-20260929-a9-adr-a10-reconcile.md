# OTV2-20260929-a9-adr-a10-reconcile

```yaml
task_id: OTV2-20260929-a9-adr-a10-reconcile
title: "ADR-0020 native client gameplay entry (A9) and A10/D3 reconciliation"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: f5db55e39fce41e7435eb467a990e89c07bfce13
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/ADR-0020-native-client-gameplay-entry.md
  - docs/architecture/ADR-0011-native-client-pre-protocol-migration-state.md   # pointer only
  - docs/architecture/reviews/OTERYN_GAME_A10_CORPSE_CONTAINER_DUR03_AMENDMENT_DECISION_2026-09-29.md   # supersession banner
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # A10 pointer defers to D3
  - docs/agents/tasks/active/OTV2-20260929-a9-adr-a10-reconcile.md
  - docs/agents/tasks/active/OTV2-20260929-a10-d39-contract-amendments.md   # archive move after #1210
  - docs/agents/tasks/archive/OTV2-20260929-a10-d39-contract-amendments.md
public_contracts: [DUR-03]
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0020 answers architect item A9 (#162 5879348805) with owner decisions D98, D129 and D138: the
production `oteryn-client` enters gameplay through a session crate extracted from
`oteryn-dev-client` and `oteryn-protocol-oteryn`, with a real Platform client grant, a minimal
renderer, mouse input, entity rendering after VIS-2 and chat after a chat lane. ADR-0011 gets a
pointer. The A10 record and its DUR-03 pointer defer to the D3 decision (#1198) where they differ
(#162 5886162546).

No code, CI, Platform or registry change is made.

## Architecture and source of truth

- `PROVEN`: ADR-0011; `apps/client`, `tools/dev-client`, `crates/renderer`, input crates,
  `crates/platform-client`; FND-04 grant profiles; `workspace-boundaries.toml`; the merge gate;
  the MOVE-RL-11 visibility decision; #1198 D3; #1210 A10.
- `UNKNOWN`: session crate name; Platform endpoint shape and timeline; chat lane design.

## High-risk authority/recovery qualification

Not applicable. Contract and ADR text only.

## Acceptance criteria

- [ ] The documents are on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, CI workflow edits, Platform changes and protocol registration.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: open the PR, freeze the head, exact-head review and Merge Queue integration
```
