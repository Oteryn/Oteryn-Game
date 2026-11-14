# MAX501 COMMIT proof qualification

Base: `b29a0855d0e40ea0f89805612489ed519ba71a9c`, Rust 1.100.0,
PostgreSQL 18.6. Only migration 2837 changes the schema. The two-second
semantic-pass deadline, retry ceiling, 501-item work bound, one physical
transaction, existing assertions and DBG instrumentation are unchanged.
The original main test file remains an exact byte prefix of the final file.

## Full proofs and locks

The first cohort proof validates at most 501 current-transaction receipts from
one root, World and reset epoch. Every original Item, placement, removal,
quantity, reservation, assignment, audit, ancestry and terminal-root-bound
predicate is retained. Ambiguous/oversized/invalid cohorts use the original
private single-Item proof and its errors. The terminal-root bound runs once and
raises the original 23514 on false. Existing receipt, Item, custody, nested and
audit deferred callbacks all still execute.

The production workflow already locks the root and every descendant Item
`FOR UPDATE` (`world_reset_cause/ground_admission.rs:36`,
`world_reset_cause/death_contents.rs:46`). The cohort helper reasserts these
locks in UUID order, making its guarantee self-contained. Deletions retain
locks on removed custody rows; new custody or parent references require FK
`KEY SHARE` on the locked Items. Those facts cannot change from another session
while the proof is cached. Receipt/evidence/edge rows are immutable or owned by
the current physical transaction. The registration proof retains its existing
counter and registration `FOR SHARE` locks (`0003_runtime_scope_assignment.sql`).
No new global authority lock is introduced.

All 24 dependencies still invalidate this connection's proofs before AND after
INSERT/UPDATE/DELETE/TRUNCATE. Own changes and savepoint rollback therefore stay
transactional. Helpers and TEMP tables are owner-only, runtime/control cannot
forge the cache, and every hit requires the physical xid. Row-trigger calls
inside writes and sessions with holdable cursors use the full uncached proof;
this includes volatile WITH HOLD queries materialized during COMMIT.

## Narrow visibility fence; write cost and lock scope

A raw PostgreSQL snapshot is cluster-wide. Parallel tests reproduced the old
failure even with the first cache: unrelated database commits forced about
1,300 single-Item proofs and exhausted COMMIT's remaining 960 ms. Production
also needs unrelated-World writes in its one database to remain independent.

Only six mutable World-eligibility inputs log committed changes:
`game_world_reset_jobs`, `game_world_reset_active`, `game_world_reset_channels`,
`game_runtime_scope_assignments`, `game_durability_admission_runtime_guards`, and
`game_durability_reconnect_sessions`. Runtime/session captures skip updates that
leave their predicate fields unchanged. Guard scope keys resolve to their
assignment's World; a move records both old and new Worlds. A whole-table
TRUNCATE records an explicit NULL/all-Worlds scope.

`game_world_reset_proof_writers` has one latest head per
**(backend TEMP namespace, WorldId)**, with NULL reserved for all-Worlds changes.
PostgreSQL reuses TEMP namespaces with backend slots; two live backends cannot
own the same namespace. A private TEMP flag deduplicates to one head upsert per
affected World per transaction. Each backend serializes its own transactions,
so replacing its previous head cannot hide any newly committed write. Distinct
sessions NEVER update the same head row. There is no global or per-database
shared counter row. Ordinary Item/custody writes add **zero fence writes**;
they only retain own-write invalidation and the existing locks.

The head and its World facts commit atomically. The exact, byte-collated vector
of visible heads for this World plus NULL is captured BEFORE the full proof.
On a changed global snapshot, an equal local vector preserves the proven
cohort; a changed relevant vector forces a fresh proof. Reading snapshot and
heads together avoids labelling old facts with a newer fence. This is a vector,
not MAX(sequence), so out-of-order commits from different writers cannot be
lost. Lookup uses the World-leading index; storage is latest heads, not a
per-retirement/per-transaction journal. TEMP proofs disappear on commit,
rollback, restart or restore; the heads alone never grant authority.

## Index, rewrite guards and tests

