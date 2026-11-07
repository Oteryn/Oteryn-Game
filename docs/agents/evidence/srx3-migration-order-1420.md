# SRX-3 migration-order checkpoint

This records an unresolved fresh-database failure, not mechanic completion.
The owner approved migration 1420 and forbids editing merged migrations.
No additional migration number is assumed to be reserved.

## PROVEN: 1420 cannot repair the earlier audit

Migration 0985 grants SELECT on `game_character_tibiadrome` to
`oteryn_game_highscore_job`. Migration 0996 rejects that existing privilege
before migration 1420 can run. The isolated reproduction used byte-identical
0996 from main `1ae39c7fb9bef09c3cd50d16410374f4134b1676` (SHA-256
`50c3a0a5ac0efa2cc7f28522697de611a9939fe0004d041ca09617efc3f2b0fc`).

The transaction applying the 1420 grant followed by 0996 produced:

```text
ERROR:  42501: pre-existing Game group role oteryn_game_highscore_job holds a privilege outside the highscore grants
CONTEXT:  PL/pgSQL function inline_code_block line 99 at RAISE
```

The audit executed in 6.446 ms; the complete psql invocation took 0.113 s.
A separate rollback-only clone probe revoked only this table's SELECT,
ran unchanged 0996, restored the grant, then successfully selected from the
actual table under the highscore role. That SELECT executed in 0.336 ms;
the complete probe took 0.121 s. Both transactions were rolled back.
These probes establish ordering and role access, not a passing migration suite.

## Pending owner decision and validation

A new cleanup migration between 0985 and 0996 could revoke that one table
privilege before the historical audit; 1420 would restore it afterward.
Candidate 0986 was requested but is not approved or added. No historical
migration or cluster-wide role workaround was changed for this proposal.
Fresh migration and own PostgreSQL mechanic cases remain pending.

The owner-directed checkpoint also defers PREPUSH-2 regeneration and complete
changed-crate unit runs. The interrupted focused run passed seven cases,
interrupted two, and did not run nine; it is not a full passing test result.
