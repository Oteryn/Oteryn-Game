# OTV2-20260807-character-authority-boundary

```yaml
task_id: OTV2-20260807-character-authority-boundary
title: Record native Character Authority and Platform lifecycle boundary
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-character-authority-boundary
pr: 90
base_sha: b006e71c86c3bfe99a51b91d83774c8f6715d3d7
head_sha: 50ae5c4ad23086161f619f30523caba907a48799
final_head_sha: 50ae5c4ad23086161f619f30523caba907a48799
final_head_frozen_at: 2026-08-07T23:10:52+02:00
owner: released
created_at: 2026-08-07T23:03:00+02:00
updated_at: 2026-08-07T23:13:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
public_contracts:
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
depends_on:
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
  - blakinio/Oteryn-Platform docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
blocks: []
cross_repository_coordination_id: OTV2-CHARACTER-LIFECYCLE-BOUNDARY
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Completed. The owner-accepted native boundary is now canonical in Oteryn-v2:

- Oteryn-v2 Character Authority is the sole semantic owner of canonical `CharacterId`, native character aggregate/lifecycle and authoritative current `AccountId <-> CharacterId` ownership.
- Oteryn Platform remains the canonical `AccountId` authority and may own UX/commercial/support orchestration, but native character mutations use versioned game-owned command boundaries.
- Native account-character listings are authorized game-owned projections; Platform caches/read models are non-authoritative.
- Final admission must revalidate current authoritative ownership rather than trusting stale Platform projection/ticket state.
- Rename, legal world transfer and legal account ownership transfer preserve CharacterId; terminal deletion never permits CharacterId reuse.
- Final name reservation/uniqueness enforcement belongs to Character Authority.
- Character Bazaar remains a Platform commercial saga while authoritative ownership mutation is atomic in the game domain and reconciled from an idempotent result/receipt.
- Platform direct SQL writes to native game character tables are forbidden as the steady-state architecture.
- Existing Canary direct-write adapters remain legacy/current compatibility evidence only.

Canonical sources:

- `docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md`
- `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`

## Delivery evidence

### Architecture PR

- PR: `#90` — `docs(architecture): define native Character Authority boundary`
- exact validated candidate head: `50ae5c4ad23086161f619f30523caba907a48799`
- changed files: exactly three declared documentation paths
- additions/deletions: `816 / 0`
- merge method: squash
- merged main commit: `10392eb89d11de2ea260c82587b4b1ef22ddd7e6`
- merge result: success

### Exact-head CI

All observed pull-request workflows for exact head `50ae5c4ad23086161f619f30523caba907a48799` completed successfully:

- Agent governance — run `31219000669` — `success`
- Dependency review — run `31219000101` — `success`
- CodeQL — run `31219000590` — `success`

No runtime workflow was required to prove this documentation-only semantic architecture package.

### Independent architecture audit

- exact head: `50ae5c4ad23086161f619f30523caba907a48799`
- PR review ID: `4886582726`
- method: adversarial full-diff review against ADR-0003, ADR-0004, ADR-0010, FND-ID CharacterId/account-link semantics, Platform ADR 0028, privacy/authorization, separate database ownership, FND-02/FND-04 and GAME-CHAR-01/DUR boundaries
- verdict: `PASS`
- material findings: zero
- unresolved inline review threads: zero

### E2E

`NOT_APPLICABLE` — this task changes documentation/architecture only and introduces no executable runtime or user journey.

## Scope verification

No Rust runtime, database schema, protocol implementation, Platform runtime, Canary runtime, deployment, production database or live state changed.

This decision deliberately does **not** complete `GAME-CHAR-01`, `DUR-01`, `DUR-02`, `DUR-03`, `FND-02` or `FND-04`. Final name namespace, slots/quotas, detailed creation templates, progression, death/respawn, deletion retention, world-transfer policy, durable schema and exact API/wire mechanics remain with their owning gates.

No write to `blakinio/Oteryn-Platform` was performed. A Platform-side consumer/legacy-compatibility clarification remains a separately authorized coordinated task under `OTV2-CHARACTER-LIFECYCLE-BOUNDARY`.

The pre-existing stale historical task `OTV2-20260807-characterid-account-link` was not modified; cleanup of that already-merged task remains independent repository hygiene.

## PR and closeout

- architecture PR: merged
- exact-head self/adversarial review: pass
- independent architecture audit: pass
- required PR workflows: pass
- unresolved review threads: zero
- archive move: this closeout change
- ownership: released

## Context checkpoint

```yaml
last_progress: ADR-0012 and CHARACTER_AUTHORITY_PLATFORM_BOUNDARY were validated on exact head 50ae5c4ad23086161f619f30523caba907a48799 and squash-merged by PR #90 as main commit 10392eb89d11de2ea260c82587b4b1ef22ddd7e6; task is terminal and ownership is released.
status: completed
branch: null
head_sha: 50ae5c4ad23086161f619f30523caba907a48799
pr: 90
final_head_sha: 50ae5c4ad23086161f619f30523caba907a48799
final_head_frozen_at: 2026-08-07T23:10:52+02:00
ci_trigger_source: pull_request
ci_check_generation: exact-head-50ae5c4
ci_checks_for_current_head: 3
ci_run_ids:
  - 31219000669
  - 31219000101
  - 31219000590
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 2
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Platform-side consumer/legacy compatibility documentation may be created only under a separately authorized Platform task; otherwise continue architecture analysis with the next owner-selected GAME-CHAR-01 decision.
```
