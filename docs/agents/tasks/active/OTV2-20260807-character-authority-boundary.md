# OTV2-20260807-character-authority-boundary

```yaml
task_id: OTV2-20260807-character-authority-boundary
title: Record native Character Authority and Platform lifecycle boundary
mode: CONTRACT
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-character-authority-boundary
pr: null
base_sha: b006e71c86c3bfe99a51b91d83774c8f6715d3d7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T23:03:00+02:00
updated_at: 2026-08-07T23:03:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-character-authority-boundary.md
  - docs/architecture/ADR-0012-character-authority-and-platform-lifecycle-boundary.md
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
public_contracts:
  - docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md
depends_on:
  - docs/architecture/ADR-0003-platform-identity-game-gateway-and-admission-boundary.md
  - docs/architecture/ADR-0004-postgresql-and-data-ownership.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
  - blakinio/Oteryn-Platform docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
blocks:
  - native character lifecycle implementation before GAME-CHAR-01 and DUR gates are complete
cross_repository_coordination_id: OTV2-CHARACTER-LIFECYCLE-BOUNDARY
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the owner-accepted native boundary in which the Oteryn-v2 game-domain Character Authority is the sole authoritative owner of `CharacterId`, character aggregate lifecycle and current `AccountId <-> CharacterId` ownership, while Oteryn Platform remains the `AccountId` authority and may orchestrate web, commercial and support workflows only through versioned game-owned commands and authorized projections.

## Architecture and source of truth

- `PROVEN`: Oteryn-v2 ADR-0004 assigns characters and durable progression to the game domain, rejects unrestricted shared writers and requires character create/rename/delete/transfer/account-binding mutations to use a game/domain-owned API or explicit migration contract.
- `PROVEN`: accepted FND-ID semantics assign canonical `CharacterId` issuance to the game-domain Character authority and preserve `CharacterId` across rename and legal world transfer while forbidding ID reuse after deletion.
- `PROVEN`: Platform ADR 0028 at verified Platform `main@5929e088df618ca35713b8a7004baa52d0e5af83` defines native `AccountId` as Platform-issued strongly typed UUIDv7 and explicitly states that authoritative AccountId-to-CharacterId ownership remains game-owned.
- `PROVEN`: current Platform Canary compatibility code can directly create Canary `players` rows and change Canary `players.account_id`; these are current compatibility mechanisms, not the native Oteryn-v2 target boundary.
- `OWNER_ACCEPTED`: on 2026-08-07 the project owner accepted the explicit Character Authority / Platform orchestration split recorded by ADR-0012 and the accompanying contract.

## Acceptance criteria

- [x] Character Authority is the sole native issuer/owner of canonical CharacterId and authoritative character lifecycle state.
- [x] Platform remains the sole AccountId identity authority and does not mint CharacterId.
- [x] The authoritative current AccountId-to-CharacterId ownership relation is game-owned.
- [x] Native AccountId-to-character listing is an authorized game query/projection; Platform caches/read models are non-authoritative.
- [x] Final admission revalidates authoritative account-character ownership and does not trust a stale Platform projection or ticket as final proof.
- [x] Native create, rename, deletion/restore/finalization, world transfer and account/Bazaar ownership transfer use versioned game-owned command boundaries.
- [x] Platform direct SQL writes to native Oteryn-v2 character/game tables are forbidden as a target design.
- [x] Rename, legal world transfer and legal account ownership transfer preserve CharacterId; terminal deletion never permits CharacterId reuse.
- [x] Final name reservation/uniqueness enforcement belongs to Character Authority; any Platform availability check is advisory UX only.
- [x] Character Bazaar remains a Platform-owned commercial saga while the final ownership mutation is game-owned, atomic within the game domain and returns an idempotent operation receipt/outcome.
- [x] Cross-database workflows use saga/idempotency/reconciliation rather than distributed ACID.
- [x] The decision does not freeze transport, wire schema, command envelope, protocol encoding or FND-02 details.
- [x] The decision does not claim completion of GAME-CHAR-01, DUR-01, DUR-02, DUR-03 or runtime implementation.
- [x] No Rust runtime, database schema, Platform runtime, Canary runtime or production state is modified.
- [ ] Exact-head documentation/governance checks pass.
- [ ] Independent architecture audit reports zero open material findings.

## Excluded scope

This task does not decide final character-name namespace scope, character-slot product limits, detailed creation templates, progression formulas, death/respawn, deletion retention periods, GDPR/legal erasure, exact Bazaar economics, world-transfer eligibility/economy policy, physical PostgreSQL schema, API transport, protocol wire encoding, authentication token format or implementation sequencing beyond semantic producer/consumer ownership.

It does not authorize writes to `blakinio/Oteryn-Platform` or any Canary repository. A coordinated Platform consumer/compatibility clarification requires its own Platform task/branch/PR and the same coordination ID.

The stale historical task `OTV2-20260807-characterid-account-link` is not modified by this task; its merged PR #68 remains historical identity evidence and its archival cleanup is separate repository hygiene.

## Implementation / findings

The accepted package is intentionally narrow:

1. ADR-0012 freezes semantic ownership and lifecycle orchestration boundaries.
2. `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` defines the cross-repository producer/consumer contract without freezing FND-02 wire mechanics.
3. Existing Canary-specific Platform create/transfer/deletion contracts remain compatibility evidence until a separately authorized Platform documentation package classifies them against this native target.

## Validation

### Focused

- command/run: documentation/governance validator selected by repository workflow
- result: pending exact final head

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation-only architecture/contract change; no executable component changed
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — this task changes no runtime behavior or user journey
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: recorded in immutable PR/check evidence after final content commit
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending immutable PR/review evidence
- method/auditor: adversarial architecture review against ADR-0003, ADR-0004, FND-ID, Platform ADR 0028 and full PR diff
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: PR #68 is merged historical identity-baseline evidence; no open overlapping PR found at task start
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner accepted the Character Authority/Platform lifecycle boundary; live mains and existing Canary compatibility implementation were verified and the dedicated docs branch was created from Oteryn-v2 main.
status: implementing
branch: docs/OTV2-20260807-character-authority-boundary
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Commit the coherent ADR/contract/task package, open the draft PR, review the exact diff, then validate the unchanged final head.
```
