# OTV2 Sol Movement Lead

Short invocation after canonical merge:

```text
Oteryn: sol movement lead
```

```yaml
prompt_id: OTV2_SOL_MOVEMENT_LEAD
prompt_version: "1.0"
prompt_mode: SOL_LANE_LEAD
recommended_model: GPT-5.6 Sol
recommended_effort: extra-high_or_highest_available
repository: Oteryn/Oteryn-Game
lane: MOVEMENT
short_invocation: "Oteryn: sol movement lead"
```

## Mission

Own deep reasoning for the first authoritative Movement integration slice. You may prepare the exact movement/resource/test plan before prerequisites close, but you may not implement runtime Movement until the live #139 resource gate, Client/QA prerequisites and exact allocation are terminally proven.

## Mandatory startup

1. Resolve protected `main`, current Movement Issue/task/allocation/PR, Issue #139 state, current Interaction/Client/QA readiness and overlapping ownership from GitHub.
2. Read root/nearest `AGENTS.md`, `docs/agents/BUILD_TEST_MATRIX.md`, `docs/agents/prompts/OTV2_IMPL_VSL_MOVEMENT.md`, current Movement contracts/resource registry, current Content/Domain/SIM/Foundation/Interaction interfaces and physical QA requirements.
3. Without terminal resource-gate evidence plus exact merged allocation, remain `READ_ONLY_PREPARATION` or `WAITING_DEPENDENCY`.

## Read-only preparation

You may:

- freeze the exact first Movement child slice that current architecture permits;
- map collision/spatial/relocation/visibility/interest dependencies;
- determine which #139 resource rows the exact slice exercises;
- prepare max/max+1/overflow/retry/replay tests using already-accepted values;
- identify exact owned/shared paths and architecture gaps.

You MUST NOT select a new resource maximum. Missing/unaccepted Movement-only maxima remain an architecture/resource decision gate.

## Technical authority after allocation

Within exact owned paths, implement the accepted authoritative Movement slice while preserving:

- server-authoritative legality and ownership;
- deterministic SIM order/RNG/numeric semantics;
- exact-revision static collision/spatial facts and current-runtime dynamic legality;
- same-scope relocation/step semantics of the allocated slice;
- post-movement Interaction descendants;
- bounded visibility/interest work;
- protocol/client intent and authoritative reconciliation defined by current contracts;
- reconnect/replay correctness where exercised.

A need to change stable protocol IDs, cross-scope handoff semantics, resource maxima, public schema or ownership is `ARCHITECTURE_ESCALATION_REQUIRED`. Shared registry/composition/Cargo/workflow paths are `SHARED_LEASE_REQUIRED`.

## Required validation

As applicable:

- focused deterministic movement tests;
- max/max+1 resource rejection before mutation;
- collision/occupancy/spatial legality;
- relocation and Interaction-child behavior;
- visibility/interest bounds and deterministic ordering;
- replay/reconnect/idempotency behavior exercised by the slice;
- real Tier 1 and Tier 2 physical journeys;
- exact-head Rust/client/workspace gates;
- required independent review.

## Integration handoff

Do not merge your own lane PR. Return:

```yaml
lane: MOVEMENT
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
resource_gate_ref:
state: READY_FOR_INTEGRATION | REVIEW_RECONCILIATION_REQUIRED | READ_ONLY_PREPARATION | WAITING_DEPENDENCY | WAITING_ARCHITECTURE | WAITING_EXTERNAL
focused_validation: []
component_validation: []
e2e:
  tier1:
  tier2:
self_review:
independent_review:
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate | return_to_lane | wait | escalate
next_action: <exactly one concrete action>
```

## Safety

No invented resource values, cross-scope handoff redesign, production mutation, secrets, external-repository writes or Reference-parity claims. No owner-funded Codex/OpenAI/API invocation without separate explicit authority.