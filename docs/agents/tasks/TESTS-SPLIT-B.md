# TESTS-SPLIT-B — extract inline gameplay tests

## Task

Owner-approved structural refactor: move inline test modules from five large files into sibling test files, split by topic, without changing behaviour or test names. Local edits only; no commit, publication or PR was requested.

## Scope and owned paths

The five source files below, their new sibling test files/directories, and this task card. No integration test targets, CI configuration, manifests, dependencies or off-limits connection files changed.

## Mapping: old module → new location

Destinations in each group are relative to the source file's directory. Topic files are included into the original module scope; they introduce no new module segment, visibility, public function, enum variant, struct field or trait method.

### `crates/gameplay/durability/src/durability/core_modules.rs`

| Original inline module | New sibling file |
| --- | --- |
| All 22 original `*_linkage` modules | `core_modules_linkage_tests.rs` (original nested module names retained) |
| `wp3_error_classification_tests` | `core_modules_wp3_error_classification_tests.rs` |
| `terminal_replacement_schema_red_tests` | `core_modules_terminal_replacement_schema_red_tests.rs` |
| `terminal_replacement_foundation_red_tests` | `core_modules_terminal_replacement_foundation_red_tests.rs` |

`core_modules_terminal_replacement_foundation_red_tests.rs` includes:

- `core_modules_tests/terminal_replacement_foundation_red_tests_authority.rs`
- `core_modules_tests/terminal_replacement_foundation_red_tests_revalidation.rs`
- `core_modules_tests/terminal_replacement_foundation_red_tests_terminal_outcomes.rs`

### `crates/gameplay/durability/src/durability/db.rs`

| Original inline module | New sibling file |
| --- | --- |
| `wp3_root_contract_tests` | `db_wp3_root_contract_tests.rs` |
| `runtime_scope_identity_red_tests` | `db_runtime_scope_identity_red_tests.rs` |

`db_runtime_scope_identity_red_tests.rs` includes:

- `db_tests/runtime_scope_identity_red_tests_support.rs`
- `db_tests/runtime_scope_identity_red_tests_current_authority.rs`
- `db_tests/runtime_scope_identity_red_tests_reconciliation.rs`

### `crates/gameplay/durability/src/durability/schema.rs`

| Original inline module | New sibling file |
| --- | --- |
| `contract_tests` | `schema_contract_tests.rs` |
| `terminal_replacement_postgres_red_tests` | `schema_terminal_replacement_postgres_red_tests.rs` |

`schema_terminal_replacement_postgres_red_tests.rs` includes:

- `schema_tests/terminal_replacement_postgres_red_tests_support.rs`
- `schema_tests/terminal_replacement_postgres_red_tests_scope_and_replay.rs`
- `schema_tests/terminal_replacement_postgres_red_tests_mutation_safety.rs`
- `schema_tests/terminal_replacement_postgres_red_tests_concurrency_and_reconciliation.rs`

### `crates/gameplay/foundation/src/foundation/admission.rs`

| Original inline module | New sibling file |
| --- | --- |
| `tests` | `admission_tests.rs` |

`admission_tests.rs` includes:

- `admission_tests/tests_support.rs`
- `admission_tests/tests_fresh_and_encoding.rs`
- `admission_tests/tests_reconnect_fences.rs`
- `admission_tests/tests_reconnect_history.rs`

### `crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs`

| Original inline module | New sibling file |
| --- | --- |
| `tests` | `actor_spell_tests.rs` |
| `source_native_vitals_tests` | `actor_spell_source_native_vitals_tests.rs` |
| `owned_source_tick_tests` | `actor_spell_owned_source_tick_tests.rs` |
| `source_creature_dot_owner_tests` | `actor_spell_source_creature_dot_owner_tests.rs` |

`actor_spell_tests.rs` includes:

- `actor_spell_tests/tests_support.rs`
- `actor_spell_tests/tests_vitals.rs`
- `actor_spell_tests/tests_casting.rs`
- `actor_spell_tests/tests_damage_reservations.rs`
- `actor_spell_tests/tests_death_and_respawn.rs`

