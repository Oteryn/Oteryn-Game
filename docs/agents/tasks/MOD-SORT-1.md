# MOD-SORT-1 — sorted mechanic registration blocks

```yaml
task_id: MOD-SORT-1
mode: REFACTOR
status: validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Summary

Consolidate protocol and durability module declarations into marked alphabetical
blocks, and expand grouped entities re-exports to one item per statement.
Preserve module visibility, attributes, public paths and runtime behavior.

## Reason

STRUCTURE-TARGET-1's owner-supplied pairwise three-way merge simulation of the last
60 mechanic PRs found 25 conflicts in 58 protocol `lib.rs` pairs (18 were not pure
inserts, with registrations split between a top block and end-of-file appends),
and nine conflicts in 25 durability `mod.rs` pairs (all end-of-file appends).
A single alphabetical block in `core_modules.rs` produced three conflicts in
68 pairs. LOOK-2 and PARTY-XP-1 also conflicted on shared entities re-export lines.
This change addresses the measured merge-conflict benefit without fragmentation.

## Scope

Owned paths:

- `crates/protocol-oteryn/src/lib.rs`
- `crates/protocol-oteryn/src/tests/lib_tests.rs`
- `crates/gameplay/durability/src/durability/mod.rs`
- `crates/gameplay/entities/src/lib.rs`
- `AGENTS.md`: one registration convention in the existing mechanic section.
- This task card.

Source-reading unit tests enforce sorted declarations and reject registrations
outside the marked blocks. Existing test-module and fuzz-support attributes stay
attached. Entities re-export attributes apply to every corresponding item.

## Excluded scope

No behavior, wire format, SQL, persistence, dependencies, public API or test
weakening changes. Do not edit durability `core_modules.rs` or its include,
repository tooling, CI or unrelated sources. No commit, publication or external
review. No PostgreSQL mechanic is changed; unrelated database suites do not run.

## Validation

Passed locally using the queued cargo wrapper at
`/home/mole/oteryn-cache/codex-bin/cargo` (no dependency downloads):

```sh
/home/mole/.cargo/bin/cargo fmt -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-entities
/home/mole/.cargo/bin/cargo fmt -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-entities --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-protocol-oteryn -p oteryn-gameplay-durability -p oteryn-gameplay-entities --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-protocol-oteryn -p oteryn-gameplay-entities --features oteryn-gameplay-entities/test-support --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib --quiet -- durability:: character_bootstrap_intent:: appearance_command_replay_tests:: --skip postgres --skip replacement_reconciliation_requires_receipt_authorization_when_request_omits_it --skip replacement_prepare_requires_receipt_authorization_when_request_omits_it --skip legacy_prepare_rejects_receipt_backed_replacement_candidate --skip terminal_replacement_preserves_committed_predecessor_reconciliation
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

Results: 416 protocol, 168 entities and 381 durability unit tests passed.
The durability command excludes 55 database tests; no PostgreSQL code changed
and no PostgreSQL tests were executed. The new guards are
`tests::module_registration_block_is_sorted_and_complete` (protocol) and
`durability::module_registration_block_is_sorted_and_complete` (durability).
The entities test command enables its existing `test-support` feature because
the default isolated test build lacks `ChannelContentPin::test` fixtures.
All-target Clippy exits successfully; existing dependency warnings remain in
vendored Tokio and SQLx. Governance validation and all 129 governance unit tests
passed.

Compared inventories against `HEAD`: all 51 protocol modules, 19 durability
modules and 108 entities re-exports preserve visibility and attached attributes.
Durability `core_modules.rs` and its include are unchanged. Re-read the whole
diff against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: no new public functions,
enum variants, struct fields or trait methods, no behavior or protocol changes,
and no existing tests deleted or weakened.

## Unfinished

None within the requested local scope. Changes remain uncommitted as requested.
