# OTV2-20261003-familiars0-fix3

```yaml
task_id: OTV2-20261003-familiars0-fix3
title: "FAMILIARS-0-FIX-3: same-session process replacement reconciles an open familiar row as a crash"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-fix3-20261003
issue: 162
pr: TBD
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0-fix3.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries P1 4174574477 from #1679, round 3, split off under D317.

- The row stores the writing runtime owner, `runtime_owner`: the RuntimeScopeAuthority semantic scope and its
  `scope_ownership_generation`. Every committed write stores it (FAMILIARS-0 §5).
- Crash predicate: the row is `open = true` and either its GameSessionId or its `runtime_owner` differs from
  the loader's. An ordinary same-session reconnect keeps the runtime owner and continues the familiar.
  A same-session continuation after process replacement (FND-04B §22) has a new ownership generation,
  so an open row is reconciled as a crash.
- The §5.1 rows, the §9 restart summary and the new F-I12 follow this rule.

## High-risk authority/recovery qualification

```yaml
applicable: true   # changes how persisted recovery evidence is read across process replacement
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [F-I1 owner character, F-I2 session generation, F-I3 runtime owner, F-I4 one write per revision, F-I5 occurrence binding, F-I6 provenance, F-I7 open discriminator, F-I8 recovery monotonicity, F-I9 atomic acquisition, F-I10 load never inserts or returns unreconciled, F-I11 clean-end write is the session's last row write until authority release, also through a same-session successor, F-I12 an open row continues only under its writing runtime owner and GameSession]
authority_invariants_changed: [F-I12 (new: crash classification by GameSessionId and runtime_owner)]
consumer_boundaries: [cast, session end (logout, channel transfer, in-fight deadline terminal release), removal events, fenced login and arrival load, same-session successor reload (FND-04B §22), return admission (immediate and delayed), runtime timer, in-memory cooldown read]
mutation_operators:
  applicable: [every write stores runtime_owner: cast insert/update, return update, clean-end update, removal update, crash reconciliation update]
  considered_not_applicable: [delete (never while the character lives; Character cascade), administrative edit (none defined)]
one_invariant_per_negative_case: true   # the amended §5.1 rows name the crash predicate (F-I12) only
independent_current_fact_sources: [GameSession CharacterId, the gameplay fence identity (game_session_id, connection_generation), session-generation fence row, RuntimeScopeAuthority ownership generation, content definition, the row's clean_end_game_session_id and runtime_owner reloaded from PostgreSQL]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: done   # every write path stores runtime_owner; login and same-session successor loads share one predicate
  protocol_versions: not_applicable   # no new wire (§8)
  direct_and_reconciled_paths: done
  fenced_durable_writes: done
  restart_retry_replay_concurrency_pg_reload: done   # same-session successor reload of an open row; reconnect inside one process
finding_dispositions:
  "4174574477": fixed (crash predicate adds runtime_owner; process replacement reconciles, reconnect continues)
```

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --cached --check`: pass (no output)
