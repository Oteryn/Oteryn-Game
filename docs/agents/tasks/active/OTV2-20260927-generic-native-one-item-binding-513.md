# OTV2-20260927-generic-native-one-item-binding-513

```yaml
task_id: OTV2-20260927-generic-native-one-item-binding-513
title: Generic native one-item binding specialization
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/generic-native-one-item-binding-513
issue: 162
pr: null
base_sha: 3c9aef87a4dff22b9a8308d82a55f5bfe8469bf6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated documentation author
created_at: 2026-09-27T21:01:40+02:00
updated_at: 2026-09-27T21:08:56+02:00
execution_policy: continuous_progress
owned_paths:
  - MODIFY docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - ADD docs/agents/tasks/active/OTV2-20260927-generic-native-one-item-binding-513.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on:
  - #162 allocation comment 5858771487
  - DUR-03 §§39.1-39.2
  - GAME-ITEM-01, ANL-01, Content typed identity, VSL-COMBAT-01
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Record the generic native item definition/state, committed loot-output cause,
actual Ground context, and staged CharacterInventory destination bindings needed
for a later one-item MINT/TRANSFER implementation. Keep Character progression
readiness and its migration-0009 composition dependency explicit. This is a
contract/documentation child only; it does not claim playable Combat.

## Architecture and source of truth

- **PROVEN** — the repository binding pins META policy v3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; the bound organization policy was read at that immutable revision.
- **PROVEN** — DUR-03 §§39.1-39.2 require separate MINT and TRANSFER transactions, complete typed evidence, semantic absence, native item identity/state and actual Ground; native MINT-first does not require resolving TRANSFER placement first.
- **PROVEN** — GAME-ITEM-01 owns canonical ItemType identity and legal typed instance state; Content defines `TypedDefinitionRef = (DefinitionFamily, ProductionKey, DefinitionRevisionRef)`.
- **PROVEN** — VSL-COMBAT-01 binds deterministic loot to committed death/output occurrence and exact semantic revisions; item IDs are fresh DUR-03 transaction-scoped identities.
- **PROVEN** — ANL-01 fixes immutable semantic envelope values and exact payload bytes for same-EventId retry; EventId/TransactionId/OperationId remain distinct authorities.
- **PROVEN** — `0009_character_progression.sql` allows bootstrap-only Character revision one and guards later root revisions against the XP state/receipt chain.
- **DERIVED** — durable loot-source non-reuse across restart must be independent of audit expiry and caller-supplied UUIDs.
- **CONFLICT** — migration 0009 does not define how a non-XP CharacterInventory transfer participates in a global CharacterRevision chain that requires an XP receipt per successor; resolve with the Character owner before destination mutation/readiness is admitted.
- **UNKNOWN** — accepted native source-reference/receipt grammar, native position/capacity policy, physical schema, registry IDs/ceilings, and runtime composition are not selected here.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation-only contract binding; no production mutation, PREPARE/COMMIT, recovery interpretation, or authority-bearing runtime change is performed.
```

## Acceptance criteria

- [x] Add a generic native binding specialization after DUR-03 §39.2, consistent with the existing Content definition/revision grammar and GAME-ITEM state legality.
- [x] Bind MINT to a committed deterministic death/output occurrence with caller UUIDs excluded as authority and restart-stable non-reuse.
- [x] Bind Ground to actual WorldId/ChannelId and accepted map/content/native-room/typed-position context.
- [x] Bind direct-root CharacterInventory to CharacterId plus its owning typed position, without inventing InventoryRootId; retain MINT-first allowance and gate TRANSFER on legal destination admission.
- [x] Separate immutable expected bindings from independently current Content, session/lease, source, and runtime authority.
- [x] State Character initialization/readiness and the migration-0009 global-revision/XP-receipt composition conflict as separate later dependencies.
- [x] Explicitly leave quantity, probability, stack maximum, capacity, formulas, HP/damage/XP values, IDs, registry ceilings, SQL/runtime permission undecided.
- [x] Keep the active task record truthful to AUTHORING/implementing; no PR, frozen SHA, CI, or completion claim.

## Excluded scope

No runtime/SQL/migration/registry/protocol/test/Cargo/Combat implementation,
MINT/TRANSFER admission, Character initialization, XP settlement, room or harness
change, event registration, owner-funded review trigger, commit, push, PR, merge,
or other path is authorized by this allocation. No Rat/Gold Coin/cheese-specific
branches or product numbers are introduced. R7 P03 is an integrated later
dependency, and no R7 path is modified.

## Implementation / findings

Bounded authoring and local governance/whitespace validation are complete.
Publication and freeze remain pending. The proposed section specializes only
native semantic bindings and explicit gates; implementation acceptance, physical
mechanisms and product values remain with their owning allocations. The existing
native room/harness is a future composition environment and is not changed here.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`; `git diff --check`
- result: PASS — repository governance validation (22 required policy documents, 9 project lanes); PASS — no whitespace errors
- command/run: `python tools/repository/validate_repository_policy.py`
- result: FAIL — protected-base `LICENSE` does not match pinned canonical MPL-2.0. **PROVEN unchanged protected-base evidence**; `LICENSE` was neither waived nor modified, and this is not a candidate-path defect.

### Component/integration

- command/run: NOT_APPLICABLE — documentation-only contract child; no runtime component changed
- result: NOT_APPLICABLE — no component or integration implementation was authorized

### E2E

- scenario: NOT_APPLICABLE — no playable/runtime claim is made
- result: NOT_APPLICABLE — this child is documentation-only

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — architecture/contract binding and item-value authority boundaries need independent review before acceptance
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: pending
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: bounded authoring complete; repository policy validator reports unchanged protected-base LICENSE mismatch
status: implementing
branch: codex/generic-native-one-item-binding-513
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
next_action: root atomic publication on the allocated remote branch, exact readback, then freeze
```
