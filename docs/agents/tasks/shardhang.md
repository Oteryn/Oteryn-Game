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
