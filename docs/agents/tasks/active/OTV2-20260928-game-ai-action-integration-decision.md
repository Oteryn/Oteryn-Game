# OTV2-20260928-game-ai-action-integration-decision

```yaml
task_id: OTV2-20260928-game-ai-action-integration-decision
title: "GAME-AI-01 AI Action Integration, first creature slice (D53-D57)"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 356673fed04758e92fc0dae430238c0e41078b0b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-game-ai-action-integration-decision.md
  - docs/agents/tasks/active/OTV2-20260928-quest-707-account-scope-decision.md   # archive move after #1102
  - docs/agents/tasks/archive/OTV2-20260928-quest-707-account-scope-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records owner decisions D53-D57 (#162 comment 5870406825), taken on 2026-09-28. They
define the first creature slice of GAME-AI-01 AI Action Integration and extend the 2026-08-25
bootstrap slice:

- a Channel owner timer lane under FND-03 §10;
- spawn realization and respawn, with a 16-spawn by 4-creature envelope;
- creature Movement adoption through the Movement owner;
- the typed AI to Ability intent boundary, which closes GAME-AI-XD-01 for this slice;
- the player-HP floor at 1;
- the resource rows.

No runtime, registry, migration or protocol change. Children AI-1 to AI-4 need their own #162
allocations.

## Architecture and source of truth

- `PROVEN`:
  - the bootstrap slice decision §3, §5, §6 and §10;
  - GAME-AI-01 candidate §3, §6-§8, §10 and §21;
  - the gate acceptance (`GLOBAL_ARCHITECTURE_DECISION_REGISTER.md:49`);
  - FND-03 §8.4, §9, §10 and §11;
  - VSL-COMBAT-01 §6, §7, §15, §16 and §24.2;
  - SPELL-D2 and D52;
  - code: `runtime_actor_carrier.rs` :1057-1083, :1159, :1310-1317, :1423-1480, :1456-1464; `movement.rs` :1-5, :184-258; `ability/intent.rs` :4-9, :31, :52, :120-126; `exact_actor_resolution.rs` :8-35;
  - `native_entry_room.json` (rat and spawn).
- `DERIVED`: 64 creatures per scope, 128 pending AI timers per scope and 64 think resolutions per
  owner cycle, all from D57.
- `UNKNOWN`: Reference rat values (routed to content).

## High-risk authority/recovery qualification

This applies at design level. The decision defines which evidence lets an AI-originated action
mutate Movement or vitals, and how timers and respawn behave across generation changes. The
negative cases bind the AI-1 to AI-4 allocations.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - A1 AI never mutates position, occupancy, vitals, effects or cooldowns directly; only the Movement and Ability owners commit
  - A2 a timer mutates only while its scope ownership generation and target actor-local generation are current
  - A3 an AI intent's issuer is the creature's current ExactActorRef; a stale or recycled issuer is rejected
  - A4 one think occurrence applies at most one action, and a retry never applies it again or redraws RNG
  - A5 live plus pending creatures of a spawn never exceed its population; a respawn always uses a new actor-local generation
  - A6 creature damage never takes a player's HP below 1
consumer_boundaries:
  - Channel owner timer input
  - Movement owner step for a creature
  - Ability commit of an AI-originated intent
  - spawn realization at activation and on respawn
mutation_operators:
  applicable:
    - stale generation (ownership generation moved, actor slot recycled, timer from an old generation)
    - mismatched identity or binding (issuer string instead of ExactActorRef, other creature's intent)
    - replay and concurrency (duplicate think occurrence, two respawn timers for one cell)
    - time (equal deadlines, missed think timers after a scheduler delay)
  considered_not_applicable:
    - "provenance substitution of durable value: no durable value, loot or XP is written by this slice"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - A1 an AI resolution that tries to write a position or HP directly -> no such code path; the owner-only API is the sole route
  - A2 a think or respawn timer after the ownership or actor generation changed -> dropped, nothing mutated
  - A3 an intent carrying a recycled creature's old ExactActorRef -> rejected before Ability legality
  - A4 the same think occurrence resubmitted -> the first result, no second bite, no RNG redraw
  - A5 a respawn due while the population is already full, or a second timer for the same cell -> rejected, no second actor
  - A6 a bite larger than the remaining HP -> HP ends at 1; the committed effect records the clamped amount
positive_cases_required_of_implementation:
  - activation spawns the rat; it wanders, perceives the player, chases one revalidated step per think, and bites when adjacent
  - after the rat dies, it respawns on its cell with a new actor-local generation after the delay
independent_current_fact_sources:
  - game_runtime_scope_assignments plus node incarnation proof
  - the Channel owner's actor slots and generations
  - the content activation pin (creature and spawn definitions)
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "player Movement step and player Ability intent are the reference paths; no second path"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "owner-only commits; no reconciliation path"
  fenced_durable_writes: "none in this slice"
  restart_retry_replay_concurrency_pg_reload: "restart re-realizes EphemeralScopeReset spawns; covered by A2, A4 and A5"
  evidence:
    - apps/game-server/src/foundation/runtime_actor_carrier.rs
    - apps/game-server/src/movement.rs
    - apps/game-server/src/ability/intent.rs
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

- Runtime code, the registry, migrations, protocol and client.
- Reference rat values, player death, threat memory, leash, summons and scripts.

## Validation

- `python3 tools/agents/validate_governance.py`: pending.
- `python3 tools/repository/validate_repository_policy.py`: pending.

## Context checkpoint

```yaml
last_progress: decision drafted from owner decisions D53-D57
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: validate, commit, open PR, freeze, one exact-head review
```
