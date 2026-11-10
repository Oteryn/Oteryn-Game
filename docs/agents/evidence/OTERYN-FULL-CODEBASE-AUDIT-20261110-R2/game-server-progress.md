# Game server — partial source review

Input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**. Frozen coverage: **660/873 complete files**, 661 with substantive ranges, **331,400 deduplicated physical text lines**, and **2,703 separately counted binary bytes**.

The parent-qualified immutable capture contains **660 complete files / 330,924 text lines / 2,703 binary bytes**, with **650 manual source reviews and 10 mixed semantic reviews**. Capture SHA-256: `cf182478e3f028f4a3d64219ed4a7b15c656fd3bfa3708ea5aee36e3bff855e5`. All 407 migrations and all server `src` files are included. The captured remaining scope is 194 test files and 19 examples. Wrapper review does not grant credit to an unread included test body. The extra selected source range in the central ledger explains the aggregate line difference.

Mixed methods combine manual control, consumer and complete semantic-class analysis with exhaustive checks of repeated structures and exact already reviewed objects. They are explicitly documented in the six `game-server-*-method-map.md` files. Parsing, indexing or equality alone does not grant full-review credit to large supporting canonical content.

The three newly reviewed DER test credentials use byte ranges. Complete certificate structure, canonical encoding, signatures, matching public identity and key relationships were checked locally. No private key values are published. These are committed test fixtures; no production credential finding or trust inference is made. No TLS handshake was run.

The root independently checked every captured immutable Git/worktree hash, full endpoint and inclusion in this frozen ledger, and read the complete method maps. This does not claim a second manual review of all 660 delegated files. Static fixture differences, donor provenance limitations and unexecuted companion behavior stay explicit. No new confirmed finding is established.

No PostgreSQL, migrations, Rust/native suite, full Cargo build, live server, complete world import or deployment was executed. The package is not REVIEWED in full or runtime VERIFIED. See `capture-validation-18.json` and the exact remaining paths in `coverage-ledger.json`.
