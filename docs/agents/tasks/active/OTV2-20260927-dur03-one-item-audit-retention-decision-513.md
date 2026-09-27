# OTV2-20260927-dur03-one-item-audit-retention-decision-513

```yaml
task_id: OTV2-20260927-dur03-one-item-audit-retention-decision-513
title: Canonicalize owner-selected DUR-03 one-item P90D audit retention
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 513
base_branch: main
branch: codex/dur03-one-item-audit-retention-decision-513
pr: null
base_sha: ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/dur03_retention_decision_writer under Combat control plane
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_DECISION_2026-09-27.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-retention-decision-513.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Canonicalize the owner's separate item-specific `ordinary_retention_ceiling=P90D`
selection for first closed one-item MINT into Ground and separate TRANSFER to
direct-root CharacterInventory, with purpose/privacy/access/export/expiry/hold/
revision terms and mandatory replay/non-reuse separation.

Allocation: [#162 comment 5856061315](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5856061315).
Owner selection: [#513 comment 5856056298](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5856056298).
Common terms/options: [#513 comment 5855850428](https://github.com/Oteryn/Oteryn-Game/issues/513#issuecomment-5855850428).
This packet captures AUTHORING metadata before final commit. Final freeze,
validation and any canonical draft PR are bound externally to the returned exact
remote head; no self-referential metadata commit is required.

## Architecture and source of truth

`PROVEN`: protected base is `main@ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c`.
[DUR-03 §39.1](../../../architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md)
accepts closed distinct MINT/TRANSFER aggregate semantics, leaving item retention
unaccepted. [ANL-01 §§15–16](../../../architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md)
requires finite purpose/privacy/access/retention acceptance before production.
The owner selection supplies the P90D item decision; this document remains
pending normal canonical qualification/integration.

`DERIVED`: the decision record maps those accepted common terms to bounded
cleanup and successor conformance scenarios without choosing runtime mechanisms.
`UNKNOWN`: serialized registry binding, actual schema/encoding/count/bytes,
numeric resource ceilings, replay horizon, physical cleanup, SQL/restart and
playable integration.
`CONFLICT`: inheriting Character bootstrap authority by duration similarity,
resetting expiry on export/hold release, or deleting replay/non-reuse protection
with audit expiry would violate this decision's scope.

The [Character retention precedent](../../../architecture/reviews/OTERYN_CHARACTER_DURABLE_AUDIT_RETENTION_DECISION_2026-09-22.md)
is process evidence only. No Reference repository is consulted as decision
authority. Nearest instructions are root AGENTS and docs/agents/AGENTS.
META binding remains `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: documentation-only canonicalization neither performs production
mutation, grants PREPARE/COMMIT, restores a controller nor interprets historical
evidence as live authority. The document explicitly preserves those boundaries.

## Acceptance criteria

- [x] Record the owner-selected separate logical item profile and P90D rolling ceiling.
- [x] Purpose, privacy floor, roles, case export, deletion, audited hold/release
  and forward-only immutable revisions preserve the selected common terms.
- [x] Audit expiry remains independent of replay/source-cause/non-reuse protections.
- [x] Mandatory decision test, realistic options, risks and later conformance
  scenarios distinguish owner selection from implementation evidence.
- [ ] Exact remote freeze/scope/readback and deterministic governance validation
  are returned in the writer packet and canonical PR evidence.
- [ ] Applicable independent review/hosted checks and protected integration
  remain control-plane work.

## Excluded scope

Exactly two new owned documents. No accepted contract, registry, EventId/schema,
resource maximum, replay horizon, evidence executable/output, runtime, Cargo,
SQL/migration, deployment or production change. No broader economy/market/trade/
history/balancing/AI purpose. No Jira mutation, provider review trigger, ready
transition, enqueue, merge or integration claim by this writer.

Jira mapping is control-plane supplied `KAN-12`, remaining `W toku` /
`readiness-active` in the allocation. No independent Jira refresh or synchronization
is claimed; programme updates remain with the active control plane.

## Implementation / findings

The new [retention decision](../../../architecture/reviews/OTERYN_DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_DECISION_2026-09-27.md)
canonicalizes the selected terms and records gates left open. Shared local
checkout changes remain untouched. Authoring uses high-level GitHub Contents API
on the sole allocated branch, with fresh head read before each write.
After the final write, require returned SHA equality, verify the two-path complete
delta, freeze, then qualify. A material repair explicitly reopens AUTHORING and
creates a newly frozen candidate; frozen-head evidence is not inherited.

## Validation

### Focused

Pending on final frozen remote head: exact base-to-head two-file inventory and
byte readback; LF/terminal-newline/whitespace and Markdown link checks; active task
size/lifecycle checks; `python tools/agents/validate_governance.py`; affected
governance/lifecycle tests where available. Results belong in exact-head external
evidence after freeze.

### Component/integration

`NOT_APPLICABLE`: no executable component changed. Hosted repository-selected
gates remain required; deterministic docs checks do not prove runtime behavior.

### E2E

`NOT_APPLICABLE`: no gameplay, SQL, restart, cleanup or retention implementation.

### Exact-head CI

Final head, trigger, run/job, runner and result: pending external freeze/PR evidence.
No hosted pass is claimed in this pre-commit snapshot.

## Self-review

Implementing writer remains responsible for full-text authority/scope review of
the final remote documents. Final exact head, findings and verdict are returned
externally after freeze; no whole-PR approval is delegated.

## Independent review

Required applicable DUR/ANL/privacy/data-integrity review is pending the active
control plane. No owner-funded/provider invocation by this direct writer.

## PR and closeout

Draft PR may be created after freeze/validation under the allocated lifecycle.
Readiness, unresolved threads, queue eligibility, merge proof and lease release
remain pending control plane. This task does not close aggregate #513 or KAN-12.

## Context checkpoint

```yaml
last_progress: Prepared two-file owner-selected item-specific P90D decision for API authoring
status: implementing
branch: codex/dur03-one-item-audit-retention-decision-513
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
next_action: Freeze final remote head and return exact two-file validation packet to the active control plane
```
