# OTV2-20260807-fnd-id01-contract

```yaml
task_id: OTV2-20260807-fnd-id01-contract
title: Complete the minimum FND-ID-01 foundation identifier contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-fnd-id01-contract
pr: 85
base_sha: 67c45efd35a4882ee414a9cd78c879a7d61a97ac
final_head_sha: 6686a4b62c1e6b518d38ab3c80326b8621abe5bb
merge_commit: 2c584543cd1e3758958755478a6cc6ed3d39a8a9
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T21:00:00+02:00
completed_at: 2026-08-07T21:14:33+02:00
execution_budget_minutes: 60
owned_paths: []
public_contracts:
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
depends_on:
  - ADR-0001 through ADR-0011
  - docs/architecture/FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_WORLD_CHANNEL_ID_ISSUANCE_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_WORLD_CHANNEL_UUIDV7_REPRESENTATION_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_INSTANCE_ID_ISSUER_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_PARTY_ID_ISSUER_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_CHARACTER_ID_ACCOUNT_LINK_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md
  - docs/architecture/FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation input)
```

## Outcome

`FND-ID-01` is complete at the architecture-contract level on `main`.

Canonical contract:

- `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.

The accepted result keeps `FND-ID-01` deliberately narrow: it freezes only foundation identities required before protocol/runtime/admission contracts can be made unambiguous and does not become a catalogue of every future game identifier.

## Accepted architecture result

The canonical foundation catalogue contains:

- `AccountId` — Platform-owned external durable identity, consumed losslessly without silent re-keying;
- `CharacterId` — global strongly typed UUIDv7 identity;
- `WorldId` — global logical-world UUIDv7 target identity owned/issued by Platform World Registry/topology authority;
- `ChannelId` — UUIDv7 scoped as `WorldId + ChannelId`, owned/issued by Platform World Registry/topology authority;
- `NodeId` — one fresh UUIDv7 for each concrete GameNode process incarnation;
- `InstanceId` — UUIDv7 scoped as `WorldId + InstanceId`, issued by the game-domain Instance/Activity allocator;
- `PartyId` — UUIDv7 scoped as `WorldId + PartyId`, issued by the world-level Party/Social authority;
- `GameSessionId` — global UUIDv7 logical gameplay-session identity issued by the game-domain Game Session / Admission authority only after successful admission;
- `HandoffId` — the one additional conditional foundation UUIDv7 identity, scoped as `WorldId + HandoffId`, for a bounded authoritative runtime-ownership transition already required by the accepted instance/handoff architecture.

`AdmissionId` and `CharacterLeaseId` are not invented by this gate. `FND-04` may request a narrow identity amendment only if its final state-machine design proves that either is an independently addressable semantic entity.

`CommandId` remains `FND-02` work. Runtime handles remain `FND-03` work. Session/admission/lease state machines remain `FND-04` work. PostgreSQL representation and later durable-domain identity catalogues remain `DUR-*` work. Analytics/audit identity catalogues remain `ANL-*` work.

Connection/channel/instance generations and party revision are ordering/fencing values, not entity identities.

## Validation

### Changed-file review

- result: `PASS`
- final delivery diff relative to then-current `main` was exactly the two declared paths:
  - `docs/agents/tasks/active/OTV2-20260807-fnd-id01-contract.md`;
  - `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`.
- the branch was synchronized with the current `main` immediately before final validation and was `0` commits behind.

### Component/integration

- result: `NOT_APPLICABLE`
- reason: architecture/documentation only; no executable runtime behavior changed.

### E2E

- result: `NOT_APPLICABLE`
- reason: no executable user/runtime journey changed.

### Exact-head CI

Exact delivery head:

- `6686a4b62c1e6b518d38ab3c80326b8621abe5bb`

Required runs:

- Agent governance run `31210390375`: `PASS`;
- Dependency review run `31210390471`: `PASS`;
- CodeQL run `31210390382`: `PASS`.

### Independent architecture audit

- exact head: `6686a4b62c1e6b518d38ab3c80326b8621abe5bb`;
- PR review ID: `4885881365`;
- result: `PASS`;
- open material findings: `0`;
- unresolved review threads: `0`.

The audit retained one non-material coordination finding: `FOUNDATION_DECISION_BACKLOG.md` and `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` still contain older pre-minimum-scope FND-ID wording. Issue #86 records the required non-destructive reconciliation. The merged FND-ID contract and later minimum-scope baseline are the semantic authority meanwhile.

## Merge and closeout

- delivery PR: #85;
- merge method: squash;
- merge result: `2c584543cd1e3758958755478a6cc6ed3d39a8a9`;
- contract is canonical on `main`;
- task ownership: released by this lifecycle closeout;
- follow-up coordination issue: #86.

## Programme consequence

The identity gate no longer blocks `FND-02` semantically.

Before the programme presents FND-02 as the clean canonical next action, issue #86 should reconcile the two stale long-lived coordination views so they no longer list the older broad FND-ID candidate catalogue.

No protocol/runtime implementation is authorized by this completion.

## Context checkpoint

```yaml
last_progress: FND-ID-01 contract passed exact-head governance/dependency/CodeQL checks and independent architecture audit, then squash-merged as 2c584543cd1e3758958755478a6cc6ed3d39a8a9.
status: completed
branch: docs/OTV2-20260807-fnd-id01-contract
pr: 85
final_head_sha: 6686a4b62c1e6b518d38ab3c80326b8621abe5bb
merge_commit: 2c584543cd1e3758958755478a6cc6ed3d39a8a9
ci_run_ids:
  - 31210390375
  - 31210390471
  - 31210390382
independent_audit_review_id: 4885881365
runner_assignment_state: completed
owner_action_required: null
blocker: null
next_action: Reconcile issue #86 in the foundation backlog/global register, then begin the bounded FND-02 protocol contract task.
```
