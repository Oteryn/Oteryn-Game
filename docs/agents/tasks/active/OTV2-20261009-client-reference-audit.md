# OTV2-20261009 — native client reference audit checkpoint

```yaml
task_id: OTV2-20261009-client-reference-audit
title: Preserve native client settings and sidebar reference audit
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/client-reference-audit-20261009
pr: 1941
issue: 1622
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: root agent, single writer
created_at: 2026-10-09
updated_at: 2026-10-10
execution_policy: continuous_progress
owned_paths:
  - apps/client/SETTINGS.md
  - apps/client/REFERENCE-AUDIT.md
  - apps/client/REFERENCE-PREFERENCE-KEYS.json
  - docs/agents/tasks/active/OTV2-20261009-client-reference-audit.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Status: authoring, not frozen. Owner explicitly requested repository preservation
and allowed a draft PR on 2026-10-09. No deployment, merge or database reset.

Canonical branch: `docs/client-reference-audit-20261009`.
Single writer: root agent. Fresh branch allocated exclusively to this task.
No concurrent writer or review allocation. This is a documentation-only checkpoint;
older private implementation edits remain separate and must not be discarded.

## Scope

- `apps/client/SETTINGS.md`: actual private implementation status and intended scope.
- `apps/client/REFERENCE-AUDIT.md`: verified coverage, complete extra-shortcut list,
  restrictions, outstanding inspection and evidence handling.
- `apps/client/REFERENCE-PREFERENCE-KEYS.json`: 213 names only, across six sections.
- This task packet: preservation boundary and next work.

## Publication boundary

Use the connector-compatible fresh-branch route under the pinned META publication
policy: immediately read the live predecessor; build one complete-delta tree on
its exact tree; create one sole-parent successor; advance only this branch with
force=false and expected predecessor; immediately verify the live branch head.
Do not reconstruct or publish older local implementation commits through this
checkpoint. Draft remains open for subsequent authoring; no qualification or merge
is implied by its creation.

## Validation

Documentation checked against verified screenshots and the scoped shortcut
inventory. Incorrectly named/stale captures are excluded. No proprietary imagery,
credentials, account values or reference assets are included. Runtime tests do
not apply to this documentation-only delta.

## Next work

Inspect outstanding extra shortcuts and character/container controls; record
empty/restricted states honestly; restore original shortcut layout; update audit
coverage and draft PR with a bounded new authoring delta after live-head refresh.
