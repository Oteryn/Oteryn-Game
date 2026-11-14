# MAX501 COMMIT proof qualification

Base: `b29a0855d0e40ea0f89805612489ed519ba71a9c`; Rust 1.100.0;
PostgreSQL 18.6. Migration 2837 is the only schema change. The two-second
semantic-pass deadline, retry ceiling, 501-item bound, physical transaction,
existing assertions and DBG instrumentation are unchanged.

## What runs at COMMIT

Subtracting `pg_stat_xact_user_functions` at the first deferred World receipt
callback from the existing completed-pass profile isolated these baseline costs
(microseconds; nested totals overlap):

| Function | COMMIT calls | Total | Self |
|---|---:|---:|---:|
| game_world_reset_item_proven | 1503 | 581541 | 210935 |
| game_player_death_retirement_entry_proven | 1500 | 167025 | 167025 |
| game_item_location_exclusive | 1500 | 108475 | 108475 |
| game_world_reset_drained | 1503 | 201322 | 93457 |
| game_item_nested_entry_proven | 500 | 307012 | 91218 |
| game_node_registration_history_valid | 1504 | 89692 | 89692 |
| game_item_instance_change_proven | 501 | 236197 | 47178 |

The counter snapshot was a diagnostic `RAISE LOG` in a disposable test database;
no diagnostic instrumentation, trigger disabling or proof replacement is part
of the test or shipped migration.

## Proof preservation

`game_world_reset_items_proven_set` checks at most 501 current-transaction
receipts from one root, World and reset epoch. It takes the same job lock,
runs the complete drain/registration proof and evaluates every original Item,
placement, removal, quantity, reservation, assignment, audit and ancestry
predicate. Only individually successful Items enter its result. The original
single-Item proof remains a private fallback, retaining its errors, for failed,
ambiguous or oversized cohorts. SQL three-valued tests preserve the original
PL/pgSQL failure-condition semantics. Existing deferred receipt, location,
instance, nested-entry and audit callbacks all still run.

A private TEMP table stores successful drain/cohort results. Its owner must be
the SECURITY DEFINER owner; runtime/control cannot write it or execute private
helpers. Entries require the physical transaction ID and a fresh MVCC snapshot,
and are deleted on COMMIT. BEFORE and AFTER statement triggers invalidate on
INSERT, UPDATE, DELETE and TRUNCATE of every one of the 24 fact dependencies.
The dependency-closure test derives these relations from the installed full
proofs and verifies both invalidators. Own writes, including savepoints and
nested trigger writes, therefore cannot leave a reusable stale proof.

Inside triggers, only top-level COMMIT callbacks reuse the cache. Row callbacks
inside a write use the complete original proof, because statement invalidation
alone cannot separate successive rows of one statement. Direct non-trigger
calls can reuse between statements. A changed visibility snapshot forces a
fresh proof, including a concurrent change made from another connection. After
such a change, cohort batching is disabled until the next dependency write, so
busy-cluster commits cannot cause repeated whole-cohort scans. Original row
locks remain held to transaction end. No trusted-time or wall-clock result is
cached. There is no cache surviving a transaction, crash or restore.

Two additional costs are reduced without dropping checks:

- Retirement fast paths compare the complete typed Item record after replacing
  only the four fields that were already allowed to change, rather than
  serializing OLD and NEW to JSON twice. Every other field is still compared.
- The batch reserves its already-frozen input array in one ordered INSERT.
  Every per-child writer still executes in the original order, with all 500
  terminal receipts/audits and the final root retirement. The existing injected
  mid-batch failure and conservation tests exercise that unchanged behavior.

## Measurement method

Each run creates a real database with the normal embedded migration ledger and
runs the unchanged `player_death_nested_maximum_501_item_reset_makes_bounded_atomic_progress`.
Baseline and final runs use the same build profile and retained PG cluster.
Loaded runs additionally run four independent Python CPU burners, each burning
25 ms of process CPU followed by 25 ms sleep, throughout all three runs (four
CPU cgroup quota). These are six separate before and six separate after runs;
test wall time also includes tree creation and is not the semantic-pass time.

The selected prototype was timed before qualification: total 1,062,762 µs,
pre-COMMIT 749,828 µs, COMMIT 312,906 µs. This met the owner's 1.3-second
idle gate; the final-migration measurements below are the acceptance evidence.

## Baseline phase lines

```text
idle run 1: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1713060 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(739684) commit_budget_us=Some(1260305) committed=true admission_completed_us=10974 tree_load_completed_us=6664 item_locks_completed_us=4745 graph_recheck_completed_us=6352 preparation_completed_us=33563 batch_completed_us=673480 root_completed_us=3490 commit_completed_us=972543
idle run 2: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1604955 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(782752) commit_budget_us=Some(1217235) committed=true admission_completed_us=13503 tree_load_completed_us=6953 item_locks_completed_us=4710 graph_recheck_completed_us=6446 preparation_completed_us=35440 batch_completed_us=711022 root_completed_us=4235 commit_completed_us=822143
idle run 3: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1621765 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(767759) commit_budget_us=Some(1232229) committed=true admission_completed_us=11437 tree_load_completed_us=6559 item_locks_completed_us=4609 graph_recheck_completed_us=7243 preparation_completed_us=36360 batch_completed_us=697672 root_completed_us=3590 commit_completed_us=853983
loaded run 1: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1670536 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(781034) commit_budget_us=Some(1218945) committed=true admission_completed_us=14378 tree_load_completed_us=8434 item_locks_completed_us=9839 graph_recheck_completed_us=14799 preparation_completed_us=56794 batch_completed_us=671090 root_completed_us=5400 commit_completed_us=889480
loaded run 2: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1777569 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(878957) commit_budget_us=Some(1121031) committed=true admission_completed_us=15070 tree_load_completed_us=9004 item_locks_completed_us=6746 graph_recheck_completed_us=7381 preparation_completed_us=34054 batch_completed_us=802144 root_completed_us=4110 commit_completed_us=898590
loaded run 3: [DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1629381 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(838504) commit_budget_us=Some(1161483) committed=true admission_completed_us=14860 tree_load_completed_us=7900 item_locks_completed_us=7573 graph_recheck_completed_us=7811 preparation_completed_us=33441 batch_completed_us=760674 root_completed_us=5941 commit_completed_us=790856
```

The unchanged terminal-root refusal regression caught duplicate bounds
evaluations (three instead of one). The cohort now checks each terminal root
once and raises the original 23514 immediately on false, before caching any
cohort result. The existing assertion and injected false proof remain unchanged.

## First corrected-migration run

The unchanged terminal-root refusal regression passed after the correction.
The three cache soundness cases passed on PG18.6; full regression qualification
continues after this early push at the supervisor's request.

```text
[DBG-audit-765f34] MAX501 phase=Some(Commit) in_flight_us=0 total_us=1079655 descendants=500 batch_rows_submitted=500 batch_rows_acknowledged=500 child_index=500 pre_commit_us=Some(772972) commit_budget_us=Some(1227016) committed=true admission_completed_us=11869 tree_load_completed_us=6665 item_locks_completed_us=4837 graph_recheck_completed_us=7374 preparation_completed_us=48769 batch_completed_us=688606 root_completed_us=4556 commit_completed_us=306572
```
