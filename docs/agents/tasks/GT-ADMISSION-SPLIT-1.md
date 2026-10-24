# GT-ADMISSION-SPLIT-1 — composed admission by mechanic

## Task

Owner-authorized structural split of the gameplay transport admission hotspot. Local authoring
only; no commit, publication, CI mutation or external review requested by this worker.

## Scope and owned paths

- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- New `gameplay_transport/composed_admission/`, `gameplay_transport/tests.rs` and
  `gameplay_transport/tests/` files under the same dispatch source directory.
- This task card. `connection.rs`, `connection/**` and other lanes remain untouched.

No behavior, wire format, SQL, error text, side-effect ordering or external public API changes.
The existing `serving` module and `serve_gameplay` re-export remain unchanged.

## Old → new mapping

All paths below are relative to `crates/gameplay/dispatch/src/gameplay_transport/`.

| Former location in `mod.rs` | New location |
| --- | --- |
| `impl ComposedFreshAdmission`: fresh admission/reservation/reconciliation | `composed_admission/admission.rs` |
| First-entry initialization | `composed_admission/first_entry.rs` |
| Death settlement and respawn | `composed_admission/death.rs` |
| Training checkpoint flushing and adoption | `composed_admission/training.rs` |
| Familiar save at actor end | `composed_admission/familiar.rs` |
| Quest admission and refresh | `composed_admission/quest.rs` |
| Wheel admission and stages | `composed_admission/wheel.rs` |
| Progression session binding | `composed_admission/progression.rs` |
| Control-loss holds, fences and retirement | `composed_admission/control_loss.rs` |
| Grace release/retries and expiry | `composed_admission/grace_expiry.rs` |
| Terminal/abandoned release and capability mismatch | `composed_admission/release.rs` |
| Premium session lifecycle | `composed_admission/premium.rs` |
| Map/position/entity observations | `composed_admission/world_map.rs` |
| Object reach, chest/door use and overlay | `composed_admission/objects.rs` |
| Spell dispatch and completion | `composed_admission/spells.rs` |
| Vitals observations and cadence | `composed_admission/vitals.rs` |
| Attack target, modes and combat state/cadence | `composed_admission/combat.rs` |
| Achievement notices and account query | `composed_admission/achievements.rs` |
| NPC conversation open/follow | `composed_admission/npc.rs` |
| Resume composition | `composed_admission/resume.rs` |
| Committed item receipt lookup | `composed_admission/items.rs` |
| Inline `tests` module | `tests.rs`, with six textual includes under `tests/` |
| Inline `death_ml_loss_postgres` module | `tests/death_ml_loss_postgres.rs`, mounted at its original module path |

`composed_admission/mod.rs` mounts the mechanic files. Existing already-delegated trait methods
and mechanic macros stay in the single compact `FreshAdmissionAuthority` impl in `mod.rs`;
21 substantial bodies now delegate to inherent helpers. The 44 pre-existing inherent methods
keep their names/signatures and effective visibility. No enum variants, struct fields or trait
methods are added. Textual test includes preserve shared private fixtures and full test paths. The six files are
`tests/admission_results.rs`, `tests/objects.rs`, `tests/listener.rs`,
`tests/session_authority.rs`, `tests/lifecycle.rs` and `tests/death.rs`.

## Validation evidence

Evidence directory: `target/GT-ADMISSION-SPLIT-1/` (local build artifacts, not committed).
Bound META 3.1.0 was resolved at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb` and
verified against protected META main before source edits.

- Baseline: `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib -- --list`
  saved to `before.txt`: 1,032 tests, including 32 `gameplay_transport::tests::` and
  two `gameplay_transport::death_ml_loss_postgres::` tests.
- After: the same inventory command saved to `after.txt`: 1,032 tests, again 32 moved unit and
  two moved PostgreSQL tests. `diff -u target/GT-ADMISSION-SPLIT-1/before.txt target/GT-ADMISSION-SPLIT-1/after.txt`
  passed with an empty `inventory.diff`; all names and counts are identical.
- `/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-gameplay-dispatch --quiet`: passed.
- `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`: passed.
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::tests:: --quiet`:
  32 passed, 0 failed, 0 ignored, 1,000 filtered out; includes real TLS listener/admission/lifecycle dispatch tests.
- `/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib gameplay_transport::death_ml_loss_postgres:: --quiet`:
  2 passed, 0 failed, 0 ignored, 1,030 filtered out; PostgreSQL invoked through the absolute wrapper.
  No whole PostgreSQL suite was run.
- `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check`: passed.
- `python3 tools/agents/validate_governance.py`: passed (22 policy documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: 129 passed.
- `git diff --check`: passed.
- `python3 target/GT-ADMISSION-SPLIT-1/verify_move.py`: source equivalence passed for all 44
  moved inherent bodies/signatures, 21 delegated trait bodies/signatures and their ordered
  arguments, both test modules, and every unrelated top-level item. Formatting differences
  are ignored. Two `include_bytes!` paths gained one `../` and resolve to the identical existing
  `tools/content-schema/native-gameplay/build-training.json` fixture. No assertions changed.
- Workspace `rg` of all 21 new helpers confirms their production callers in the trait impl;
  matches saved to `helper-call-sites.txt`. No new public API, enum variant, struct field or
  trait method requires consumer changes. The existing public inherent methods remain public.
- Full structural diff reviewed against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`:
  no new SQL/writes, transport behavior, deadlines, capability gates, protocol codes or
  side-effect changes; no tests removed, renamed, ignored or weakened.
- `mod.rs`: 7,436 → 2,131 lines. Thirty new Rust files; largest is 564 lines
  (`tests/listener.rs`), largest mechanic file is 438 lines (`composed_admission/admission.rs`).
  No standalone source-file-size guard was found in the repository check tools.

The first test-target clippy attempt identified the two relative fixture paths; they were
corrected before the successful final clippy, inventory and test runs. Logs are retained in the
evidence directory. Existing vendored Tokio/SQLx warnings were emitted without failing checks.

## Unfinished

None for this local structural task. No publication performed; the owner retains the local diff
for integration and repository CI. No off-lane files were changed.
