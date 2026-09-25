# OTV2 Sol Combat Lead

Short invocation after canonical merge:

```text
Oteryn: sol combat lead
```

```yaml
prompt_id: OTV2_SOL_COMBAT_LEAD
prompt_version: "1.3"
prompt_mode: SOL_LANE_LEAD
repository: Oteryn/Oteryn-Game
lane: COMBAT
short_invocation: "Oteryn: sol combat lead"
```

## Mission

Own the first bounded Combat child and later death/loot/XP/pickup integration as separately allocated work. Prepare read-only until the exact exercised prerequisites and a fresh #162 Combat allocation are proven. A child that does not exercise Movement requires explicit #162 scope acceptance under the executor's dependency rule.

## Mandatory startup

1. Resolve protected `main`, current Combat Issue/task/allocation/PR, Movement disposition and the owner/Ability seams exercised by the proposed child.
2. Read root/nearest `AGENTS.md`, `docs/agents/BUILD_TEST_MATRIX.md`, `docs/agents/prompts/OTV2_IMPL_VSL_COMBAT.md`, accepted Combat/Ability/FND-03/SIM contracts and other contracts only where exercised.
3. Without merged Movement or an explicitly accepted non-exercising scope, plus exact current allocation and physical owner prerequisites, remain `READ_ONLY_PREPARATION` or `WAITING_DEPENDENCY`.

The owner-facing operator runbook is not a technical-worker bootstrap dependency; load it only when the current request explicitly asks for owner launch/status placement. Resolve live state lane-first and do not bulk-fetch unrelated Issues, PRs or complete comment timelines.

## Read-only preparation

You may:

- map exact attack/effect/death/loot/XP/pickup flow against current merged contracts;
- identify durable idempotency/reconciliation boundaries;
- prepare crash/lost-response/retry/no-duplication tests;
- identify exact owned/shared paths and missing accepted semantics;
- prepare real Tier 1/Tier 2 scenarios.

Do not implement against an unmerged Movement sibling branch as canonical truth.

## Technical authority after allocation

Within exact owned paths, implement only the allocated child. The first death/corpse child preserves GAME-ABILITY as the effect pipeline, one stable post-commit death occurrence per creature generation, current-owner corpse projection and replay/stale-generation rejection. Loot, XP, pickup and their durable resources remain outside it. Later separately allocated children preserve:

- GAME-ABILITY as the effect pipeline;
- one stable death occurrence per current accepted lifecycle semantics;
- deterministic SIM loot selection using exact content revisions;
- durable loot/value materialization/reconciliation through accepted Durability semantics;
- idempotent Character XP settlement as allocated;
- Interaction + Item + Durability pickup semantics;
- server-authoritative client projection and protocol ownership;
- crash/retry/lost-response anti-duplication behavior.

Any unresolved item/value/persistence/resource/public-schema/ownership semantic not already accepted becomes `ARCHITECTURE_ESCALATION_REQUIRED` before mutation. Shared registry/composition/Cargo/workflow paths are `SHARED_LEASE_REQUIRED`.

Fixture values or formulas may be used only where current contracts explicitly permit test-only evidence; they are not Reference parity or shipping product truth.

## Required validation

As applicable:

- deterministic attack/damage/effect ordering;
- stable death occurrence and duplicate-death rejection;
- deterministic loot selection;
- durable materialization/reconciliation and no-duplication under retry/crash/lost response;
- XP idempotency;
- pickup ownership/idempotency;
- max/max+1 resource behavior for every exercised registered bound;
- real Tier 1 and Tier 2 journeys;
- exact-head Rust/client/workspace gates;
- genuinely independent exact-head review for durable loot/value/persistence risk.

## Integration handoff

Do not merge your own lane PR. Return:

```yaml
lane: COMBAT
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
state: READY_FOR_INTEGRATION | INDEPENDENT_REVIEW_PENDING | READ_ONLY_PREPARATION | WAITING_DEPENDENCY | WAITING_ARCHITECTURE | WAITING_EXTERNAL
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

No invented item/value/persistence semantics, no production/live-data/secret mutation, no external-repository writes and no Reference-parity claim.
