# TESTS-SPLIT-A — split large inline test modules

```yaml
task_id: TESTS-SPLIT-A
title: Split large inline test modules into sibling topic files
mode: IMPLEMENT
status: waiting
local_result: complete
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
worktree: split-testsa
head_state: detached
pr: null
base_sha: 670633ad93893a7d7e82bec64481ed878d8cc1a7
owner: Codex standalone task worker
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
```

## Task and scope

Owner-approved structural refactor only. Move the inline test modules out of the
five allocated production files, splitting their bodies by topic. Preserve every
test name, assertion, helper visibility and full module path. No production API,
protocol, SQL, error text or side-effect changes. No Git writes or publication.

The immutable META 3.1.0 policy was read at bound revision
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; no publication or external review is
part of this local worker task.

## Owned paths

- `crates/session/src/lib.rs`, new sibling `tests.rs` and `session_tests_*.rs`.
- `crates/gameplay/content/src/content/reference_artifact.rs` and new sibling
  `reference_artifact_*tests*.rs`.
- `crates/gameplay/world/src/world_object_revert.rs` and new sibling
  `world_object_revert_tests*.rs`.
- `crates/gameplay/world/src/world_runtime.rs` and new sibling
  `world_runtime_tests*.rs`.
- `tools/dev-client/src/lib.rs`, new sibling `tests.rs` and `client_tests_*.rs`.
- `docs/agents/tasks/TESTS-SPLIT-A.md`.

Generated local validation evidence lives under `target/TESTS-SPLIT-A/`.

## Mapping: old module to new location

| Old location | New sibling root and topic files |
|---|---|
| session `lib.rs::tests` | `tests.rs`; `session_tests_{admission_cast,combat_logout,vitals,capabilities,push,entities,chat,items}.rs` |
| content `reference_artifact.rs::native_item_batch_tests` | `reference_artifact_native_item_batch_tests.rs` |
| content `reference_artifact.rs::typed_item_codec_tests` | `reference_artifact_typed_item_codec_tests.rs`; `reference_artifact_tests_{v4,v5,profiles,v6,beds,bed_sets}.rs` |
| world `world_object_revert.rs::tests` | `world_object_revert_tests.rs`; `world_object_revert_tests_{duke,lifecycle,authority,scheduling,create,rearm}.rs` |
| world `world_runtime.rs::tests` | `world_runtime_tests.rs`; `world_runtime_tests_{b1,fixtures,authority,overlay,use}.rs` |
| dev-client `lib.rs::tests` | `tests.rs`; `client_tests_{admission,snapshots,command_fixtures,commands,command_errors,liveness}.rs` |

Roots use `#[cfg(test)]` and the original module name with a sibling-file mount
(`mod tests;` for session; `#[path = "..."] mod <original_name>;` elsewhere). Topic files
use `include!` inside the original module, keeping private fixtures and test paths
unchanged. The session depot module's relative path is adjusted for the moved
root; existing appearance, depot, tutorial and guild test sources are untouched.
All 37 new files are at most 645 lines. Fixture includes remain in the same
source directory, so their original relative paths still resolve.

## Validation

PROVEN: reconstructing each of the six moved module bodies and formatting both
sides yields identical source. The original five files are byte-identical to
HEAD after replacing only their inline test-module bodies with external mounts.
Evidence: `target/TESTS-SPLIT-A/verify_moves.py`, `module-*.diff` (empty).

Inventory commands, run before and after extraction (all passed):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-session --lib -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-content --lib -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-world --lib -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-dev-client --lib -- --list
```

| Package | Before | After | Exact test-name set |
|---|---:|---:|---|
| `oteryn-session` | 72 | 72 | identical |
| `oteryn-gameplay-content` | 346 | 346 | identical |
| `oteryn-gameplay-world` | 189 | 189 | identical |
| `oteryn-dev-client` | 34 | 34 | identical |

Lists are saved as `<package>.before.txt` / `<package>.after.txt`; normalized
set diffs are `<package>.inventory.diff` under the evidence directory. The full
list files are also byte-identical (`<package>.full-list.diff`, all empty):
641 package tests before and after.

All following commands passed. `--offline` prevents dependency downloads; all
Cargo invocations use the queued wrapper.

Focused unit tests (196 selected tests, no failures or ignored tests):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-session --lib --quiet -- tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-content --lib --quiet -- content::reference_artifact::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-world --lib --quiet -- world_object_revert::tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-world --lib --quiet -- world_runtime::tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-dev-client --lib --quiet -- tests::
```

Counts in command order: **72, 29, 29, 32, 34**, all passed. Session's substring
filter also covers its existing mapping/guild modules. No moved test was omitted.
No moved module contains PostgreSQL tests; no unrelated PG suite was run.

Strict all-target package Clippy:

```sh
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-session --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-content --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-world --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-dev-client --all-targets --quiet -- -D warnings
```

Other checks:

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 target/TESTS-SPLIT-A/verify_moves.py
git diff --check
```

Formatting: pass, including a separate `rustfmt --edition 2024 --config
skip_children=true --check` over all 37 new files (listed in
`target/TESTS-SPLIT-A/new-files.json`), since rustfmt does not follow `include!`.
Governance: pass, 22 documents / 9 lanes. Governance unit tests: **129 passed**.
Source-equivalence and whitespace checks: pass. No standalone file-size guard
or `tools/governance/` exists in this checkout; the scope/size check confirms
all 37 new files are 66–645 lines and all 43 changed paths are allocated.
Evidence: `scope-size-check.txt`, `source-equivalence.log`, `fmt.log`,
`topic-fmt.log`, `governance*.log`, package test/Clippy logs and
`final-validation-results.json` under `target/TESTS-SPLIT-A/`.

## Self-review and reference parity

Reference parity and authority/recovery qualification: NOT_APPLICABLE; this task
moves existing tests without changing production behavior or any contract.
No enum variants, struct fields, trait methods or public functions are added.
PROVEN: reviewed the complete tracked diff and relocated source against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository checklist,
using exact reconstructed-body equivalence to account for every moved line.
No test is deleted, renamed, weakened, ignored or compiled into an additional
target. No fields, variants, trait methods or public API were introduced, so
workspace consumer updates and new production-dispatch tests are not applicable.
All original helper paths/visibility and all 641 inventoried test paths remain
identical. No checklist findings remain.

## Unfinished

No local implementation or validation work remains. Task lifecycle is waiting
for control-plane integration. Publication, PR creation, CI, task-card archival
and merge are outside this worker scope; Git mutations are explicitly prohibited.
The existing worktree is at a detached HEAD; no branch/ref/index was changed.
