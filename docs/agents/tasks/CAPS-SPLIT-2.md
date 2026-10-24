# CAPS-SPLIT-2 — capability golden snapshots as one file per capability

Part 2 of the owner-requested anti-conflict plan. Follows CAPS-SPLIT-1 (#2361).

## Delivered

- `crates/protocol-oteryn/src/tests/capabilities_golden/<id>-<slug>.json`: one explicit golden
  snapshot per registered capability, named like its runtime fragment. Contents are byte-identical
  to the former `src/tests/capabilities_golden.rs` entries.
- The protocol golden test and the dispatch offered-set goldens read that directory and sort by id.
  The single ordered list, and the manual re-sorting it needed after parallel merges, is gone.
- The goldens stay independent of the runtime fragments: a fragment change without its golden file
  still fails both tests, and so does a missing or extra golden file.

## Adding a capability after this card

Add the runtime fragment and a golden file with the accepted facts. Parallel capability PRs touch
different files.

## Validation

- protocol-oteryn `--lib capability`: 31 passed; dispatch `--lib capabilit`: 96 passed
- Negative checks: an altered golden file fails the protocol and dispatch goldens; a removed one
  fails the protocol golden
- `cargo fmt --all -- --check`, clippy `-D warnings`, governance
