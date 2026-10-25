# protocol-oteryn v1 registry

`PROTOCOL_OTERYN_V1_REGISTRY.json` is stored as one file per capability, command type and
state domain, so parallel changes do not conflict (REGISTRY-SPLIT-1):

- `registry.json` holds every other key, with `"capabilities": []`, `"command_types": []` and
  `"state_domains": []` as placeholders;
- `capabilities/`, `command_types/` and `state_domains/` hold one entry per file, named
  `<id, four digits>-<name in lower case, dashes for underscores>.json`, de-indented by four
  spaces and ending with one newline.

To register a new mechanic, add one file per new entry, for example
`capabilities/0026-new-mechanic-v1.json`. Ids still come from `allocation_ranges` in
`registry.json`; the file order is the id order.

`crates/protocol-oteryn/build.rs` assembles the aggregate for Rust (read it as
`oteryn_protocol_oteryn::PROTOCOL_OTERYN_V1_REGISTRY_JSON`, never with `include_str!`) and
refuses a file whose name prefix is not its `id`, a duplicate `id` or a missing required
field. Python reads it with `tools/contracts/registry_fragments.py`.

## Adding a mechanic without shared-line edits

Add the allocated capability, command-type and state-domain contract fragments here and one
matching runtime fragment under `crates/protocol-oteryn/capabilities/`. Add the mechanic's Rust
codec module and its tests. The runtime fragment's owning const path, requirements, gates and
offer flags generate the registered ids, acceptance requirements, GATED table and all three
dispatch offered sets. Do not edit those generated tables or add another list of ids.

Module declarations remain additive insertions in the protocol and dispatch modules; the
`oteryn-additive` merge driver already handles them. Full-set property tests read the fragments.
CAPS-SPLIT-1 also retains explicit golden snapshots to prove this refactor preserves every
current value. CAPS-SPLIT-2 stores them as one file per capability in
`crates/protocol-oteryn/src/tests/capabilities_golden/`, named like the runtime fragment; the
protocol golden test and dispatch `capability_offers_golden_tests.rs` read that directory. An
intentional future capability-set change adds or updates its golden file as well as its runtime
fragment, so parallel capability changes touch different files and need no ordering. Existing
mechanic-specific assertions remain in their modules.

Regenerate one golden with `python3 tools/contracts/update_capability_golden.py <fragment>`.
For an allocated renumbering, pass `--previous <old-fragment>` to replace the old golden.

Command decoding/routing still edits `gameplay_transport/connection.rs`. CAPS-SPLIT-1 defers
per-mechanic route registration to a separate change because the current chain also determines
check order, decoding and error results. This split changes neither that chain nor wire behavior.
