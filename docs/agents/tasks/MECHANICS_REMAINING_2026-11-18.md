# Mechanics remaining — 2026-11-18

Work in progress. Static source audit of Game main at `3ffe8d05116b56d752944370a7c43341e0ca8bc8`.

Ledger inventory, counted from its table: 187 rows, DONE 58, PARTIAL 79, MISSING 42, OWNER-EXCLUDED 5, OUT-OF-SCOPE 3. This refresh covers 107 previously PARTIAL or MISSING rows after applying the owner scope.

| Row | ID | State | Evidence file:line | What remains | In-flight branch |
|---|---|---|---|---|---|
| Starting items and backpack contents | C19b | DONE | `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/first_entry.rs:374`, `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/reaudit_starting_progression.rs:181` | — | — |
