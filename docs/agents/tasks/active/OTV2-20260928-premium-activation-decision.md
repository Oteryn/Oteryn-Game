# OTV2-20260928-premium-activation-decision

```yaml
task_id: OTV2-20260928-premium-activation-decision
title: "Premium activation decision (D69-D75)"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 117a89985c4f4d8af61fc018b542c745fc334d33
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-premium-activation-decision.md
  - docs/agents/tasks/active/OTV2-20260928-character-appearance-decision.md   # archive move after #1115
  - docs/agents/tasks/archive/OTV2-20260928-character-appearance-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the explicit, product-specific Premium activation decision for the Reference
profile, from owner decisions D69-D75 (#162 5871244356 and 5871433126):

- Premium is derived only from Platform's entitlement through the `PROD-ENTITLEMENTS-01` consumer
  fence, with per-surface policies;
- promotion is durable, and effective only while Premium is current, evaluated at login;
- soul 200 for effectively promoted characters only; soul above the maximum is kept until spent;
- Premium blessings are Premium-gated to buy and kept until death;
- Premium areas: entry refused at once, relocation at login;
- Premium spells are checked at cast time.

No runtime, registry, migration, protocol or Platform change. The Game children PREM-1 to PREM-5
need #162 allocations. The Platform producer (PREM-P) needs a Platform lane and separate
cross-repository authority.

## Architecture and source of truth

- `PROVEN`:
  - `PROD-ENTITLEMENTS-01` consumer contract §2, §6, §9-§12;
  - architecture README :165-167;
  - SPELL-D5;
  - GAME-CHAR-01 Stage B DELTA_02 E1;
  - the spell schema `requirements.premium`;
  - Global sources in #162 5871433126: tibia.com manual 5.1.4 and 5.1.6, the world section,
    Premium features and support 83; TibiaWiki Fandom and BR.
- `UNKNOWN`: Global mid-session expiry and soul clamping (decided as policy by D73 and D74); the
  Premium region map (content).

## High-risk authority/recovery qualification

This applies at design level. The decision defines which evidence grants a Premium benefit, and how
expiry, reconnect and replay behave. The negative cases bind PREM-1 to PREM-4.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - Q1 Game derives Premium only from the Platform entitlement through the consumer fence; no Game record grants or extends Premium
  - Q2 an expired Premium never returns through reconnect, recovery or a replayed producer decision
  - Q3 losing Premium never blocks base login and never forces a logout
  - Q4 the durable promotion never changes because Premium lapsed; only its effect is suspended
  - Q5 Premium spells and Premium-area entry are refused once Premium is not current, checked at use
consumer_boundaries:
  - fresh admission
  - reconnect and recovery
  - running session at use (spell cast, area entry)
  - login relocation out of a Premium area
mutation_operators:
  applicable:
    - stale generation (stale or replayed producer decision, old fence high water)
    - mismatched identity or binding (another account's entitlement, wrong product or version)
    - time (expiry during a session, clock skew, finite authority interval)
    - replay and concurrency (duplicate grant evidence, reconnect racing expiry)
  considered_not_applicable:
    - "provenance substitution of durable value: no item or currency is written"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - Q1 a Game-side flag or cached value without current producer authority -> no benefit
  - Q2 a reconnect after expiry, or a replayed older active decision -> benefit not restored
  - Q3 an expired account logs in -> login succeeds without benefits; no forced logout mid-session
  - Q4 lapse then renewal -> promotion suspended at the next login, then restored at the next login after renewal, with no fee
  - Q5 a Premium spell cast or a Premium-area entry after expiry -> refused at use
positive_cases_required_of_implementation:
  - an operator grant makes the account Premium; promotion, the Premium blessings, soul 200 (promoted) and Premium areas and spells work
independent_current_fact_sources:
  - the consumer fence rows
  - the producer authority interval
  - character session fence rows
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "PROD-ENTITLEMENTS-01 §10-§12 surfaces are the reference"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "producer evidence through the consumer fence only"
  fenced_durable_writes: "consumer fence high water and promotion state"
  restart_retry_replay_concurrency_pg_reload: "covered by Q2 and Q4"
  evidence:
    - docs/architecture/PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred: []
```

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Runtime code, schema, protocol, the Platform producer, payment, the Store, houses and VIP.

## Validation

- `python3 tools/agents/validate_governance.py`: pending.
- `python3 tools/repository/validate_repository_policy.py`: pending.

## Context checkpoint

```yaml
last_progress: decision drafted
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: validate, commit, open PR, freeze, one exact-head review
```
