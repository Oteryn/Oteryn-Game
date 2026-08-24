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
- Content evidence delivery PR: `#58`
- Content evidence delivery merge: `8f99f25d0b1b3472d40504cd54b463cf752ebe7a`
- State: `WAVE1_CONTENT_EVIDENCE_MERGED_PRODUCTION_BLOCKED`

## Authority rule

This record is the live coordinator allocation required by `OTV2_IMPLEMENTATION_COORDINATOR.md`. Root governance, active task/PR/CI state and merged `main` outrank stale coordination prose.

PR #82 merged and transferred the serialized shared composition lease to CONTENT. CONTENT used that authority only for minimum evidence-only composition in PR #58, which has now merged. Production VSL activation, permanent format selection and registry/contract mutation remain unauthorized.

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
status: evidence_delivery_merged_production_blocked
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
future_write_authority: requires_new_coordinator_allocation
```

The bounded non-production VSL evidence seam is merged and composed through `apps/game-server`. Exact-head `game-gate`, whole-diff self-review and genuinely independent exact-head review were clean. `OrdinaryRelease` and gameplay availability remain fail-closed. Issue #54 stays open only for production acceptance: accepted DUR-04/VSL hard maxima and production activation authority are still absent and are not inferred by this programme.

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
lease_state: released_unassigned
current_owner: null
previous_owner: OTV2-IMPL-CONTENT
previous_owner_status: evidence_delivery_merged_production_blocked
next_candidate: OTV2-IMPL-QA
next_candidate_authority: not_granted_pending_concrete_need
```

CONTENT no longer needs the serialized shared paths for its merged evidence seam. QA does not receive them implicitly; a later coordinator action must prove a concrete shared-path need before granting another one-writer turn.

## Deferred allocations and concrete readiness

```yaml
OTV2-IMPL-DURABILITY:
  status: dependency_ready_pending_new_allocation
  write_authority: none
  evidence: FOUNDATION and DOMAIN concrete merged seams now exist
OTV2-IMPL-ABILITY:
  status: dependency_ready_pending_new_allocation
  write_authority: none
OTV2-IMPL-INTERACTION:
  status: dependency_ready_pending_new_allocation
  write_authority: none
OTV2-IMPL-AI:
  status: dependency_ready_pending_new_allocation
  write_authority: none
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
  status: dependency_ready_pending_new_allocation
  write_authority: none
  note: evidence-only spike; permanent-format decision remains separately owner-gated
OTV2-IMPL-ANALYTICS:
  status: not_allocated
  blocker: concrete producer event registrations do not yet exist
```

No production/protected/live-data/Platform/external-repository authority is introduced by this closeout or lease transfer.
