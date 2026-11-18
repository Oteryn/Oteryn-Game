# m49_current_arrival_recovers_after_lost_projection_response

- Base SHA: `bdbbbd1f95930b2fb9a2080c197b1dc88d508c4d`
- PostgreSQL: 18.6
- Result: PASS
- Exit code: 0
- Elapsed wall-clock seconds: 20

```bash
source /workspace/oteryn-env/env.sh
OTERYN_DISPATCH_PG_TESTS=m49_elemental_arrival CARGO_BUILD_JOBS=2 cargo test -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests::m49_elemental_arrival::m49_current_arrival_recovers_after_lost_projection_response -- --exact --nocapture > /tmp/m49-q3259/m49_current_arrival_recovers_after_lost_projection_response.log 2>&1
```

The case emitted no `POSTGRES_CASE_COMPLETED` line. The named libtest result and PostgreSQL fixture output are reproduced below.

```text
running 1 test
PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163476 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163483 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163486 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163622 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163626 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163628 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163662 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163690 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163723 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163756 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979163766 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178771 level=info module=durability event=db_tx_end outcome=ok ms=46 operation=begin attempts=1
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_window window_ms=15379 held_ms=309 passes=13 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:313:14 passes=1 held_ms=114 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:181:14 passes=1 held_ms=71 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/admission_journal.rs:87:38 passes=3 held_ms=57 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:1175:36 passes=1 held_ms=25 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:787:14 passes=1 held_ms=11 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:346:14 passes=1 held_ms=7 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:302:14 passes=1 held_ms=7 refused=0
oteryn-game-server ts=1794979178795 level=info module=durability event=holder_usage caller=maintenance passes=1 held_ms=7 refused=0
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178838 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178866 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178905 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178916 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178921 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178927 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178953 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178962 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178982 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979178994 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179005 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179079 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179091 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179106 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179109 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179184 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179190 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179194 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179201 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179207 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179210 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179221 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179227 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179342 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179349 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179355 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179362 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179574 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179584 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179591 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179682 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179699 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179706 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179803 level=info module=durability event=db_tx_end outcome=ok ms=2 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179809 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179817 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179907 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179911 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179918 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179928 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179933 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179943 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179947 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179955 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179957 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179959 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179961 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179964 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179968 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179974 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179979 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179987 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979179992 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180005 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180016 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180024 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180027 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180032 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180038 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979180041 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
test gameplay_transport::monk_save::postgres_tests::m49_elemental_arrival::m49_current_arrival_recovers_after_lost_projection_response ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 19.02s
```
