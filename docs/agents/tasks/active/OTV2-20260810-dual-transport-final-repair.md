# OTV2-20260810-dual-transport-final-repair

```yaml
task_id: OTV2-20260810-dual-transport-final-repair
title: Repair final architecture review findings after PR 145 rotation
mode: REPAIR
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260810-dual-transport-final-repair
pr: null
base_sha: 9bf162e9d78f41706e92253c41f36d745e33382e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture repair coordinator
created_at: 2026-08-10T23:18:00+02:00
updated_at: 2026-08-10T23:27:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260810-dual-transport-final-repair.md
  - docs/agents/tasks/active/OTV2-20260810-architecture-review-dual-transport.md
  - docs/agents/tasks/archive/OTV2-20260810-architecture-review-dual-transport.md
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
  - docs/architecture/README.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_TRANSPORT_POLICY.json
depends_on:
  - exhausted review candidate PR 145 at 9bf162e9d78f41706e92253c41f36d745e33382e
  - merged governance PR 146
  - merged FND-ID lifecycle cleanup PR 147
blocks:
  - merge of the owner-accepted architecture/transport documentation programme
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

Rotate the exhausted PR #145 candidate into a fresh bounded repair task and fix only the three material P2 findings from its final required independent review, without reopening already accepted transport/admission/security decisions or changing runtime authority.

The three repairs are:

1. machine-readable client mode availability must not claim a transport mode is runtime-available before any TCP/QUIC gameplay adapter/listener exists;
2. canonical architecture source hierarchy must give an explicit later superseder precedence over the older owning contract for the named superseded scope;
3. `ImplementationStatus=PROVEN` rows for FND-01/VSL-02 must name exact revision/evidence, or be downgraded.

## Architecture and source of truth

- `PROVEN`: PR #145 final head `9bf162e9d78f41706e92253c41f36d745e33382e` passed exact-head Agent Governance `31432242537`, Dependency Review `31432242336`, CodeQL `31432242339` and mandatory self-review `4900964849`.
- `PROVEN`: its final required Codex review `4901019165` on commit `9bf162e9d7` produced exactly three new P2 threads; PR #145 repair budget was already `3/3`, so the candidate rotated instead of receiving a fourth repair.
- `PROVEN`: current status says TCP/QUIC gameplay adapters/listeners are not implemented/authorized, so client-mode runtime availability cannot be true even when TCP profile `1` is registered/default architecture.
- `PROVEN`: an explicit later superseding ADR must win over the older owning contract only for the scope it explicitly supersedes; outside that scope the owning contract remains authoritative.
- `PROVEN`: PR #50 final head `5092f868a42d545f47a98c0b9723210570cd9d45` passed named Rust/migration/governance/security checks and squash-merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`; this is exact evidence for the applied 19-member workspace/cutover claims used by FND-01/VSL-02.
- `OWNER_ACCEPTED`: preserve every already reviewed NET-TRANSPORT-01 invariant from PR #145; no new transport profile, adapter, listener, library, Platform write or production authorization is introduced.

## Acceptance criteria

- [x] Rotate PR #145 task record to archive/blocked state without rewriting its review history.
- [x] Make every client mode in `PROTOCOL_OTERYN_TRANSPORT_POLICY.json` report runtime availability truthfully; registered/default TCP architecture remains distinct from runtime implementation availability.
- [x] Put explicit named superseders ahead of older domain owners in architecture source hierarchy for the named scope only.
- [x] Attach exact PR #50 head/merge/check evidence to `FND-01` and `VSL-02` `PROVEN` status rows.
- [x] Do not alter any other accepted transport/admission/security semantic from the exhausted candidate.
- [ ] Mandatory full-diff self-review on exact final head reports zero material findings.
- [ ] Exact-head repository checks pass.
- [ ] Required independent review passes because this repair still touches high-risk transport/admission architecture; Codex is invoked only for the final frozen head.

## Excluded scope

- no fourth repair on PR #145;
- no new QUIC profile ID;
- no gameplay transport implementation;
- no FND-04 grant-profile implementation/revision beyond the already accepted blocked dependency statement;
- no Platform repository write;
- no production/live change;
- no unrelated architecture cleanup.

## Implementation / findings

- Archived the exhausted PR #145 task as `blocked/rotated`, preserving final head, exact CI/self-review evidence, final Codex review `4901019165`, repair budget `3/3` and the three transferred P2 findings; removed the old active record to release overlapping ownership.
- `PROTOCOL_OTERYN_TRANSPORT_POLICY.json` revision 4 now separates architecture registration/default policy from runtime implementation. TCP profile `1` remains the currently registered initial/default architecture profile, but both TCP and QUIC adapters/listeners plus **all** client modes have `runtime_available_now=false`; the native client remains pre-native-protocol.
- `docs/architecture/README.md` now gives an explicit later superseder precedence over the older domain owner only for the scope the later decision explicitly names, while preserving the older owner everywhere else.
- `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` now names PR #50 exact final head `5092f868a42d545f47a98c0b9723210570cd9d45`, merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0` and exact PASS run IDs for FND-01; VSL-02 names the same exact cutover head/merge and its Linux/Windows/fail-closed/supply-chain evidence rather than asserting unnamed proof.
- NET-TRANSPORT delivery remains `OPEN / NOT_STARTED` in the repair candidate; no runtime or production authority is created.

## Validation

### Focused

- source findings: exact final PR #145 Codex review `4901019165` and its three P2 threads
- PR #50 evidence: direct GitHub PR metadata verified: final head `5092f868a42d545f47a98c0b9723210570cd9d45`, merge `78988f72a80cc904aa9176ae850c50d4efa0b0f0`, Agent Governance `31095853261`, Dependency Review `31095853437`, CodeQL `31095853606`, Rust workspace `31095853343`, adversarial migration audit `31095053578`
- transport-policy semantic check: registration/default fields are separate from runtime adapter/listener/client-mode availability; all runtime availability remains false
- result: PASS pending full-diff final-head review

### Component/integration

- result: `NOT_APPLICABLE` — documentation/contract repair only

### E2E

- result: `NOT_APPLICABLE` — no executable runtime behavior

### Exact-head CI

- final head: pending after PR metadata/task checkpoint freeze
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent, mandatory and nondelegable
- material findings: pending
- verdict: pending

## Independent review

- required: `YES` — transport/admission/security contract repair remains high risk
- method/auditor: Codex only on the final frozen head because a genuinely independent review is required and Codex is the appropriate available mechanism
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: bounded successor repair plus inherited PR #145 architecture delivery; final compare pending
- unresolved review threads: none exist on successor yet; original PR #145 final findings are transferred and repaired here
- related/superseded PRs: PR #145 to be closed as rotated/superseded after successor PR opens
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Applied exactly the three final-review repairs and archived/released the exhausted PR #145 task record.
status: implementing
branch: docs/OTV2-20260810-dual-transport-final-repair
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: pre-pr
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
next_action: Open the bounded successor PR, close PR #145 as rotated/superseded, then freeze the successor head for self-review, exact-head CI and one required final Codex review.
```
