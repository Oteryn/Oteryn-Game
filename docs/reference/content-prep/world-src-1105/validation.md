# Historical validation of the received c8b3890e patch

This is prior-base evidence only. Fresh continuation validation is recorded in
`continuation-validation.md`; no earlier result is claimed for the rebased tree.

# WORLD-SRC focused validation

Base: origin/main `c8b3890e601e6073713dcac2ea9ee271f4a257e8`.
No commit, push, tool installation, new dependency or merged-migration edit.
Source stops are the five-line decisions in `seasonal.md` and `map.md`.

## Production regressions and actual PostgreSQL execution

Every admitted mechanic has a retained failing-before / passing-after regression.
Compiler/layer tests initially failed with missing layer support; conversion failed
against legacy-only geometry; gates lacked a production movement owner; Flower
lacked shared-attempt ownership and a repeatable calendar; Furyosa lacked its typed
layer dependency; client tests failed on missing reducers, app callers and default
capability negotiation. Red logs are retained under `/tmp/world-src/` and
`/tmp/w05b-*red*`; green logs below are actual command output, not fixtures.

| Final case | Evidence |
| --- | --- |
| Canonical conversion and donation production dispatch | `/tmp/world-src/w12-final-pg.log`: 2 passed, 0 failed, 55.47s; actual session command traces, assertions on SQL monotonicity SQLSTATE23514 and duplicate legacy import23505, completed progress retained and legacy25m imported once |
| Native gate movement | `/tmp/world-src/gates-final-green.log`: 3 passed, 0 failed, 69.45s; 9 `W05B_DAILY_GATE_POSTGRES_CASE_COMPLETED` scenes, including premium/promoted level60 entrance and off-day return |
| Physical Flower spawn/replay | `/tmp/world-src/flower-physical-green.log`: 1 passed, 0 failed, 65.01s; `WORLD_SRC POSTGRES_EXECUTED Flower physical native25 replay25` |
| Flower calendar/repeat and Furyosa dated layer | `/tmp/w05b-source-final-green.log`: 3 passed, 0 failed, 7.63s; `POSTGRES_EXECUTED WORLD-SRC flower-cycle repeat-and-replay` and `POSTGRES_EXECUTED WORLD-SRC furyosa source-layer admission`; third case is zero-epoch boot |
| Layer composition/collision/native spawn admission/relabel guard | `/tmp/w05b-layer-final-green.log`: 2 passed, 0 failed |
| Raid generator | `/tmp/world-src/raid-generator-final.log`: 16 passed |
| Client UI/runtime/session/application | `/tmp/world-src/client-final-tests.log`: 329 passed; all four changed client packages; `/tmp/world-src/client-final-clippy.log`: all-target Clippy with `-D warnings`, exit0 |
| Client architecture | `/tmp/world-src/client-architecture.log`: `cargo run --locked -p oteryn-architecture-check --quiet -- client-structure .`, exit0 |

The PostgreSQL route came from the required prepared environment, using fresh
per-case databases and the full migrations through0963. No credentials are recorded.
Broad library tests run without that route to avoid launching unrelated PostgreSQL
suites; they are not offered as proof of database execution. The cases above are.
Windows native controls follow the existing platform ownership model. Linux checks
exercise their portable queue/session path; no Windows target build or UI run is claimed.

## Generated artifacts and identifiers

Repository raid generator produced239 records/136 active; registration generator
updated0057's fixture. Compiler `pin-refresh` and `pin-check` produced inputs digest
`6aba70ccc7d219ec6ed83a663a9b86499b3910e3f93344786db76910c3936e06`
and world digest `aee6c8fce5e56800290ed88909a740e81ceab78f13c1ba6972999b0ab3924200`.
No pin or golden was manually authored. The index contains the compiler inputs
because the repository's digest tool hashes indexed blobs.

Migration0963 is the only new migration. Reused capability57/commands72,73;
next free origin/main identifiers276/492 were inspected but not allocated.

## Map-wire recording and package checks

The broad dispatch run completed with2310 passed,18 ignored and one stale recording
failure (`/tmp/world-src/unit-oteryn-gameplay-dispatch.log`). Its324 updates and
5,431,963 payload bytes were unchanged; bundle format7 changes fixture identity.
`python3 tools/contracts/refresh_world_map_golden.py` regenerated the recording only
after reconstructing v6 identity for every fixture and verifying the entire retained
v6 wire hash. It replaces only the32-byte `MapViewHeaderV1.bundle_digest`, checking
its protobuf tags, lengths and current value before substitution. Every other
recorded byte, revision, update and length must reproduce the retained v6 hash.
The normal test, with recording mode off, then passed1/0 in6.07s, verifying both
hashes (`/tmp/world-src/map-golden-green.log`). Current generated hash:
`c114d516f4bd770c7d270c29447490a1368e98c921a6a2c4b6937b977cfb543a`.
No production legacy reader or fixture-golden value was hand-authored.

Final dispatch all-target Clippy with `-D warnings` passed again after the recording
helper was added (`/tmp/world-src/dispatch-final-clippy.exit`:0).
Final `cargo fmt --all --check` passed (`/tmp/world-src/fmt-final.exit`:0).
Other backend changed packages passed the consolidated all-target Clippy command
with `-D warnings` (`/tmp/world-src/backend-clippy.exit`:0); dependency warnings
from unchanged vendored dependencies are not first-party Clippy failures.

Library checks use separate package invocations, `cargo test --locked -j1 -p NAME
--lib --quiet -- --test-threads=4`, preserving their normal feature graphs. Entities
also uses its existing `--features test-support`. PostgreSQL is unset only for this
broad lane; the actual PostgreSQL cases above ran with the prepared route present.
Initial combined-package/resource-exhausted attempts are excluded from pass counts
and from mechanic RED evidence. No tool installation, altered profile, RUSTFLAGS,
linker substitution, incremental-disable flag or prepared-cache deletion was used.

| Library package | Result |
| --- | --- |
| gameplay-world | 216 passed,1 existing ignored |
| gameplay-world-change | 14 passed |
| gameplay-encounter-rules | 61 passed |
| gameplay-entities | 247 passed |
| gameplay-durability | 671 passed |
| gameplay-dispatch | 2310 passed in full run; sole stale golden corrected and passed separately;18 existing ignored |
| game-server | 498 passed,3 existing ignored |
| world-bundle | 4 passed |
| world-bundle-compiler library | 4 passed |
| architecture-check library | 32 passed |

Additional focused gates passed: compiler package `--tests`56 passed/1 existing
ignored, including full-world double compilation; `integration_rest content_`123
passed; `content_world_project_repository`5 passed in153.55s. Logs and exit0 markers:
`/tmp/world-src/compiler-tests.*`, `/tmp/world-src/content-integration.*`,
`/tmp/world-src/content-world.*`. These ran serially after the library checks.
Final diff whitespace check passed; forbidden paths are unchanged; HEAD and
origin/main remain the stated base. No commit or push was made.
