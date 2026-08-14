# OTV2-20260814-ability-combat-first-evidence-fixtures

```yaml
task_id: OTV2-20260814-ability-combat-first-evidence-fixtures
title: Add first ABILITY_COMBAT evidence cases and fixture specs
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/ability-combat-first-evidence-fixtures
pr: 255
base_sha: 996da4270beadc548781fb81e95ea342e84b6376
head_sha: b83be016c24e8b670ffa1125495f5616892b80fe
final_head_sha: null
final_head_frozen_at: null
owner: architecture continuation agent
created_at: 2026-08-14T12:47:53+02:00
updated_at: 2026-08-14T13:20:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260814-ability-combat-first-evidence-fixtures.md
  - docs/agents/evidence/OTV2-20260814-ability-combat-official-spell-library.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md
depends_on:
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json
  - docs/architecture/GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md
  - docs/architecture/GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md
blocks:
  - target-continuity and provenance-cleared evidence for the first representative ABILITY_COMBAT cases
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Register the first bounded representative `ABILITY_COMBAT` target-case shapes for Light Healing and Ice Strike, plus non-executable pending fixture blueprints, while proving that indexed post-boundary official content cannot be promoted into the immutable 2026-07-28 target without both continuity evidence and cleared source provenance.

## Architecture and source of truth

- **PROVEN:** `main@996da4270beadc548781fb81e95ea342e84b6376` has accepted/pinned/lifecycle-closed Reference manifest v1 revision 2 with `cases=[]` and `ABILITY_COMBAT=NO_MECHANIC_CASES_REGISTERED`.
- **PROVEN:** `REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` requires post-target observations to remain `UNKNOWN` unless admissible continuity evidence bridges the target cut; patch-note/search absence is not continuity evidence; any provenance state other than `CLEARED` independently blocks evidence promotion.
- **PROVEN:** GAME-ABILITY partial baselines require typed targeting/legality, explicit commit/cost/cooldown anchors, distinct healing/damage semantics and catalogue-local evidence/fixture bindings.
- **INDEXED OFFICIAL DISCOVERY EVIDENCE:** exact official Tibia Library locators for Light Healing and Ice Strike were surfaced through search on 2026-08-14. The search index reported an approximately two-week-old crawl; direct page opens from the research environment returned HTTP 403 and exact crawl time is unavailable.
- **PENDING PROVENANCE:** exact official locator/source identity is known, but indexed content could not be directly revalidated against the official page; source and case provenance therefore remain `PENDING` for promotion.
- **UNKNOWN FOR TARGET:** continuity of every surfaced field/description to the 2026-07-28 cut, equality with the live page at retrieval time, plus exact heal magnitude, damage formula, exact range, complete legality, mitigation/resistance and RNG semantics.
- **PROVEN:** Oteryn GAME-ABILITY runtime implementation remains `NOT_STARTED`; no passing parity fixture is available.

## Acceptance criteria

- [x] Evidence capture contains only bounded paraphrased official-public indexed content, exact official locators, retrieval limitations and explicit continuity/provenance uncertainty; no proprietary assets/code.
- [x] Manifest increments to revision 3, changes only `ABILITY_COMBAT` inventory plus bounded cases/history, and preserves the immutable target/policy/normative-contract envelope.
- [x] Every new target case is `UNKNOWN`, Oteryn `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`, with source provenance `PENDING`, case provenance `PENDING`, legal review `PENDING`, no exact Oteryn revision and no fixture/test link.
- [x] Manifest source retrieval timestamps use RFC3339 UTC; exact crawl/live observation time remains null/unknown rather than invented.
- [x] No case claims target-cut or verified-live heal/damage metadata, magnitude, exact range, complete legality, resistance or RNG behavior.
- [x] Human-readable package separates indexed official content from live-page truth and immutable-target truth, projects source/case provenance `PENDING`, and defines pending fixture blueprints without calling them passing/executable parity fixtures.
- [x] `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` remains untouched in delivery because its currently selected action is this active package; lifecycle closeout will advance the canonical next action to continuity + provenance-cleared evidence.
- [ ] Complete fresh exact-head full-diff self-review, repository validation and lifecycle closeout.

## Excluded scope

No target classification above `UNKNOWN`; no `CLEARED` source/case provenance claim for indexed content; no verified-live-page claim; no runtime/client/protocol implementation; no physical catalogue schema/serializer/runner; no exact combat formulas or RNG; no DDL/Platform/production/external-repository writes; no new stable gate ID; no Codex/OpenAI/owner-funded AI use without separate exact authorization.

## Implementation / findings

Issue #254 / draft PR #255 own this bounded evidence package. Light Healing exercises the healing/self catalogue shape and Ice Strike the targeted attack/damage shape, but no immutable-target behavior is asserted from indexed current-state-near content alone.

**Material self-review finding 1 — repaired:** post-boundary indexed official content was initially classified as target `OBSERVED`. Owning continuity policy requires `UNKNOWN` without continuity evidence.

**Material self-review finding 2 — repaired:** the evidence artifact initially described the source too strongly as a retrieved current/live page. Re-verification showed search-indexed content for exact official locators, approximate crawl freshness and direct-fetch HTTP 403; exact crawl/live observation time is not invented.

**Material self-review finding 3 — repaired and propagated:** source/case `provenance_state=CLEARED` was too strong for indexed content that could not be directly revalidated against the official page. Evidence and manifest now use `PENDING` source/case provenance, and the human catalogue/blueprint package explicitly carries provenance as an independent prerequisite/result blocker.

The repairs make the recommended next evidence action precise: obtain time-appropriate **and provenance-cleared** evidence for these four representative cases before broadening mechanic inventory or freezing physical catalogue tooling.

## Validation

### Focused

- manifest JSON construction/parse: **PASS**.
- accepted schema v1 manual shape audit: **PASS** — `UNKNOWN` + `PARITY_PENDING_EVIDENCE`, `PENDING` source/case provenance, `PENDING` legal review, null exact revisions, empty fixture links and RFC3339 retrieval timestamp are all valid v1 shapes.
- fail-closed invariant audit: **PASS after repair 3** — 4 unique cases; all target `UNKNOWN`; all Oteryn `NOT_STARTED`; all parity `PARITY_PENDING_EVIDENCE`; all exact revisions null; all fixture/test links empty; all source provenance `PENDING`; all case provenance `PENDING`; all legal review `PENDING`; no case-level `OBSERVED`, `CLEARED` provenance or `PARITY_CONFIRMED`.
- inventory audit: **PASS** — only `ABILITY_COMBAT` changes to `MECHANIC_CASES_REGISTERED`; other domain coverage remains unchanged.
- continuity policy audit: **PASS after repair 1** — indexed post-boundary content is discovery evidence only and cannot promote target evidence.
- retrieval-mode audit: **PASS after repair 2** — exact official locators retained; search-index crawl freshness/direct-fetch 403 recorded; exact crawl/live time left unknown; retrieval timestamp is `2026-08-14T11:01:22Z`.
- source-provenance audit: **PASS after repair 3** — exact official locator identity is recorded, indexed content remains provenance `PENDING`, and the same blocker is propagated into catalogue projections/fixture blueprints.
- official indexed field readback: **PASS** for the bounded surfaced Light Healing and Ice Strike metadata/qualitative descriptions; no broader behavior inferred.

### Component/integration

- command/run: `NOT_APPLICABLE` — paper-only evidence/architecture package.
- result: `NOT_APPLICABLE`.

### E2E

- scenario: `NOT_APPLICABLE` — no executable gameplay behavior.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- all CI from superseded heads is stale after provenance repair 3 propagation.
- trigger source: pull_request.
- fresh resulting-head generation: required.
- result: pending.

## Self-review

- superseded head before propagation checkpoint: `b83be016c24e8b670ffa1125495f5616892b80fe`.
- method/reviewer: implementing/coordinating architecture continuation agent.
- material findings: 3 evidence classification/retrieval/provenance findings; all repaired and propagated across evidence/manifest/catalogue/metadata layers.
- verdict: pending final resulting-head full-diff recheck.

## Independent review

- required: NO unless a later material finding/risk expansion changes classification; this is paper-only public-evidence registration with no executable/security/durable-value authority.
- exact head: NOT_APPLICABLE.
- method/auditor: NOT_APPLICABLE.
- material findings: NOT_APPLICABLE.
- verdict: NOT_APPLICABLE.
- owner-funded AI: **NOT AUTHORIZED** for PR #255; prior authorizations do not carry forward.

## PR and closeout

- PR: #255 (draft).
- changed-file review: pending final resulting-head recheck; intended paths = 4.
- unresolved review threads: pending readback.
- related/superseded PRs: #239/#240 Dependabot and #191 GAME-CHAR are disjoint.
- protected auto-merge: pending.
- merge commit/result: pending.
- ownership release: pending.

## Context checkpoint

```yaml
last_progress: Propagated source/case provenance PENDING into the human catalogue and pending fixture blueprints; target continuity and provenance remain independent promotion blockers.
status: validating
branch: docs/ability-combat-first-evidence-fixtures
head_sha: b83be016c24e8b670ffa1125495f5616892b80fe
pr: 255
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: fresh resulting head required
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: false
blocker: null
next_action: Perform final four-path diff/schema/review-thread/main-drift audit on the resulting head, record immutable self-review evidence, and inspect fresh exact-head repository gates without marking PR ready.
```