The partial receipt index starts with `created_xact_id` and includes World,
epoch and root. Cohort discovery and current-transaction receipt reads use
index conditions, not a scan of permanent receipt history. PostgreSQL 18 marks
`pg_current_xact_id()` STABLE and supplies the xid8 btree opclass. The EXPLAIN
regression checks all four bounds and the transaction-only prefix (sequential
scan disabled solely because its empty fixture would otherwise prefer a scan).
The index is built by the normal transactional migration; schedule migration
application with retirement writers stopped for the historical index build.

Seven source-sha256 pins fail closed before copying or rewriting the original
proofs, including all three string-replacement rewrites. Typed Item record
comparison still compares every immutable field. Reservation INSERTs are
batched, while all 500 per-child writers retain their original order and the
root remains last. No trigger is disabled and no proof predicate is removed.

The focused tests exercise actual kind=2 production execution, own invalidation,
foreign xid rejection, concurrent committed authority changes and rollback from
a positive inner proof to invalid outer authority. An unrelated-World commit
must preserve the cohort. A holdable cursor must bypass it. Per-case recovery
directory tags prevent libtest threads from deleting each other's fence files.
Removing xid, snapshot, local-fence comparison, cohort production or the cursor
guard in disposable databases makes the corresponding positive test RED; all
five mutation runs were observed and their private databases removed.

## Before measurements

The baseline ran three times idle and three times with four CPU burners, each
25 ms process CPU then 25 ms sleep, under the same four-CPU cgroup. All six
baseline cases passed locally; they reproduced low headroom rather than the
intermittent CI timeout. The test itself was unchanged.

```text
idle run 1: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1713060 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(739684) commit_budget_us=Some(1260305) committed=true admission_completed_us=10974 tree_load_completed_us=6664 item_locks_completed_us=4745 graph_recheck_completed_us=6352 preparation_completed_us=33563 batch_completed_us=673480 root_completed_us=3490 commit_completed_us=972543
idle run 2: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1604955 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(782752) commit_budget_us=Some(1217235) committed=true admission_completed_us=13503 tree_load_completed_us=6953 item_locks_completed_us=4710 graph_recheck_completed_us=6446 preparation_completed_us=35440 batch_completed_us=711022 root_completed_us=4235 commit_completed_us=822143
idle run 3: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1621765 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(767759) commit_budget_us=Some(1232229) committed=true admission_completed_us=11437 tree_load_completed_us=6559 item_locks_completed_us=4609 graph_recheck_completed_us=7243 preparation_completed_us=36360 batch_completed_us=697672 root_completed_us=3590 commit_completed_us=853983
loaded run 1: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1670536 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(781034) commit_budget_us=Some(1218945) committed=true admission_completed_us=14378 tree_load_completed_us=8434 item_locks_completed_us=9839 graph_recheck_completed_us=14799 preparation_completed_us=56794 batch_completed_us=671090 root_completed_us=5400 commit_completed_us=889480
loaded run 2: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1777569 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(878957) commit_budget_us=Some(1121031) committed=true admission_completed_us=15070 tree_load_completed_us=9004 item_locks_completed_us=6746 graph_recheck_completed_us=7381 preparation_completed_us=34054 batch_completed_us=802144 root_completed_us=4110 commit_completed_us=898590
loaded run 3: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1629381 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(838504) commit_budget_us=Some(1161483) committed=true admission_completed_us=14860 tree_load_completed_us=7900 item_locks_completed_us=7573 graph_recheck_completed_us=7811 preparation_completed_us=33441 batch_completed_us=760674 root_completed_us=5941 commit_completed_us=790856
```

## Rejected intermediate measurements

These two requested outliers are from the earlier global-snapshot-only cache,
NOT the final World-scoped fence:

| Condition | Pre-COMMIT µs | COMMIT µs | Total µs |
|---|---:|---:|---:|
| Post-build idle outlier | 1162599 | 445552 | 1608440 |
| CPU-loaded outlier | 931062 | 538779 | 1469865 |

The same intermediate code then failed MAX501 in the two-thread player_death
run: pre-COMMIT 1040007 µs, COMMIT interrupted at 959550 µs, total 1999558 µs.
Its completed-function profile contained 1304 uncached Item proofs. This failure
motivated the World-scoped fence; no deadline or assertion was loosened.

## Final World-scoped measurements

The continuous writer commits every 5 ms. The other-database case updates its
own counter. The same-database writer advances an unrelated World's valid reset
job revision; its target is cloned from the existing synthetic fixture and adds
no Items. Both writers remain active through the measured pass. Commit counts
are observed at the phase output, not counts restricted to the COMMIT phase.

