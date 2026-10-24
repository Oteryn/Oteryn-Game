# PROTO-LIB-SPLIT-1

```yaml
task_id: PROTO-LIB-SPLIT-1
title: Split protocol crate root tests without changing behavior
mode: REPAIR
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: null
base_sha: 670633ad93893a7d7e82bec64481ed878d8cc1a7
head_sha: null
owner: Codex standalone task worker, owner-allocated split-protolib worktree
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/lib.rs
  - crates/protocol-oteryn/src/tests/** (new files only)
  - crates/protocol-oteryn/build*.rs (only if needed)
  - crates/protocol-oteryn/src/ (new sibling modules only)
  - docs/agents/tasks/PROTO-LIB-SPLIT-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Task and scope

Owner-authorized structural refactor only: remove the inline root test module and
avoid shared capability-allocation edits. Preserve every public path, wire value,
assertion and full test path. No publication or Git mutations are authorized;
the worktree arrived detached at the base SHA above.

PROVEN: META policy 3.1.0 was read from the immutable binding commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb` before material work.

## Architecture and source of truth

PROVEN: the capability allocation goal is already satisfied on this baseline.
`src/registered_capabilities.rs` includes build-generated registration and
acceptance-requirement tables; `build_capabilities.rs::render` derives them from
`capabilities/*.json`, sorted by ID with one entry per line. `build.rs` scans the
fragments and checks them against the owning registry. No shared allocation
comment or next-unallocated line remains in `lib.rs`.

Existing fragment tests guard duplicate/unknown/missing IDs, deterministic output,
requirements and ownership, golden facts and fragment-only additions. Retain those
checks and the existing generator unchanged. Do not add a redundant next-ID test.

High-risk authority/recovery qualification: NOT_APPLICABLE; verbatim test moves
only, with no production behavior or authority change.
Reference parity and dispatch E2E: NOT_APPLICABLE; no mechanic, new public function,
enum variant, struct field or trait method is introduced.

## Mapping old to new locations

`src/lib.rs::tests` is mounted using `#[path = "tests/lib_tests.rs"] mod tests;`.
Topic files use `include!` in that same module, preserving `tests::<name>`.

| Previous location in src/lib.rs | New location under src/tests/ |
|---|---|
| tests module declaration/imports | lib_tests.rs |
| Independent vectors, encoder seams, snapshots and frame boundaries (2567–2995) | codec_seam_tests.rs |
| Shared wire fixture helpers (2997–3161) | support.rs |
| Handshake semantics and capability/build limits (3163–3423) | handshake_tests.rs |
| Command limits, liveness, resync and bootstrap bounds (3425–3703) | client_message_tests.rs |
| Envelope semantics and wire identifier boundaries (3705–3802) | envelope_tests.rs |
| Error/disposition/registry, refusal classes and UUID semantics (3804–4110) | error_registry_tests.rs |
| Command fixture, command/result codecs and liveness round trip (4112–4464) | command_tests.rs |
| State delta codecs and snapshot begin target sequence (4466–4540) | state_sync_tests.rs |

The existing `tests::capability_selection_tests` child module and all golden JSON
files are unchanged. New files are at most 428 lines; `lib.rs` shrinks from 4,553 to 2,575 lines.

## Validation

Evidence is stored locally under `target/PROTO-LIB-SPLIT-1/` (ignored artifacts).
All Cargo commands use `/home/mole/oteryn-cache/codex-bin/cargo`; no dependency
installation, PostgreSQL suite, manifest or lockfile changes are needed.

- Before inventory: `cargo test --locked -p oteryn-protocol-oteryn --lib --quiet -- --list`
  saved to `before-list.txt`: pass, 414 tests.
- After inventory: same command, saved to `after-list.txt`: pass, 414 tests.
- `diff -u target/PROTO-LIB-SPLIT-1/before-list.txt target/PROTO-LIB-SPLIT-1/after-list.txt`: pass, empty diff; identical full paths.
- `cargo test --locked -p oteryn-protocol-oteryn --lib --quiet`: pass, 414 passed, 0 failed/ignored/filtered. No integration targets touched.
- `cargo clippy --locked -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings`: pass (exit 0; existing vendored Tokio diagnostics remain).
- `cargo fmt --all -- --check`: pass. Included topic files are also checked explicitly below, because Cargo fmt does not traverse `include!`.
- `python3 tools/agents/validate_governance.py`: pass, 22 required policy documents and 9 lanes.
- `python3 -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check`: pass. No standalone repository source-size guard was found; largest added file is 428 lines.

Exact included-file formatting command (pass):

```sh
rustfmt --edition 2024 --config skip_children=true --check crates/protocol-oteryn/src/tests/{lib_tests,codec_seam_tests,support,handshake_tests,client_message_tests,envelope_tests,error_registry_tests,command_tests,state_sync_tests}.rs
```

Coverage filters contained in the passing full unfiltered library run:
`tests::` (54 tests: 51 moved plus 3 existing capability-selection tests),
`registered_capabilities::tests::` (4 generator/fragment tests),
`capability_tables::tests::` (1 table/registry check), and
`tests::capability_selection_tests::` (the 3 existing selection tests).
The inventory comparison proves these test names and all other names are unchanged.

Additional check: `python3 -m unittest discover -s tools/contracts/tests`:
4 passed, 1 pre-existing failure; see Unfinished. Protocol capability checks above pass.

## Self-review

Re-read the complete changed source and reviewed the full move against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: no findings. Production source outside
the module mount is byte-identical. Formatting both the original module body and
the reassembled topic files produces identical nonblank lines, including every
assertion, fixture and comment (`source-equivalence.txt`, `before-body.rs`,
`after-body.rs`). Every test full path is unchanged. The existing child module
and golden JSON files remain untouched.

No enum variants, struct fields, trait methods, public functions or public paths
were added, so workspace match/constructor/implementation updates and new real-dispatch
E2E cases are not applicable. No new capability allocations or code values.
No credentials, protected CI paths, other worktrees or Git state were modified.

## Unfinished

Local implementation and required checks are complete. The task remains waiting
for owner/control-plane publication; no PR, freeze, CI or external review is claimed.
This session is authorized for local edits and checks only.

The optional Python contract-suite check has one pre-existing failure:
`test_registry_fragments.RegistryFragmentsTests.test_split_of_the_aggregate_is_the_committed_tree`
(resource-limit subtest). The round trip derives `0613-GUILDCHAT-RL-01.json`, while
the committed fragment is named `0620-GUILDCHAT-RL-01.json`. All 818 Python registry
test/input files were compared byte-for-byte against HEAD and are unchanged.
This is outside owned paths; it is recorded, not repaired or skipped. See
`target/PROTO-LIB-SPLIT-1/contract-tests.log`.
