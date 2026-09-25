# OTV2-20260924-worldproject-v2-full-cardinality-scale

```yaml
task_id: OTV2-20260924-worldproject-v2-full-cardinality-scale
title: WorldProject v2 full-cardinality scale measurement
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-worldproject-v2-scale-r7-sharded
issue: 162
predecessor_pr: 869
```

Allocation: R7, Issue #162 comment 5827206602. Admission protected main: `bf729de00a9f692a7fb3a4c3311ecb09be150eff`. The current protected main may be newer; the R7 branch retains its allocated ancestry and does not rebase/path-merge unrelated work. Predecessor PR #869 is integrated at the admission revision. Continue the assigned branch at its live head and preserve the R6 measurements below.

## Scope and acceptance

Harness-only deterministic sharded measurement. Keep the existing 2,000-placement smoke and the 24,502,036-placement monolithic full run/resource-limit classification. Add a sequential shard mode covering exactly 24,502,036 global placement identities. Every shard must use the existing real v2 path: ordinary harness draft construction, `CanonicalProjectDocuments::from_v2_draft` canonicalization/validation, snapshot creation, JSON syntax parse, schema-aware snapshot load, and staged role-file write.

Acceptance:

- fixed deterministic half-open ordinal ranges with no missing or duplicate identity, checked against each loaded placement key and exact total;
- synthetic `CandidateOnly` placements, one definition/world/map revision/coordinate frame; no corpus, donor, source, or file-path inputs;
- per-shard and aggregate canonical role bytes, total bytes, and phase timings; supervised maximum RSS for the sequential shard runner;
- explicit limitation that sequential shard RSS is not monolithic full-world peak/edit amplification, and does not select a production shard size/layout;
- no product code, schema, contracts, production placement limit, or ProjectV3 change;
- exact-head hosted smoke/full/sharded runs and uploaded JSON/time/stderr/raw-output artifacts; independent review and governed integration remain coordinator-owned.

The shard size is a bounded measurement parameter only (currently 500,000 identities per shard), not a production-layout recommendation.

## Measurement and decision boundaries

The result reports canonical bytes by role and total, build/canonical-validate/syntax-parse/schema-load/staged-write timings, and maximum supervised shard-runner RSS from `/usr/bin/time -v`. It reports completed only after all 24,502,036 requested identities have been built, canonicalized/validated, parsed, loaded, identity-checked, and staged. The sharded workflow fails closed on any missing/duplicate identity, invalid JSON, missing supervised RSS, incomplete range coverage, failed toolchain/build/process, or unexpected exit. Failure artifacts upload before the final gate.

The existing monolithic full run remains separately supervised at a 6 GiB virtual-memory ceiling and 75-minute wall-clock limit. It may report only completed exact cardinality, confirmed `resource_limit` with explicit supervisory evidence, or failed. Preserve the protected R6 `resource_limit` result as a truthful prior measurement; do not reinterpret it as product capacity failure or let shard success overwrite it.

The shard result must state: `monolithic_full_world_peak_rss = NOT_MEASURED_BY_SHARDED_RUN`; full-world edit amplification was not measured because no canonical full-world snapshot diff was performed; and `production_layout_selected = false`. The harness is synthetic scale evidence only. Do not change validators or production behavior to make a measurement pass.

## Non-goals

- No project schema, evidence-default, production-limit, chunking/storage, or architecture decision.
- No donor/source-world data, production population, executable lowering, or runtime behavior.
- No claim of monolithic full-world memory or edit amplification from sequential shards.
- No recommendation or selection of a production sharding layout.

## Protected R6 predecessor evidence

PR #869 integrated the R6 bounded harness repair at `bf729de00a9f692a7fb3a4c3311ecb09be150eff`. Exact-head run `36095594156` passed smoke and full job gates. Smoke artifact `10847067292` (SHA-256 `c3ebb7dd94a8176638fa3983121a17d8f3051a8ed6191596657cf978b3c54c53`) completed and loaded all 2,000 placements. Full artifact `10847820851` (SHA-256 `ec447a384bb50c9a5d94ce4410c7a345bd2e115ea6b5d836090e02d33715e154`) honestly reported `resource_limit`: `try_reserve_exact` failed during build before the 24,502,036-placement corpus was constructed, exit 75, with supervised peak RSS 3,534,848 bytes. This is bounded-run evidence, not a product-capacity conclusion. R7 preserves both modes and does not reuse R6 exact-head qualification for R7.

Earlier R5/R6 diagnostics remain in the evidence record. R5 smoke exposed the harness's fixed cumulative string budget; R6 repaired only the harness budget. Production canonical validation was not changed.

## Validation handoff

Record the R7 exact-head workflow run identity and hosted artifacts for smoke (2,000), monolithic full (24,502,036; completed or proven resource-limit), and sharded full coverage (24,502,036 completed). Preserve the explicit limitations in the evidence artifact. This task record does not authorize ready-for-review, enqueue, merge, or any protected integration action.
