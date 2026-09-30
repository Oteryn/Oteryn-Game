# OTV2-20260930-charm-5-reg

```yaml
task_id: OTV2-20260930-charm-5-reg
title: CHARM-5-REG register the Bestiary and Charm wire (capability 1, commands 4-5, domains 4-5)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/charm-5-reg
pr: 1384
base_sha: 7be06776d617e7dd18cc88296466ec0228084bbf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: control-plane task worker (hard), lane charm, #162 owner answer Q2a 5913704837
created_at: 2026-09-30T00:00:00Z
updated_at: 2026-09-30T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto
  - docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md
  - crates/protocol-oteryn/src/{bestiary,charm,charm_wire,lib}.rs
  - crates/session/src/lib.rs
  - apps/client/src/cyclopedia.rs
  - apps/game-server/src/combat/{charm_effects,charm_effects_tests}.rs
  - apps/game-server/src/gameplay_transport/charm.rs
  - docs/agents/tasks/active/OTV2-20260930-charm-5-reg.md
  - docs/agents/tasks/archive/OTV2-20260930-charm-5-reg.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/charm_bestiary_v1.proto
depends_on: ["#1301 CHARM-5 (merged)", "#1303 CHARM-4 (merged)"]
blocks: [CHARM-5-COMP]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The CHARM-5 wire accepted by Sol is registered: capability 1 `BESTIARY_CHARMS_V1` (not offered, D170),
command types 4 and 5, state domains 4 and 5, `CHARM5-RL-01..05`, and the proto file. `CharmViewV1` gains the
server-derived per-charm `effect_active` (Sol ruling #162 5913269950, owner D235); measured bound 554 bytes.

## Architecture and source of truth

- PROVEN: Sol protocol acceptance #162 5907282001 (IDs, capability-gated, revision semantics).
- PROVEN: Sol ruling #162 5913269950 (490 acknowledged; `effect_active` ruled; this PR acknowledges the new bound).
- DERIVED: bound 554 = 32 × (15 + 2) + 8 + 2, asserted exactly by `the_full_view_at_its_bounds_is_exactly_the_byte_bound`.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no production mutation, fence, persistence or recovery path; registry, codec and pure view changes only.

## Acceptance criteria

- [x] Registries and proto name the IDs once, under these names, with the codec bounds (`registries_bind_the_charm_wire_ids_and_limits`).
- [x] `effect_active` field 6 encodes, decodes and fails closed on values other than 0/1.
- [x] Server derives `effect_active` from `CharmMissingSystem` (`CharmDefinition::effect_active`, matrix test: 9 active).
- [x] Client row shows "Effect not yet active".
- [x] Capability 1 recognised by the foundation codec but not offered (server selects none).

## Excluded scope

CHARM-5-COMP (routing in `gameplay_transport/connection.rs`, `CharmProgressionPort` binding), offering capability 1
(CHARM-6, D170), migrations.

## Validation

### Focused

- command/run: `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-client -p oteryn-game-server --all-targets -- -D warnings`
- result: PASS

### Component/integration

- command/run: `cargo test --locked -p oteryn-protocol-oteryn -p oteryn-session -p oteryn-client -p oteryn-game-server`; `python3 tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`; `git diff --check`
- result: PASS (all test suites ok, 0 failed)

### E2E

- scenario: NOT_APPLICABLE: nothing is composed or offered until CHARM-5-COMP and CHARM-6

### Exact-head CI

- final head: the frozen head in the FREEZE_SHA comment on #162
- result: pending on that head

## Self-review

- method/reviewer: implementing agent, whole-diff review
- material findings: none open. Capability 1 is recognised by the foundation codec (`REGISTERED_CAPABILITY_IDS_V1`) but every server path selects `[]`, so it is not offered (D170).
- verdict: ready for protocol review

## Independent review

- required: YES (protocol review of the registry, the proto and the 554-byte bound; triggered by the control plane only)
- exact head: the FREEZE_SHA head
- verdict: pending

## PR and closeout

- PR: #1384
- merge commit/result: squash merge of #1384 (resolve with `git log --grep "(#1384)"`)
- auto-merge: not enabled by the worker
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: final authoring commit, archived for freeze
status: completed
branch: claude/charm-5-reg
head_sha: null
pr: 1384
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
next_action: control plane triggers protocol review on the frozen head
```
