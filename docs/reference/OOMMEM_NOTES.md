# Dispatch library test build memory

One dispatch lib-test rustc process exceeds the hosted memory budget.
Separating the generated PostgreSQL mount is the smallest tested
structural direction. Two scratch executables cover the complete
4,339-name baseline at a measured per-process peak of about 14.52 GiB.

The requested base, `2c1e9158c30b411de28ec064bc7198ea4f87dd44`, is an ancestor
of the fetched `origin/main` at `fe9a5a18a58abfb779b0196579acb1b3c83a0c7f`.
The base is the merge base of the resubmitted branch and fetched main.
The resubmitted head, `5faf34c3327c3cf2b641ceac5a2c12d0ca295ffa`, differs
from the pinned base only in this report.
Rust 1.100.0, 16 GiB cgroup, no swap. Linux `wait4` supplied peak RSS and
wall time because `/usr/bin/time` was unavailable. RSS records the largest
individual process peak. Concurrent processes also contribute to cgroup usage.

| Run | Cargo jobs | Peak RSS (KiB) | Wall (s) | Result |
| --- | ---: | ---: | ---: | --- |
| Dispatch lib test, dependencies rebuilt | 2 | 16,437,864 | 1067.773 | rustc SIGKILL before link |
| Dispatch lib test, fresh resubmission on the same source | 2 | 16,590,240 | 1428.314 | rustc SIGKILL before link |
| Dispatch lib test, dependencies cached | 1 | 16,559,372 | 392.963 | rustc SIGKILL before link |
| Dispatch lib test, 32 codegen units, dependencies cached | 1 | 16,551,756 | 622.273 | rustc SIGKILL before link |
| Dispatch MIR-only replay, incremental retained | 1 | 16,660,920 | 688.945 | emitted MIR, then OOM-killed |
| Dispatch lib test, incremental off | 2 | 16,581,872 | 306.471 | rustc SIGKILL before link |
| Dispatch lib test, debug = 0 | 2 | 16,567,508 | 398.737 | rustc SIGKILL before link |
| Dispatch lib test, jemalloc preload | 2 | 16,529,876 | 711.561 | rustc SIGKILL before link |
| Game-server lib test | 2 | 4,824,940 | 941.839 | Built and listed 861 tests (863 output lines) |
| Game-server LLD replay | — | 3,328,964 | 4.970 | Linked |

The game-server build peaked in the production dispatch compiler process
with the test-support feature. The game-server executable is 868,769,624
bytes. Its link used 239 rlibs (1,953,996,922 bytes) and 263 objects
(397,694,920 bytes). Dispatch test rustc had 48 direct rlib inputs
(1,163,839,310 bytes). The baseline never produced a test executable or
invoked a linker. A stable `rustc --test --emit=mir` replay of the same
verbose command recovered 4,339 unique registered test names and a
4,339-entry libtest descriptor array. The output was emitted before
rustc was OOM-killed, without LLVM object generation. The generated PG
mounts account for 2,238 registrations. The remaining mounts and modules
account for 2,101. The retained offline-death retry is registered exactly once.
The MIR inventory gives an exact baseline for comparing names after a
split. The baseline executable size and final link input totals are
unavailable.

In the resubmitted baseline, the dispatch test rustc high-water mark was
15,092,520 KiB (14.39 GiB) before the LLVM coordinator thread appeared.
RSS continued rising after codegen started. The cgroup recorded one OOM
kill when rustc received SIGKILL. At the last sample the cgroup held
17.01 GB of anonymous memory and 23.02 MB of file-backed memory.
The dispatch test process was the largest resident process and there was
no final linker invocation. The dispatch source tree has 4,238 test
attributes and 29 direct lifted-test mounts in `lib.rs`. The build script
generates another 735 PostgreSQL module mounts for the same test crate.
The PostgreSQL subtree contains 2,171 test attributes, including nested
helper files. A repeat of the baseline confirmed all 735 generated module
mounts and the identical 48 direct archive inputs by size. The fresh verbose command also
confirmed line tables and an active incremental cache. Neither nightly nor
`cargo llvm-lines` is installed in the retained environment.

