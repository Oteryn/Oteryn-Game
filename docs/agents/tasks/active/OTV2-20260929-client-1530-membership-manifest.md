# OTV2-20260929-client-1530-membership-manifest

```yaml
task_id: OTV2-20260929-client-1530-membership-manifest
title: Emit the 15.30 client appearance membership manifest (ITEM-ID-1 prerequisite)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/client-1530-membership-manifest
issue: 162
pr: null
allocation: "#162 work coordinator allocation (A12 #1237 prerequisite)"
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: client membership manifest worker (claude-code-session-01PwTJFS62J35S88Srpqnrgx)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/client_appearance_census.py
  - tools/content-schema/item-authoring/test_client_appearance_census.py
  - tools/content-schema/item-authoring/README.md
  - imports/official/client-assets/15.30/README.md
  - docs/agents/tasks/active/OTV2-20260929-client-1530-membership-manifest.md
public_contracts: []
depends_on: []
blocks: [A12 #1237 digest-bound full membership manifest]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`client_appearance_census.py --membership-out PATH` writes an id-only, deterministic manifest
(`OTERYN_CLIENT_APPEARANCE_MEMBERSHIP/v1`) of the pinned 15.30 `appearances-2dfa943b….dat`, which is
loaded through the existing size and sha256 guard (fail closed). It needs no engine-source arguments.
The proprietary `.dat` is never committed; the owner runs the tool locally and commits only
`imports/official/client-assets/15.30/appearance-ids.json`. This task does not produce that file.
