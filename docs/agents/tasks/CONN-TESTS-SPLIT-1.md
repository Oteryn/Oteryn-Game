# CONN-TESTS-SPLIT-1 — split connection unit tests by topic

```yaml
task_id: CONN-TESTS-SPLIT-1
mode: IMPLEMENT
status: local_validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Scope

Pure relocation of the inline tests and test-only lifecycle helper from
`crates/gameplay/dispatch/src/gameplay_transport/connection.rs`. No dependency,
production behavior, protocol, persistence, enum, field or trait-method additions.
Production source is byte-identical after excluding the test-only items.

Owned paths: `connection.rs`, the new files below its `connection/` directory,
and this task card. Existing capability, appearance, item-look and included test
sources remain unchanged. No commit, publication or external review requested.

## File mapping and line counts

Paths below are relative to `crates/gameplay/dispatch/src/gameplay_transport/`.
Original ranges refer to the saved, 8,695-line `connection.rs`.

| New file | Original lines / responsibility | Lines |
| --- | --- | ---: |
| `connection/test_support.rs` | 1409–1429: existing test-only `serve_connection` | 23 |
| `connection/tests.rs` | 4831–4847, 5405–5408: imports and test mounts | 36 |
| `connection/tests/support.rs` | 4949–4953, 5054–5201, 5410–5622 and shared spell fixtures from 6984–7071 | 453 |
| `connection/tests/use_capabilities.rs` | 4848–4948: use disposition gates | 100 |
| `connection/tests/pending_spell.rs` | 4954–5053: unresolved cast outcome | 99 |
| `connection/tests/bank_resume.rs` | 5202–5404: retained bank command | 202 |
| `connection/tests/liveness.rs` | 5623–5906: probes, acknowledgements, refusal bounds | 283 |
| `connection/tests/steps_pacing.rs` | 5907–6302: sequencing and step pacing | 395 |
| `connection/tests/admission.rs` | 6303–6538: admission and invalid commands | 235 |
| `connection/tests/overlay.rs` | 6539–6728: join overlays and use deltas | 189 |
| `connection/tests/use_commit.rs` | 6729–6983: write failure, fence and disposition | 254 |
| `connection/tests/spell_vitals.rs` | 7057–7336, excluding shared vitals constant: cadence and cast | 269 |
| `connection/tests/spell_resume.rs` | 7337–7533: missing vitals base and resume | 196 |
| `connection/tests/achievements_highscores.rs` | 7534–7805: account queries and highscores | 271 |
| `connection/tests/notices.rs` | 7806–8105: achievement notices and reconnect | 300 |
| `connection/visibility_tests.rs` | 8112–8365: existing visibility fixtures and appearance mount | 255 |
| `connection/visibility_tests/interest_set.rs` | 8366–8690: spatial visibility and resume | 325 |

`connection.rs`: **8,695 → 4,825 lines**. Every new Rust file is under
600 lines; the largest is 453 lines. The existing item-look declaration stays
in `connection.rs`. The lifecycle helper retains its existing test-only public
path through a gated re-export.

Topic files use `include!` in their original test module, preserving module
paths, private access, attributes and test bodies verbatim except indentation.
The original includes remain mounted with adjusted relative paths. Shared
fixtures live in `tests/support.rs` with `pub(super)` access; spatial and spell
references use module imports at the new nesting depth. No production item
visibility changes. Visibility fixtures retain their original module and are
shared with the existing appearance tests.

## Test inventory and move proof

Evidence is saved locally under ignored `target/conn-tests-split-1/`:

- `before.raw`, `after.raw`: output from
  `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib -- --list`.
- `before.tests`, `after.tests`: lines matching `: test`, sorted.
- `before.names`, `after.names`: sorted inventories with module paths removed.
- `test-list.diff`, `test-name.diff`: inventory comparisons.
- `connection.before.rs`: original source; `pure-move.log`: source comparison.

Before/after: **1,032 / 1,032** library tests, including **73 / 73** connection
tests. Both full-path and path-stripped inventories are identical. All **50**
directly moved test definitions (attributes, names, signatures and bodies) are
identical except indentation. The other 23 connection tests remain mounted from
their existing source files. The test-only lifecycle helper is byte-identical.
Workspace fixture constructor/impl searches are saved as `fixture-sites.txt`;
there are no newly introduced variants, fields, methods or production functions.

## Validation

All Cargo commands use the absolute queued wrapper; no dependencies are
downloaded. The only executed
dispatch library test filter is `gameplay_transport::connection`, so no unrelated
PostgreSQL suite runs.

- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets -- -D warnings`: PASS.
- `cargo test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::connection`:
  PASS, 73 passed, 0 failed, 0 ignored, 959 filtered out.
- Both complete library inventories: 1,032 tests; full-path and path-stripped
  diffs empty. The listing commands enumerate tests without executing them.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, 129 tests.
- `cargo run --locked -p oteryn-architecture-check --quiet -- workspace .`: PASS.
- New Rust source size check: PASS, all 17 files below 600 lines.
- `git diff --check`, including separate whitespace checks of new files: PASS.
- Whole-change self-review against the owner's `CODEX-CHECKLIST.md`: PASS;
  production text, test definitions, original included sources and fixture
  constructor/impl sites checked. No tests weakened or added to ignored paths.

## Unfinished

None. Changes remain uncommitted as requested.