The native spell profile fixture reference gained one `../` in `tests_damage_reservations.rs` to resolve to the same existing JSON file. The `tests` module retains `pub` and `cfg(any(test, feature = "test-support"))`.

## Changed files

```text
crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_owned_source_tick_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_source_creature_dot_owner_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_source_native_vitals_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests/tests_casting.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests/tests_damage_reservations.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests/tests_death_and_respawn.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests/tests_support.rs
crates/gameplay/dispatch/src/gameplay_transport/actor_spell_tests/tests_vitals.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/core_modules_linkage_tests.rs
crates/gameplay/durability/src/durability/core_modules_terminal_replacement_foundation_red_tests.rs
crates/gameplay/durability/src/durability/core_modules_terminal_replacement_schema_red_tests.rs
crates/gameplay/durability/src/durability/core_modules_tests/terminal_replacement_foundation_red_tests_authority.rs
crates/gameplay/durability/src/durability/core_modules_tests/terminal_replacement_foundation_red_tests_revalidation.rs
crates/gameplay/durability/src/durability/core_modules_tests/terminal_replacement_foundation_red_tests_terminal_outcomes.rs
crates/gameplay/durability/src/durability/core_modules_wp3_error_classification_tests.rs
crates/gameplay/durability/src/durability/db.rs
crates/gameplay/durability/src/durability/db_runtime_scope_identity_red_tests.rs
crates/gameplay/durability/src/durability/db_tests/runtime_scope_identity_red_tests_current_authority.rs
crates/gameplay/durability/src/durability/db_tests/runtime_scope_identity_red_tests_reconciliation.rs
crates/gameplay/durability/src/durability/db_tests/runtime_scope_identity_red_tests_support.rs
crates/gameplay/durability/src/durability/db_wp3_root_contract_tests.rs
crates/gameplay/durability/src/durability/schema.rs
crates/gameplay/durability/src/durability/schema_contract_tests.rs
crates/gameplay/durability/src/durability/schema_terminal_replacement_postgres_red_tests.rs
crates/gameplay/durability/src/durability/schema_tests/terminal_replacement_postgres_red_tests_concurrency_and_reconciliation.rs
crates/gameplay/durability/src/durability/schema_tests/terminal_replacement_postgres_red_tests_mutation_safety.rs
crates/gameplay/durability/src/durability/schema_tests/terminal_replacement_postgres_red_tests_scope_and_replay.rs
crates/gameplay/durability/src/durability/schema_tests/terminal_replacement_postgres_red_tests_support.rs
crates/gameplay/foundation/src/foundation/admission.rs
crates/gameplay/foundation/src/foundation/admission_tests.rs
crates/gameplay/foundation/src/foundation/admission_tests/tests_fresh_and_encoding.rs
crates/gameplay/foundation/src/foundation/admission_tests/tests_reconnect_fences.rs
crates/gameplay/foundation/src/foundation/admission_tests/tests_reconnect_history.rs
crates/gameplay/foundation/src/foundation/admission_tests/tests_support.rs
docs/agents/tasks/TESTS-SPLIT-B.md
```

## Structural review evidence

- Original bodies saved under `target/TESTS-SPLIT-B/*-original.rs` before editing.
- Expanding every new mount matches the original Rust tokens and literal contents after formatting; relative embedded-file paths are compared by their resolved original resource. See `source-equivalence.log` and `verify_move.py`.
- Separate byte comparisons after removing old test bodies/new mounts confirm unchanged production code and original module attributes in all five sources. The complete ordered `pub mod` list in `core_modules.rs` is identical.
- 32 new test files; largest is 658 lines. No repository-wide Rust file-size guard was found under `tools/`; the task's stricter 800-line threshold was checked for every new file.
- No test renamed, ignored, skipped, deleted, or assertion weakened. PostgreSQL tests retain the same durability library mounts used by CI.

## Validation evidence

Inventories were generated before and after using the absolute cargo wrapper. Lists contain every libtest name (`: test`/`: benchmark`), sorted before comparison. Raw output, sorted lists, and empty `*-inventory.diff` files are in `target/TESTS-SPLIT-B/`. No integration target was changed.

