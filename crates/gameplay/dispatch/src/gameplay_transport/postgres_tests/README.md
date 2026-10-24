# Auto-mounted dispatch PostgreSQL tests

`crates/gameplay/dispatch/build.rs` mounts every `*.rs` file in this directory into
`gameplay_transport::monk_save::postgres_tests` (`include!` in
`monk_save_postgres_tests.rs`), sorted by name, as `mod <file stem>;`.

A new mechanic's live-dispatch PostgreSQL test goes here as one new file instead of
another `#[path] mod` line in `monk_save_postgres_tests.rs`, the shared list every
mechanic used to edit. The module is a sibling of `fixture`, `fresh_fixture` and the
other explicit mounts, so `super::fixture` keeps resolving. A test that needs routed
PostgreSQL returns early when no admin URL is configured.
