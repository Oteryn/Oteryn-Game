# OTV2-20260812-game-item-01-architecture

```yaml
task_id: OTV2-20260812-game-item-01-architecture
title: GAME-ITEM-01 item model and equipment architecture
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: agent/otv2-20260812-game-item-01-architecture
pr: 205
base_sha: 93a49731ad91620748b87cdaba9525c9df70bc12
head_sha: 746f2a3e61632a1ff86da350c460a24887acc183
final_head_sha: null
final_head_frozen_at: null
owner: architecture-coordinator/current-session
created_at: 2026-08-12T11:37:00+02:00
updated_at: 2026-08-12T13:15:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
repair_budget_extension: USER_SOURCE-2026-08-12-one-additional-cycle
owned_paths:
  - docs/agents/tasks/active/OTV2-20260812-game-item-01-architecture.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md
  - docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
public_contracts:
  - GAME-ITEM-01
depends_on:
  - GAME-VISION-01
  - DUR-01
  - DUR-02
  - ANL-01
  - ADR-0005
blocks:
  - DUR-03
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Deliver one bounded, paper-only `GAME-ITEM-01` architecture candidate defining native item semantics, equipment/container legality and item-definition compatibility. The candidate remains nonbinding while PR #205 is open.

`DUR-03` remains blocked until **both** the accepted GAME-ITEM delivery merge and the separate GAME-ITEM lifecycle-closeout merge complete. No runtime, PostgreSQL DDL/migration, entitlement activation or production authority is granted.

## Owner repair-budget extension

The repository default is `max_repair_cycles_per_gate: 3`. Four material repair cycles had already occurred and the task correctly stopped `blocked` at commit `8b2fa27003202351e93e86dc20ff9c10418f4378`.

On 2026-08-12 the owner explicitly replied `wykonaj` to the requested authorization for **one additional GAME-ITEM-01 repair cycle**. This is `USER_SOURCE` authority for exactly one further material cycle. It does not waive any other governance, review, CI, merge, production or cross-repository rule.

The authorized additional cycle is used only to repair independent-review P1 `3765563501` and resume validation. No further material repair cycle is authorized by this instruction.

## Binding sources and boundaries

- `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`: first Reference target is production-observable Global Tibia after the 2026-07-28 server-save/maintenance boundary; unresolved exact behavior stays fail-closed.
- ADR-0005: stable namespaced content identity is canonical; compact numeric IDs are revision-scoped mappings.
- DUR-01: `ItemInstanceId` is strongly typed UUIDv7; DUR-03 owns split/merge/transform/create/destroy identity transitions.
- DUR-02: common persistence/transaction substrate only; item/currency/value conservation remains downstream.
- ANL-01: event/audit identity and durable evidence; analytics is never gameplay authority.
- `PROD-ENTITLEMENTS-01` remains `PROPOSED / PLANNED / NOT_STARTED`; GAME-ITEM may consume Platform-owned entitlement facts only through a separately accepted Oteryn-v2 consumer/enforcement contract.

## Accepted candidate design

The candidate selects typed capability composition over stable ItemType definitions and separates:

```text
ItemType          -> immutable/versioned authored semantic definition
ItemInstance      -> concrete mutable lifecycle with DUR-01 ItemInstanceId
StaticItemPlacement -> authored world placement, not automatically a durable instance
```

It defines typed stack, charge, durability, temporal/decay, equipment, container, binding/restriction and upgrade/modifier capability state; rejects authoritative JSON/EAV/free-form script escape hatches; defines server-authoritative atomic equipment occupancy; requires bounded acyclic containment; and requires explicit definition compatibility/migration rather than silent reinterpretation.

DUR-03 retains atomic location, idempotency, retry/crash handling, stale-writer rejection, item/currency/value conservation and identity-transition rules.

## Acceptance criteria

- [x] Candidate definition/instance/static-placement boundary.
- [x] Typed bounded capability model with no generic authoritative data escape hatch.
- [x] Server-authoritative equipment occupancy and typed requirements.
- [x] Deterministic item-modifier contribution ordering requirement without capturing SIM formulas.
- [x] Bounded acyclic container legality while leaving atomic moves to DUR-03.
- [x] Explicit item-definition compatibility and migration classes.
- [x] World/binding/location/authorization concepts remain distinct.
- [x] Item versus non-item currency/value distinction.
- [x] Reference-sensitive exact behavior remains `PARITY_PENDING_EVIDENCE` unless separately proven.
- [x] Maintained programme/status/register/horizon/index overlays remain exact live-main while candidate PR is open.
- [x] `PROD-ENTITLEMENTS-01` consumer/enforcement remains separately gated.
- [x] Independent-review P1 `3765563501` repaired: DUR-03 becomes eligible only after GAME-ITEM accepted merge **and** lifecycle closeout.
- [x] Task uses repository-supported status `validating`.
- [x] Owner explicitly authorized one additional repair cycle beyond the default budget.
- [ ] Terminal exact-head self-review PASS on the resulting head.
- [ ] Independent exact-head review PASS with zero material findings.
- [ ] Required exact-head CI PASS.
- [ ] Zero unresolved material review threads.
- [ ] PR #205 squash-merged unchanged after all gates.
- [ ] Separate lifecycle closeout promotes canonical overlays, archives this task and releases ownership.

## Repair history

### Cycle 1 — status-overlay evidence typo

A transient historical DUR-01 SHA typo was detected and corrected; the status overlay was later restored bit-for-bit to live `main`, so it is not in the delivery diff.

### Cycle 2 — PR metadata governance / invalid CI recovery

Agent Governance `31586112278` correctly failed the original PR title/body metadata. Metadata was repaired. A task-only commit was then incorrectly used solely to retrigger CI; Codex P2 `3765519022` correctly identified the process violation. It is recorded and has not been repeated.

### Cycle 3 — premature acceptance/unblocking overlays

Codex P1 `3765519016` correctly identified pre-merge `ACCEPTED`/DUR-03-unblocked status in maintained overlays. All four maintained overlay/index files were restored to exact live-main blobs. PR #205 was narrowed to task + analysis + candidate contract.

### Cycle 4 — entitlement consumer boundary self-audit

Commit `f0d62331d71deedcb2b9a3b1fe0a1a32578200ed` removed wording that could imply an accepted Oteryn-v2 entitlement consumer and replaced pre-acceptance `accepts representation capability` wording with `defines representation capability`.

### Owner-authorized additional cycle — lifecycle-unblock repair

Independent Codex P1 `3765563501` remained applicable: the contract said accepted merge alone unblocked DUR-03 although lifecycle policy required accepted merge + closeout. Owner explicitly authorized one additional repair cycle. Commit `746f2a3e61632a1ff86da350c460a24887acc183` repairs the contract so DUR-03 remains blocked through the post-merge/pre-closeout interval.

P2 `3765563506` (unsupported `reviewing` task status) had already been fixed by the blocker checkpoint. This resumed record uses valid status `validating`.

## Validation history

Last pre-authorization material head `f0d62331d71deedcb2b9a3b1fe0a1a32578200ed` had:

- implementing-agent self-review `4915520214`: PASS for then-known content;
- Agent Governance `31587838775`: PASS;
- Dependency Review `31587838756`: PASS;
- CodeQL `31587838750`: PASS.

Those results are superseded for terminal evidence by the authorized lifecycle repair and this validation-state update. The resulting exact head must receive a new self-review, independent review and CI generation.

Component/integration/runtime E2E remains `NOT_APPLICABLE` because this is documentation-only architecture delivery with no executable behavior.

## PR and closeout discipline

PR #205 must remain candidate/nonbinding until merged. No maintained canonical overlay may promote GAME-ITEM or expose DUR-03 from this delivery PR.

After a clean accepted merge, exactly one bounded lifecycle-closeout task/PR may:

1. move this task active -> archive and record immutable delivery evidence;
2. promote GAME-ITEM-01 to `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` in maintained status/register/horizon/index sources;
3. expose DUR-03 as the next eligible **paper-only** architecture gate;
4. preserve runtime/DDL/production authority as `NONE`/unauthorized;
5. release GAME-ITEM ownership.

## Context checkpoint

```yaml
last_progress: Owner authorized one additional repair cycle; lifecycle-unblock P1 3765563501 was repaired so DUR-03 remains blocked until accepted GAME-ITEM merge plus lifecycle closeout, and task validation resumed with supported status.
status: validating
branch: agent/otv2-20260812-game-item-01-architecture
head_sha: 746f2a3e61632a1ff86da350c460a24887acc183
pr: 205
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: PR #205 synchronize after authorized material repair
ci_check_generation: pending resulting exact head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 1
owner_action_required: false
blocker: null
next_action: Treat the resulting branch head after this task-record commit as the frozen delivery head; run full exact-head self-review, independent Codex review and required CI, merge #205 only if all are clean, then perform the one allowed lifecycle-closeout task and continue to DUR-03 only after closeout merge.
```
