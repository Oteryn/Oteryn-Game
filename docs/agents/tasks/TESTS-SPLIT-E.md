# TESTS-SPLIT-E — split inline content and companion lifecycle tests

```yaml
task_id: TESTS-SPLIT-E
mode: IMPLEMENT
status: local_validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Scope

Pure relocation of the inline unit tests in `production.rs`, the Linux publication
tests in `project_fs.rs`, and both test modules in `companion_lifecycle.rs`.
All 50 test definitions retain their names, bodies, assertions and attributes.
No dependencies, production behavior, visibility, enums, fields or trait methods
are added or changed. The existing test-only population helper also moves; the
helpers shared with the production `test-support` feature stay in place.

Owned paths: the three source files below, their sibling `tests/` directories,
this task card, and the requested root `testse-report.md`. No commit, staging,
publication, external review or PostgreSQL suite is part of this task.
Bound META policy 3.1.0 was resolved read-only at commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

The requested `TESTS-SPLIT-B.md` template is absent from this checkout (and its
path has no local Git history). This card follows the existing
`CONN-TESTS-SPLIT-1.md` format with the requested scope, mapping, validation,
inventory and line-count sections.

## File mapping and line counts

Original ranges refer to the saved pre-move source files. `production/tests/`
ranges refer to `production.rs`, `project_fs/tests/` to `project_fs.rs`, and
`companion_lifecycle/tests/` to `companion_lifecycle.rs`.

| New file | Original lines / responsibility | Lines |
| --- | --- | ---: |
| `crates/gameplay/content/src/content/production/tests/compiler.rs` | 3299–3490 | 191 |
| `crates/gameplay/content/src/content/production/tests/integrity.rs` | 3973–4224 | 252 |
| `crates/gameplay/content/src/content/production/tests/limits.rs` | 3630–3785 | 155 |
| `crates/gameplay/content/src/content/production/tests/mod.rs` | thin import/include mount | 8 |
| `crates/gameplay/content/src/content/production/tests/population.rs` | 3109–3123 | 15 |
| `crates/gameplay/content/src/content/production/tests/references.rs` | 3786–3972 | 186 |
| `crates/gameplay/content/src/content/production/tests/staging.rs` | 3491–3629 | 138 |
| `crates/gameplay/content/src/content/project_fs/tests/linux.rs` | thin import/include mount | 1 |
| `crates/gameplay/content/src/content/project_fs/tests/publication.rs` | 3049–3623 | 575 |
| `crates/gameplay/spell/src/spell/companion_lifecycle/tests/cooldown_source.rs` | 3040–3067 | 28 |
| `crates/gameplay/spell/src/spell/companion_lifecycle/tests/familiar_login_lifecycle.rs` | thin import/include mount | 6 |
| `crates/gameplay/spell/src/spell/companion_lifecycle/tests/lifecycle.rs` | 3403–3748 | 346 |
| `crates/gameplay/spell/src/spell/companion_lifecycle/tests/login.rs` | 3075–3372 | 297 |
| `crates/gameplay/spell/src/spell/companion_lifecycle/tests/support.rs` | 3373–3402 | 29 |

| Original source | Before | After |
| --- | ---: | ---: |
| `crates/gameplay/content/src/content/production.rs` | 4,225 | 3,281 |
| `crates/gameplay/content/src/content/project_fs.rs` | 3,625 | 3,051 |
| `crates/gameplay/spell/src/spell/companion_lifecycle.rs` | 3,749 | 3,044 |

Every new Rust file is below 600 lines (largest: 575). Original child module names
and full test paths are preserved through thin `#[cfg(test)]` / `#[path]` mounts.
Topic fragments use `include!` in the original test-module scope, preserving
private access and `super::` references without visibility changes or body reflow.
The nested Linux module uses `#[cfg_attr(test, path = "project_fs")]` solely to
resolve its child test file under `project_fs/tests/`; this attribute is absent
from production builds. The familiar profile `include_str!` gains two `../`
segments to resolve the same existing JSON fixture from its new file.

## Test inventory and move proof

Local evidence is retained under ignored `target/testse-evidence/`:

- `*.rs.before`, `mapping.json`, `pure-move.log`: original source, exact ranges,
  and full comparison of all extracted content, allowing only indentation and
  the fixture path adjustment. Remaining production text is identical after
  excluding test-only items, module mounts and the test-only path attribute.
- `content.before.tests`, `content.after.tests`, `spell.before.tests`,
  `spell.after.tests`: sorted library inventories.
- `*.before.names`, `*.after.names`: sorted inventories with module prefixes
  stripped; `*.full.diff` and `*.names.diff` are empty.
- `constructor-sites.txt`: workspace searches across `apps/`, `crates/`, `tools/`
  for moved helper/fixture types and their existing constructors/callers. No new
  enum variants, struct fields, trait methods or production functions exist.

Each before/after inventory uses the requested exact command (once per crate
and phase):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p <crate> --lib -- --list 2>/dev/null | grep ': test' | sort
```

| Crate | Before | After | Full-path diff | Prefix-stripped diff |
| --- | ---: | ---: | --- | --- |
| `oteryn-gameplay-content` | 346 | 346 | empty | empty |
| `oteryn-gameplay-spell` | 224 | 224 | empty | empty |

Moved test coverage selectors, all included in the required complete library run:

- `content::production::tests::` — 24 tests.
- `content::project_fs::linux::tests::` — 12 tests.
- `spell::companion_lifecycle::cooldown_source_tests::` — 1 test.
- `spell::companion_lifecycle::familiar_login_lifecycle_tests::` — 13 tests.

## Validation

All Cargo commands use `/home/mole/oteryn-cache/codex-bin/cargo` with offline
resolution; no dependencies were downloaded.

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --locked --offline -p oteryn-gameplay-content -p oteryn-gameplay-spell --all-targets -- -D warnings`: PASS.
- `cargo test --locked --offline -p oteryn-gameplay-content -p oteryn-gameplay-spell --lib`: PASS; content 345 passed / 1 existing ignored, spell 224 passed. All 50 moved tests passed.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `git diff --check`, including new-file whitespace checks: PASS.
- Full move proof: PASS, 50 test definitions preserved, no production behavior change.
- Whole-change review against the owner's `CODEX-CHECKLIST.md`: PASS; no new
  runtime/public API, authorization, persistence, protocol or CI surface.

## Unfinished

None. Changes remain uncommitted as requested. The content suite retains its
existing ignored full-manifest test, unchanged by this move.
