# CAPS-SPLIT-1 — capability runtime facts as one fragment per capability

Part 1 of the owner-requested anti-conflict plan ("wykonaj"). Owner-delegated decision: the
per-mechanic command-routing split (`connection.rs`) is deferred to a follow-up.

## Delivered

- `crates/protocol-oteryn/capabilities/<id>-<slug>.json`: one runtime fragment per registered
  capability (id, name, constant, requires, acceptance_requires, commands, domains, supersedes,
  offers). Rules are in `crates/protocol-oteryn/capabilities/README.md`.
- `crates/protocol-oteryn/build_capabilities.rs` (included from `build.rs`) validates the fragments
  against the contract fragments and generates `OUT_DIR/registered_capabilities.rs`: registered ids,
  acceptance requirements, the GATED table and the offered sets.
- Dispatch builds its offered arrays from the generated pairs at compile time.
- Golden tests pin the full registered set, acceptance facts, GATED rows and offered lists, so the
  refactor carries no behaviour change.

## Adding a capability after this card

Add one runtime fragment next to the contract fragment and codec module. Update the goldens only when
the accepted set really changes. Open PRs that still edit the old hand-maintained lists port their
entry into a fragment when they merge main.

## Validation

- `cargo fmt --all -- --check`: passed
- protocol-oteryn `--lib`: passed; clippy `-D warnings` for protocol-oteryn: passed
- merge-driver unittest (12 tests): passed; `git diff --check`: passed
- dispatch build and tests: GitHub CI on the PR head
