# OTV2-20260811-game-char-stage-b-b1-b3-evidence

```yaml
task_id: OTV2-20260811-game-char-stage-b-b1-b3-evidence
title: Acquire GAME-CHAR Stage B naming deletion creation evidence
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-game-char-stage-b-b1-b3-evidence
pr: 185
base_sha: 1411994c70abbf065273c0502c88413b61ca5ca0
head_sha: null
final_head_sha: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T23:05:00+02:00
updated_at: 2026-08-11T23:18:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-game-char-stage-b-b1-b3-evidence.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_B1_B3_EVIDENCE_ACQUISITION.md
public_contracts: []
depends_on:
  - docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_RECONCILIATION.md
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
blocks:
  - target-quality naming normalization/recycling evidence
  - target-quality deletion/quota continuity evidence
  - target-quality creation/starter-state evidence
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Acquire and reconcile historical primary evidence for Stage-B blockers B1-B3 against the accepted 2026-07-28 Reference target. Prefer dated official Tibia/CipSoft pages and provenance-clear historical snapshots. Keep current-only rules `UNKNOWN` when July-28 continuity cannot be established.

## Acceptance criteria

- [x] Searched dated official primary sources for naming restrictions, uniqueness, rename and deleted-name reuse.
- [x] Searched dated official primary sources for deletion grace, quota interactions and undelete behavior; target-era proof for 60/30/recycling was not found.
- [x] Searched dated official primary sources for creation/starter flow and established Newhaven/Targuna chronology while preserving exact starter template gaps.
- [x] Attempted historical/archive discovery only with provenance requirements; no usable July-28 account-manual snapshot was resolved through the available search path.
- [x] Reconciled current official manuals only as post-target evidence and did not project them backward without continuity support.
- [x] Identified B1 and B3 claims that can move to strong continuity candidates while B2 remains largely target-UNKNOWN.
- [x] Did not accept Stage B or update current status/register/horizon.
- [x] Did not implement runtime/schema/content/protocol or write external repositories.
- [ ] Complete final exact-head self-review and repository-required documentation CI, merge and archive.

## Findings

### B1 naming

- official 2008 primary evidence explicitly establishes Tibia-wide/global character-name uniqueness;
- 2008 technical rules establish a long-lived name-format restriction family (including no >3 words and no special characters such as hyphen);
- 2010 Character Name Change introduction plus current official product manual provide strong continuity for six-month old-name lookup/history after rename;
- current max-29/no-number/no-special-character rules remain current-only for exact target continuity;
- deleted-name release is not equivalent to rename history: current account manual says at least six months and official support warns availability can take months or years, so exact recycling remains unresolved.

### B2 deletion/quota

- active-character limit 25 remains a strong derived continuity candidate from official 2025 20→25 change plus current manual;
- current 60-day deletion, total 30 and undelete/quota rules are clearly documented now;
- no provenance-clear target-era primary snapshot/change chronology was found for those exact values despite targeted search, so they remain `UNKNOWN` for July 28;
- generic repeat search is stopped pending a new historical source/hypothesis.

### B3 creation/starter

- Newhaven production launch on 2025-10-21 is primary proof that it became the new entry point before target;
- official pre-release Newhaven materials plus production launch/current Newhaven evidence support `DERIVED` strong continuity for tutorial→Newhaven/vocation-selection flow and the no-casual-re-vocation/return model; teaser semantics are not overclaimed as direct target proof;
- March 2026 official chronology plus follow-up live fixes establish Targuna as the post-Newhaven level-8+ continuation before July 28;
- current Quick Start still says Thais Peninsula/Blue Valley, a verified stale-official-documentation conflict;
- current skip-tutorial level-2 rule and exact starter stats/items/home/citizenship remain target-`UNKNOWN`.

## Material review repair

Repair cycle 1: the first report draft classified several Newhaven teaser semantics too strongly as `PROVEN pre-target release semantics`. The corrected report distinguishes the 2025 production launch (`PROVEN`) from individual teaser-described flow details, which are now `DERIVED` strong continuity unless a direct target/release-state source proves them. No accepted architecture or owner decision changed.

## Validation

### Focused

Result: **PASS after repair cycle 1** against accepted evidence classifications, July-28 target, Stage-A boundaries and no-schema/no-runtime constraints.

### Component/integration

`NOT_APPLICABLE` — nonbinding paper-only evidence analysis.

### E2E

`NOT_APPLICABLE` — no executable/player-visible behavior changes.

### Exact-head CI

Pending final immutable PR head after this bookkeeping commit.

## Self-review

- material findings before final freeze: `1`, repaired;
- final-head findings: pending.

## Independent review

- required: `NO` unless final diff unexpectedly changes accepted high-risk authority/security/protocol/persistence/production semantics; intended scope remains nonbinding evidence analysis only.

## PR and closeout

- delivery PR: #185;
- expected changed files: exactly task record + B1-B3 nonbinding evidence report;
- status/register/horizon changes: none;
- PR #162: parallel/disjoint and untouched;
- merge/archive: pending exact-head gates.

## Context checkpoint

```yaml
last_progress: B1-B3 evidence report is in PR #185; B1/B3 are partially reduced, B2 remains largely target-UNKNOWN, and repair cycle 1 corrected teaser/release evidence overclassification.
status: validating
branch: docs/OTV2-20260811-game-char-stage-b-b1-b3-evidence
pr: 185
final_head_sha: null
ci_check_generation: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: none for bounded evidence delivery; full Stage B remains evidence-blocked
next_action: Freeze final head, perform exact-head full-diff self-review, run documentation CI, merge/archive if all gates pass, then continue B4/B5 evidence acquisition.
```
