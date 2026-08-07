# OTV2-20260807-account-single-online-character

```yaml
task_id: OTV2-20260807-account-single-online-character
title: Record one-online-character-per-account invariant
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-account-single-online-character
pr: 70
base_sha: 52588b36adaffd1e86bf705dfae14a0832630d91
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T12:43:00+02:00
updated_at: 2026-08-07T12:47:00+02:00
execution_budget_minutes: 30
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-account-single-online-character.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
depends_on:
  - FND-ID-01_OWNER_DECISION_CHECKPOINT_2026-08-07.md
  - FND-ID-01 CharacterId/AccountId linkage baseline from PR #68
blocks:
  - FND-04 must consume this invariant before production Game Session/admission/lease behavior is frozen
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the owner-accepted invariant that one Platform-owned `AccountId` may have at most one authoritative gameplay character online at any moment across all of that account's characters, worlds, channels and instances.

## Accepted decision

- one `AccountId` -> at most one authoritative online `CharacterId`;
- the invariant is account-global, not world-local or channel-local;
- character switching must not create overlapping authoritative gameplay ownership;
- reconnect or transport overlap may exist only as a bounded transition if `FND-04` proves that exactly one authoritative gameplay lease/session remains valid;
- Platform web/OAuth sessions are not counted as an online gameplay character;
- exact duplicate-login policy (reject newcomer vs fenced replacement/handoff) remains unresolved for `FND-04`;
- exact `GameSessionId`, admission and character-lease representation/issuer/storage remain unresolved for `FND-04`;
- no runtime, protocol, database or Platform implementation is authorized by this task.

## Acceptance criteria

- [x] Define the concurrency scope at `AccountId` level.
- [x] Apply it across every CharacterId, WorldId, ChannelId and InstanceId owned/used by that account.
- [x] Preserve CharacterId identity independently from session/lease authority.
- [x] Require fail-closed prevention of two simultaneous authoritative gameplay characters for one account.
- [x] Keep reconnect/handoff mechanics deferred.
- [x] Keep Platform web authentication sessions outside this gameplay-presence invariant.
- [x] Do not implement runtime/protocol/persistence/Platform behavior.
- [ ] Exact-head checks and independent audit before merge readiness.

## Validation

PR #70 owns exactly the two declared documentation files. The initial governance run reached PR metadata validation before checkout and failed because the PR body omitted the required `## Scope` heading; the PR metadata was corrected without changing architecture semantics. This task metadata update records the live PR number and triggers a fresh exact-head pull-request validation generation.

Documentation-only architecture change. Exact-head repository checks and independent audit are required before merge readiness.
