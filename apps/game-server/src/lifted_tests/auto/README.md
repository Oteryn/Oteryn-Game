# Auto-mounted library tests

`apps/game-server/build.rs` mounts every `*.rs` file in this directory into the
`oteryn-game-server` library as `#[cfg(test)] mod <file stem>;` (`include!` in
`src/lib.rs`), sorted by name.

A new mechanic's library test goes here as one new file instead of another
`#[cfg(test)] #[path] mod` block in `src/lib.rs`, which every mechanic edited and
which therefore conflicted between parallel branches. The module is a direct child
of the crate root, so `crate::` and `super::` resolve exactly as for the explicit
mounts in `src/lib.rs`.
