# Auto-mounted dispatch PostgreSQL tests

`crates/gameplay/dispatch/build.rs` mounts every `*.rs` file in this directory into
`gameplay_transport::monk_save::postgres_tests` (`include!` in
`monk_save_postgres_tests.rs`), sorted by name, as `mod <file stem>;`.

A new mechanic's live-dispatch PostgreSQL test goes here as one new file instead of
another `#[path] mod` line in `monk_save_postgres_tests.rs`, the shared list every
mechanic used to edit. The module is a sibling of `fixture`, `fresh_fixture` and the
other explicit mounts, so `super::fixture` keeps resolving. A test that needs routed
PostgreSQL returns early when no admin URL is configured.

## Local filtered builds (`OTERYN_DISPATCH_PG_TESTS`)

Building every file here into the dispatch lib-test crate peaks above 16 GB RAM even at
`CARGO_BUILD_JOBS=1`. For a local run of a few tests, set `OTERYN_DISPATCH_PG_TESTS` to
comma-separated substrings of file stems: the build then mounts only the matching files plus
every sibling they, or the explicit mounts in `monk_save_postgres_tests.rs`, reach by path
(`super::sibling`, `sibling::item`, `::{.., sibling}`), scanned by
`crates/gameplay/dispatch/build_pg_shard.rs`. Unset or empty (CI and the default) mounts every
file, so nothing is skipped there. A filtered build warns about helpers its shard leaves unused;
a sibling the scan missed fails the build rather than dropping a test.
