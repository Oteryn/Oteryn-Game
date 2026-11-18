# Dispatch library test build memory

Base verified against origin/main at `2c1e9158c30b411de28ec064bc7198ea4f87dd44`.
Rust 1.100.0, 16 GiB cgroup, no swap. Linux `wait4` supplied peak RSS and
wall time because `/usr/bin/time` was unavailable. RSS is the largest
process peak, not the sum across concurrently running processes.

| Run | Cargo jobs | Peak RSS (KiB) | Wall (s) | Result |
| --- | ---: | ---: | ---: | --- |
| Dispatch lib test, dependencies rebuilt | 2 | 16,437,864 | 1067.773 | rustc SIGKILL before link |
| Dispatch lib test, dependencies cached | 1 | 16,559,372 | 392.963 | rustc SIGKILL before link |
| Dispatch lib test, incremental off | 2 | 16,581,872 | 306.471 | rustc SIGKILL before link |
| Dispatch lib test, debug = 0 | 2 | 16,567,508 | 398.737 | rustc SIGKILL before link |
| Dispatch lib test, jemalloc preload | 2 | 16,529,876 | 711.561 | rustc SIGKILL before link |
| Game-server lib test | 2 | 4,824,940 | 941.839 | Built and listed 861 tests (863 output lines) |
| Game-server LLD replay | — | 3,328,964 | 4.970 | Linked |

The game-server build peaked in the production dispatch compiler process
with the test-support feature. The game-server executable is 868,769,624
bytes. Its link used 239 rlibs (1,953,996,922 bytes) and 263 objects
(397,694,920 bytes). Dispatch test rustc had 48 direct rlib inputs
(1,163,839,310 bytes). It never produced a test executable or invoked a
linker, so its full test list and link input totals remain unavailable.

Dispatch reached about 14 GiB before the LLVM coordinator appeared. Its
source tree has 4,238 test attributes and 29 direct lifted-test mounts in
`lib.rs`. The build script generates another 735 PostgreSQL module mounts
for the same test crate. The PostgreSQL subtree contains 2,171 test
attributes, including nested helper files. The mount-count experiment
will quantify the contribution of that subtree.

## Local mount ablation

An empty generated mount list (0 mounted files, 0 PostgreSQL test
attributes) reached 4,281,120 KiB after 120.644 s with two Cargo jobs.
This run failed at compile time. Existing test modules import helpers from
ten of the removed modules, including `inspection`,
`scheduled_event_rates`, and `appearance_change_postgres`. Its peak is
therefore a failed type-check measurement, rather than a complete test build.
The test sources and build script were untouched. All ablations use temporary
generated mount lists.

A fixture-preserving zero-registration control built successfully. It used
scratch copies of 249 dependency modules reachable from the ten helper roots
and disabled the local PostgreSQL `#[test]` functions in those copies.
With two Cargo jobs, rustc peaked at 11,635,704 KiB (11.10 GiB) and the build
took 573.102 s. The resulting binary listed 2,102 tests (2,104 output lines)
and measured 1,132,276,288 bytes. LLD received 241 rlibs (1,487,365,036
bytes) and 263 objects (1,200,533,024 bytes). These are control counts.
