# Triage pełnego sweepu repository tests

Źródło: `docs/agents/evidence/OTV2-20261122-test-ci-value-audit/repository_suite.json`, pomiar `2026-11-22T22:49:44.525649+00:00`, baza `8e9f0e16631aab1beb8416f8d5ca707aa3031083`.

**FACT: 30/32 entrypoints PASS. Dwa pozostałe są zablokowane brakującymi narzędziami. Nie wykryto błędu spowodowanego proponowanym diffem, ale pełny sweep nie jest zielony.**

| Entry point | Przyczyna | Klasyfikacja |
|---|---|---|
| `tools/repository/test_check_dispatch_pg_shards.py` | `rustc` nie istnieje w PATH, brak override `RUSTC`. `check_dispatch_pg_shards.py:117` nie może uruchomić kompilacji build script. Jeden test daje `FileNotFoundError`; dwa następne failures są wtórne, bo oczekiwane wyniki CLI nigdy nie powstają. | BLOCKED_MISSING_TOOLCHAIN |
| `tools/repository/test_validate_merge_group_pg_sim.py` | `_powershell_native_canaries():49` jawnie wymaga `pwsh`; `shutil.which` zwraca `None`. Wcześniejsze policy/routing checks wypisały PASS. | BLOCKED_MISSING_DEPENDENCY |

Niezależnie sprawdzono `shutil.which('rustc') is None`, `shutil.which('pwsh') is None` i brak `RUSTC` override. Oba failing test entrypoints oraz `check_dispatch_pg_shards.py` są byte-for-byte identyczne z bazowym HEAD; dokładne SHA-256 i oryginalne failure traces zapisano w `./repository_suite_failure_review.json`.

Nie wykonano baseline overlay ani kolejnego test sweepu: tracebacki wskazują jednoznacznie brak executable, nie zależną od candidate assertion. Nie instalowano narzędzi, nie osłabiano guards i nie zmieniano repozytorium. Do pełnej kwalifikacji potrzebne jest wykonanie tych dwóch entrypoints w środowisku z rzeczywistym Rust toolchain i PowerShell. Wyników nie należy zastępować stubami ani oznaczać jako PASS.
