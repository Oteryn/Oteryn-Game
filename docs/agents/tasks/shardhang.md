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
