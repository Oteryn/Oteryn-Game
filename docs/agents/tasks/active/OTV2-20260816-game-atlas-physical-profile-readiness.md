# OTV2-20260816-game-atlas-physical-profile-readiness

```yaml
task_id: OTV2-20260816-game-atlas-physical-profile-readiness
title: Benchmark Game -> Atlas physical profile readiness
mode: AUDIT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: research/game-atlas-physical-profile-readiness
issue: 291
pr: 292
base_sha: 8722e565c6a0556934209820e3c14ee4f2dc6093
spike_evidence_head_sha: 8d43167d44efc7933b47713b47a35d71bf7ff708
spike_run_id: 31938999246
spike_job_id: 95145312378
spike_report_sha256: c20cc40de3ac1811574c29e116249314234caebab05276cfabb8c9d6b524d4f9
final_head_sha: null
final_head_frozen_at: null
owner: SENIOR OTERYN ECOSYSTEM ARCHITECT / MIGRATION COORDINATOR
created_at: 2026-08-16T11:05:23+02:00
updated_at: 2026-08-16T11:29:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260816-game-atlas-physical-profile-readiness.md
  - docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-readiness.md
  - docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-report.json
  - tools/game-atlas-profile-spike/**
  - .github/workflows/game-atlas-profile-spike.yml
public_contracts: []
depends_on:
  - docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - issue:#291
blocks:
  - canonical Oteryn-Game spatial/coordinate profile v1
  - first executable Game -> Atlas physical profile contract
  - Game-owned Atlas exporter implementation
  - Atlas consumer implementation
cross_repository_coordination_id: OTERYN-GAME-ATLAS-V1-PHYSICAL-READINESS
external_repositories: []
readiness_verdict: EVIDENCE_GAP
repair_cycles_for_current_gate: 3
ordinary_repair_budget_state: exhausted_no_fourth_cycle_without_owner_action_or_rescope
```

## Outcome

Produce a bounded, reproducible, proprietary-data-free evidence package that determines whether the first executable Game -> Atlas physical profile can now be authored without inventing canonical coordinate semantics.

**Result: `EVIDENCE_GAP`.** Final spike evidence is successful. It narrows encoding/chunk/floor-layout, locality, reviewability and boundedness trade-offs, but accepted Game -> Atlas semantics require coordinate/floor/order/anchor authority that current canonical Game architecture/implementation does not yet freeze at executable-profile precision.

Smallest missing owner evidence: one canonical Oteryn-Game spatial/coordinate profile v1 defining axes/orientation, numeric coordinate domain/bounds, floor identity/domain/order, point/bounds validity/inclusion, deterministic same-position stack/layer ordering, and presentation anchor/displacement semantics where canonical presentation references require them.

## Architecture and source of truth

- **PROVEN** — accepted Game -> Atlas semantic v1 requires a versioned `coordinate_profile` before executable producer/consumer compatibility and deliberately defers physical bytes, coordinate ceilings, chunk dimensions, compression and digest profile.
- **PROVEN** — ADR-0005 makes canonical World/Content independent from OTBM and leaves final technical chunk dimensions/vertical packing benchmark-sensitive.
- **PROVEN** — DUR-04 requires bounded physical-format evidence and fail-closed handling of unknown physical details/numeric limits.
- **PROVEN** — `RESOURCE_LIMITS_REGISTRY.json` requires hard maxima before implementation acceptance; this task does not promote synthetic harness caps into production limits.
- **PROVEN** — current executable `crates/client-domain::Position { x: i32, y: i32, floor: i16 }` is explicitly non-authoritative client projection and is not canonical Game coordinate authority.
- **PROVEN** — current workspace has no canonical world spatial/schema implementation crate proving the missing coordinate authority.
- **PROVEN** — multichannel scope makes public map definition one immutable World revision shared across channels; channel runtime overlays are not static Atlas source authority.
- **PROVEN** — final dedicated spike run `31938999246`, job `95145312378`, checked out exact head `8d43167d44efc7933b47713b47a35d71bf7ff708`, used Python `3.12.13`, passed `24/24` matrix rows plus encoding-neutral negative-cap checks, and produced report SHA-256 `c20cc40de3ac1811574c29e116249314234caebab05276cfabb8c9d6b524d4f9`.
- **DERIVED** — per-floor packaging is the stronger current access-locality baseline, but not a canonical decision.
- **DERIVED** — `32x32` versus `64x64` remains a locality/file-count/aggregate-size trade-off; no global winner is proven.
- **DERIVED** — JSON and JSONL compress nearly equivalently; JSONL has dramatically smaller changed source lines in this synthetic matrix, but reviewability alone does not select the distribution serializer.
- **DERIVED** — binary baseline is smaller but does not justify a public binary schema/dependency.
- **UNKNOWN** — canonical coordinate numeric domain/bounds, floor count/domain, production Atlas resource ceilings, public serializer, compression/container and representative browser/cache workload.
- **CONFLICT** — none identified.

## Decision timing

1. **Must decide now?** `YES` for the readiness verdict; `NO` for permanent serializer/chunk/coordinate choices inside this task.
2. **Concrete downstream work blocked:** canonical coordinate profile, first Game Atlas physical-profile contract, exporter, Atlas parser/validator and clean extraction around the executable boundary.
3. **What becomes harder later:** adopting OTBM/client projection/viewer conventions as authority would create a second world truth and migration lock-in.
4. **Supersession evidence:** accepted canonical World spatial contract/implementation, representative-world measurements, browser/cache performance and security/resource-limit evidence.
5. **Deliberately not decided:** permanent encoding, compression, chunk geometry, coordinate ceilings, storage/CDN, delta protocol, final World Project/Bundle format and repository migration mechanics.

