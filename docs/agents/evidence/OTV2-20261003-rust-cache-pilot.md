# Rust cache pilot: local server library baseline

PROVEN source: accepted main852dfca07f2d39eaf28d649d547d37876232cd8c.
Rust1.94.0, sccache0.18.0, actual workspace Cargo.lock and unmodified server library.
Two repetitions per warm/plain case; one cold sccache seed. Local disk cache,
stable checkout/target path, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0,
CARGO_BUILD_JOBS=4. This is not the hosted workspace's default debug profile.
Dependencies were fetched before compilation timing.

| Compilation case | Samples | Median seconds | Range seconds |
| --- | ---: | ---: | ---: |
| plain_cold | 2 | 110.011 | 107.179–112.843 |
| plain_warm_target | 2 | 0.324 | 0.278–0.370 |
| sccache_cold | 1 | 134.617 | 134.617–134.617 |
| sccache_warm_empty_target | 2 | 30.152 | 28.530–31.774 |
| sccache_warm_target | 2 | 0.374 | 0.299–0.449 |

Warm sccache with an empty target recorded181 Rust,258 C/C++ and111 assembler
hits, zero misses and zero read/write/timeout errors in both samples. Linking,
proc-macro/unsupported crate invocations and Cargo orchestration still execute.
Retaining the Cargo target was faster than reconstructing it from compiler cache.
Cold sccache added overhead; these results support a bounded pilot, not a blanket
replacement for current Cargo caching or a promised whole-CI speedup.

Measured scriptSHA256: f62b2507cf65a83944169aefbf918055f5737787e04c5917ec50bc04879043ad.
The final controller additionally clears configured Cargo wrappers with explicit
empty values and binds a private sccache configuration file. It uses two complete
interleaved cycles, resetting the private compiler cache before each cold seed;
the historical measurements above used a single cold seed. Free-disk preflight,
per-phase deadlines and a75minute script budget preserve time for report upload.
Isolation regressions and real sccache0.18.0 startup confirm the private config.
Numeric results above belong to the measured script and immutable Rust source,
not to the later hardened/interleaved controller.
Published sccache archiveSHA256:45f1447fbe231e3037bde351ef70677dd212216c8d62ae7ca409fecc4d6acc89.

The manual-only hosted pilot binds its controller to protected main, accepts only
an ancestor sourceSHA, pins the archive, and repeats the actual locked workspace
all-targets build. Isolated Unix-domain sccache endpoints and temporary targets
prevent sharing writable Cargo output between processes. Any cache storage error
invalidates the experiment. It uploads timings/stats/logs, including failures.

Remote cache restore/upload: NOT_PERFORMED. Report artifact upload is not a cache
transfer benchmark. No usable shared agent backend was found in accepted code or the agent
environment. Repository and organization secret/variable metadata are UNKNOWN:
the connection receives HTTP403 for those endpoints. This pilot does not need
those secrets or an external backend. No canonical source or
Merge Queue job is changed, and no cache result grants integration authority.
Hosted workspace evidence and before/after job-wide speedup remain pending until
the reviewed pilot is merged and executed on protected main.
