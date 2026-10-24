# ROUTE2-A merge resolution

Branch: `claude/happy-davinci-ijz3wy-routing`.
HEAD: `c36a17210` (CAPS-SPLIT-3).
MERGE_HEAD: `59a6e8a99` (EVENTS-WORLD-1, #2353).

## Conflicts and resolution

- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs`: removed two conflict blocks. Kept CAPS-SPLIT-3's private `routing::Dispatch` and all per-mechanic modules. `WorldEvent(Result<Vec<u8>, ()>)` and its command body already exist in the extracted routing files, so the incoming inline copies are unnecessary. Kept daily reward -> world event -> highscores order, with the same world-event reply/error mapping and disconnect on `None`. The resolved connection is byte-identical to HEAD; its test bodies are byte-identical to both parents.
- `crates/protocol-oteryn/src/tests/capabilities_golden.rs`: accepted the incoming deletion, retaining CAPS-SPLIT-2's per-capability JSON fragments and their incoming loaders. Compared all 40 HEAD golden rows against all 40 fragments; every fact matches, including capability 53, WORLD_EVENT_V1. No additional fragment was necessary.
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/world_event.rs`: the automatic merge had concatenated both complete parent files. Retained the incoming updated file exactly once, byte-identical to MERGE_HEAD, including its real-dispatch continuity, replay, restart, and fence assertions. No test bodies or assertions were edited during resolution.

All connection sibling files and routing modules remain byte-identical to HEAD. Other incoming changes remain intact. No new behavior, public API, dependency, SQL, migration, or protocol changes were authored. No forbidden paths or Git mutation commands were used.

## Validation

Every Cargo invocation used `/home/mole/oteryn-cache/codex-bin/cargo`.

| Exact Cargo arguments | Result |
| --- | --- |
| `fmt --all --check` | PASS |
| `clippy --locked --offline -p oteryn-gameplay-dispatch -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings` | PASS, exit 0 |
| `test --locked --offline -p oteryn-protocol-oteryn --lib --quiet` | PASS: 415 passed, 0 failed |
| `test --locked --offline -p oteryn-gameplay-dispatch --lib connection:: --quiet` | PASS: 85 passed, 0 failed, 948 filtered out |
| `test --locked --offline -p oteryn-gameplay-dispatch --lib gameplay_transport::capabilities:: --quiet` | PASS: 46 passed, 0 failed, 987 filtered out |
| `test --locked --offline -p oteryn-gameplay-dispatch --lib world_event_postgres_real_dispatch_reset_restart_and_fences -- --nocapture` | PASS: 1 passed, 0 failed, 1032 filtered out; PostgreSQL executed both reset epochs (1 and 2), each reporting wire/receipt/restart/fences passed |
| `test --locked --offline -p oteryn-gameplay-dispatch --lib -- --list` | PASS: **1033** lines ending in `: test` |

`git diff --check` passed. `rg -n '^(<<<<<<<|=======|>>>>>>>)' crates apps` returned no matches. Re-read the completed diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; checked the relocated Dispatch variants and production routing sites across `apps/`, `crates/`, and `tools/`. Resolution adds no enum variants, struct fields, trait methods, or public functions.

Existing vendored Tokio/SQLx dependency warnings appeared during builds; requested clippy still exited successfully. Vendor files were not modified.

## Changed files in this resolution

- `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` (removed conflict blocks).
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/world_event.rs` (removed the duplicate older parent copy).
- `crates/protocol-oteryn/src/tests/capabilities_golden.rs` (deleted as requested).
- `route2-A-report.md` (this report).

## Unfinished

- No implementation or validation failures remain.
- The Git index intentionally remains unmerged/unstaged: the driver must stage the resolved files and deletion, then commit. This worker did not run `git add` or `git commit`.
- The requested `/home/mole/oteryn-cache/route2-A-report.md` is outside the allowed writable workspace and conflicts with the current-directory-only rule. This report is saved at `/home/mole/oteryn-cache/rs-wt/routing/route2-A-report.md` for the driver to collect.
