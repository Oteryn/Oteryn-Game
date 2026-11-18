# m49_missing_charge_keeps_corpse_and_boss_absent

- Base SHA: `bdbbbd1f95930b2fb9a2080c197b1dc88d508c4d`
- PostgreSQL: 18.6
- Result: PASS
- Exit code: 0
- Elapsed wall-clock seconds: 263

```bash
source /workspace/oteryn-env/env.sh
OTERYN_DISPATCH_PG_TESTS=m49_elemental_arrival CARGO_BUILD_JOBS=2 cargo test -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests::m49_elemental_arrival::m49_missing_charge_keeps_corpse_and_boss_absent -- --exact --nocapture > /tmp/m49-q3259/m49_missing_charge_keeps_corpse_and_boss_absent.log 2>&1
```

The case emitted no `POSTGRES_CASE_COMPLETED` line. The named libtest result and PostgreSQL fixture output are reproduced below.

```text
running 1 test
PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979136972 level=info module=durability event=db_tx_end outcome=ok ms=1 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979136982 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979136985 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137144 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137151 level=info module=durability event=db_tx_end outcome=ok ms=1 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137153 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137202 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137234 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137384 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137428 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979137445 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158583 level=info module=durability event=db_tx_end outcome=ok ms=121 operation=begin attempts=1
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_window window_ms=21725 held_ms=679 passes=13 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:313:14 passes=1 held_ms=238 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/admission_journal.rs:87:38 passes=3 held_ms=189 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:181:14 passes=1 held_ms=161 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:1175:36 passes=1 held_ms=39 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:787:14 passes=1 held_ms=12 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/character_authority.rs:346:14 passes=1 held_ms=10 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=maintenance passes=1 held_ms=8 refused=0
oteryn-game-server ts=1794979158621 level=info module=durability event=holder_usage caller=crates/gameplay/durability/src/durability/runtime_scope_assignment.rs:302:14 passes=1 held_ms=6 refused=0
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158722 level=info module=durability event=db_tx_end outcome=ok ms=1 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158801 level=info module=durability event=db_tx_end outcome=ok ms=34 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158854 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158869 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158873 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158878 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158915 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158923 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158944 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158955 level=info module=durability event=db_tx_end outcome=ok ms=1 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979158963 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159046 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159056 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159068 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159072 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159152 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159156 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159160 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159165 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159172 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159175 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159183 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159284 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159289 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159294 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159299 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159507 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159515 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159520 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159789 level=info module=durability event=db_tx_end outcome=ok ms=33 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159847 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159855 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159951 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159955 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979159960 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160043 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160047 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160051 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160058 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160062 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160070 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160073 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160080 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160082 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160084 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160086 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160089 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160092 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160097 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160100 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160106 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
oteryn_gameplay_dispatch-346ea2b916a74613 ts=1794979160109 level=info module=durability event=db_tx_end outcome=ok ms=0 operation=begin attempts=1
test gameplay_transport::monk_save::postgres_tests::m49_elemental_arrival::m49_missing_charge_keeps_corpse_and_boss_absent ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 25.73s
```
