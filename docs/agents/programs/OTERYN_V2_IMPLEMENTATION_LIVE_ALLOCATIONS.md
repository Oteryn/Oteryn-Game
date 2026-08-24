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
- Foundation closeout merge: `1f69677b40851551953caf853c08b37ce7b29c68`
- Coordinator DOMAIN lease-transfer PR: `#76`
- Coordinator DOMAIN lease-transfer merge: `6945e962035bac83d1f19b00984df5b82719ebb9`
- Coordinator DOMAIN lease-active PR: `#78`
- Coordinator DOMAIN lease-active merge: `3988e4263569d6c90bf6e794688b810073ad5184`
- Foundation post-merge audit PR: `#81`
- Foundation post-merge audit merge: `55e30e23c3d5775ce760c6b210ea77f152b359ae`
- Domain delivery PR: `#56`
- Domain delivery merge: `0facd7f89edc1b0685e67c5531839e8e6f04c466`
- State: `WAVE1_CONTENT_LEASE_TRANSFER_PENDING_CLOSEOUT_MERGE`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance, active task/PR/CI state and merged `main` outrank stale coordination prose.

DOMAIN product delivery is merged, but this closeout/lease-transfer revision is not authoritative until its own PR merges to `main`. Before that merge, CONTENT still may not mutate serialized shared paths. After lawful closeout merge, DOMAIN lifecycle ownership is released and the established next shared turn transfers to CONTENT.

Unmerged sibling output is never an implicit dependency. Stable registries/contracts, `.github/workflows/**`, architecture policy/tooling and new workspace/crate topology remain unallocated unless a later merged coordinator allocation explicitly grants them.

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

## Domain delivery — lifecycle closeout pending this PR merge

```yaml
lane_id: OTV2-IMPL-DOMAIN
task_id: OTV2-20260822-impl-domain-core
worker_alias: Oteryn: impl domain core
status: completed_pending_archive_merge
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
final_head_sha: a76c999a2b03c4271fda9b4395cc3d76c346987b
delivery_pr: 56
delivery_merge_sha: 0facd7f89edc1b0685e67c5531839e8e6f04c466
issue: 55
issue_state: completed
source_branch_present: false
owned_paths_after_this_record_merges: []
shared_lease_after_this_record_merges: released
```

DOMAIN is merged into the production game-server crate. Its exact-head tests/Clippy/security gate, whole-diff self-review and genuinely independent exact-head review are clean. Runtime/Tier E2E remains `NOT_EVALUATED` because DOMAIN introduced no production gameplay listener/client journey.

## Active Wave 1 — Content

```yaml
lane_id: OTV2-IMPL-CONTENT
task_id: OTV2-20260822-impl-vsl-content
worker_alias: Oteryn: impl vsl content
status: shared_composition_ready_after_closeout_merge
risk: High
allocation_pr: 45
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
exact_base_pr: 46
worker_base_sha: 33cec30b8075c73290d7d76e9f59df4701771650
branch: agent/otv2-impl-vsl-content-01
pr: 58
observed_head_sha: ec68df7a461a011a6480898c9a6d9ee60703189e
relative_to_domain_merged_main:
  ahead_by: 7
  behind_by: 12
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
shared_lease_after_this_record_merges: active
registered_production_vsl_limits: not_found
production_activation: forbidden
permanent_format_selection: forbidden
```

After this closeout revision merges, CONTENT may reconcile current `main` and use the serialized shared paths only for the minimum evidence-only/fail-closed composition of its existing semantic/compiler/loader seam. Missing accepted DUR-04/VSL hard maxima continue to block production VSL activation. This lease does not authorize choosing permanent World Project/Bundle encoding, mutating registries/contracts/workflows, broad content import or claiming Reference parity.

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
shared_lease: waiting_for_content_if_needed
```

QA contains a real test-side evidence shell with focused/component validation, but no delivery PR and no real Tier 1/Tier 2 gameplay journey. Synthetic-shell evidence remains valid only for the shell; Tier 1/Tier 2 stay `NOT_EVALUATED` until the required merged production seams exist.

## Serialized shared-mutation lease candidate

```yaml
shared_paths:
  - apps/game-server/src/lib.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml
lease_state: transfer_pending_closeout_merge
current_owner_on_main: OTV2-IMPL-DOMAIN
current_owner_after_this_record_merges: OTV2-IMPL-CONTENT
previous_owner_after_merge: OTV2-IMPL-DOMAIN
previous_owner_status_after_merge: completed_and_released
remaining_order_after_content:
  - OTV2-IMPL-QA
```

The lease transfer is effective only after this closeout PR lawfully merges. CONTENT receives no stable-ID/registry/contract/workflow/new-crate or production authority through this transfer.

## Deferred allocations and concrete readiness

```yaml
OTV2-IMPL-DURABILITY:
  status: dependency_ready_pending_new_allocation
  write_authority: none
  evidence: FOUNDATION and DOMAIN concrete merged seams now exist
OTV2-IMPL-ABILITY:
  status: not_allocated
  blocker: CONTENT not yet merged/integration-ready
OTV2-IMPL-INTERACTION:
  status: not_allocated
  blocker: CONTENT not yet merged/integration-ready
OTV2-IMPL-AI:
  status: not_allocated
  blocker: CONTENT not yet merged/integration-ready
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
  status: not_allocated
  blocker: CONTENT semantic/compiler seam not yet merged; permanent-format decision remains separately owner-gated
OTV2-IMPL-ANALYTICS:
  status: not_allocated
  blocker: concrete producer event registrations do not yet exist
```

No production/protected/live-data/Platform/external-repository authority is introduced by this closeout or lease transfer.
