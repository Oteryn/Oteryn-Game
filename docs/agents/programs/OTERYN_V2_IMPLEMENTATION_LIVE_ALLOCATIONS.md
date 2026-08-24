# Oteryn v2 Implementation Live Allocations

- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Coordinator task: `OTV2-20260818-implementation-coordinator`
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
- Foundation closeout merge / reconciliation base: `1f69677b40851551953caf853c08b37ce7b29c68`
- State: `WAVE1_DOMAIN_LEASE_TRANSFER_PENDING_MERGE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance, active task/PR/CI state and merged `main` outrank stale coordination prose.

This revision is coordination-only until its own PR lawfully merges. It does **not** authorize DOMAIN to mutate serialized shared paths merely because the candidate branch exists. After this revision merges to `main`, the serialized shared-path lease transfers from the completed/released FOUNDATION lane to DOMAIN.

Unmerged sibling output is never an implicit dependency. Stable registries/contracts, `.github/workflows/**`, architecture policy/tooling and new workspace/crate topology remain unallocated unless this record or a later merged coordinator allocation explicitly grants them.

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
independent_review_terminal: passed
owned_paths: []
branch: null
shared_lease: released
```

Foundation FND-02/FND-03/FND-04 primitives are merged and lifecycle-closed. The merged composition root still reports gameplay unavailable and intentionally has no production gameplay listener/client-entry side effects; Foundation completion therefore does not by itself satisfy CLIENT or gameplay-VSL integration readiness.

## Active Wave 1 — Domain

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: implementation_ready_for_shared_composition_after_coordinator_merge
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
branch: agent/otv2-impl-domain-core-01
pr: 56
observed_head_sha: 674d1ccd637f3565c25750e5d5fe6c56df6fde32
relative_to_reconciliation_base:
  ahead_by: 5
  behind_by: 8
owned_paths:
  - apps/game-server/src/domain/**
  - docs/agents/tasks/active/OTV2-20260822-impl-domain-core.md
shared_lease_after_this_record_merges: active
```

DOMAIN has a substantial Character/Item semantic core and focused/workspace validation evidence, but PR #56 remains Draft and is not integration-ready while its branch is behind current `main` and the shared composition transfer is unmerged. After this coordinator revision merges, DOMAIN may reconcile current `main` and make only the minimum contract-valid shared composition change required to compile its real module through `apps/game-server`.

## Active Wave 1 — Content

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: implementing_primary_path_waiting_shared_lease
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
branch: agent/otv2-impl-vsl-content-01
pr: 58
observed_head_sha: ec68df7a461a011a6480898c9a6d9ee60703189e
relative_to_reconciliation_base:
  ahead_by: 7
  behind_by: 8
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
shared_lease: waiting_for_domain
registered_production_vsl_limits: not_found
```

CONTENT has a bounded semantic graph/compiler/loader evidence seam, but PR #58 remains Draft. DOMAIN holds the next serialized composition turn after this record merges. CONTENT must remain on explicit finite non-production evidence profiles until accepted DUR-04/VSL hard maxima exist in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`; it receives no authority here to choose a permanent World Project/Bundle encoding or production activation policy.

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
relative_to_reconciliation_base:
  ahead_by: 3
  behind_by: 11
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
shared_lease_for_current_shell: not_required
```

QA contains a real test-side evidence shell with focused/component validation, but no delivery PR and no real Tier 1/Tier 2 Foundation journey. Its historical synthetic-shell evidence remains valid only for the shell; Tier 1/Tier 2 stay `NOT_EVALUATED` until production transport/admission/persistence and native-client boundaries actually exist. QA may continue inside its non-overlapping primary paths while DOMAIN owns serialized shared composition.

## Serialized shared-mutation lease

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: transfer_pending_this_record_merge
previous_owner: OTV2-IMPL-FOUNDATION
previous_owner_status: completed_and_released
next_owner_after_merge: OTV2-IMPL-DOMAIN
lease_order_after_merge:
  - OTV2-IMPL-DOMAIN
  - OTV2-IMPL-CONTENT
  - OTV2-IMPL-QA
```

DOMAIN may not mutate these shared paths before this allocation revision lawfully merges to `main`. After DOMAIN itself reaches terminal merge/archive and releases the lease, the coordinator will decide whether CONTENT needs the next shared turn; QA receives a shared turn only if a concrete test-composition need is proven.

## Deferred allocations and concrete blockers

```yaml
OTV2-IMPL-DURABILITY:
  status: not_allocated
  blocker: DOMAIN concrete consumer/composition is not yet merged
OTV2-IMPL-ABILITY:
  status: not_allocated
  blocker: DOMAIN and CONTENT are not yet merged/integration-ready
OTV2-IMPL-INTERACTION:
  status: not_allocated
  blocker: DOMAIN and CONTENT are not yet merged/integration-ready
OTV2-IMPL-AI:
  status: not_allocated
  blocker: DOMAIN and CONTENT are not yet merged/integration-ready
OTV2-IMPL-CLIENT:
  status: not_allocated
  blocker: current merged game-server composition is explicitly GameplayAvailability::UnavailableBootstrap and no production gameplay listener/client-entry seam is merged
OTV2-IMPL-MOVE:
  status: not_allocated
  blocker: Interaction, Client and real QA E2E prerequisites are not integration-ready
OTV2-IMPL-COMBAT:
  status: not_allocated
  blocker: Movement and the remaining generic/value prerequisites are not merged
OTV2-IMPL-CHANNEL:
  status: not_allocated
  blocker: DOMAIN and DURABILITY are not merged
OTV2-CONTENT-FORMAT-SPIKE:
  status: not_allocated
  blocker: CONTENT semantic/compiler seam is not merged; permanent-format decision remains separately owner-gated
OTV2-IMPL-ANALYTICS:
  status: not_allocated
  blocker: concrete producer event registrations do not yet exist
```

No production/protected/live-data/Platform/external-repository authority is introduced by this reconciliation.
