# OTV2-20260927-dur03-one-item-audit-contract-decision-513

```yaml
task_id: OTV2-20260927-dur03-one-item-audit-contract-decision-513
title: DUR-03 one-item typed audit and resource decision packet
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 513
base_branch: main
branch: codex/dur03-one-item-audit-contract-decision-513
pr: 969
base_sha: 76c2da68bde1928ab35e4e0f7828c675133cd297
head_sha: 4fc5543766413d37649539bbddde5bcdbaf5c8f4
final_head_sha: 4fc5543766413d37649539bbddde5bcdbaf5c8f4
final_head_frozen_at: null
owner: released
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_DUR03_REFERENCE_ONE_ITEM_AUDIT_RESOURCE_DECISION_PACKET_2026-09-27.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-one-item-audit-contract-decision-513.md
public_contracts: []
depends_on:
  - OTV2-20260917-dur03-reference-one-item-resource-evidence-513
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Provide a reviewable, nonbinding typed evidence recommendation and exact proposed
schema/registry/measurement follow-on scope for the actual DUR03-RL-07 audit gap.
Allocation: [#162 comment 5854665056](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5854665056).
Current phase is AUTHORING/pre-freeze; no canonical PR or final candidate exists.

## Architecture and source of truth

`PROVEN`: at the consumed base, DUR-03 §39 requires sufficient bounded immutable
mandatory evidence; ANL-01 and GAME_EVENT_FOUNDATION_REGISTRY define envelope,
membership, exact payload-byte stability and production privacy/retention gates.
RESOURCE_LIMITS_REGISTRY supplies shared ANL ceilings but contains no DUR03-RL-01..08
entries. The retained one-item report leaves actual audit count/bytes null.

`DERIVED`: prefer a typed transaction aggregate with closed MINT/TRANSFER
alternatives for one item. This recommendation does not grant implementation authority.
`UNKNOWN`: actual item-event schema/count/bytes, item retention approval, numeric
resource ceilings, PostgreSQL atomicity/restart and Reference parity.
`CONFLICT`: synthetic probes/private receipt encodings or Character bootstrap
policy cannot substitute for item-audit evidence/retention acceptance.

Control-plane supplied read-only pins: Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446`; CrystalServer
`ff7ede593c69d4c658b382c97443e8155926924a`. They support only coarse ordering and
separate save/load. Exact repository/path provenance and limits are in the packet;
no external repository mutation is performed or authorized.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this documentation-only allocation does not perform a production
mutation, authorize PREPARE/COMMIT, install authority, interpret recovery as live
authority or implement a persisted recovery path. The packet preserves those
boundaries and specifies independent-current-fact adversarial successor tests.

## Acceptance criteria

- [x] Packet answers every mandatory decision question and labels evidence status.
- [x] MINT/Ground and TRANSFER/direct-root fields, typed shape, privacy gap and
  RL-01..08 matrix trace to consumed Oteryn authority.
- [x] Exact successor path proposal and adversarial tests preserve evidence/runtime boundaries.
- [x] Exactly the two owned files pass focused documentation/governance checks.
- [ ] Control plane publishes/freezes and obtains applicable review/hosted checks
  before protected integration; the writer does not claim that phase complete.

## Excluded scope

No runtime/server/prototype edit, SQL, migration, protocol, IDL, registry or
dependency edit; no event identifier or production hard maximum selected; no
payload-byte measurement, production durability, natural loot probability or
Reference parity claim. No commit, push, PR/comment, external write or `.codex` edit
by this writer. The proposed successor is not a granted allocation.

## Implementation / findings

Two-file authoring packet records the actual audit gap, field-by-field semantics,
mandatory decision test, aggregate recommendation, privacy/retention gate,
resource evidence dispositions and one exact proposed next task. Owning acceptance
and candidate-specific review remain control-plane work. Known closeout metadata
is prepared before freeze; no self-referential commit/head claim is made.

## Validation

### Focused

- `python tools/agents/validate_governance.py`: PASS, 22 required policy documents
  and nine project lanes. Local Markdown link and whitespace checks: PASS.
- Scope: exactly two new allocated documents; pre-existing untracked `.codex/`
  preserved. Active task fits the governance size limits. `git diff --check`: PASS
  for tracked state; direct whitespace checks cover these untracked documents.
- These are authoring checks, not hosted validation of a frozen candidate.

### Component/integration

`NOT_APPLICABLE`: documentation-only change; no executable component modified.
Repository-selected hosted gates remain required on the eventual frozen candidate.

### E2E

`NOT_APPLICABLE`: no gameplay/runtime behavior introduced or conformance claimed.

### Exact-head CI

- Final head/trigger/run/job/runner: pending control-plane publication and freeze.
- Classification: pre-freeze; no hosted qualification claimed.
- Result: pending.

## Self-review

- Exact head: pending; current review is of the authoring files at the named base.
- Method/reviewer: implementing writer full-text authority/scope review; control
  plane retains mandatory final candidate self-review.
- Material findings: none in bounded authoring full-text review.
- Verdict: authoring scope/authority checks PASS; final frozen-candidate self-review pending.

## Independent review

- Required: owning DUR/ANL/privacy semantic review before schema acceptance;
  candidate review selection remains bound risk-policy/control-plane work.
- Exact head/method/findings/verdict: pending; no provider invocation by writer.

## PR and closeout

Changed-file review pending control plane. Canonical PR, review threads, protected
queue integration, merge proof and ownership release pending; no ready/merged claim.
Prior resource evidence is preserved, not superseded as a runtime implementation.

## Context checkpoint

```yaml
last_progress: PR 969 merged; terminal DUR-03 decision packet archived after lease release
status: completed
branch: codex/dur03-one-item-audit-contract-decision-513
head_sha: 4fc5543766413d37649539bbddde5bcdbaf5c8f4
pr: 969
final_head_sha: 4fc5543766413d37649539bbddde5bcdbaf5c8f4
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
next_action: none for completed child; aggregate #513 and KAN-12 remain outside this archive
```


## Completion

`PROVEN`: [PR #969](https://github.com/Oteryn/Oteryn-Game/pull/969) merged exact head
`4fc5543766413d37649539bbddde5bcdbaf5c8f4` as protected merge commit
`419a7cbc8539c9c98bd83d9220e331255e7e8f12` at `2026-09-27T09:59:02Z`.
This terminal packet is archived after that integration; its original validation,
review and evidence gaps above remain historical and are not refreshed by this move.
The aggregate #513 / KAN-12 remains outside this child closeout. No original exact
freeze timestamp is retained, so `final_head_frozen_at` remains `null`.