## Acceptance criteria

- [x] Exact live coordinate/world/profile evidence is inventoried and truth-classified.
- [x] Synthetic fixtures contain no proprietary OTBM/Crystal/Canary/Tibia bytes or assets.
- [x] Research compares canonical JSON, canonical JSONL and a clearly non-public deterministic binary lower-bound baseline.
- [x] Research compares `32x32` vs `64x64` and per-floor vs packed-floor grouping.
- [x] Final cycle proves all 24 matrix cells deterministic and semantic-round-trip stable.
- [x] Evidence records raw/compressed size, chunk/file count, point/viewport bytes, floor-locality, data/package edit granularity, text-review granularity and corruption behavior.
- [x] Final cycle proves encoding-neutral research caps with negative cases; harness caps remain explicitly non-production.
- [x] Machine-readable normalized report and human evidence review pin exact final run/head evidence.
- [x] Final readiness verdict is `EVIDENCE_GAP` with one exact missing owner contract and one next action.
- [ ] Full final delivery-head diff self-review has zero open material findings.
- [ ] Required exact-head repository CI passes before merge.
- [x] Runtime/component/browser E2E is `NOT_APPLICABLE` with concrete reason.

## Excluded scope

No permanent/public serializer/schema, production exporter/consumer, canonical coordinate range/floor count, production resource-limit registry additions, final World Project/Bundle encoding, legacy migration/history extraction, repository create/rename/transfer, external-repository write, Synology/DNS/secret/deployment/production mutation, proprietary asset use or Codex/owner-funded AI.

## Implementation / findings

Issue #291 and PR #292 own the bounded research scope. Live preflight found no duplicate physical-profile work or overlapping active ownership; concurrent architecture PRs are path-disjoint.

The stdlib-only spike uses two deterministic `128 x 128 x 6` fixtures (`98,304` records each) and evaluates 24 combinations of three encodings, two chunk dimensions and two floor-packings.

Final stable observations:

- `64x64` canonical JSON/per-floor saves roughly `4-6%` aggregate gzip and reduces chunks `96 -> 24`, but costs roughly `3.8-3.9x` point bytes and `~1.7x` representative viewport bytes versus `32x32`;
- packing six synthetic floors saves roughly `6-8%` aggregate gzip at `32x32`, but multiplies one-floor access bytes roughly `5.6x`;
- compact JSON changed-line length ranges `95,514..3,291,861` bytes while JSONL remains `160..198` bytes;
- one semantic record edit changes exactly one data chunk plus `manifest.json` in every tested package;
- binary lower-bound size advantage is useful evidence but not sufficient for public-format selection.

## Repair history

### Cycle 1

Repaired invalid binary text-diff interpretation and added maximum textual line-size evidence.

### Cycle 2

Repaired asymmetric text generation/decoding chunk/count bounds.

### Cycle 3 — final ordinary repair

Made record/string/object-count validation encoding-neutral, added negative-cap tests, and corrected local edit evidence to distinguish changed data files (`1`) from changed package files (`2`, including manifest). Full spike reran successfully on exact head `8d43167d44efc7933b47713b47a35d71bf7ff708`.

**Ordinary repair budget is now exhausted at `3/3`.** Any new material defect requires owner action or an appropriately re-scoped successor; it must not be silently patched as cycle 4.

Initial standalone Agent governance run `31938342362` failed only PR-title metadata because the initial type `research` is unsupported. PR title was corrected to `test(atlas): benchmark physical profile readiness`; that failed generation is historical and not accepted final evidence.

## Validation

### Focused / synthetic spike

- exact accepted spike head: `8d43167d44efc7933b47713b47a35d71bf7ff708`
- run: `31938999246`
- job: `95145312378`
- Python: `3.12.13`
- compile: PASS
- self-test/matrix: `24/24` PASS
- encoding-neutral negative-cap checks: PASS
- report SHA-256: `c20cc40de3ac1811574c29e116249314234caebab05276cfabb8c9d6b524d4f9`

### Component/integration

Dedicated workflow checked out and verified exact spike head and emitted the complete machine-readable report. Result: PASS.

### E2E

`NOT_APPLICABLE` — research-only synthetic artifact measurement; no runtime exporter, Atlas browser or user journey change.

### Exact-head CI

Final delivery head is pending after evidence/task normalization. Required context: `Merge gate / validate` on the unchanged final delivery head.

## Self-review

- exact final delivery head: pending freeze
- method/reviewer: implementing/coordinating agent
- material findings: three repair cycles consumed; final whole-diff review pending
- verdict: pending

## Independent review

- required: `NO`
- reason: non-canonical synthetic research only; no public schema, runtime trust boundary, durable state, security authority, governance safety gate, production behavior or repository authority is accepted here; the later contract that freezes a public profile gets separate risk review
- verdict: `NOT_APPLICABLE`

## PR and closeout

- delivery PR: #292
- changed-file review / unresolved threads: pending final exact-head review
- merge: pending
- ownership release: separate post-merge lifecycle closeout required

## Context checkpoint

```yaml
last_progress: final cycle-3 spike evidence normalized; repair budget exhausted
status: validating
branch: research/game-atlas-physical-profile-readiness
spike_evidence_head_sha: 8d43167d44efc7933b47713b47a35d71bf7ff708
pr: 292
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending-final
ci_run_ids:
  - 31938999246
ci_job_ids:
  - 95145312378
runner_assignment_state: completed-for-final-spike
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 3
owner_action_required: null
blocker: null
next_action: freeze final delivery head, perform whole-diff self-review, require exact-head Merge gate PASS, merge #292, then archive/release task in a separate closeout PR
```
