# Mechanic registration

A new mechanic adds its Rust sources and one `<ordinal>-<snake_case_name>.mechanic`
fragment here. No list file or dependency is needed. `build.rs` scans and sorts fragment
file names, validates unique names and command ids, and writes the method lists,
transport modules, capability offer hooks and command router into `OUT_DIR`. It watches
this directory and every fragment. The existing directory-mount helper discovers Rust
command bodies under `src/gameplay_transport/connection/routing/` automatically.

The format is deliberately small: `key=value` header lines, then `[route]` and a Rust
expression. Keep the expression small; put the command body in the mechanic's Rust file.
The expression runs after the existing ingress, input and capability gates. It has the
same `authority`, `session`, `command`, `stream`, and `outcome` bindings as the old router.
Return the same `ServeEnd` on an unknown outcome as the owning contract requires.

```text
name=example
commands=123,124
transport=example
composed=example
authority=example
offer=40,example::offered,example_available
[route]
routing::example::route(authority, &session.admitted, command).await
```

- `name` and `commands` are required. Commands are positive decimal `u32` ids registered
  by the owning protocol contract. Duplicate ids (including within one fragment) and
  duplicate names fail the build. Unknown or duplicate header fields also fail.
- `transport` optionally mounts `src/gameplay_transport/<module>.rs`. Prefix with `pub `
  only when its existing public path needs preserving. For child modules use explicit
  source-relative `#[path = "example/child.rs"]`, as `#[path]` mounts do not infer that
  directory. Rustfmt does not traverse generated includes; format these files explicitly.
- `composed` and `authority` optionally contribute the named module's existing
  `composed_methods!` and `authority_methods!` macros. Macro types should use `$crate`
  paths so moving a declaration never changes its meaning.
- `offer` optionally contributes a capability-offer filter: numeric order, transport
  function path, and authority availability method. Orders are unique. Preserve existing
  ordering dependencies; current hooks are appearance (10), daily reward (20), Cyclopedia
  (30), after the weather-source filter. This registry does not reorder owner transactions
  or actor lifecycle operations.
- `[route]` is copied verbatim into the generated match arm. A reply-only route can return
  `Dispatch::Reply(CommandStatus, Vec<u8>)`, using the existing command-result encoder.
  Effects, unknown commits, capability gates and result errors still belong to the mechanic.

Ordinals retain the old router's order and leave gaps for additions. Choose a unique file
name; existing files never need renumbering. The immutable `route_table_before.txt` fixture
proves membership and order of the replaced routes; its test projects out future mechanic
names, so adding a mechanic does not edit the fixture. Add the mechanic's real dispatch
tests in the existing automatically mounted test directories. Run `--lib registry`, the
relevant capability tests and only that mechanic's PostgreSQL filter through the cargo wrapper.