| Condition | Pre-COMMIT µs | COMMIT µs | Total µs | Writer commits observed |
|---|---:|---:|---:|---:|
| Idle | 816591 | 355824 | 1172438 | — |
| Concurrent writer, another database | 853647 | 380682 | 1234351 | 6571 |
| Concurrent writer, same database / another World | 1009900 | 395787 | 1405858 | 215 |

All three pass the unchanged two-second deadline. The idle candidate meets the
~1.3-second gate; concurrent same-database commits no longer collapse the cache.

```text
idle: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1172438 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(816591) commit_budget_us=Some(1183380) committed=true admission_completed_us=17354 tree_load_completed_us=8206 item_locks_completed_us=7593 graph_recheck_completed_us=7734 preparation_completed_us=32516 batch_completed_us=738033 root_completed_us=4850 commit_completed_us=355824
other_db: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1234351 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(853647) commit_budget_us=Some(1146339) committed=true admission_completed_us=14880 tree_load_completed_us=8122 item_locks_completed_us=7781 graph_recheck_completed_us=7985 preparation_completed_us=33604 batch_completed_us=776007 root_completed_us=4970 commit_completed_us=380682
other_world: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1405858 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(1009900) commit_budget_us=Some(990086) committed=true admission_completed_us=14275 tree_load_completed_us=6692 item_locks_completed_us=4987 graph_recheck_completed_us=6813 preparation_completed_us=33609 batch_completed_us=934672 root_completed_us=8397 commit_completed_us=395787
```

## Function profiles

Whole-pass counters (nested totals overlap):

Baseline idle run 1

```text
[DBG-audit-765f34] MAX501 function=game_world_reset_item_proven(uuid) calls=1503 total_us=620225 self_us=217174
[DBG-audit-765f34] MAX501 function=game_player_death_retirement_entry_proven(uuid) calls=1500 total_us=173980 self_us=173980
[DBG-audit-765f34] MAX501 function=game_player_death_retire_entry(uuid,numeric,uuid,uuid,smallint,bigint,bigint,text,bigint) calls=500 total_us=566949 self_us=161675
[DBG-audit-765f34] MAX501 function=game_player_death_reset_entry_capture() calls=500 total_us=118857 self_us=117259
[DBG-audit-765f34] MAX501 function=game_world_reset_drained(uuid,numeric) calls=1503 total_us=222298 self_us=103164
[DBG-audit-765f34] MAX501 function=game_item_instance_guard() calls=501 total_us=124473 self_us=102832
[DBG-audit-765f34] MAX501 function=game_node_registration_history_valid() calls=1510 total_us=100907 self_us=100907
[DBG-audit-765f34] MAX501 function=game_item_nested_entry_proven() calls=500 total_us=351710 self_us=97035
[DBG-audit-765f34] MAX501 function=game_item_location_exclusive(uuid) calls=1502 total_us=89505 self_us=89505
[DBG-audit-765f34] MAX501 function=game_account_closure_custody_guard() calls=2508 total_us=87501 self_us=87501
[DBG-audit-765f34] MAX501 function=game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint) calls=1 total_us=669307 self_us=73246
[DBG-audit-765f34] MAX501 function=game_item_instance_change_proven() calls=501 total_us=246136 self_us=57500
[DBG-audit-765f34] MAX501 function=game_world_reset_receipt_revision() calls=501 total_us=41736 self_us=41736
[DBG-audit-765f34] MAX501 function=game_item_audit_envelope_size_proven() calls=501 total_us=31377 self_us=25157
[DBG-audit-765f34] MAX501 function=game_item_loop_line_current(uuid) calls=502 total_us=20942 self_us=20942
[DBG-audit-765f34] MAX501 function=game_node_lock_current_registration(uuid,numeric) calls=1503 total_us=119133 self_us=19099
[DBG-audit-765f34] MAX501 function=game_character_is_uuid_v7(uuid) calls=6518 total_us=15896 self_us=15896
[DBG-audit-765f34] MAX501 function=game_house_disposition_item_fence() calls=501 total_us=9543 self_us=9543
[DBG-audit-765f34] MAX501 function=game_item_location_exclusivity_guard() calls=1002 total_us=98980 self_us=9474
[DBG-audit-765f34] MAX501 function=game_item_audit_item_proven() calls=501 total_us=8318 self_us=8318
```