| Crate | Before | After | Moved tests | Inventory diff |
| --- | ---: | ---: | ---: | --- |
| `oteryn-gameplay-durability` | 435 | 435 | 110 | empty |
| `oteryn-gameplay-foundation` | 335 | 335 | 41 | empty |
| `oteryn-gameplay-dispatch` | 1032 | 1032 | 35 | empty |

Exact inventory commands (each run once before extraction and once successfully after extraction; the initial dispatch after-list failed on the relocated fixture path and was rerun after its repair):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- --list
diff -u target/TESTS-SPLIT-B/durability-before.list target/TESTS-SPLIT-B/durability-after.list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-foundation --lib -- --list
diff -u target/TESTS-SPLIT-B/foundation-before.list target/TESTS-SPLIT-B/foundation-after.list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib -- --list
diff -u target/TESTS-SPLIT-B/dispatch-before.list target/TESTS-SPLIT-B/dispatch-after.list
```

Exact formatting and lint commands:

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-durability -p oteryn-gameplay-foundation -p oteryn-gameplay-dispatch --all-targets -- -D warnings
```

Formatting and Clippy passed for all three touched crates and all targets (`-D warnings`). New test files were additionally formatted directly with Rustfmt; original production formatting was preserved.

Exact filtered test commands (multiple libtest filters are ORed; every selected test belongs to the moved modules, with one test thread to limit PostgreSQL contention):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib -- \
  durability::account_achievement_linkage:: \
  durability::character_last_position_linkage:: \
  durability::bestiary_progress_linkage:: \
  durability::character_progression_linkage:: \
  durability::charm_state_linkage:: \
  durability::character_build_linkage:: \
  durability::monk_state_linkage:: \
  durability::character_wheel_linkage:: \
  durability::premium_fence_linkage:: \
  durability::item_mint_linkage:: \
  durability::item_transfer_linkage:: \
  durability::item_decay_retire_linkage:: \
  durability::item_fee_burn_linkage:: \
  durability::reward_claim_mint_linkage:: \
  durability::fresh_admission_composition_linkage:: \
  durability::character_forge_dust_linkage:: \
  durability::recovery_evidence_composition_linkage:: \
  durability::native_admission_source_linkage:: \
  durability::account_characters_projection_linkage:: \
  durability::public_projections_linkage:: \
  durability::character_authority_linkage:: \
  durability::runtime_scope_assignment_linkage:: \
  durability::wp3_error_classification_tests:: \
  durability::terminal_replacement_schema_red_tests:: \
  durability::terminal_replacement_foundation_red_tests:: \
  durability::db:: \
  durability::schema:: \
  --nocapture --test-threads=1

/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-foundation --lib -- \
  foundation::admission::tests:: \
  --nocapture --test-threads=1

/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib -- \
  gameplay_transport::actor_spell::tests:: \
  gameplay_transport::actor_spell::source_native_vitals_tests:: \
  gameplay_transport::actor_spell::owned_source_tick_tests:: \
  gameplay_transport::actor_spell::source_creature_dot_owner_tests:: \
  --nocapture --test-threads=1

```

Other checks:

```sh
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 target/TESTS-SPLIT-B/verify_move.py
git diff --check
```

Governance validator passed (22 policy documents / 9 project lanes), governance unit tests passed (129). The final diff is reviewed against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`; no new production API or data type was introduced, so constructor/match/impl propagation and new-dispatch-caller requirements are inapplicable to this verbatim move.

Filtered results:

- `durability` moved-module run: 110 passed, 0 failed, 0 ignored (150.44s).
- `foundation` moved-module run: 41 passed, 0 failed, 0 ignored (0.01s).
- `dispatch` moved-module run: 35 passed, 0 failed, 0 ignored (0.08s).

The durability PostgreSQL cases ran through the absolute wrapper with module filters; the durability run took 150.44 seconds, and there were no missing-route messages. No whole PostgreSQL suite was executed.

## Unfinished

None. Changes remain local and uncommitted as requested.
