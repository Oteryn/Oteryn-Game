# Oteryn provenance — Tokio 1.53.1

## Immutable source

- Package: crates.io `tokio` 1.53.1.
- Download URL: `https://static.crates.io/crates/tokio/tokio-1.53.1.crate`.
- SHA-256: `202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`.
- Upstream repository commit: `75fef53d0a8590c2d1dbb63672aa7b7d1ef51155`.
- Upstream license: MIT, retained verbatim in `LICENSE`.

The complete package was extracted without normalizing untouched files. The delta
manifest records every archive file SHA-256 and the complete authored path set.
The root patch selects this exact package version; it does not change Tokio's
version or existing feature selection.

## Post-amendment RED and exact stop boundary

The first archive-import attempt at `53f9ad90b3cead9ad5a25d026910f0a6fcc00ff3`
failed before compiling the intended test because crates.io's normalized manifest
has automatic test discovery disabled. That setup failure receives no RED credit.
The corrected manifest registered the focused test, and distinct RED commit
`45da01b13b848785ad7fe068c6e100b7cc3eebe5` reached Tokio test compilation and
failed only with `E0432`: `tokio::task` has no `spawn_blocking_owned`,
`BlockingOwner`, `BlockingOwnerConfig`, or `OwnedSpawnError` surface.

Implementation then stopped before GREEN source because the protected authored
allowlist is insufficient by one exact source path:

`SHARED_LEASE_REQUIRED = vendor/tokio-1.53.1/src/task/mod.rs`

The required symbol is the `cfg_rt!` public re-export list currently containing
only `pub use blocking::spawn_blocking` (lines 280–281 in the exact package).
The authorized implementation location `src/task/blocking.rs` is a private module;
there is no public path by which SQLx can name an owned spawn function, owner
trait/configuration, or bounded denial error unless `src/task/mod.rs` re-exports
the new surface. SQLx cannot call a private module, and changing an unrelated
public Tokio type to smuggle the API would broaden semantics rather than remove
this need. No source file, including the listed Tokio owner/drop surfaces, was
modified after this discovery.

Smallest next amendment: authorize only the re-export statement in
`vendor/tokio-1.53.1/src/task/mod.rs` for the Oteryn owned-blocking API implemented
in already-listed `src/task/blocking.rs`. This does not authorize changes to
ordinary `spawn_blocking`, features, numeric limits, or any other task module.
After protection/application, resume the required owner queue/task/worker RED to
GREEN matrix. `TLS_BLOCKING_OWNER` remains unproven, so TLS composition,
PostgreSQL accounting, the shared include-only target, and real TLS/PG evidence
remain OPEN.
