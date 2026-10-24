# Runtime capability fragments

One `<id, four digits>-<name in lower case, dashes for underscores>.json` file describes
each registered capability. This is runtime composition metadata; the accepted contracts
remain in `docs/contracts/protocol-oteryn/v1/registry/` (REGISTRY-SPLIT-1).

- `constant` is the owning Rust module's capability const path, relative to the protocol crate.
  Generated compile-time assertions check its value.
- `requires`, `commands` and `domains` match the contract fragment. `domains: null` is the
  capability 6 exception: its codec gates the extension of core domain 1, not the whole domain.
- `acceptance_requires` controls inclusion in acceptance-message closure validation. Preserve
  the existing asymmetric coverage: 18, 25, 36 and 37 have registry/offer requirements that
  acceptance messages do not currently enforce. Changing this is a separate wire behavior task.
- `offers` declares production fallback, Item view and bundle World membership. Requirements
  are identical in each offered set. Per-connection weather filtering remains in dispatch.
- `supersedes` records the existing MAP-WIRE-1 core-domain replacement (18 supersedes 2).

`build.rs` scans the directory, checks exact registry coverage, filenames, unique ids/names,
requirements, command/domain ownership and offer closure, and emits sorted Rust into `OUT_DIR`.
It never invents reserved or missing ids. The registry JSON aggregates remain unchanged.
No generated file is committed. Dispatch materializes its existing `OfferedCapability` arrays
from the generated protocol pairs in a `const fn`; there is no runtime parsing or allocation.
