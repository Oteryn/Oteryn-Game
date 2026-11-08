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

## Superseded by the final shipment instruction

The owner subsequently authorized renumbering this branch's own unmerged
migrations while merging main `47db34f09d760b2f435860c2f8d4ddf12a893e05`.
The next-free-number tool included main and every open PR and reported 1357.
The original 0984/0985 files are now 1357/1358, with their SQL unchanged;
1420 remains the forward grant. No merged migration was edited. This removes
the problematic grant from before 0996 without an extra cleanup migration.

PROVEN: an isolated fresh database applied all 338 current migration bodies
in numeric order, each in its own transaction, in 4.879 seconds. Versions
996, 1357, 1358 and 1420 all completed. A subsequent actual SELECT under
`oteryn_game_highscore_job` succeeded. This direct psql probe also populated
migration checksums; the separate production-harness cases validate the
SQLx migration path. The earlier rollback-only reproduction remains historical
evidence of the original ordering defect, not the current migration layout.

## Final forward scope fix and validation boundary

The post-merge native run executed five real PostgreSQL scenarios: Grand Mother
Foulthorn (46.633 s), Tibiadrome (45.224 s), Primal (115.139 s), and Soulpit
(101.271 s) passed. The Instance corpse/reward case failed in 85.668 s because
Scavenge's immutable 0983 race-provenance guard still compared two NULL Channel
IDs. Its assertion was retained unchanged.

New migration **1421** extends only that provenance predicate to disjoint,
exact Channel/Instance scopes. The recorded 19-case PostgreSQL guard probe
passed in 0.085562 s, including the original Instance rejection and cross-scope,
identity, generation, world and transaction refusals. It preserves function
ACL, owner and search path; it does not add Instance skinning authority.
A fresh database subsequently applied all **339** migration bodies, including
1421, in **4.622 s**; the actual highscore-role SELECT took **0.318 ms**.

The 25 changed-crate library build reached a linker bus error after **585.105 s**
while disk space was low; no unit cases executed in that attempt. Completed
build caches were reclaimed. Full unit execution and the post-1421 native
scenario rerun remain pending under PREPUSH-2's cadence exception; neither is
reported as green. CI with heavy tests remains required on the shipment head.

## Owner-authorized main 6f61 integration

The owner lifted the freeze to merge `6f61ac15d93bbda9f9d718721233c0d067f40530`.
Main 1258 had renamed the ordinal constraint expected by our unchanged
migration body. New **1422** verifies the complete current predicate and its
semantic expression tree, then only renames that constraint. The former
1358 file is now **1423**, byte-identical to its prior SQL. Migration 1357 and
forward grants/provenance fixes 1420/1421 remain unchanged. The owned sequence
is therefore **1357, 1420, 1421, 1422, 1423**.

The actual bridge plus complete 1423 passed 27 positive/negative PostgreSQL
CHECK cases in 0.188882 seconds inside a rolled-back transaction. An altered
main predicate was refused in 0.024800 seconds. A separate fresh database
applied all 364 current migration bodies in 4.716 seconds; the actual
highscore-role SELECT executed in 0.281 ms. These SQL probes do not substitute
for the separate native production-scenario replay.

## Main Soulpit owner supersedes the older floor rewrite

The requested `98d6ae8ce` merge includes main 1140/1141. The shared owner in
1141 requires runtime/LastPosition floor exactly +8; native TilePos alone
negates the map projection (ADR-0021; Crystal Soulpit staging script lines
37–77). The older dual-floor rewrite in our unmerged 1423 is now duplicate
work and was removed, retaining the stricter merged predicate. No merged
migration was edited. Earlier byte-identical 1423 observations above describe
the preceding 6f61 snapshot; the final combined schema is checked separately.
