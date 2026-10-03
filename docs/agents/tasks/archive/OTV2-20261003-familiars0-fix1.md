# OTV2-20261003-familiars0-fix1

```yaml
task_id: OTV2-20261003-familiars0-fix1
title: "FAMILIARS-0-FIX-1: session end fences return admission; one invariant per negative case"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/familiars0-fix1-20261003
issue: 162
pr: 1674
head_sha: "exact frozen head in the control plane FREEZE_SHA entry"
owner: Sol Supervising Architect
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-familiars0-fix1.md
public_contracts: []
external_repositories: []
```

## Outcome

This task carries the two round-3 P1s that #1669 deferred (D317).

- **4174359097.** Logout and channel transfer now close return admission and drain an in-flight
  return before the clean-end write, and they hold the close until authority release (FAMILIARS-0
  §5). The new F-I11 states it, and a §5.2 negative case covers the cross-boundary race.
- **4174359104.** The compound §5.2 cases are split so each names one invariant:
  - reconciliation failure becomes three cases: F-I2, F-I3 and F-I10;
  - the delayed-return race becomes F-I4 (removal) and F-I7 (cast);
  - sibling creation becomes F-I6 and F-I7.

Review round 1 (Codex):

- **4174432631.** Return admission reopens only after the clean-end write is proven not committed.
  An unknown outcome is first reconciled from durable state (DUR-02).
- **4174432634.** The same-occurrence retry (F-I5) and the refused cast's spend (F-I9) are now cases
  of their own, apart from F-I4 and F-I7.

Review round 2 (Codex):

- **4174455168.** The return fence covers every clean-end path, including the terminal release at
  the in-fight deadline. A negative case covers that path.
- **4174455172.** A PostgreSQL negative case injects a lost clean-end commit response. Admission
  stays closed when the durable row proves the commit.
- **4174455176.** The qualification block below is complete.
- **4174455179.** Each validation command has its result.

D317: round 2 is the last P1 round on this PR; further P1s go to a follow-up.

## High-risk authority/recovery qualification

```yaml
applicable: true   # changes fenced Character writes and how persisted recovery evidence is read
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants: [F-I1 owner character, F-I2 session generation, F-I3 runtime owner, F-I4 one write per revision, F-I5 occurrence binding, F-I6 provenance, F-I7 open discriminator, F-I8 recovery monotonicity, F-I9 atomic acquisition, F-I10 load never inserts or returns unreconciled, F-I11 clean-end write is the session's last row write until authority release]
authority_invariants_added: [F-I11]
consumer_boundaries: [cast, session end (logout, channel transfer, in-fight deadline terminal release), removal events, fenced login and arrival load, return admission (immediate and delayed), runtime timer, in-memory cooldown read]
mutation_operators:
  applicable: [cast insert/update, return update, clean-end update, removal update, crash reconciliation update]
  considered_not_applicable: [delete (never while the character lives; Character cascade), administrative edit (none defined)]
one_invariant_per_negative_case: true   # FAMILIARS-0 §5.2: compound cases split (F-I2/F-I3/F-I10, F-I4/F-I5, F-I7/F-I9, F-I6/F-I7)
independent_current_fact_sources: [GameSession CharacterId, session-generation fence row, RuntimeScopeAuthority ownership generation, content definition, durable row re-read after an unknown commit outcome (DUR-02)]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: done   # all three clean-end paths fence return admission
  protocol_versions: not_applicable   # no new wire (§8)
  direct_and_reconciled_paths: done
  fenced_durable_writes: done
  restart_retry_replay_concurrency_pg_reload: done   # lost clean-end response (PostgreSQL), lost return response (F-I5), reconciliation failure (F-I2/F-I3/F-I10)
finding_dispositions:
  "4174359097": fixed (§5 return fence, F-I11)
  "4174359104": fixed (compound cases split)
  "4174432631": fixed (clean-end outcome reconciled before admission reopens)
  "4174432634": fixed (F-I5 and F-I9 cases)
  "4174455168": fixed (in-fight deadline terminal release fenced)
  "4174455172": fixed (lost clean-end response case, PostgreSQL)
  "4174455176": fixed (this block)
  "4174455179": fixed (validation results)
```

## Validation

- `python3 tools/agents/validate_governance.py`: pass
- `python3 tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 54 tests, OK
- `git diff --cached --check`: pass (no output)
