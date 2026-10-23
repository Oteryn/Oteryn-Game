# Auto-mounted library tests

`apps/game-server/build.rs` mounts every `*.rs` file in this directory into the
`oteryn-game-server` library as `#[cfg(test)] mod <file stem>;`, sorted by name. The
generated list is included inside the test-only module `lifted_tests_auto` in
`src/lib.rs`: the combat fence gate cannot read a generated file, so it accepts the
include only under `#[cfg(test)]`.

A new mechanic's library test goes here as one new file instead of another
`#[cfg(test)] #[path] mod` block in `src/lib.rs`, which every mechanic edited and
which therefore conflicted between parallel branches. Each module is a child of
`crate::lifted_tests_auto`, so name crate items with `crate::` paths; `super::` is
that wrapper module, not the crate root.
