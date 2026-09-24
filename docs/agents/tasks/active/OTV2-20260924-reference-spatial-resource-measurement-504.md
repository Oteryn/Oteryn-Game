# OTV2-20260924-reference-spatial-resource-measurement-504

```yaml
task_id: OTV2-20260924-reference-spatial-resource-measurement-504
title: Synthetic one-destination spatial resource candidate
mode: CONTRACT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-reference-spatial-resource-measurement-504
issue: 504
pr: 853
base_sha: 663052f3139e2e5fff98f904fc1e8a02fd205115
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated #162 sole writer
created_at: 2026-09-24T00:00:00Z
updated_at: 2026-09-24T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/reference-spatial-resource-profile/spatial_resource_profile.py
  - tools/reference-spatial-resource-profile/spatial_resource_profile_self_test.py
  - docs/architecture/OTERYN_REFERENCE_SPATIAL_RESOURCE_PROFILE_V1.md
  - docs/agents/tasks/active/OTV2-20260924-reference-spatial-resource-measurement-504.md
  - docs/agents/evidence/OTV2-20260924-reference-spatial-resource-measurement-504.json
public_contracts: []
depends_on: ["#483 exact field evidence for later production admission"]
blocks: ["#504 numeric acceptance review"]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A reproducible synthetic server/client spatial sidecar and measured finite
one-destination envelope are available for independent #504 review. The
resource profile is a proposal; no production or registry change is made.

## Architecture and source of truth

- **PROVEN:** #504 comment 5818657159 accepts only carrier semantics and leaves numeric ceilings open. #162 comment 5818682264 allocates the five paths and branch.
- **PROVEN:** D1 keeps stable placement identity, ordered placement relation and separate collision/presentation footprints.
- **DERIVED:** one-cell candidate measurements in the evidence JSON are outputs of the Python encoder and checked envelope arithmetic.
- **UNKNOWN:** exact July-28 static collision, selected Reference generation cardinality, actual presentation/placement/footprint records.
- Bound protected META 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this tool makes no production mutation or runtime authority decision; it explicitly refuses real Reference admission.

## Acceptance criteria

- [x] Binary server/client candidate has versioned exact key, view separation, fail-closed mismatches and unsupported D1 fields.
- [x] One-cell dimensional and encoded byte envelope with reproducible evidence and max/max+1/overflow tests.
- [ ] Independent exact-head architecture review and owner #504 disposition; neither is self-granted.

## Excluded scope

No Content/Movement/Item production implementation, target claim promotion,
registry mutation, Item v1–v4 reinterpretation, bootstrap profile widening,
real map dimensions or RL-03 admission.

## Implementation / findings

Separate 64-byte header, capped canonical JSON binding, 56-byte single-entry
index and server 33/client 1-byte records. The capacity is intentionally one
destination per generation; a two-cell generation is refused even if a lookup
would touch only one. A broader profile requires new evidence and versioning.
No placements/footprints are accepted or silently discarded.

## Validation

### Focused

- `python3 tools/reference-spatial-resource-profile/spatial_resource_profile_self_test.py`: four test groups, pass in local scratch against these candidate bytes.
- `python3 tools/reference-spatial-resource-profile/spatial_resource_profile.py --output docs/agents/evidence/OTV2-20260924-reference-spatial-resource-measurement-504.json`: deterministic synthetic output.

### Component/integration

- NOT_APPLICABLE: no production consumer changed.

### E2E

- NOT_APPLICABLE: #483 target evidence and actual linker absent.

### Exact-head CI

- Pending PR creation and frozen-head workflow.

## Self-review

- Exact head: pending final API write.
- Method: implementing writer reviewed owned-path delta, numeric arithmetic, mismatch handling and D1 omissions.
- Material findings: none before freeze.
- Verdict: pending exact remote readback.

## Independent review

- Required: YES, owner numeric-resource proposal and candidate codec.
- Exact head/method/findings/verdict: pending separate reviewer.

## PR and closeout

- Changed-file review, unresolved review threads, related PRs, protected queue,
  merge and ownership release: pending coordinator after independent review.

## Context checkpoint

```yaml
last_progress: candidate tool and documentation prepared
status: investigating
branch: agent/otv2-reference-spatial-resource-measurement-504
head_sha: null
pr: 853
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
next_action: independent exact-head review after draft PR freeze
```
