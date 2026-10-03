# OTV2-20261003-familiars0-fix2

```yaml
task_id: OTV2-20261003-familiars0-fix2
title: "FAMILIARS-0-FIX-2: the return-admission close survives same-session process replacement"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-fix2-20261003
issue: 162
pr: 1679
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0-fix2.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries P1 4174474873 from #1674 round 3, split off under D317.

- The clean-end write records `clean_end_game_session_id` in the same compare-and-set (FAMILIARS-0 §5).
- Return admission is refused while that value equals the admitting session's generation.
- A same-session successor after process replacement (FND-04B §22) reloads the value, so the close
  survives without in-memory state.
- F-I11 now names this source.
- A PostgreSQL reload case covers the interval between the clean-end commit and authority release.

Review round 1 (Codex), 4174542788: the close is bound to the closing GameSessionId
(`clean_end_game_session_id`), not a bare generation. A fresh admission starts at connection
generation 1 with a new GameSessionId. §5 defines a session's "generation" as the fence identity
(`game_session_id`, `connection_generation`), and a case covers a fresh login at generation 1 after
a clean logout.

## High-risk authority/recovery qualification

```yaml
applicable: true   # changes how persisted recovery evidence is read across process replacement
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [F-I1 owner character, F-I2 session generation, F-I3 runtime owner, F-I4 one write per revision, F-I5 occurrence binding, F-I6 provenance, F-I7 open discriminator, F-I8 recovery monotonicity, F-I9 atomic acquisition, F-I10 load never inserts or returns unreconciled, F-I11 clean-end write is the session's last row write until authority release, also through a same-session successor]
authority_invariants_changed: [F-I11 (fact source adds clean_end_game_session_id)]
consumer_boundaries: [cast, session end (logout, channel transfer, in-fight deadline terminal release), removal events, fenced login and arrival load, same-session successor reload (FND-04B §22), return admission (immediate and delayed), runtime timer, in-memory cooldown read]
mutation_operators:
  applicable: [cast insert/update (clears clean_end_game_session_id), return update (clears it), clean-end update (sets it), removal update, crash reconciliation update]
  considered_not_applicable: [delete (never while the character lives; Character cascade), administrative edit (none defined)]
one_invariant_per_negative_case: true   # the new §5.1 row names F-I11 only
independent_current_fact_sources: [GameSession CharacterId, the gameplay fence identity (game_session_id, connection_generation), session-generation fence row, RuntimeScopeAuthority ownership generation, content definition, the row's clean_end_game_session_id reloaded from PostgreSQL]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: done   # all three clean-end paths write clean_end_game_session_id
  protocol_versions: not_applicable   # no new wire (§8)
  direct_and_reconciled_paths: done
  fenced_durable_writes: done
  restart_retry_replay_concurrency_pg_reload: done   # same-session successor reload after the clean-end commit
finding_dispositions:
  "4174474873": fixed (durable clean_end_game_session_id, PostgreSQL reload case)
  "4174542788": fixed (close bound to GameSessionId; session identity defined; fresh-login case)
```

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --cached --check`: pass (no output)
