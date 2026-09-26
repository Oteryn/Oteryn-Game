# OTV2-20260926-monster-batch2-current-wiki

```yaml
task_id: OTV2-20260926-monster-batch2-current-wiki
title: Replace historical monster Wiki comparison with current source observations
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: claude/nice-edison-h9aqh0
branch: codex/current-monster-wiki-20260926
pr: 945
issue: 162
jira: KAN-16
base_sha: f6d7dbf813aab16276bdb0a2abb2f8a3e685aee9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-session-01a0de2c-f7f8-7b63-b235-b0f90df79fd8
created_at: 2026-09-26
updated_at: 2026-09-26
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/monster-authoring/**
  - docs/agents/tasks/active/OTV2-20260926-monster-batch2-current-wiki.md
public_contracts: []
depends_on:
  - OTV2-20260926-monster-authoring-batch-2
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Direct owner continuation of the three follow-ups after #938. The owner explicitly
replaced the historical 2026-07-28 Wiki target with current sources.
Claude concurrently published #943. This bounded follow-up uses its exact head as
a dependency and preserves its second batch/schema. It supersedes the duplicate
Codex #944 candidate; the Claude branch is never mutated.
Delivery scope is a reviewable PR; autonomous protected integration is NOT_REQUIRED.

## Architecture and source of truth

- PROVEN: #938 merged as `c72925db52ee8cc19265b63d76e8e3ca863d93fc`.
- PROVEN: #941 archival merged as `9267c21968d12ebffed019c95aca874649447439`.
- DEPENDENCY: #943 head `f6d7dbf813aab16276bdb0a2abb2f8a3e685aee9` supplies the second
  batch and candidate schema; its architecture decisions and whole-PR qualification
  remain owned by that task.
- PROVEN: Canary source pin `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- UNKNOWN: canonical Item/asset admission, runtime behavior and Global parity.
- #942 independently handles Bestiary Notes / v2 Loot mapping. No duplicate mutation.

## High-risk authority/recovery qualification

NOT_APPLICABLE: offline evidence tooling only; no runtime, identity, persistence,
authentication, protected integration or production trust change.

## Acceptance and findings

- Both Wiki APIs read latest revisions on 2026-09-26 without a historical selector;
  acquisition/server timestamps, revision IDs/URLs and content digests bind the facts.
- Fandom: 259 rows, 229 MATCH / 12 DIFF / 8 WIKI_UNKNOWN / 10 NOT_COMPARABLE.
- BR: 225 rows, 173 MATCH / 13 CONFLICT / 20 UNKNOWN / 19 NOT_COMPARABLE.
- Sources stay separate, with different field coverage and observed discrepancies.
- BR `defense` is displayed armor; damage modifiers are received damage, not reduction.
- Historical snapshot is replaced; old caches are rejected. No gameplay overwrite.
- Canonical Git blob IDs survive Windows line-ending conversion; wrong/modified
  source checkouts are rejected before conversion.
- Second-batch/schema acceptance remains in #943. Archival is already complete via #941.

## Validation and disposition

Required final delta readback/freeze and exact-candidate validation follow the final
canonical PR metadata write. Offline formal, source-census, current-source semantics
and both existing batches are qualified on that exact candidate.
Component/E2E: NOT_APPLICABLE, no executable game behavior change.
Paid AI review: NO, low-risk offline tooling under bound META review economy.
Authoring route: high-level createCommitOnBranch with expectedHeadOid on one
exclusive branch. No local push, force/reset/rebase or frozen-head write.

## Context checkpoint

Final AUTHORING metadata write binds canonical PR #945; freeze the returned SHA. After qualification, publish
exact-head evidence in PR metadata without another source write. The stacked PR
depends on #943; protected-main qualification and integration must follow repository
rules when that dependency is integrated. Jira KAN-16 remains aggregate W toku.
