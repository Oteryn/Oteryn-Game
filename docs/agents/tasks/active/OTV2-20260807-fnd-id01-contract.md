# OTV2-20260807-fnd-id01-contract

```yaml
task_id: OTV2-20260807-fnd-id01-contract
title: Complete the minimum FND-ID-01 foundation identifier contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-fnd-id01-contract
pr: 85
base_sha: 67c45efd35a4882ee414a9cd78c879a7d61a97ac
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T21:00:00+02:00
updated_at: 2026-08-07T21:09:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-fnd-id01-contract.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
public_contracts:
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
depends_on:
  - ADR-0001 through ADR-0011
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
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
blocks:
  - FND-02 must not freeze dependent protocol identity fields before this contract is accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only reconciliation input; no writes authorized)
```

## Outcome

Create the complete minimum `FND-ID-01` foundation identifier contract required before `FND-02`, without expanding the gate into a whole-game identifier catalogue or implementing runtime/protocol/persistence behavior.

The contract consolidates the owner-accepted identity decisions into one normative candidate surface, resolves only remaining identity-level ambiguities required by downstream foundation contracts, and explicitly assigns wire/runtime/session/persistence mechanics to their owning gates.

## Architecture and source of truth

- `PROVEN` — `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` identifies `FND-ID-01` as the next ordered gate and requires the complete contract before `FND-02` freezes dependent protocol identity fields.
- `PROVEN` — `FND-ID-01_MINIMUM_CROSS_BOUNDARY_SCOPE_OWNER_BASELINE.md` fixes the minimum catalogue: `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId`, `GameSessionId`, plus only conditionally necessary foundation identities.
- `PROVEN` — detailed owner baselines already freeze semantic scope, UUIDv7 direction/representation for adopted Oteryn-owned identities, owner/issuer boundaries and identity-versus-authority separation.
- `PROVEN` — accepted instance handoff architecture requires every authoritative ownership transition to have a unique transfer identity and to be idempotent/generation-fenced.
- `DERIVED` — `HandoffId` is the narrow foundation identity required to satisfy that already-accepted transfer invariant without making `FND-02`, `FND-03` and `FND-04` invent incompatible transfer namespaces.
- `DERIVED` — `NodeId` issuance is closed by local fresh UUIDv7 generation during each authenticated GameNode bootstrap, followed by trusted registration; this preserves one ID per process incarnation and the accepted no-central-UUID-service direction without making NodeId a credential.
- `CONFLICT` — `FOUNDATION_DECISION_BACKLOG.md` and `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` retain an older broad FND-ID-01 candidate list that predates the later owner-accepted minimum-scope baseline. Issue #86 records the required non-destructive reconciliation; the later minimum-scope baseline is semantic authority for this contract.
- `PROVEN` — the pinned external Platform native protocol revision remains reconciliation evidence only; it does not prove a final AccountId wire representation or override later Oteryn-v2 identifier semantics.
- `PROVEN` — no runtime, protocol, database or Platform implementation is authorized by the current architecture mode.

## Acceptance criteria

- [x] Provide one normative minimum identifier catalogue and owner/issuer/scope/lifecycle/representation/visibility matrix.
- [x] Preserve all accepted semantics for `AccountId`, `CharacterId`, `WorldId`, `ChannelId`, `NodeId`, `InstanceId`, `PartyId` and `GameSessionId`.
- [x] Resolve the identity-level `NodeId` generator/issuer ambiguity without making registration or channel authority implicit in identifier possession.
- [x] Classify the already-required unique ownership-transfer identity without designing the downstream handoff state machine.
- [x] Explicitly reject premature `AdmissionId` and `CharacterLeaseId` creation unless a later accepted foundation amendment proves an independent semantic entity is required.
- [x] Define the minimum foundation fencing/revision vocabulary as non-identity values, including connection and channel/instance ownership generations, without stealing exact runtime/protocol mechanics.
- [x] Preserve strong semantic typing, full 128-bit UUIDv7 where accepted, explicit scope, no nil sentinels, no reuse, and lossless validation.
- [x] Preserve external `AccountId` authority without silent re-keying.
- [x] Define public/internal visibility defaults and compact-handle boundaries.
- [x] Keep `CommandId`, wire byte order/IDL/framing, runtime handle bit layout, session/lease state machines and PostgreSQL layout in their owning later gates.
- [x] Preserve architecture/analysis-only mode; no Rust/runtime/protocol/persistence/Platform implementation.
- [x] Review the complete changed-file scope against the exact base; only the declared task and contract paths are changed on the contract branch.
- [ ] Obtain an independent architecture audit on the frozen exact head with zero open material findings.
- [ ] Pass required exact-head repository checks.

