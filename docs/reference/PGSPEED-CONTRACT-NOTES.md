# PGSPEED-1 PostgreSQL fixture contracts

The lane starts at main `3209d3d61` and merges `f5b584ef1` (the owner-approved 24-partition change). It allocates no migrations, command IDs, capabilities or mechanic ordinals and changes no workflow.

## Measured shared costs

Opt-in `OTERYN_TEST_TIMINGS=1` measures fixture phases with monotonic `Instant` values. Five serial real-PG baseline cases passed: A4 invisible 105.551 s; Forge pending revision refusal 145.468 s; Hallowed exchange 83.746 s; Knight outfit 84.485 s; Pirate outfit 83.272 s (nextest execution total 502.528 s). Quest setup included 4.2–4.6 s migration, 6.2–6.3 s manifest loading and 29.2–30.1 s activation. Dependencies took approximately 31 ms. A separate call to the actual embedded quest loader measured 2.894 s; it is not the dominant setup phase. These measurements do not attribute the unmeasured remainder to SQL or catalogue loading.

Native decoding is repeated by `crates/gameplay/content/src/content/native_gameplay.rs:813` (compiler validation) and `crates/gameplay/content/src/content/production.rs` (staging). Candidate telemetry measured cold decoding of a 49 MB envelope at 10.9–12.2 s and exact-byte cache hits at 42–65 ms. Preliminary candidate timings overlap the full baseline suite and are not an isolated before/after speedup claim.

## Isolation and invalidation

The native decoder cache exists only under `test` or `test-support`, retains at most two immutable successful decodes per process, and compares the complete encoded bytes. Every World binding, section, hash, version and trailing byte remains part of identity. Cache misses use the unchanged decoder; corrupt inputs cannot enter the cache. The shared activation fixture also retains at most two immutable qualified rooms per process, keyed by World and derived equality of every input field (actual bytes and declared pins). Every call still creates a fresh controller and performs the sealed activation. Controllers, activation issuance, actors, authority, sessions and database state remain fresh. A negative/positive unit case checks mutated bytes, trailing bytes and different World bindings.

Nextest launches each test in its own process. The migrated schema therefore lives in a local PG template, keyed by canonical PG version plus every migration version/checksum. A session advisory lock serializes construction; connection-disabled state marks completed construction. Each test receives its own database clone, and its existing migrator still validates migration checksums. PostgreSQL does not copy database-level ACLs, so every clone explicitly reapplies the PUBLIC CONNECT refusal from migration `0006_node_boot_control_plane.sql:31–38`. A real-PG red reproduction showed PUBLIC CONNECT=true in an uncorrected clone (1.865 s); the clone regression test checks this boundary too. The helper lives under `apps/game-server/tests/support/postgres_fixture.rs` and is exported only for tests/test-support.

The template excludes the four decision-root secrets. Every clone initializes fresh secrets with the expressions from migrations 0313:97–98, 0337:65, 0575:110–112 and 0723:8 before any application owner exists. Normal clone triggers remain enabled. A real-PG test creates clones concurrently, verifies distinct secrets and isolated DDL/rows, corrupts one clone's migration checksum and verifies rejection while the other remains valid.

`OTERYN_TEST_FIXTURE_CACHE=0` disables both optimizations for controlled same-build comparisons. No assertions, loop counts or PG cases are removed, weakened or ignored.

## Outlier call paths

House authority rechecks deliberately create 27 independent databases (three shapes by nine operators); all remain independent. The 100,001-delivery case intentionally executes the real delivery volume and retains its ordinal/distinctness assertions. The world-loader qualification compiles actual source regions and performs world loading/equivalence; it is not a PG migration fixture. Real-PG baseline SP02 passed in 340.684 s. Its repeated native qualification measured 20–23 s plus 11–22 s staging decodes, while physical-bundle construction measured only 40–85 ms. The world-loader baseline passed in 232.474 s: existing production telemetry reported 79.263 s source-region compilation and 152.600 s world boot (231.863 s combined). The boot includes five identical-size native-envelope decodes at 10.98–15.05 s each: `apps/game-server/src/qualification/client_phases/world.rs:60–79` qualifies the input, then its activation path qualifies/stages it again. Native decoding is therefore a shared cause here too; source-region compilation is separate and remains real work. No PG migration occurs in this case. Remaining outlier timings and full-suite totals are pending measured runs, not inferred from CI durations.

Current focused validation: seven native-fixture cases passed (117.52 s), the decoder regression passed (8.18 s), and the single real-PG isolation/checksum/secret/ACL regression passed (8.05 s). Full before/after suite evidence remains in progress.

## Benchmark execution context

The full before-run shares the machine with compilation and content regeneration. A disposable nested-Docker VFS attempt temporarily exhausted disk; 43 database-creation cases failed with ENOSPC, and one earlier field-rune case hit a deadline. Those failures are retained and require isolated reruns; raw full-suite wall times must not be presented as a controlled speedup percentage. The focused serial before/after cases provide the controlled comparison.

Local full node qualification uses the unmodified script and pinned Platform checkout. A temporary Compose build overlay adds the execution environment's public proxy CA certificates to the qualification PHP image; it preserves the pinned base, application code, commands and TLS verification. The overlay, proxy configuration and disposable Docker state remain outside the repository.