The historical change associated with the first CI symptoms, #3056
(`e2152d050`), grew the generated mount list from 541 to 730 roots.
Lexical test attributes in the mounted subtree grew from 1,701 to 2,145,
and dispatch-wide attributes grew from 3,595 to 4,212. On the pinned
base the same inventory is 735 roots, 2,171 mounted test attributes and
4,238 across dispatch. These source counts exclude externally mounted
files and are distinct from libtest's registered test count.

## Local mount ablation

An empty generated mount list (0 mounted files, 0 PostgreSQL test
attributes) reached 4,281,120 KiB after 120.644 s with two Cargo jobs.
This run failed at compile time. Existing test modules import helpers from
ten of the removed modules, including `inspection`,
`scheduled_event_rates`, and `appearance_change_postgres`. Its peak
measures the failed type-check run only.
The test sources and build script were untouched. All ablations use temporary
generated mount lists.

A fixture-preserving zero-registration control built successfully. It used
scratch copies of 249 dependency modules reachable from the ten helper roots
and disabled the local PostgreSQL `#[test]` functions in those copies.
With two Cargo jobs, rustc peaked at 11,635,704 KiB (11.10 GiB) and the build
took 573.102 s. The resulting binary listed 2,102 tests (2,104 output lines)
and measured 1,132,276,288 bytes. LLD received 241 rlibs (1,487,365,036
bytes) and 263 objects (1,200,533,024 bytes). These are control counts.

The first half-mount run selected 367 of 735 module roots, including the
helper dependencies found by a source scan. Those roots contain 1,126
`#[test]` functions in 548 root and nested files. The last sampled rustc
high-water mark was 13,002,912 KiB at two Cargo jobs. At 535.421 s the
cgroup recorded two OOM kills and Cargo exited by SIGKILL. The process peak
is a lower bound because Cargo and rustc both died. No linker ran and no
test list was produced. A one-job run measures the same mount set.

The one-job retry was also OOM-killed before link at 12,996,504 KiB
after 313.443 s. Both early half-mount measurements were confounded by
3.38 GiB of retained evidence in RAM-backed `/tmp`, charged to the same
16 GiB cgroup. The compiler RSS values remain observations, but the
failures are not representative of a runner with disk-backed scratch.
The saved objects and binary were moved off tmpfs and the two-job
measurement was restarted.

The clean two-job half-mount run built and linked. It peaked at
15,813,712 KiB (15.08 GiB) after 683.603 s. The executable listed
3,281 tests (3,283 lines). Comparing names with the fixture-only
control found 1,179 additions and zero missing names. The executable
was 1,344,527,264 bytes. LLD received 241 rlibs (1,487,365,036 bytes)
and 263 objects (1,538,117,592 bytes).

## Generated PG mount isolation control

The two-job scratch replay uses the baseline test compiler invocation and
retains all generated PG mounts. In a disk-backed copy of dispatch, it
disables the test registrations outside the generated directory while
retaining their fixture definitions. This measures isolating the generated
mounts with the original assertions. The source files in the checked-out
branch stay intact. At LLVM coordinator startup the replay was approximately
12.7 million KiB RSS, compared with the baseline's approximately 15.1 million
KiB high-water mark at that phase.

The replay built in 714.308 s at 14,623,808 KiB peak RSS (13.95 GiB),
11.9% below the fresh full build, with the same two-job limit and cached
dependencies. The 1,121,028,728-byte executable listed 2,455 tests
(2,457 lines). All 2,238 generated PG test names matched the base exactly.
The executable also carried 217 non-mount tests retained by the scratch
control. No names were added relative to the base. The link used
241 rlibs (1,487,365,036 bytes) and 258 objects (1,162,448,280 bytes).
Its measured peak was 4,134,660 KiB over 13.190 s. Replaying the captured
link command independently peaked at 4,136,800 KiB over 9.687 s. The
executable identifies LLD 23.1.5 in its ELF `.comment` section.

