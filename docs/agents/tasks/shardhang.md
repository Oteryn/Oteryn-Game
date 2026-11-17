# Unit-test shard timeout investigation

Base and live main verified at `382ba2e5891680675c40ffa10a9a255500647c67`.

The workflow uses Cargo 1.100.0 and nextest 0.9.146 with `--locked`,
`--workspace --lib --bins` in workspace mode, the generated `rust-fast`
profile and its priority overrides, a selected-package filter, and
`--partition hash:N/5 --no-tests=pass --no-fail-fast`. The local baseline
combines all partitions and adds a 60-second slow report with termination at
300 seconds. The PostgreSQL admin URL is unset.

PR #3265, run 44162666745, job 134324639255 (3/5), has the annotation
"The job has exceeded the maximum execution time of 45m0s." The nextest step
started at 20:02:43 UTC. Job 134324639481 (4/5) completed the same step in
15m17s. The job log download returned HTTP 403. The timeout therefore
doesn't distinguish compilation from test execution. #3260 records that
dispatch's lib-test build grew to 17.3 GiB peak RSS at one build job after
#3056 and adds swap on hosted runners. Local baseline compilation is in
progress. No culprit test has been identified.

## Hosted job comparison

In run 44162666745 (#3265), shards 1, 3 and 5 have the 45-minute timeout
annotation, while shards 2 and 4 completed their nextest steps in 12m10s and
15m17s. Shard 4 timed out in run 44174301267 (#3269), and shards 3, 4
and 5 timed out in run 44177371114 (#3271). The affected steps had
already restored the target cache and installed mold.

Several #3263 cancellations in runs 44162049615 and 44165495269 were
instead annotated "Canceling since a higher priority waiting request for
merge-gate-3263 exists". These runs cannot establish a test hang.

Governance validation and all 135 agent-tool unit tests passed for this
notes-only checkpoint. The local baseline is still compiling workspace
dependencies.

The cold local baseline reached the client dependency build after more than
12 minutes, with no test launched and no cgroup OOM events. Peak cgroup
memory was 12,035,960,832 bytes so far. This is local compilation time and
cannot be compared directly with the hosted warm-cache step duration.

The earlier five-shard change (#3251) reported 3,575 of 4,621 tests passing
before a three-shard timeout on #3056, with no test failures at cutoff.
Its reported fixed setup/build cost was about 19 minutes. That earlier run
establishes that aggregate execution has exhausted this lane's budget
before, but does not explain the later 45-minute cancellations on its own.
The exact-base local run has reached dispatch and game-server compilation.

A direct run of the freshly built client test binary passed
`srx3_native_missile_crosses_real_session_into_play_scene` in 0.16s.
The test has an unbounded channel receive in its peer task, but the
controller reached the send and the test joined both tasks on this run.
The workspace baseline is still compiling the dispatch lib-test target.

A serial direct run of the fresh client unit-test binary completed with 886
passed, zero failed in 54.73s. The workspace baseline is still compiling,
including the dispatch lib-test target. No Nextest test execution timings
are available yet.

After 34 minutes, local Nextest still had not launched tests. The dispatch
lib-test rustc process had been running for almost 12 minutes and had
about 10.7 GiB resident. The cgroup recorded memory pressure with zero OOM
kills at this point. This is direct evidence that time spent inside the
nextest command can be substantial before any test starts.

At roughly 38 minutes the two-job workspace baseline exited before test
execution with Cargo code 101. The gameplay content lib-test and dispatch
lib-test rustc processes were killed by SIGKILL. The cgroup recorded two
OOM kills and has a 16 GiB memory limit with no swap. A one-job retry is
running against the same base and retained artifacts.

## Archive-only failure on PR #3268

Run 44187588857, job 134408139208, entered the library archive step at
22:21:59 UTC after mold installation and swap setup. The archive builds
and packages lib-test binaries without executing test bodies. GitHub's
annotation says: "The hosted runner lost communication with the server.
Anything in your workflow that terminates the runner process, starves it
for CPU/Memory, or blocks its network access can cause this error."

Both lanes compile the dispatch library with `cfg(test)`, which includes
the PostgreSQL test modules via
`crates/gameplay/dispatch/src/gameplay_transport/monk_save.rs:220` and
`crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs:40`
even when the runtime PostgreSQL URL is absent. The mount generator in
`crates/gameplay/dispatch/build.rs:21` calls the local filesystem helper
at `apps/game-server/build_mounts.rs:9`. It does not connect to a database
or run an external command. #3056 added a net 77,673 lines across 344 files
in the dispatch PostgreSQL test tree. The local OOM hit rustc during the
two lib-test builds. The serial retry is actively accumulating CPU time in
dispatch rustc. Hosted archive failure remains consistent with build-time
resource pressure, but the annotation alone does not identify the exact
local process or a linker deadlock.

From the fresh game-server test binary,
`bundle_spawn_owner_tests::a_bundle_spawned_rat_killed_by_a_lethal_hit_respawns_on_the_owner_turn`
passed in 30.16s.
