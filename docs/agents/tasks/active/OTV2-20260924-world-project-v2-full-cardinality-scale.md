# OTV2 — World Project v2 full-cardinality scale measurement

Status: implementation and repeatable measurement harness. Measurement results are emitted by the workflow artifact; this task does not decide a production limit or storage format.

## Scope

Measure the existing canonical World Project v2 writer and reader using exactly 24,502,036 synthetic `CandidateOnly` placements, one synthetic declarative WorldObject definition, one world, one map revision, and one coordinate frame. The harness uses the real `oteryn-game-server` project canonicalization, snapshot parse, and v2 validation APIs. It separately times draft construction, canonical serialization/validation, JSON syntax parsing, schema-aware project loading, and writing the canonical role files into a temporary staged directory.

Only the explicit `--mode smoke` and `--mode full` modes are accepted. The probe accepts no file path, donor corpus, source artifact, or reference-world input. All generated records are labeled `SYNTHETIC_SCALE_STRESS_ONLY`; placements remain `CandidateOnly`. No source import, executable lowering, production world data, or game-runtime behavior is exercised.

## Measurement and resource boundary

The result reports bytes by canonical document role and in total, phase timings, Linux peak resident set size when available, and one-placement edit amplification. Amplification is defined in the result as the complete canonical project byte count rewritten for a one-record change divided by changed bytes in that serialized placement record. The run reports `completed` only if every requested placement is serialized and loaded. A caught allocation/canonicalization/load failure reports `resource_limit`; the supervised workflow also classifies process memory-ceiling termination and wall-clock timeout as `resource_limit` with an explicit reason.

Pull requests run the bounded 2,000-placement smoke test. The full test is manual (`workflow_dispatch`, `run_full=true`) and is capped at 6 GiB virtual memory and 75 minutes. Its result is an artifact, not a gate that changes the production schema.

## Non-goals / decision fence

- Do not change project schema, evidence defaults, production placement limits, or chunking/storage strategy.
- Do not import or inspect donor/source-world datasets.
- Do not interpret a resource-limited harness run as proof that the production path fails; report the exact phase/ceiling and leave the decision to the owner.
- A completed synthetic measurement is evidence about the measured implementation and runner only, not a production capacity guarantee.

## Validation handoff

Run the PR workflow smoke job for code-level qualification. When the owner explicitly supervises the scale exercise, dispatch the workflow with `run_full=true`, retain the artifact and workflow run identity, and update the evidence JSON from the artifact without changing the scope or decision fence above.