The focused
`boss_activity_postgres_offline_native_death_deadline_retry_keeps_retained_trigger`
case passed against the retained PostgreSQL 18.6 service in 56.46 s.
It emitted the offline-death deadline, native entry, scope barrier,
and state-and-replay completion markers.

The complementary scratch replay keeps the library registrations
outside the generated mount. It preserves dependency paths and copies
all 735 generated module roots with their test attributes disabled.
That deliberately retains more helper code than the earlier
fixture-preserving control. Both scratch replays use the same two
workers and original test names, allowing their executable listings
to be checked against the full MIR registry.

The complementary replay built in 1,038.778 s at 15,228,376 KiB peak
RSS (14.52 GiB). Its 1,127,211,160-byte executable listed 2,102 tests
(2,104 lines), including every one of the 2,101 expected lib-slice
tests and one registration from the generated PG mount. Its link peaked
at 4,186,520 KiB over 14.952 s, with 241 rlibs (1,487,365,036 bytes)
and 258 objects (1,195,435,168 bytes).

The two executable lists have 4,339 unique names in their union, with
zero missing or added names against the full libtest registry. The
scratch lists share 218 names. Assigning ownership by the generated
mount gives 2,238 PG tests and 2,101 lib tests, each listed by its
intended executable. The production split must register each in one
target. Neither scratch replay caused an OOM kill.

## Full suite with 256 codegen units and incremental off

With the full 735-mount input restored, one Cargo job, and line tables
retained, rustc reached at least 16,500,396 KiB (15.74 GiB) before the
cgroup killed both rustc and Cargo. Wall time was 644.269 s. The verbose
command confirmed `-C codegen-units=256` and omitted `-C incremental`.
No linker ran. The package override did not produce a usable binary.
The clean half-mount link replay used 4,898,860 KiB in 9.891 s.

## One-job trial with 32 codegen units

The single-variable package override was
`--config profile.test.package.oteryn-gameplay-dispatch.codegen-units=32`
on the same `cargo test --locked --lib --no-run` command with cached
dependencies. It compiled with `-C codegen-units=32`, keeping line
tables and incremental compilation. The sampled rustc high-water mark
was 15,097,208 KiB before the LLVM coordinator appeared, within 4,688 KiB
of the default two-job build at that point. The nearly identical
pre-codegen footprint narrowed the potential gain to the LLVM phase.
The trial reached 16,551,756 KiB, added one cgroup OOM kill, and exited
before linking. This package-scoped profile change did not produce a
test binary.

## Hosted build scope

The PR PostgreSQL archive selects the dispatch, game-server and durability
library targets through `tools/ci/select_postgres_packages.py`. On the
pinned base, that job installs mold and enables 8 GiB of swap. The unit
shards enable 16 GiB of swap and set `CARGO_BUILD_JOBS=1`. The PR #3287
commit message describes serializing the PR PostgreSQL archive. Its actual
workflow delta sets one job in the PR unit shard and Merge Queue lib
archive. The PR lib archive command on the pinned base still uses the
default Cargo job count. The one-job dispatch
measurement still reaches the local memory limit before linking. Serial
Cargo scheduling limits overlapping dependency builds while one large
dispatch compiler process can still exhaust the available RAM.

