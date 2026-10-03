# OTV2-20261003-familiars0-recovery-sweep

```yaml
task_id: OTV2-20261003-familiars0-recovery-sweep
title: "FAMILIARS-0 authority and recovery finding-family sweep (review 4173381895, #1644)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-recovery-sweep-20261003
issue: 1622
pr: "exact PR in the control plane FREEZE_SHA entry"
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0-recovery-sweep.md
public_contracts: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

FAMILIARS-0 §5.2 completes the authority and recovery qualification that review 4173381895
asked for on #1644. It is a precondition for allocating FAMILIAR-1.

Rulings that tighten the decision:
- §4 check 4 refuses a cast while a familiar waits to return.
- A logout or channel transfer completes only after its clean-end write commits, as
  TIMED-ITEM-0B §6.1 orders its checkpoints.
- `WorldReset` writes as a removal with the time kept.

## High-risk authority/recovery qualification

```yaml
applicable: true   # defines fenced Character writes and interprets persisted recovery evidence
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [F-I1 owner character, F-I2 session generation, F-I3 runtime owner, F-I4 one write per revision, F-I5 occurrence binding, F-I6 provenance, F-I7 open discriminator, F-I8 recovery monotonicity, F-I9 atomic acquisition, F-I10 load never inserts or returns unreconciled]
consumer_boundaries: [cast, session end (logout, channel transfer, in-fight deadline), removal events, fenced login and arrival load, return admission, runtime timer, in-memory cooldown read]
mutation_operators:
  applicable: [cast insert/update, return update, clean-end update, removal update, crash reconciliation update]
  considered_not_applicable: [delete (never while the character lives; Character cascade), administrative edit (none defined)]
one_invariant_per_negative_case: true   # FAMILIARS-0 §5.1 and §5.2 tables
independent_current_fact_sources: [GameSession CharacterId, session-generation fence row, RuntimeScopeAuthority ownership generation, content definition]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: done   # ordinary SUMMON-1, Summon Creature, convince, administrative creature creation
  protocol_versions: not_applicable   # no new wire (§8)
  direct_and_reconciled_paths: done
  fenced_durable_writes: done
  restart_retry_replay_concurrency_pg_reload: done
```

## Validation

`python3 tools/agents/validate_governance.py`, `python3 tools/repository/validate_repository_policy.py`,
`git diff --check`: pass.

```yaml
status: completed
owner_action_required: null
blocker: null
next_action: "control plane: review the frozen head; allocate FAMILIAR-1 after merge"
```
