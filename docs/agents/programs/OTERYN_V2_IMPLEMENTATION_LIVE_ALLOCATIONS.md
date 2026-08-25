# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260825-close-next-wave-blockers` - Issue #131 active coordinator allocation
- Canonical repository: `Oteryn/Oteryn-Game`
- Bootstrap delivery PR: `#10`
- Bootstrap closeout PR: `#11`
- Simulation allocation PR: `#12`
- Simulation exact-base PR: `#13`
- Simulation delivery PR: `#14`
- Simulation delivery merge: `66619daf5837f31f7c54676e9f8351ed4ae220b0`
- Simulation archive PR: `#15`
- Wave 1 allocation source main: `7694c8a5e1ebc1dbffa937adf6b5cb775f7745f2`
- Wave 1 allocation PR: `#45`
- Wave 1 allocation merge: `33cec30b8075c73290d7d76e9f59df4701771650`
- Wave 1 exact-base PR: `#46`
- Wave 1 exact-base merge: `fd39c6aa026e82062a8b29af24811d467c115f19`
- Foundation delivery PR: `#59`
- Foundation delivery merge: `a70318484b1ffdd328b53cdc70a4386a516d0109`
- Foundation closeout PR: `#74`
- Foundation closeout merge: `1f69677b40851551953caf853c08b37ce7b29c68`
- Coordinator DOMAIN lease-transfer PR: `#76`
- Coordinator DOMAIN lease-transfer merge: `6945e962035bac83d1f19b00984df5b82719ebb9`
- Coordinator DOMAIN lease-active PR: `#78`
- Coordinator DOMAIN lease-active merge: `3988e4263569d6c90bf6e794688b810073ad5184`
- Foundation post-merge audit PR: `#81`
- Foundation post-merge audit merge: `55e30e23c3d5775ce760c6b210ea77f152b359ae`
- Domain delivery PR: `#56`
- Domain delivery merge: `0facd7f89edc1b0685e67c5531839e8e6f04c466`
- Content evidence delivery PR: `#58`
- Content evidence delivery merge: `8f99f25d0b1b3472d40504cd54b463cf752ebe7a`
- Content activation repair PR: `#87`
- Content activation repair merge: `db95bc720529b643531c79f708086f69dd612d22`
- State: `NEXT_WAVE_BLOCKER_CLOSURE_FND04_ALLOCATION_PENDING_MERGE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance, active task/PR/CI state and merged `main` outrank stale coordination prose.

PR #58 merged the bounded CONTENT evidence seam. A later genuinely independent exact-tree review reproduced a P0 activation-boundary defect, which Issue #85 and PR #87 repaired by removing the non-production activation state machine from the production public API. The repair is merged and independently re-reviewed; production VSL activation, permanent format selection and registry/contract mutation remain unauthorized.

Unmerged sibling output is never an implicit dependency. Stable registries/contracts, `.github/workflows/**`, architecture policy/tooling and new workspace/crate topology remain unallocated unless a later merged coordinator allocation explicitly grants them.

## Active coordinator — close next-wave blockers

```yaml
lane_id: OTV2-CLOSE-NEXT-WAVE-BLOCKERS
task_id: OTV2-20260825-close-next-wave-blockers
issue: 131
worker_alias: Oteryn: close next-wave blockers
status: active_child_fnd04_allocation_pending_merge
allocation_pr: 132
allocation_merge_sha: 8b6f8e6c0ab0f849a87a7a3a8eb97d8367649d26
base_sha: 9cc23cdbfe68d0a0f13df054874929b5e5dbe418
branch: null
current_child_issue: 115
current_child_allocation_pr: 145
current_child_allocation_branch: coord/allocate-fnd04-verifier-115
owned_paths:
  - docs/agents/tasks/active/OTV2-20260825-close-next-wave-blockers.md
  - docs/superpowers/plans/2026-08-25-oteryn-close-next-wave-blockers-implementation-plan.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
shared_lease: not_acquired_child_paths_remain_unallocated
depends_on:
  - issue:128
blocks:
  - issue:115
```

This allocation grants only coordinator documentation. The resource registry, Foundation/Cargo paths and every product implementation path remain released until a separately merged child allocation acquires one exact serialized lease. Issue #131 may coordinate child lifecycles but may not consume unmerged sibling output.

## Completed decision/evidence — next-wave first-slice limits

```yaml
lane_id: OTV2-NEXT-WAVE-LIMIT-DECISIONS
issue: 133
status: completed_released
allocation_pr: 137
delivery_pr: 140
delivery_merge_sha: 88ad620169d6d08ebad6e49886ba1098da728480
owned_paths: []
resource_registry_authority: none
```

The evidence/decision child is terminal. Its accepted 24 values are canonical through merged registry PR #144 / c1020b2db62ecfa18c411bee56fa004430b28923.

## Completed registry — next-wave first-slice limits

```yaml
lane_id: OTV2-NEXT-WAVE-REGISTRY
issue: 142
status: completed_released
allocation_pr: 143
delivery_pr: 144
delivery_merge_sha: c1020b2db62ecfa18c411bee56fa004430b28923
owned_paths: []
resource_registry_authority: none
```

Registry Issue #142 is terminal; #93, #116 and #123 were rechecked on current main and closed. The registry lease is released.


## Production FND-04 verifier/consumer allocation candidate

```yaml
lane_id: OTV2-FND04-VERIFIER-CONSUMER
task_id: OTV2-20260825-fnd04-verifier-consumer
allocation_task: OTV2-20260825-fnd04-verifier-allocation
issue: 115
status: allocation_pending_merge
risk: XHigh
allocation_branch: coord/allocate-fnd04-verifier-115
branch_after_allocation_merge: feat/fnd04-verifier-consumer-115
worker_base_policy: exact_allocation_merge_sha
owned_paths_after_allocation_merge:
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - docs/architecture/reviews/OTERYN_GAME_FND04_VERIFIER_CONSUMER_DELIVERY_2026-08-25.md
  - docs/agents/tasks/active/OTV2-20260825-fnd04-verifier-consumer.md
shared_cargo_lease: exclusive_after_allocation_merge
resource_registry_authority: none
independent_exact_head_security_review: required
```

The verifier allocation becomes effective only after its coordinator PR merges. It grants no listener, production key/KMS/config/deployment, durable-journal, Platform or external-repository authority.

## Completed allocation — Bootstrap

```yaml
lane_id: OTV2-IMPL-BOOTSTRAP
task_id: OTV2-20260818-impl-bootstrap
status: completed
final_head_sha: 43243c4998224517a4c828bc05e735264b3e3394
delivery_pr: 10
delivery_merge_sha: 0809004252db228e8f3fac3cdb6638c3c2a7fbda
archive_pr: 11
owned_paths: []
branch: null
```

## Completed allocation — Simulation

```yaml
lane_id: OTV2-IMPL-SIM
task_id: OTV2-20260818-impl-simulation
worker_alias: Oteryn: impl simulation
status: completed
execution_mode: serial_workspace_mutation
allocation_pr: 12
allocation_merge_sha: 2fc59dd83a3d13e7de8954d4dbcce5415e346389
exact_base_pr: 13
worker_base_sha: 977e98b05738076744540a123d4e35c32cd94c2c
final_head_sha: 7a0d71bbabdd00c54951aa8e0084d62f3dce748b
delivery_pr: 14
delivery_merge_sha: 66619daf5837f31f7c54676e9f8351ed4ae220b0
archive_pr: 15
owned_paths: []
branch: null
```

SIM delivered the bounded production `oteryn-simulation-determinism` core consumed by `apps/game-server`. Whole-contract/VSL proof remains separately evidence-gated.

## Completed allocation — Foundation

```yaml
lane_id: OTV2-IMPL-FOUNDATION
task_id: OTV2-20260822-impl-foundation-runtime
worker_alias: Oteryn: impl foundation runtime
status: completed
risk: XHigh
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
delivery_pr: 59
delivery_merge_sha: a70318484b1ffdd328b53cdc70a4386a516d0109
archive_pr: 74
archive_merge_sha: 1f69677b40851551953caf853c08b37ce7b29c68
independent_review_required: true
historical_pre_merge_independent_exact_head_gate: NOT_PROVEN
post_merge_independent_audit: PASS
post_merge_audit_issue: 77
post_merge_audit_pr: 81
post_merge_audit_merge_sha: 55e30e23c3d5775ce760c6b210ea77f152b359ae
post_merge_material_findings:
  P0: 0
  P1: 0
  P2: 0
owned_paths: []
branch: null
shared_lease: released
```

Foundation product implementation is byte-equivalent to its final PR tree and passed the post-merge independent audit with zero P0/P1/P2 implementation findings. Retained history does **not** prove the mandatory independent review was completed on the final pre-merge head; the historical gate therefore remains `NOT_PROVEN` and is not retroactively rewritten.

## Completed allocation — Domain

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: completed
risk: High
final_head_sha: a76c999a2b03c4271fda9b4395cc3d76c346987b
delivery_pr: 56
delivery_merge_sha: 0facd7f89edc1b0685e67c5531839e8e6f04c466
closeout_pr: 82
closeout_merge_sha: 30c733c8c8cb4a1fbcf63010bcb6709a9109dde6
issue: 55
issue_state: completed
owned_paths: []
branch: null
shared_lease: released
```

DOMAIN is merged into `apps/game-server`, its Issue is completed, its branch is absent and its serialized shared lease was released by PR #82.

## Content evidence delivery — production acceptance blocked

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: evidence_delivery_merged_repair_complete_production_blocked
risk: High
final_head_sha: ab0b4241c107bfb2c6052e58aec241da130774c7
delivery_pr: 58
delivery_merge_sha: 8f99f25d0b1b3472d40504cd54b463cf752ebe7a
issue: 54
issue_state: open_blocked
source_branch_present: false
owned_paths: []
shared_lease: released
registered_production_vsl_limits: not_found
production_activation: forbidden
permanent_format_selection: forbidden
repair_issue: 85
repair_issue_state: completed
repair_pr: 87
repair_final_head_sha: c9d3570f528acc8e22e3055e4f8de712e9057abd
repair_merge_sha: db95bc720529b643531c79f708086f69dd612d22
future_write_authority: requires_new_coordinator_allocation
```

The bounded non-production VSL evidence seam is merged and composed through `apps/game-server`. Pre-merge checks were green; a later exact-tree review found one P0 in the public activation boundary. Issue #85 reproduced it and PR #87 repaired it on exact head `c9d3570f528acc8e22e3055e4f8de712e9057abd` with fresh independent review P0=0/P1=0/P2=0 and exact-head `game-gate`, then merged as `db95bc720529b643531c79f708086f69dd612d22`. The evidence seam is no longer AT_RISK for that defect. The separate Issue #54 production blocker remains unchanged: accepted DUR-04/VSL hard maxima and production activation authority are absent.

## Completed repair — Content evidence activation fence

```yaml
lane_id: OTV2-REPAIR-CONTENT-ACTIVATION-FENCE
task_id: OTV2-20260824-content-evidence-activation-fence-repair
status: completed
risk: High
issue: 85
issue_state: completed
allocation_pr: 86
implementation_pr: 87
final_head_sha: c9d3570f528acc8e22e3055e4f8de712e9057abd
merge_sha: db95bc720529b643531c79f708086f69dd612d22
owned_paths: []
branch: null
shared_lease: not_required
independent_exact_head_review: PASS
material_findings:
  P0: 0
  P1: 0
  P2: 0
```

The repair closes the reproduced public activation-boundary P0. Production consumers cannot import `content::ActivationSlot`; the internal evidence activation state machine remains test-only. No parser format, content values, product limits, permanent format or production activation authority changed.

## Completed preparation - Durability topology

```yaml
lane_id: OTV2-PREP-DURABILITY-TOPOLOGY
task_id: OTV2-20260824-prep-durability-topology
worker_alias: Oteryn: prep durability topology
status: completed_blocked_on_dur03_owner_decision
risk: High
issue: 94
issue_state: completed
allocation_pr: 118
allocation_merge_sha: 58459c275ba62714741e6794b92d8935b140a37c
worker_base_sha: 58459c275ba62714741e6794b92d8935b140a37c
delivery_pr: 122
final_head_sha: 5f6d4c4440694b5edddf46f4b211e1a30955a4c6
delivery_merge_sha: c92d2d0615ae1e969003d152b4b0dfa87acfb72d
blocker_issue: 123
owned_paths: []
branch: null
shared_lease: not_required
runtime_ddl_migration_dependency_cargo_registry_workflow_authority: none
implementation_authority: none
```

Issue #94 preparation is complete: the merged packet freezes SQLx 0.9.0, a game-server-local Durability module, one game-owned migration ledger, dedicated migration execution, fail-closed schema compatibility, isolated PostgreSQL DB-E2E and the PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE boundary. `OTV2-IMPL-DURABILITY` is not released: Issue #123 owns the required Durability-specific hard-max owner decision and later serialized registry lifecycle.

## Active Wave 1 — QA

```yaml
lane_id: OTV2-IMPL-QA
task_id: OTV2-20260822-impl-qa-e2e
worker_alias: Oteryn: impl qa e2e
status: implementing_primary_path_no_pr
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
branch: agent/otv2-impl-qa-e2e-01
pr: null
checkpoint_head_sha: 58d64130cc0526001bd1c9a00a179e1c39ad6e51
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
shared_lease: not_assigned_pending_concrete_need
```

QA contains a real test-side evidence shell with focused/component validation, but no delivery PR and no real Tier 1/Tier 2 gameplay journey. Synthetic-shell evidence remains valid only for the shell; Tier 1/Tier 2 stay `NOT_EVALUATED` until the required merged production seams exist.

## Serialized shared-mutation lease

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: fnd04_allocation_pending_merge
current_owner: null
previous_owner: OTV2-IMPL-CONTENT
previous_owner_status: evidence_delivery_merged_repair_complete_production_blocked
next_candidate: OTV2-FND04-VERIFIER-CONSUMER
next_candidate_authority: exclusive_after_allocation_merge
```

CONTENT no longer needs the serialized shared paths. Issue #115 is the next candidate because its standards-conformant verifier requires direct dependency wiring; the one-writer lease activates only after the #115 allocation PR merges. QA receives no shared path implicitly.

## Next-wave readiness findings

- `PROVEN`: CONTENT repair #87 and closeout #89 are terminal; the prior P0 no longer blocks downstream preparation.
- `PROVEN`: QA remains an allocated branch-only evidence shell with no PR; Tier 1/Tier 2 remain `NOT_EVALUATED`.
- `PROVEN`: accepted DUR-02 selects PostgreSQL / one game-owned migration history but intentionally leaves the Rust driver, migration library and physical DDL to a separately authorized implementation task. Current main has no database client dependency or migration ledger.
- `PROVEN`: accepted GAME-ABILITY, GAME-INTERACTION, GAME-AI and VSL-MOVE contracts require finite resource bounds while deliberately deferring exact numeric maxima; current resource registry contains no owning lane-specific entries for those required work dimensions.
- `PROVEN`: `apps/game-server` remains fail-closed with no production gameplay listener/client-entry seam, so CLIENT cannot be released yet.
- `RECOMMENDATION`: immediate parallel preparation is QA reconciliation, DURABILITY topology/allocation, CONTENT-FORMAT-SPIKE allocation and the Wave-2 resource-limit decision packet. Ability/Interaction/AI implementation follows only after their required numeric bounds are accepted or explicitly fenced out of the executable slice.
- `RECOMMENDATION`: use one game-server-local durability module plus one game-owned migration ledger for the first implementation increment unless implementation evidence proves a dedicated immediate-consumer crate is necessary; any Cargo/workspace/migration paths still require explicit serialized allocation.

## Completed preparation - Content Format Spike

```yaml
lane_id: OTV2-CONTENT-FORMAT-SPIKE
task_id: OTV2-20260824-content-format-spike
worker_alias: Oteryn: content format spike
status: completed_released_owner_format_decision_pending
risk: Medium
issue: 95
issue_state: completed
allocation_pr: 112
allocation_merge_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
delivery_pr: 125
delivery_final_head_sha: 8c2f957f972b2dafa4bf22f239ab6a446c06b23a
delivery_merge_sha: a909f432cfa887c7e99191f18bd9cbb5ca58fc7a
results_sha256: afb871a435dd5d4333087fdb6456568c5ac01784dc25118448618d3b16da464e
dossier_sha256: 68aedf28ee8425b36969829239e0893fb65e589a3b14f4bc01d8def7c8718afd
independent_exact_head_review: PASS
independent_review_packet_sha256: 18786ac8ca57b215a9ada3e6c8d512a92dc7fd1526072c3b8be8d40940f38f4c
independent_review_response_sha256: e2c63281f316bb829c07263234b08dfe27965f5a2d0153136ed1fcea0a0b0f53
exact_head_ci:
  agent_governance_run: 32774705978
  architecture_semantic_audit_run: 32774705997
  merge_authority_audit_run: 32774706008
  merge_gate_run: 32774706140
  merge_gate_validate_job: 97585439345
  game_gate_job: 97585462248
owned_paths: []
branch: null
shared_lease: not_required
write_authority: none
permanent_format_selection: owner_decision_pending
```

The evidence spike is terminally delivered and ownership is released. It compares three bounded physical-representation candidates but does not select a permanent World Project or World Bundle format. `SPIKE_RESULT != OWNER_FORMAT_DECISION` remains binding; the separate owner action is to select, rework, or defer the permanent formats using the dossier and any later Studio/import/runtime evidence.

## Deferred allocations and concrete readiness

```yaml
OTV2-PREP-DURABILITY-TOPOLOGY:
  status: completed_released
  write_authority: none
  issue: 94
  delivery_pr: 122
  delivery_merge_sha: c92d2d0615ae1e969003d152b4b0dfa87acfb72d
  owned_paths: []
  branch: null
OTV2-NEXT-WAVE-LIMIT-DECISIONS:
  status: completed_released
  issue: 133
  delivery_pr: 140
  delivery_merge_sha: 88ad620169d6d08ebad6e49886ba1098da728480
  write_authority: none
  resource_registry_authority: none
OTV2-NEXT-WAVE-REGISTRY:
  status: completed_released
  issue: 142
  delivery_pr: 144
  delivery_merge_sha: c1020b2db62ecfa18c411bee56fa004430b28923
  write_authority: none
  resource_registry_authority: none
OTV2-FND04-VERIFIER-CONSUMER:
  status: allocation_pending_merge
  issue: 115
  write_authority: none_until_allocation_merge
  branch_after_allocation_merge: feat/fnd04-verifier-consumer-115
  shared_cargo_lease: exclusive_after_allocation_merge
  independent_exact_head_security_review: required
OTV2-IMPL-DURABILITY:
  status: blocked_dur03_hard_max_owner_decision
  write_authority: none
  topology_packet: docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md
  topology_merge_sha: c92d2d0615ae1e969003d152b4b0dfa87acfb72d
  blocker_issue: 123
  selected_stack: sqlx_0_9_0
  evidence: topology is frozen; implementation allocation waits for exact first-slice DUR-03 hard-max acceptance/registration or fail-closed exclusion
OTV2-IMPL-ABILITY:
  status: architecture_ready_resource_limits_pending_allocation
  write_authority: none
  blocker: accepted GAME-ABILITY requires concrete registered hard maxima before executable acceptance
OTV2-IMPL-INTERACTION:
  status: architecture_ready_resource_limits_pending_allocation
  write_authority: none
  blocker: accepted GAME-INTERACTION keeps numeric cascade/resource/retry bounds owner-task controlled
OTV2-IMPL-AI:
  status: architecture_ready_resource_limits_pending_allocation
  write_authority: none
  blocker: accepted GAME-AI requires concrete registered AI/path/spawn/retry hard maxima before executable acceptance
OTV2-WAVE2-RESOURCE-LIMITS:
  status: owner_decision_packet_required
  write_authority: none
  blocker: accepted Ability/Interaction/AI/Movement contracts intentionally defer numeric hard maxima; required values must be accepted/registered before executable acceptance
OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM:
  status: not_allocated
  write_authority: none
  blocker: Foundation protocol/runtime/admission semantics are merged, but apps/game-server has no production gameplay listener/client-entry transport seam
OTV2-IMPL-CLIENT:
  status: not_allocated
  blocker: merged game-server still exposes fail-closed gameplay availability with no production gameplay listener/client-entry seam
OTV2-IMPL-MOVE:
  status: not_allocated
  blocker: Interaction, Client and real QA E2E prerequisites are not integration-ready
OTV2-IMPL-COMBAT:
  status: not_allocated
  blocker: Movement and remaining generic/value prerequisites are not merged
OTV2-IMPL-CHANNEL:
  status: not_allocated
  blocker: DURABILITY not yet allocated/merged
OTV2-CONTENT-FORMAT-SPIKE:
  status: completed_released_owner_format_decision_pending
  write_authority: none
  issue: 95
  issue_state: completed
  delivery_pr: 125
  delivery_merge_sha: a909f432cfa887c7e99191f18bd9cbb5ca58fc7a
  evidence_dossier: docs/agents/evidence/OTV2-20260824-content-format-spike.md
  evidence_results: docs/agents/evidence/OTV2-20260824-content-format-spike-results.json
  permanent_format_selection: owner_decision_pending
  note: evidence-only spike is complete; no production activation or permanent-format authority follows automatically
OTV2-IMPL-ANALYTICS:
  status: not_allocated
  blocker: concrete producer event registrations do not yet exist
```

No production/protected/live-data/Platform/external-repository authority is introduced by this closeout or lease transfer.