On [PR #3281](https://github.com/Oteryn/Oteryn-Game/pull/3281), the
[hosted PostgreSQL lib build](https://github.com/Oteryn/Oteryn-Game/actions/runs/44220523121/job/134552950881)
lost communication with GitHub at 52:00 elapsed while the library
archive step was in progress. The runner annotation lists CPU, memory
and connectivity as possible causes. The
[fourth PR unit shard](https://github.com/Oteryn/Oteryn-Game/actions/runs/44220523256/job/134530329590)
was cancelled at 55:00 elapsed and its annotation reports the
45-minute execution limit. Local rustc and cgroup measurements locate
the memory failure inside the dispatch test compilation.

On the pinned main push
([run 44254498335](https://github.com/Oteryn/Oteryn-Game/actions/runs/44254498335)),
the Merge Queue PostgreSQL build completed its registered E2E archive.
The job was cancelled at 55 minutes with the library archive step still
open, about 48 minutes after that step started. The same run's workspace
library jobs had different outcomes: `lib-rest` completed successfully
while `lib-content` reached 55 minutes with its test step open. These CI
statuses establish where the jobs stopped. The local cgroup counter and
process measurements establish the reproduced OOM.

## Proposed structural fix

Move the 2,238 cases from the generated mount into a single
`dispatch-pg` integration target. Keep the remaining 2,101 cases in the
dispatch lib target. Extract the shared fixtures imported across that
boundary, keeping the same cases, assertions and production dispatch
calls. Expose the required test hooks through the existing
`test-support` feature and require the feature for the new target. The offline-death retry belongs to the generated mount. Retain
the current test namespace so the exact-name filter keeps selecting it.

The two-job controls give a planning peak of about 14.6 GiB across
the two targets, 8.2% below the full build. Their timings cover direct
rustc replays with cached dependencies. The original baseline includes
dependency builds. Both real targets still need measurement after
fixture extraction, a sorted test-name comparison with the 4,339-entry
baseline registry, and a hosted archive run to verify the job's wall time.

Compile the two targets serially in the existing archive. The PG
planner lists both binary IDs and assigns each test once. The PR unit
shards include the new target when dispatch is selected and keep their
existing hash partition. The Merge Queue integration planner discovers
it through Cargo metadata. The integration-rest command must also enable
`test-support` because the planner emits test names from metadata without
forwarding their required features.

The proposed patch selects the target after it exists and updates the
merge-authority workflow pin and command assertion. It passes
`git apply --check --whitespace=error` against the pinned build tree.
Applying it to a scratch Git index verified both proposed workflow blob
pins. All five updated YAML documents parsed and the six changed run
blocks passed `bash -n`. The offline-death step preserved a simulated
Cargo exit 17 through `tee` even when the output contained success markers.
The protected-base
audit rejects a candidate that modifies the audit itself and checks the
other workflow blobs against its own pins. This patch therefore requires
an owner-authorized audit rotation. The corresponding command assertions
in protected repository validators also need approved updates. The duration keys also need the new binary ID to retain
existing scheduling estimates. The branch contains the investigation
and proposed workflow diff.

### Proposed workflow diff

```diff
--- a/.github/workflows/pr-postgres-harness.yml
+++ b/.github/workflows/pr-postgres-harness.yml
@@ -341,5 +341,11 @@
           test "${#args[@]}" -gt 0
           mkdir -p "$RUNNER_TEMP/pg-archives"
-          cargo +1.100.0 nextest archive --locked "${args[@]}" --lib \
+          pg_targets=()
+          for package in "${packages[@]}"; do
+            if [[ "$package" == oteryn-gameplay-dispatch ]]; then
+              pg_targets=(--features oteryn-gameplay-dispatch/test-support --test dispatch-pg)
+            fi
+          done
+          CARGO_BUILD_JOBS=1 cargo +1.100.0 nextest archive --locked "${args[@]}" --lib "${pg_targets[@]}" \
             --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"

--- a/.github/workflows/merge-gate.yml
+++ b/.github/workflows/merge-gate.yml
@@ -1006,5 +1006,9 @@
           code=0
           mapfile -t package_args < "$RUNNER_TEMP/rust-fast-package-args.txt"
-          cargo +1.100.0 nextest run --locked "${package_args[@]}" --lib --bins \
+          pg_targets=()
+          if grep -Fxq -e '--workspace' -e 'oteryn-gameplay-dispatch' "$RUNNER_TEMP/rust-fast-package-args.txt"; then
+            pg_targets=(--features oteryn-gameplay-dispatch/test-support --test dispatch-pg)
+          fi
+          cargo +1.100.0 nextest run --locked "${package_args[@]}" --lib --bins "${pg_targets[@]}" \
             --config-file "$RUNNER_TEMP/nextest.toml" --profile rust-fast -E "$expression" \
             --partition "hash:${SHARD}" --no-tests=pass --no-fail-fast < /dev/null || code=$?
--- a/.github/workflows/merge-group-gate.yml
+++ b/.github/workflows/merge-group-gate.yml
@@ -630,3 +630,6 @@
           mapfile -t shard_args < "$RUNNER_TEMP/shard-args.txt"
           test "${#shard_args[@]}" -gt 0
+          if [[ "$SHARD" == integration-rest ]]; then
+            shard_args+=(--features oteryn-gameplay-dispatch/test-support)
+          fi
           cargo +1.100.0 nextest run --locked --workspace "${shard_args[@]}" --no-fail-fast
@@ -833,5 +836,5 @@
         run: |
           set -euo pipefail
-          cargo +1.100.0 nextest archive --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"
+          CARGO_BUILD_JOBS=1 cargo +1.100.0 nextest archive --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib --features oteryn-gameplay-dispatch/test-support --test dispatch-pg --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"

       - name: Record archive digests and candidate
--- a/.github/workflows/integration-regressions.yml
+++ b/.github/workflows/integration-regressions.yml
@@ -168,5 +168,5 @@
           set -euo pipefail
           test -n "$OTERYN_TEST_POSTGRES_ADMIN_URL"
-          cargo +1.100.0 test --locked -p oteryn-gameplay-dispatch --lib \
+          CARGO_BUILD_JOBS=1 cargo +1.100.0 test --locked -p oteryn-gameplay-dispatch --features test-support --test dispatch-pg \
             'gameplay_transport::monk_save::postgres_tests::boss_activity::boss_activity_postgres_offline_native_death_deadline_retry_keeps_retained_trigger' \
             -- --exact --nocapture 2>&1 | tee "$RUNNER_TEMP/offline-death-retry-postgres.log"
--- a/.github/workflows/merge-authority-audit.yml
+++ b/.github/workflows/merge-authority-audit.yml
@@ -29,5 +29,5 @@
-      EXPECTED_MERGE_GROUP_GATE_BLOB: "67e38c236b1964e03f47f5180540168e39456c71"
+      EXPECTED_MERGE_GROUP_GATE_BLOB: "aa14418aba596be4c78233dd9ffb9a9fbadac2df"
       EXPECTED_AGENT_GOVERNANCE_BLOB: "42d84c9089cdef928c6cb17268269156c87c0abb"
-      EXPECTED_MERGE_GATE_BLOB: "0f72c838a4a374f2b18da314c0b72ac74038a0f0"
+      EXPECTED_MERGE_GATE_BLOB: "15e5f7d4c48775da0ccedb63737409c083788e12"
     steps:
       - name: Validate control-plane candidate as inert data
@@ -296,5 +296,5 @@
               '              archived_targets+=(--test "$name")\n',
               '          cargo +1.100.0 nextest archive --locked --workspace "${archived_targets[@]}" --archive-file "$RUNNER_TEMP/pg-archives/pg-e2e.tar.zst"\n',
-              '          cargo +1.100.0 nextest archive --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"\n',
+              '          CARGO_BUILD_JOBS=1 cargo +1.100.0 nextest archive --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-game-server --lib --features oteryn-gameplay-dispatch/test-support --test dispatch-pg --archive-file "$RUNNER_TEMP/pg-archives/pg-lib.tar.zst"\n',
               '          cargo-nextest nextest run --archive-file "$RUNNER_TEMP/pg-archives/pg-e2e.tar.zst" \\\n'
               '            --workspace-remap "$GITHUB_WORKSPACE" --tool-config-file "oteryn-pg-plan:$RUNNER_TEMP/pg-plan/priority.toml" \\\n'
```
