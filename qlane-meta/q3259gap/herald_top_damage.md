# srvenc_m24_postgres_herald_top_damage

- Base SHA: `bdbbbd1f95930b2fb9a2080c197b1dc88d508c4d`
- PostgreSQL: 18.6
- Result: PASS
- Exit code: 0
- Elapsed wall-clock seconds: 19

```bash
source /workspace/oteryn-env/env.sh
OTERYN_DISPATCH_PG_TESTS=srvenc_source_deaths CARGO_BUILD_JOBS=2 cargo test -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests::srvenc_source_deaths::srvenc_m24_postgres_herald_top_damage -- --exact --nocapture > /tmp/herald-rerun/herald_top_damage.log 2>&1
```

```text
running 1 test
PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED srvenc_m24_postgres_herald_top_damage duration=16.97680058s success=true
test gameplay_transport::monk_save::postgres_tests::srvenc_source_deaths::srvenc_m24_postgres_herald_top_damage ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3032 filtered out; finished in 16.98s
```
