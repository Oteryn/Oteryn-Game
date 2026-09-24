# OTV2 — WorldProject v2 full-cardinality scale measurement

Allocation: R3, Issue #162 comment 5821985312. Branch: `agent/otv2-worldproject-v2-full-cardinality-scale-r3`.

Implementation base: `6185045c20631b8614b1d4ad1dacc5d918f73b0a`. This successor supersedes the pre-PR R2 head `362319d8c8cf6ee81420f3424b2cd93fa170107b`. It makes the supervised full measurement mandatory on the exact pull-request head, pins workflow actions to repository-standard immutable SHAs, installs Rust 1.94.0, and persists GNU `time -v` peak RSS in full-result JSON. Status: implementation and repeatable measurement harness. Measurement results are emitted by workflow artifacts; this task does not decide a production limit or storage format.

## Scope

Measure the existing canonical World Project v2 writer and reader using exactly 24,502,036 synthetic `CandidateOnly` placements, one synthetic declarative WorldObject definition, one world, one map revision, and one coordinate frame. The harness uses the real `oteryn-game-server` project canonicalization, snapshot parse, and v2 validation APIs. It separately times draft construction, canonical serialization/validation, JSON syntax parsing, schema-aware project loading, and writing the canonical role files into a temporary staged directory.

Only the explicit `--mode smoke` and `--mode full` modes are accepted. The probe accepts no file path, donor corpus, source artifact, or reference-world input. All generated records are labeled `SYNTHETIC_SCALE_STRESS_ONLY`; placements remain `CandidateOnly`. No source import, executable lowering, production world data, or game-runtime behavior is exercised.

## Measurement and resource boundary

The result reports bytes by canonical document role and in total, phase timings, and peak resident set size from the supervising `/usr/bin/time -v` process report in both completed and resource-limited full runs. The one-placement amplification value remains explicitly `ESTIMATE_NOT_GATE_COMPLETE`: it divides estimated whole-project rewrite bytes by an isolated serialized record difference and does not diff two full canonical snapshots or list changed files/bytes. It must not be treated as exact amplification or as a gate result. The run reports `completed` only if every requested placement is serialized and loaded. A caught allocation/canonicalization/load failure reports `resource_limit`; the workflow classifies process memory-ceiling termination and wall-clock timeout as `resource_limit` and validates/uploads the fallback result.

Pull requests run the bounded 2,000-placement smoke test separately and also run the supervised 24,502,036-placement full test against `github.event.pull_request.head.sha`. Every explicit `workflow_dispatch` runs the same full test. It is capped at 6 GiB virtual memory and 75 minutes. Its result is an artifact, not a gate that changes the production schema.

## Non-goals / decision fence

- Do not change project schema, evidence defaults, production placement limits, or chunking/storage strategy.
- Do not import or inspect donor/source-world datasets.
- Do not interpret a resource-limited harness run as proof that the production path fails; report the exact phase/ceiling and leave the decision to the owner.
- A completed synthetic measurement is evidence about the measured implementation and runner only, not a production capacity guarantee.

## Validation handoff

Every pull request runs the separate smoke and supervised full-cardinality jobs on the exact PR head; manual dispatch runs the full job too. Retain both the workflow run identity and uploaded result artifact. Full-snapshot changed-file/changed-byte amplification remains not gate-complete and must be reported as an estimate, not upgraded to a measured result without a real canonical snapshot diff.