Final same-database / other-World writer

```text
[DBG-audit-765f34] MAX501 function=game_player_death_retire_entry(uuid,numeric,uuid,uuid,smallint,bigint,bigint,text,bigint) calls=500 total_us=881528 self_us=212999
[DBG-audit-765f34] MAX501 function=game_item_instance_guard() calls=501 total_us=235659 self_us=142909
[DBG-audit-765f34] MAX501 function=game_player_death_reset_entry_capture() calls=500 total_us=158025 self_us=139030
[DBG-audit-765f34] MAX501 function=game_world_reset_proof_cache_invalidate() calls=7022 total_us=133130 self_us=133130
[DBG-audit-765f34] MAX501 function=game_account_closure_custody_guard() calls=2508 total_us=104792 self_us=104792
[DBG-audit-765f34] MAX501 function=game_item_nested_entry_proven() calls=500 total_us=189440 self_us=80122
[DBG-audit-765f34] MAX501 function=game_world_reset_items_proven_set(uuid) calls=1 total_us=76922 self_us=71541
[DBG-audit-765f34] MAX501 function=game_world_reset_item_proven(uuid) calls=1503 total_us=144746 self_us=65647
[DBG-audit-765f34] MAX501 function=game_item_location_exclusive(uuid) calls=1502 total_us=63704 self_us=63704
[DBG-audit-765f34] MAX501 function=game_world_reset_receipt_revision() calls=501 total_us=87322 self_us=50401
[DBG-audit-765f34] MAX501 function=game_item_instance_change_proven() calls=501 total_us=56583 self_us=36800
[DBG-audit-765f34] MAX501 function=game_item_loop_line_current(uuid) calls=502 total_us=29600 self_us=29600
[DBG-audit-765f34] MAX501 function=game_player_death_retire_entries(uuid[],uuid[],uuid[],uuid[],bigint[],text[],bytea[],smallint[],uuid,numeric,uuid,uuid,smallint,uuid,bigint,uuid,numeric,smallint,text,numeric,bytea,bytea,uuid,numeric,bigint,text,bigint) calls=1 total_us=927097 self_us=21114
[DBG-audit-765f34] MAX501 function=game_item_audit_envelope_size_proven() calls=501 total_us=25590 self_us=20225
[DBG-audit-765f34] MAX501 function=game_character_is_uuid_v7(uuid) calls=6518 total_us=16426 self_us=16426
[DBG-audit-765f34] MAX501 function=game_house_disposition_item_fence() calls=501 total_us=9599 self_us=9599
[DBG-audit-765f34] MAX501 function=game_starting_item_current(uuid) calls=501 total_us=9504 self_us=9504
[DBG-audit-765f34] MAX501 function=game_world_reset_proof_touch_world(uuid) calls=501 total_us=17930 self_us=9101
[DBG-audit-765f34] MAX501 function=game_world_reset_proof_cache_init() calls=503 total_us=8917 self_us=8917
[DBG-audit-765f34] MAX501 function=game_public_v2_item_source() calls=501 total_us=6668 self_us=6668
```

The baseline COMMIT-only counter delta (sampled in a disposable schema at its
first deferred callback) isolated 1503 World Item proofs (581541 µs total),
1500 ancestry proofs (167025 µs), 1503 drain proofs (201322 µs), and 1504
registration-history checks (89692 µs). Diagnostic instrumentation is not part
of the migration or changed test.

## Validation before the held repair push

- Focused PostgreSQL proof/index/cursor/World-isolation cases: **8/8**.
- Guard-removal mutation checks: **5/5 RED as expected**, normal cases green.
- MAX501: **3/3**, including continuous writers in another database and another
  World of the same database; one physical transaction, all 501 receipts/audits.
- `cargo test -p oteryn-game-server --test character_authority_postgres player_death -- --test-threads=2`: **38 passed**, no failure/ignore.
- `cargo fmt --all --check` and migration-number uniqueness passed.
- Existing release World compiler `pin-check . /tmp/max501-pin-check` passed.

The supervisor explicitly requested the long world_reset/player_death nextest
regression and final clippy run AFTER this push. Their results are reported in
the task's final report. The earlier pre-review code passed its full 37-case PG
set, 14 durability unit tests and two-crate clippy, but those are not substituted
for the final repair's follow-up checks.