## Excluded scope

This task does not:

- implement Rust identifiers or crates;
- create `protocol-oteryn` schemas/codecs/listeners;
- implement GameNode registration, runtime ownership, Game Session admission, reconnect or leases;
- implement persistence schemas, migrations or database indexes;
- modify `blakinio/Oteryn-Platform`;
- define item/economy/analytics/quest/content/social/operations identifier catalogues beyond the minimum foundation requirement;
- freeze `CommandId` or protocol sequencing representation;
- finalize product/gameplay policy such as disconnect protection, combat, transfer pricing or Party Finder matching;
- silently edit the stale long-lived coordination registers; issue #86 owns the required follow-up reconciliation so the conflict remains visible until corrected.

## Implementation / findings

The candidate contract now:

- consolidates the accepted eight-identity minimum catalogue;
- admits one conditional foundation identity, `HandoffId`, because the accepted ownership-transition architecture already requires a unique cross-runtime transfer identity;
- fixes NodeId lifecycle issuance to one fresh locally generated UUIDv7 per process incarnation, with trusted registration separate from mutation authority;
- keeps AccountId externally owned and losslessly wrapped rather than silently re-keyed;
- keeps `AdmissionId` and `CharacterLeaseId` out of the foundation until `FND-04` proves an independent semantic lifecycle;
- classifies connection/channel/instance generations and party revision as non-identity ordering/fencing values;
- leaves protocol encoding, runtime handle layout, admission/lease state machines and persistence mechanics to their downstream gates;
- records issue #86 for stale backlog/register scope language discovered during consistency review.

No runtime implementation is permitted in this task.

## Validation

### Focused

- exact-base changed-file review: `PASS` for the two task-owned paths;
- architecture consistency self-review: `PASS` with one documentation conflict external to the task-owned paths recorded as issue #86;
- result: pending independent audit and exact-head repository checks.

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture contract only; no executable behavior changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable user/runtime journey changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending; the branch head after this metadata correction is the candidate validation head
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: separate architecture consistency review against accepted ADRs and owner baselines
- material findings: pending
- verdict: pending
- known environment constraint: automatic Codex code review is currently reporting account review-usage exhaustion on contemporaneous PR #84; this does not waive the independent-audit requirement for PR #85.

## PR and closeout

- delivery PR: #85
- changed-file review: task + new foundation contract only
- unresolved review threads: pending
- related/superseded PRs: PR #84 is separate minimum-scope lifecycle closeout only; no path overlap with this contract
- documentation consistency follow-up: issue #86
- protected auto-merge: pending all gates
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Complete minimum FND-ID-01 candidate contract is drafted in PR #85; self-review found no material semantic conflict in the contract and separately registered stale backlog/register scope wording as issue #86. PR metadata now contains the exact governance-required Summary/Scope/Validation headings.
status: validating
branch: docs/OTV2-20260807-fnd-id01-contract
head_sha: null
pr: 85
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending next exact head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: independent architecture audit must be available and pass before merge; automatic review quota is currently exhausted on the adjacent closeout PR
next_action: Freeze the new exact head, inspect exact-head CI and independent review availability, and do not merge unless every required gate is proven.
```
