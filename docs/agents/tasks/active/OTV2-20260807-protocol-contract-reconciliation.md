# OTV2-20260807-protocol-contract-reconciliation

```yaml
task_id: OTV2-20260807-protocol-contract-reconciliation
title: Reconcile legacy Platform native gameplay contract with Oteryn v2 architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260807-protocol-contract-reconciliation
pr: 63
base_sha: 6804f5d67b63f1374a9efa3710bcaad10805c801
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-07T10:18:00+02:00
updated_at: 2026-08-07T10:18:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260807-protocol-contract-reconciliation.md
  - docs/architecture/FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
public_contracts:
  - docs/architecture/FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md
  - docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
depends_on:
  - ADR-0001 native Rust stack and protocol-oteryn-only target
  - ADR-0003 Platform Identity/Game Gateway/World Registry boundary
  - ADR-0008 protocol-canary reference-only disposition
  - ADR-0011 pre-native-protocol client state
  - FND-ID-01 remains the next ordered foundation gate
  - blakinio/Oteryn-Platform canonical native gameplay contract at c0b8703d326a04b43ae8e06f6192b0cb91c859b7
blocks:
  - accepting the existing Platform native gameplay contract as FND-02 without reconciliation
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Record the repository-owner accepted decision that the existing `blakinio/Oteryn-Platform` native gameplay contract is retained as bounded reconciliation input but is not the final `protocol-oteryn` contract for Oteryn v2. Preserve useful Platform admission/security semantics while preventing stale Canary/Otheryn/runtime and prematurely frozen wire assumptions from becoming implicit `FND-02` authority.

## Architecture and source of truth

- `PROVEN`: ADR-0001 makes the authoritative Rust server and one production `protocol-oteryn` family the Oteryn v2 target.
- `PROVEN`: ADR-0008 makes `protocol-canary` reference-only and forbids production negotiation, fallback, listener and translation roles.
- `PROVEN`: ADR-0003 preserves Platform Identity, Game Login Ticket, Game Gateway and World Registry as the external authentication/routing control plane.
- `PROVEN`: `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` locks Platform commit `c0b8703d326a04b43ae8e06f6192b0cb91c859b7`, schema revision `2`, while recording `accepted_for_fnd02: false`.
- `CONFLICT`: the locked Platform contract still describes Canary as an offered family, Otheryn as the authoritative gameplay runtime, and a concrete native transport/schema tuple before Oteryn v2 has completed `FND-ID-01` and `FND-02`.
- `OWNER_ACCEPTED`: on 2026-08-07 the repository owner accepted that this Platform contract remains reconciliation input only and must not be treated as the final `protocol-oteryn` contract; `FND-02` will freeze the final wire contract after `FND-ID-01`.

The Platform repository remains read-only for this task. This task records the Oteryn-v2 side of the cross-repository decision and the required future Platform-side reconciliation.

## Acceptance criteria

- [x] Add a dedicated owner-accepted reconciliation baseline under `docs/architecture/`.
- [x] Make clear which Platform contract semantics remain valid inputs and which conflicting assumptions are not accepted as final Oteryn-v2 protocol authority.
- [x] Preserve `FND-ID-01 -> FND-02 -> FND-03/FND-04` ordering.
- [x] Clarify the machine-readable contract lock without pretending an external merged revision disappeared or was rewritten.
- [x] Do not mutate `blakinio/Oteryn-Platform`.
- [x] Do not implement protocol, runtime, admission, persistence or production behavior.
- [ ] Review the final complete diff and verify applicable documentation/governance validation on the exact final head.

## Excluded scope

- no writes to `blakinio/Oteryn-Platform`;
- no superseding Platform ADR/contract in this repository on Platform's behalf;
- no `protocol-oteryn` IDL, framing, transport or codec implementation;
- no production Canary compatibility;
- no Rust runtime changes;
- no Game Session/admission/lease implementation;
- no persistence/database changes;
- no production activation or rollout.

## Implementation / findings

The existing Platform contract is useful evidence for fail-closed authentication/admission concepts, ticket/session binding, World Registry policy, downgrade prevention and bounded compatibility checks. Those concepts remain inputs where consistent with accepted Oteryn-v2 architecture and are re-expressed by their owning gates.

The same document cannot be accepted wholesale as `FND-02` because it also freezes assumptions that conflict with accepted Oteryn-v2 architecture or gate ordering. The reconciliation is therefore selective and explicit rather than treating the entire external contract as either canonical Oteryn-v2 protocol authority or discarded evidence.

Implemented Oteryn-v2-side records:

- `docs/architecture/FND-02_PLATFORM_PROTOCOL_RECONCILIATION_OWNER_BASELINE.md` records the owner-accepted interpretation and preserve/evidence/reject classification;
- `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` records that the external Platform contract is reconciliation input only while `FND-ID-01` remains the next ordered gate;
- `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` preserves the immutable external revision but marks its Oteryn-v2 disposition `RECONCILIATION_INPUT_ONLY`, `reconciliation_required: true` and `accepted_for_fnd02: false`.

## Validation

### Focused

- command/run: pending exact-head documentation/governance workflow evidence
- result: pending

### Component/integration

- command/run: `NOT_APPLICABLE` — documentation/contract-state task only; no runtime component changes
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable user/runtime outcome changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: recorded in immutable PR/workflow evidence after final commit
- trigger source: pull request/push workflow event if emitted
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending immutable PR evidence
- method/auditor: pending independent review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: initial full diff reviewed; final-head recheck pending after this metadata commit
- unresolved review threads: pending final-head inspection
- related/superseded PRs: none found in Oteryn-v2 at task start
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #63 now contains the owner-accepted reconciliation baseline, current-status clarification and machine-readable reconciliation-only contract-lock disposition.
status: validating
branch: docs/OTV2-20260807-protocol-contract-reconciliation
head_sha: null
pr: 63
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
next_action: Freeze the final diff and verify exact-head documentation/governance checks and independent audit state.
```
