# OTV2-20260805-owner-foundation-decisions

```yaml
task_id: OTV2-20260805-owner-foundation-decisions
title: Record owner-approved repository, admission and PostgreSQL decisions
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/foundation-decisions-20260805
pr: 5
base_sha: 0eb3c329fd04211e1bb5c6e3f219f4daad5e500a
head_sha: 758a5345613f2822bac29870698a23d360e7cb39
owner: chatgpt-github-agent
created_at: 2026-08-05T09:28:00+02:00
updated_at: 2026-08-05T09:46:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/agents/tasks/active/OTV2-20260805-owner-foundation-decisions.md
public_contracts:
  - docs/architecture/ADR-0002-repository-ownership-and-client-migration.md
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
depends_on:
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
  - blakinio/otclient
  - blakinio/Otheryn
```

## Outcome

Persist the owner decisions made on 2026-08-05 so future work does not depend on chat history:

- move the native Rust client into the Oteryn v2 repository;
- keep Identity and Game Gateway in Oteryn Platform;
- keep the initial Game Gateway implementation in Go;
- do not create a second classic login server or password/OAuth authority;
- use PostgreSQL as the target database technology;
- keep Platform and game persistence in separate logical databases with separate owners;
- record truthfully that `protocol-oteryn` is not yet implemented end to end.

## Architecture and source of truth

### PROVEN

- ADR-0001 establishes the native Rust client/server stack, `protocol-oteryn`, multichannel-first runtime and external Platform authority.
- The current native Rust client remains in `blakinio/otclient/oteryn-client` and does not have an implemented `protocol-oteryn` crate.
- Oteryn Platform contains a standalone Go Game Gateway and disabled-by-default producer-side native protocol support.
- No authoritative Rust game server or complete native gameplay E2E currently exists.

### ACCEPTED

- ADR-0002 makes `blakinio/Oteryn-v2` the future canonical repository for client, server and shared Rust protocol/domain crates.
- ADR-0003 keeps Platform Identity and Game Gateway in `blakinio/Oteryn-Platform`, retains Go for the initial Gateway and assigns the Rust server only the Game Session admission/gameplay boundary.
- ADR-0004 selects PostgreSQL and separate `oteryn_platform` / `oteryn_game` ownership boundaries.

### UNKNOWN / deferred

- exact client migration source SHA and history mechanism;
- exact Cargo workspace graph;
- exact `protocol-oteryn` v1 framing and IDL reconciliation;
- exact Game Session token, key rotation, replay and lease behaviour;
- detailed PostgreSQL schema, migration, isolation, backup and recovery contract;
- actual Platform database migration implementation.

## Acceptance criteria

- [x] Client migration direction and canonical repository are recorded.
- [x] Platform Identity and Game Gateway ownership are recorded.
- [x] The initial Go Gateway decision and later-rewrite gate are recorded.
- [x] No second login/password/OAuth authority is permitted.
- [x] Current native protocol state is not overstated.
- [x] PostgreSQL and separate Platform/game databases are recorded.
- [x] Cross-system SQL ownership and Redis non-authority are recorded.
- [x] Governance CI passed on reviewed head `758a5345613f2822bac29870698a23d360e7cb39`.
- [x] Independent audit has zero open material findings.
- [ ] Final checkpoint commit receives exact-head governance success.
- [ ] PR is squash-merged and this task is archived.

## Excluded scope

- no runtime implementation;
- no Rust workspace creation;
- no client code migration;
- no write to external repositories;
- no Gateway rewrite;
- no native protocol activation;
- no database provisioning or production migration.

## Implementation / findings

- Added ADR-0002 for repository ownership and client migration.
- Added ADR-0003 for Identity, Game Gateway and admission boundaries.
- Added ADR-0004 for PostgreSQL and separate data ownership.
- Preserved the existing full foundation-preimplementation checkpoint after audit identified that replacing it would remove useful detailed requirements.

## Validation

### Focused

- method: `Agent governance` workflow and complete diff review
- reviewed head: `758a5345613f2822bac29870698a23d360e7cb39`
- workflow/run: `30985869942`
- result: `PASS`

### Component/integration

- result: `NOT_APPLICABLE` — documentation-only architecture work

### E2E

- result: `NOT_APPLICABLE` — no executable runtime outcome

### Exact-head CI

- final head: pending this checkpoint commit
- workflow/run: `Agent governance`, must rerun automatically
- result: pending

## Independent audit

- reviewed head: `758a5345613f2822bac29870698a23d360e7cb39`
- method/auditor: adversarial complete-diff review against ADR-0001, live Platform/otclient evidence and owner decisions
- material findings:
  - resolved: initial task-file replacement removed detailed foundation content; original blob restored before readiness
  - none open
- verdict: `PASS`

## PR and closeout

- changed-file review: exactly three ADRs and this bounded task record
- unresolved review threads: none observed
- related/superseded PRs: none
- merge commit/result: pending
- ownership release: pending archive after merge

## Context checkpoint

```yaml
last_progress: ADR-0002 through ADR-0004 passed governance validation and independent audit; the task is ready after one final exact-head governance run for this checkpoint update.
status: ready
branch: docs/foundation-decisions-20260805
head_sha: 758a5345613f2822bac29870698a23d360e7cb39
pr: 5
ci_check_generation: 30985869942
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: Verify Agent governance succeeds on the checkpoint commit, then mark PR #5 ready and squash-merge it.
```
