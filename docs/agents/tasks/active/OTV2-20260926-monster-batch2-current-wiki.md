# OTV2-20260926-monster-batch2-current-wiki

```yaml
task_id: OTV2-20260926-monster-batch2-current-wiki
title: Continue uncommon monster samples and current Wiki comparison
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/monster-batch2-wiki-20260926
pr: null
issue: 162
jira: KAN-16
base_sha: 4f22d85aed9e4c6a0ba99b2c57253541d2651bf4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-session-01a0de2c-f7f8-7b63-b235-b0f90df79fd8
created_at: 2026-09-26
updated_at: 2026-09-26
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/monster-authoring/**
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/tasks/active/OTV2-20260926-monster-batch2-current-wiki.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Direct owner continuation of the three requested follow-ups after #938. The owner
explicitly replaced the historical 2026-07-28 Wiki target with current sources.
One writer owns the new task branch; no Claude branch or external repository is mutated.
Delivery scope is a reviewable PR; autonomous protected integration is NOT_REQUIRED.

## Architecture and source of truth

- PROVEN: #938 merged as `c72925db52ee8cc19265b63d76e8e3ca863d93fc`.
- PROVEN: Canary source is `47dfd51f45280a59a1d3e50ba7edd573d7234446`, unchanged tracked checkout.
- PROVEN: authoring candidate, converter, source blobs and latest Wiki revision-bound observations.
- UNKNOWN: canonical Item/asset admission, runtime behavior and Global parity.
- PR #939 independently owns Bestiary Notes and Loot mapping follow-up; reconcile overlapping edits before integration.
- Archival is already covered by #941. Its packet remains historical `validating`;
  GitHub merged state governs #938. This task does not duplicate the four-file archival move.

## High-risk authority/recovery qualification

NOT_APPLICABLE: offline authoring evidence only; no runtime, identity, persistence,
authentication, protected integration or production trust boundary changes.

## Acceptance and findings

- Second fixed batch: 10/10 structurally valid; 5/10 declared manifests ready,
  with six explicitly retained source gaps across the other five.
- Bosstiary increments preserved with additive optional candidate field and total consistency validation.
- Familiar baseline 900000 ms from pinned default configuration; owner-dependent speed remains unresolved.
- First batch regeneration is unchanged, including all pinned source blob identities.
- Latest Wiki comparison: 10 pages, 225 field rows; 164 MATCH, 22 CONFLICT,
  20 UNKNOWN, 19 NOT_COMPARABLE. No historical date selector or gameplay overwrite.
- Third follow-up: #941 already publishes the archival move; terminal integration remains external/pending.

## Validation

Focused authoring checks: 176/176 formal cases; 11 converter tests; 5 Wiki tests;
242 source paths accounted; all 20 bundles valid with the six expected readiness gaps.
Required exact-candidate checks and self-review occur after publication and freeze.
Component/E2E: NOT_APPLICABLE, no executable game behavior changes.
Independent paid AI review: NO, low-risk offline tooling/evidence under bound META review policy.
Authoring route: repository-native GraphQL createCommitOnBranch with expectedHeadOid,
proven schema/authentication; local Git is used for source/read-only validation, not push.

## Context checkpoint

AUTHORING. Publish bounded delta on the exclusively owned branch, bind the returned
remote SHA, verify full remote delta, freeze, then qualify and open/reconcile the PR.
Jira KAN-16 remains W toku because the aggregate Story is incomplete.
