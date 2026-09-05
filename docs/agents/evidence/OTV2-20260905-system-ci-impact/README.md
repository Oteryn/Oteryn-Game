# CI impact-routing audit — immutable admission evidence

Programme status belongs only to [Issue #308](https://github.com/Oteryn/Oteryn-Game/issues/308). This document and `audit.json` are a static audit snapshot, not a task status database.

## Population and method

Protected admission: `b9b1a4317858bffc25ad6af3cffcf7b5eff93445`. Latest 100 distinct run-head SHAs selected from 800 recent runs (160 SHAs), with 507 associated runs. Run IDs, exact immutable base/head/merge-base comparisons, rename before/after paths, jobs, actual start/end, runner labels and hosted classification evidence are in `audit.json`. All 100 comparisons returned at most 12 files, below the API's 300-file comparison cap. Repeated PR lifecycle runs count as allocated cost, not independent changes. The history crosses deployment of #283 and #304; predeployment costs are not attributed to current behavior. Historical classifier decisions unavailable before activation or because execution was cancelled remain UNKNOWN. One transient job-read transport failure was recovered without retriggering CI.

37 recent dedicated runs supplement the population; they are separately reported rather than silently added to the 100-head denominator. Workflow registry reports 55 active entries: 17 sources present on current main, 37 historical absent paths, one dynamic Dependabot workflow. Registry-active alone does not establish runnable protected-main source.

Allocated runner-time sums actual non-skipped job start/end durations, including cancelled partial work; it is not billing. Workflow elapsed can be computed from created_at to last actual job end; updated_at is retained only as a terminal API proxy. Observed first-job-start to last-job-end span includes inter-job scheduling and is not pure critical-path execution. Full historical per-revision dependency DAG critical paths and per-job eligible queue delays remain UNKNOWN. Initial queue delay is first job start minus created_at. Never attribute queue changes to this optimization. No controlled A/B exists.

## Measured allocation totals

| Sample lane | Runs | Allocated runner-min |
| --- | ---: | ---: |
| Canonical PR merge gate | 108 | 890.2833 |
| Full Merge Queue | 13 | 183.3833 |
| Protected-main Rust | 12 | 155.2000 |
| Historical standalone PR Rust, already removed by #283 | 17 | 33.7000 |
| Push CodeQL | 13 | 25.0167 |
| Standalone PR governance | 108 | 17.4333 |
| Protected merge-authority audit | 108 | 12.6833 |
| Architecture semantic audit | 97 | 10.4667 |

Dedicated Atlas is already narrowly triggered. Largest sampled Thais allocation is 467–702 seconds, only 12 lifetime runs; animation 81–106 seconds, 17 lifetime runs. Other successful sampled Atlas jobs are 28–104 seconds. Physical profile spike and Tibia tooling have zero recorded runs. Synology's two jobs allocate 10 seconds each; one failed run has 3365 seconds initial queue delay, not avoidable execution waste. Settings and lifecycle writers and dedicated runners retain distinct trust boundaries. Deterministic double generation provides product equivalence evidence and must not be removed as duplicate setup.

## Canonical model and ranked opportunities

Reuse `classify_pr_test_lanes.py`: protected-base PR code/Cargo graph, reverse normal/dev/build/optional/target dependency closure, reviewed non-Cargo consumer snapshots, exact identity/range/mode checks. Current outputs are rust/windows. Unknown/stale/malformed/incomplete stays FULL. Protected push currently permits only server/durability Windows omission. Always-required authority/security/governance gates and FULL Merge Queue remain outside optimization.

1. **Reviewed docs snapshot restoration.** 18 hosted PR runs / 14 distinct heads selected stale-document FULL; seven successful runs allocate 13.05–15.9667 runner-min, of which 10.9333–13.7167 are Rust jobs. Example #307 run33969063940, head96b7af0daba78631a1036e3679933187757d7026, protected base62590071b7e47e3221af0e180c73bbc7cdf37c31: job101314251779 logs the exact archived-task rename and unreviewed-document-consumer-inputs. Total958s, Rust823s. Historical scoped-doc run33957757290 allocates131s with runtime jobs skipped. These are separate hosted samples, not achieved savings from this programme. INFERENCE: 10.93–13.72 runner-min of runtime job cost per comparable successful qualifying run could be avoided while the reviewed snapshot remains valid. High confidence dependency proof, low implementation complexity, material control-plane review; rollback restores prior conservative digest. No frozen workflow/fan-in or ownership conflict.
2. **Protected-main neutral-doc selection.** Reuse the same verified classifier, expose rust as well as windows in rust.yml, retain policy/supply-chain/governance/security. Separate bounded slice only after predecessor qualification. Unknown/malformed/output failure and dispatch FULL. Runtime omission requires reviewed docs consumer digest. Potential savings are inference until natural postdeployment hosted samples; frequency is insufficient for monthly projection. Changes would require rust.yml, existing adapter/tests, reviewed Rust workflow pin and matrix; no MQ/audit/fundamental fan-in change.
3. **Server workspace or harness narrowing.** Server-only still runs workspace build/Clippy/tests and synthetic client harness. No assumption that cargo -p server runs dependency test suites. Full package narrowing lacks sufficient evidence; a harness-only omission requires step-level measured value before admission. Not implemented just to reach a quota.

Global static checks are cheap in this history; protected audit/CodeQL/settings writers cannot be consolidated across permissions or trust levels. Atlas trigger coverage needs a separate safety investigation: semantic-search executes creature exporter outside its enumerated producer paths, and static-creatures may omit an identity helper. Do not narrow those triggers without resolving dependency coverage. Merge Queue remains FULL and frozen; historical totals do not establish it as the dominant remaining cost after hypothetical optimizations.

## Reviewed consumer delta for first slice

From #297 protected9631cbfe718e75d6bc530352fb811e08a444b6b0 to admission, only five server test consumers changed: authority_invariants.rs, durability_postgres.rs, server_ci_qualification.rs, support/authority_matrix.rs, support/authority_recovery.rs. No production/Cargo/build/migration input changed. They construct in-memory data, query PostgreSQL, restart current_exe with explicit scenario env, or launch the Cargo-built server binary with CLI args. No new neutral-doc reader appeared. Existing include macros are package-local SQL/Rust; architecture-check reads workspace-boundaries.toml, already a build input. Nonserver digest9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8 remains current. All-consumer digestf8eed774249df64a5a64612b4a169a73bac093a7bcbfb21e59ea0e06dd2ddc26 requires explicit reviewed adoption. Further consumer edits intentionally stale the snapshot and restore FULL; automatic snapshot approval is prohibited.

## Admission/ownership and safety

Current native records: #288 SUPERSEDED by accepted queue work; #243 STALE_HISTORICAL read-only source per #247; #294 ACTIVE docs allocation only; #293/#292 ACTIVE runbook docs only; #262 and #150 ownership UNKNOWN on their historical paths, no classifier/Rust overlap. Closed #283/#304 released their implementation ownership; retained historical task prose is not a live lease. Dependabot upload action overlaps only dedicated Atlas workflows; Cargo proposals are reconciliation candidates, not classifier writers.

Ruleset20991995 is active: sole required game-gate, squash Merge Queue, no bypass, review-thread resolution. This snapshot is not final safety readback. First-wave changes must preserve frozen MQ/audit/fundamental gate semantics, exact-head qualifications, negative tests, and trust separation. No achieved saving, completed slice or terminal programme result is asserted here; consult current Issue #308.
