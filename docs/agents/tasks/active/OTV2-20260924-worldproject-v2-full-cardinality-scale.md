# OTV2-20260924-worldproject-v2-full-cardinality-scale

```yaml
task_id: OTV2-20260924-worldproject-v2-full-cardinality-scale
title: WorldProject v2 full-cardinality scale measurement
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-worldproject-v2-full-cardinality-scale-r5
issue: 162
pr: 869
```

Allocation: R6, Issue #162 comment 5826848733 (bounded repair on the existing R5-owned paths). Branch: `agent/otv2-worldproject-v2-full-cardinality-scale-r5`.

Implementation lineage base: `c516182255d3ea1724e91671c8d2187eb622e3df` (authorized R5 base refresh, Issue #162 comment 5822887886). R6 is a single bounded successor to frozen R5 head `79306e0c11743681e0f0f395e824185bb5e9a783` under Issue #162 comment 5826848733; do not rebase onto path-disjoint protected-main movement #870. No R4 qualification is reused. It preserves the standalone tool's local SQLx/Tokio patches, fixes probe identifiers rejected by the production atom/key validator, and raises the per-document cumulative JSON string budget proportionally to the number of synthetic placements. The canonical validator remains unchanged. The supervised workflow accepts only a complete exact-cardinality success or a confirmed supervised timeout/VM/OOM-like resource outcome. Invalid JSON, unexpected exits, build/test errors, and unconfirmed probe failures remain `failed`; JSON, `/usr/bin/time -v`, stderr, and raw output are uploaded before a separate final gate fails the job. Status: implementation and repeatable measurement harness. The workflow result is evidence only and does not decide a production limit or storage format.

## Scope

Measure the existing canonical World Project v2 writer and reader using exactly 24,502,036 synthetic `CandidateOnly` placements, one synthetic declarative WorldObject definition, one world, one map revision, and one coordinate frame. The harness uses the real `oteryn-game-server` project canonicalization, snapshot parse, and v2 validation APIs. It separately times draft construction, canonical serialization/validation, JSON syntax parsing, schema-aware project loading, and writing the canonical role files into a temporary staged directory.

Only the explicit `--mode smoke` and `--mode full` modes are accepted. The probe accepts no file path, donor corpus, source artifact, or reference-world input. All generated records are labeled `SYNTHETIC_SCALE_STRESS_ONLY`; placements remain `CandidateOnly`. No source import, executable lowering, production world data, or game-runtime behavior is exercised.

## Measurement and resource boundary

The result reports bytes by canonical document role and in total, phase timings, and peak resident set size from the supervising `/usr/bin/time -v` process report in both completed and confirmed resource-limited full runs. The one-placement amplification value remains explicitly `ESTIMATE_NOT_GATE_COMPLETE`: it divides estimated whole-project rewrite bytes by an isolated serialized record difference and does not diff two full canonical snapshots or list changed files/bytes. It must not be treated as exact amplification or as a gate result. The run reports `completed` only if every requested placement is serialized and loaded. Only a supervised wall-clock timeout, signal-9 kill under the bounded VM ceiling, or allocator failure accompanied by explicit allocation/OOM evidence is `resource_limit`; every other unexpected exit or invalid result is `failed` with its exit and reason preserved. Full `completed` and `resource_limit` results require the `/usr/bin/time -v` peak RSS measurement. The full artifact is uploaded before a distinct final step fails the workflow for `failed`.

Pull requests run the bounded 2,000-placement smoke test separately and also run the supervised 24,502,036-placement full test against `github.event.pull_request.head.sha`. Every explicit `workflow_dispatch` runs the same full test. It is capped at 6 GiB virtual memory and 75 minutes. Its result is an artifact, not a gate that changes the production schema.

## Non-goals / decision fence

- Do not change project schema, evidence defaults, production placement limits, or chunking/storage strategy.
- Do not import or inspect donor/source-world datasets.
- Do not interpret a resource-limited harness run as proof that the production path fails; report the exact phase/ceiling and leave the decision to the owner.
- A completed synthetic measurement is evidence about the measured implementation and runner only, not a production capacity guarantee.

## R5 diagnostic and R6 repair

R5 exact-head workflow run `36065094697` recorded smoke `failed` (2,000 requested; exit 75 at `serialize_validate`: `project JSON string bytes exceed evidence limit`; peak RSS 1,674,924,032 bytes) and full `resource_limit` (explicit `try_reserve_exact` allocator failure before corpus construction; exit 75; `/usr/bin/time -v` peak RSS 3,543,040 bytes). R5 smoke artifact `10835264665` (SHA-256 `52f854447ccc1c938231af3c0c3268be1ac6f9d950682c587c6f523a11cc7faa`); full artifact `10836325885` (SHA-256 `5e8f28d17b25dd2d453431c4e45988c632645c93ea56afe326155d2d82dd7eb0`). The smoke failure came from the harness setting `max_string_bytes` to a fixed 1,024 even though the canonical parser applies it as a cumulative per-document string budget. R6 scales that harness budget by 512 bytes per placement plus a fixed 4 KiB allowance; smoke must still validate all 2,000 placements. Production validation and the full-cardinality target are unchanged.

## Validation handoff

Every pull request runs separate smoke and supervised full-cardinality jobs on the exact PR head; manual dispatch runs the full job too. Retain the workflow run identity and JSON/time/stderr/raw-output artifacts. Smoke must complete all 2,000 placements. Full may complete all 24,502,036 placements or report a confirmed resource limit; unexpected failures remain failing checks after artifact upload. Full-snapshot changed-file/changed-byte amplification remains not gate-complete and must be reported as an estimate, not upgraded to a measured result without a real canonical snapshot diff.
