# TESTS-SPLIT-C — split carrier tests into topic files

```yaml
task_id: TESTS-SPLIT-C
mode: IMPLEMENT
status: local_validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Scope

Pure relocation of `crates/gameplay/entities/src/carrier/tests.rs` into
`carrier/tests/mod.rs`, ten topic files and `support.rs`. The existing sibling
`carrier/*_tests.rs` files and carrier implementation modules remain unchanged.
The sole change in `carrier.rs` points its existing test module at `tests/mod.rs`.
Test names, bodies, assertions, attributes, comments and helper code are unchanged.
No functions, enum variants, struct fields, trait methods or dependencies were added.

Owned paths: `crates/gameplay/entities/src/carrier.rs`, the old
`crates/gameplay/entities/src/carrier/tests.rs`, its replacement
`crates/gameplay/entities/src/carrier/tests/*.rs`, this task card and
`TESTS-SPLIT-C-NOTES.md` at the worktree root. All changes remain uncommitted.
No Git mutation, publication, external review or dependency download is part of
this task. Bound META policy 3.1.0 at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb` was resolved read-only.

The stale attempt in `/home/mole/oteryn-cache/rs-wt/split-testsc` was read only
for topic names, layout and task-card structure. Every moved item comes from
this worktree's current source after CARRIER-SPLIT-1. This card also follows
the current `TESTS-SPLIT-D.md` front matter, scope, mapping and validation format.

## File mapping and line counts

Paths below are relative to `crates/gameplay/entities/src/carrier/tests/`.

| New file | Responsibility | Lines | Tests |
| --- | --- | ---: | ---: |
| `mod.rs` | Original imports and sorted topic includes | 15 | 0 |
| `admission.rs` | Capacity, lookup, removal, slot reuse and creatures | 170 | 8 |
| `continuity_visibility.rs` | Namespace claims, fencing and visible actors | 211 | 7 |
| `corpse_projection.rs` | Corpse retention, reward occurrences and pins | 155 | 3 |
| `respawn.rs` | Occupied index, respawn and timer binding | 290 | 5 |
| `runtime_assignment.rs` | Assignment, reservations, control loss and entry | 453 | 8 |
| `spawn_activation.rs` | Activation, interest and initial realization | 310 | 10 |
| `spawn_backlog.rs` | Death backlog, lookup, reuse and placement | 249 | 5 |
| `spawn_limits.rs` | Per-source and per-channel ceilings | 141 | 3 |
| `spawn_mixed_sources.rs` | Per-point source facts and scale | 312 | 4 |
| `spawn_owner_windows.rs` | Bounded activation, scans and respawn windows | 343 | 6 |
| `support.rs` | Unchanged continuity, runtime, corpse and spawn helpers | 319 | 0 |

The original file had 2,966 lines and 59 tests. Every new file is below
600 lines; the largest has 453. Topic files use `include!` within the original
`carrier::tests` module, preserving all test paths and helper visibility.
In particular, existing sibling consumers still reach `tests::runtime` and
`tests::session`. Includes are sorted by name and mount each file exactly once.

## Test inventory and move proof

The owner's exact source-attribute count command was run before and after:

```sh
grep -rE '#\[(tokio::)?test' crates/gameplay/entities | wc -l
```

Result: **170 before / 170 after**, including **59 before / 59 after** in
`carrier::tests`. No tests were added, renamed, deleted, ignored or weakened.

Local evidence is under ignored `target/testsc/`: original `tests.before.rs`
and `carrier.before.rs`, per-item `mapping.json`, `pure-move.log`, before/after
library inventories and command logs. Strict comparison accounts for every
original function and constant exactly once, including comments and attributes.
Only leading blank lines at new file boundaries are removed. The parent source
matches its original exactly after replacing the test module path. No production
API or data shape changed, so no constructor, match or impl updates are needed.
Existing fixture references were searched across `apps/`, `crates/` and `tools/`.

Compiled library inventories are **170 before / 170 after**. The complete
sorted test-path diff and the diff after stripping module prefixes are both
empty. All **59** relocated tests retain their exact original paths.

## Validation

All Cargo invocations use `/home/mole/oteryn-cache/codex-bin/cargo`. Build and
test commands add `--locked --offline` to prevent dependency downloads.

The following required checks all passed; full logs are in `target/testsc/`:

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-entities --all-targets --all-features -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-entities --lib --all-features
```

Library tests: **170 passed, 0 failed, 0 ignored, 0 filtered out**. Formatting
and all-target/all-feature clippy with `-D warnings` passed without repairs.
Additional required repository checks also passed:

- `python3 tools/agents/validate_governance.py`: 22 documents, 9 lanes.
- `python3 -m unittest discover -s tools/agents/tests`: 129 tests.
- `git diff --check` and whitespace review of all new files.
- Whole-diff self-review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`:
  no findings; strict move proof covers every unchanged test and helper, with
  no new API surface or production behavior changes.

The exact focused filter for all relocated cases is `carrier::tests::` with
the same package, library and feature flags. The required full library test
command covers the moved cases and unchanged sibling consumers. This crate
contains no PostgreSQL cases, so no unrelated PostgreSQL suite is run.

## Unfinished

Nothing remains unfinished. Changes remain uncommitted as requested.
