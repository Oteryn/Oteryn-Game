# TESTS-SPLIT-D — move inline unit tests into topic files

```yaml
task_id: TESTS-SPLIT-D
mode: IMPLEMENT
status: local_validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Scope

Pure relocation of all four inline test modules from
`crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs` and
`crates/gameplay/durability/src/durability/spell_item_transaction.rs`.
Production text, visibility, dependencies, existing external test mounts and
test-support feature helpers remain unchanged. No enum variants, struct fields,
trait methods or public functions were added.

Owned paths: the two source files, their new `<file_stem>/tests/*.rs` files,
this task card and `testsd-report.md`. Changes remain uncommitted; no Git
mutation, publication, external review or dependency download was performed.

The requested `TESTS-SPLIT-B.md` template is absent from this worktree. This
card follows the existing `CONN-TESTS-SPLIT-1.md` structure and requested fields.

## File mapping and line counts

Original ranges were verified against the saved pre-move sources. Paths in
the table are relative to the corresponding source file's containing directory.

| Original source | Original lines | New file | Lines | Tests |
| --- | --- | --- | ---: | ---: |
| `admission_recovery_inner.rs` | 1795–1955 | `admission_recovery_inner/tests/durability_reconnect_v1_tests.rs` | 158 | 8 |
| `admission_recovery_inner.rs` | 2063–2290 | `admission_recovery_inner/tests/durability_reconnect_v2_commit_phase_regression_tests.rs` | 225 | 2 |
| `spell_item_transaction.rs` | 2474–2527 | `spell_item_transaction/tests/cast_cost.rs` | 51 | 2 |
| `spell_item_transaction.rs` | 3184–3208 | `spell_item_transaction/tests/map_protection_tags.rs` | 22 | 1 |

| Source file | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| `admission_recovery_inner.rs` | 5,048 | 4,665 | 383 |
| `spell_item_transaction.rs` | 3,521 | 3,448 | 73 |
| Total | 8,569 | 8,113 | 456 |

Every new Rust file has fewer than 600 lines; the largest has 225. Each original
module is now a thin `#[cfg(test)]`, `#[path = "..."]`, `mod ...;` declaration.
Module names, their parent, `super::*` privacy and test paths are preserved.
All helper code and inner attributes move with their original modules, with
only one indentation level removed. No moved module has an include path to fix.

## Test inventory and move proof

Local evidence is saved under ignored `target/testsd/`:

- `foundation-before.txt`, `foundation-after.txt`, `durability-before.txt` and
  `durability-after.txt`: complete sorted library inventories.
- `foundation-full.diff`, `foundation-names.diff`, `durability-full.diff` and
  `durability-names.diff`: all empty.
- `admission_recovery_inner.before.rs`, `spell_item_transaction.before.rs`:
  original sources; `pure-move.log`: strict text comparison results.
- `test-list-diff.log`: inventory counts and the 13 moved test names.

Both before and after inventories use exactly:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p <crate> --lib -- --list 2>/dev/null | grep ': test' | sort
```

| Crate | Before | After | Moved tests | Full-path / prefix-stripped diff |
| --- | ---: | ---: | ---: | --- |
| `oteryn-gameplay-foundation` | 335 | 335 | 10 | Empty / empty |
| `oteryn-gameplay-durability` | 435 | 435 | 3 | Empty / empty |

Strict comparison confirms each extracted module body, including tests,
assertions, attributes, helpers and imports, is identical after removing its
outer indentation. Replacing only the four original blocks with their new
declarations reproduces both final source files exactly. No production text
changed, so no constructor, match or impl sites require updates.

## Validation

All Cargo commands use `/home/mole/oteryn-cache/codex-bin/cargo`.

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --locked --offline -p oteryn-gameplay-foundation -p oteryn-gameplay-durability --all-targets -- -D warnings`: PASS.
- `cargo test --locked --offline -p oteryn-gameplay-foundation --lib`:
  PASS, 335 passed, 0 failed, 0 ignored.
- `cargo test --locked --offline -p oteryn-gameplay-durability --lib spell_item_transaction`:
  PASS, 3 passed, 0 failed, 0 ignored, 432 filtered out.
- Full library inventories: PASS, 335 / 335 and 435 / 435; both comparison forms
  are identical. Listing does not execute unrelated PostgreSQL tests.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check` and whitespace checks of all new files: PASS.
- Whole-diff self-review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`:
  PASS; no production changes or added API surface, and all 13 tests preserved.

The shared-PC owner rule overrides the requested combined unfiltered library
test command: foundation's library has no PostgreSQL test suite; durability
runs only the affected `spell_item_transaction` module. All moved tests are
synchronous unit tests; none requires PostgreSQL. The successful checks emit
an existing vendored `sqlx-core` deprecation warning; neither touched crate has
a clippy warning, and vendor files remain unchanged.

## Unfinished

No implementation or affected-test work remains. The full unfiltered durability
library/PostgreSQL suite is CI-only under the owner's scope rule. Changes are
uncommitted as requested.
