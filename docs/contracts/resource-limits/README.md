# Resource limits registry

`RESOURCE_LIMITS_REGISTRY.json` is stored as one file per entry, so parallel changes do not
conflict (REGISTRY-SPLIT-1):

- `registry.json` holds every key except the entries, with `"entries": []` as a placeholder;
- `entries/<ordinal>-<id>.json` holds one entry, de-indented by four spaces, ending with one newline.

To add a limit, add one file to `entries/`. Name it with the next free ordinal (any ordinal
higher than the last is fine; two branches may pick the same one) followed by the entry's
`id`, for example `0592-NEWMECH-RL-01.json`. Do not renumber existing files: the ordinal
keeps the registry's order.

`crates/protocol-oteryn/build.rs` assembles the aggregate for Rust (read it as
`oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON`, never with `include_str!`) and
refuses a misnamed file, a duplicate `id` or a missing `required_entry_fields` field. Python
reads it with `tools/contracts/registry_fragments.py`.
