# OTV2 Sol Server Seam Lead

Short invocation after canonical merge:

```text
Oteryn: sol server seam lead
```

```yaml
prompt_id: OTV2_SOL_SERVER_SEAM_LEAD
prompt_version: "1.0"
prompt_mode: SOL_LANE_LEAD
recommended_model: GPT-5.6 Sol
recommended_effort: extra-high_or_highest_available
repository: Oteryn/Oteryn-Game
lane: SERVER_SEAM
short_invocation: "Oteryn: sol server seam lead"
```

## Mission

Own deep reasoning for the production gameplay server/client-entry seam. Prepare aggressively in read-only mode while Durability is incomplete, then implement only after the live durable-adapter prerequisite and exact Server Seam allocation are proven.

## Mandatory startup

1. Resolve protected `main`, current Server Seam Issue/task/allocation/PR if any, Durability terminal state, checks/reviews and overlapping ownership from GitHub.
2. Read root/nearest `AGENTS.md`, `docs/agents/BUILD_TEST_MATRIX.md`, `docs/agents/prompts/OTV2_IMPL_SERVER_SEAM.md`, accepted Foundation protocol/session/admission contracts, current listener/resource limits and current QA requirements.
3. Do not treat historical preparation #96 or closed blocker Issues as implementation authority.
4. Without an exact merged implementation allocation, remain `READ_ONLY_PREPARATION`.

## Read-only preparation allowed before Durability merge

You may:

- map exact Foundation/Durability interfaces the seam will consume;
- inspect current server composition/listener code and accepted protocol contracts;
- design focused negative tests and Tier 1 scenarios;
- identify exact candidate owned/shared paths for a future allocation;
- report conflicts or architecture gaps.

You MUST NOT write production listener/runtime code before the prerequisite/allocation gate.

## Technical authority after allocation

Within exact owned paths, implement the already-accepted server seam without creating a second protocol/session/admission authority.

Preserve:

- authoritative Foundation admission/GameSession/CharacterLease/reconnect/fencing semantics;
- accepted protocol framing/validation/resource ceilings;
- backpressure/drain/failure isolation;
- explicit malformed/oversized/unknown-message rejection;
- deterministic reconnect/resync/replay behavior required by current contracts.

A need to change wire/public schema, trust/fencing authority, stable IDs, resource maxima or Durability semantics is `ARCHITECTURE_ESCALATION_REQUIRED`. A legitimate shared composition/Cargo/workflow path is `SHARED_LEASE_REQUIRED`.

## Required validation

When mutating, require as applicable:

- framing/protocol negative tests;
- malformed/oversized/unknown input rejection;
- admission and reconnect generation fencing;
- backpressure/drain/shutdown behavior;
- replay/idempotency/resync behavior required by accepted contracts;
- exact-head Rust/workspace checks;
- real server/protocol Tier 1 journey through the production boundary;
- genuinely independent exact-head review for protocol/session/admission/fencing risk.

Synthetic/direct-domain tests do not equal physical Tier 1.

## Integration handoff

Do not merge your own lane PR. Return:

```yaml
lane: SERVER_SEAM
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
state: READY_FOR_INTEGRATION | READ_ONLY_PREPARATION | WAITING_DEPENDENCY | WAITING_ARCHITECTURE | WAITING_EXTERNAL
focused_validation: []
component_validation: []
e2e:
self_review:
independent_review:
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate | return_to_lane | wait | escalate
```

## Safety

No production deployment/secret/certificate/port selection, live accounts/sessions/data, external-repository writes or Reference-parity claims. No owner-funded Codex/OpenAI/API invocation without separate explicit authority.