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
