# OTV2-20260811-game-char-stage-b-evidence-delta-02

```yaml
task_id: OTV2-20260811-game-char-stage-b-evidence-delta-02
title: Reduce GAME-CHAR Stage B B4-B8 evidence blockers with primary-source delta
mode: COORDINATE
status: implementing
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02
delivery_pr: 187
delivery_final_head_sha: 2d93f3211a1a27f0e88f08fd13612fae9cf50193
delivery_merge_sha: dad86260b22744248cbe333eac0a650af919c993
postmerge_repair_branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02-postmerge-repair
postmerge_repair_pr: null
recovery_base_sha: dad86260b22744248cbe333eac0a650af919c993
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T23:12:00+02:00
updated_at: 2026-08-11T23:54:00+02:00
execution_budget_minutes: 60
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-game-char-stage-b-evidence-delta-02.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md
public_contracts: []
depends_on:
  - docs/architecture/GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_RECONCILIATION.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_B1_B3_EVIDENCE_ACQUISITION.md
  - docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md
blocks:
  - repair of merged source-date provenance defect
  - honest reduction of GAME-CHAR Stage-B B4-B8 UNKNOWN/CONFLICT evidence
  - eventual Stage-B owner decision package
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one nonbinding B4-B8 evidence delta against the accepted 2026-07-28 Reference target, strengthening progression/promotion/death/offline/build ownership evidence without rewriting historical dossiers, duplicating B1-B3 acquisition or accepting Stage B.

The original delivery PR #187 merged, but a previously reported factual provenance defect remained in the merged report. This task is recovered as the same canonical task for a bounded post-merge repair before lifecycle archive.

## Source of truth

- `PROVEN`: original task started from `main@1411994c70abbf065273c0502c88413b61ca5ca0` and was reconciled through lifecycle-closed B1-B3 work.
- `PROVEN`: delivery PR #187 final head `2d93f3211a1a27f0e88f08fd13612fae9cf50193` passed Agent Governance `31538521240` / #858, Dependency Review `31538521182` / #614 and CodeQL `31538521188` / #746 before squash merge `dad86260b22744248cbe333eac0a650af919c993`.
- `PROVEN`: delivery report remained nonbinding; `GAME-CHAR-01` remained `PROPOSED / PLANNED / NOT_STARTED` and no runtime/schema authority was created.
- `PROVEN`: an overlap/dependency review comment before merge identified that E1 incorrectly described Tibia news `id=122` as `Official 2001 release material`.
- `PROVEN`: official Tibia source `https://www.tibia.com/news/?id=122&subtopic=newsarchive`, titled `The Update is here!`, is dated **February 18, 2002** and still supports the substantive level-20 + Premium promotion core claim.
- `PROVEN`: #187 merged before this date defect was corrected; the active task also remained stale on `main` after merge, so recovery/closeout is required.
- `PROVEN`: open PR #162 owns disjoint CI/repository-governance paths and remains out of scope.

## Acceptance criteria

- [x] Added separate B4-B8 evidence delta and preserved B1-B3 authority in #185.
- [x] Strengthened B4-B8 evidence using official primary sources.
- [x] Preserved `UNKNOWN`/`CONFLICT` where target continuity remains insufficient.
- [x] Kept formula-vs-schema separation nonbinding.
- [x] Preserved Platform entitlement, GAME-ITEM/DUR-03 and ruleset/content-definition boundaries.
- [x] Did not accept Stage B or overall GAME-CHAR.
- [x] Did not update current status/register/horizon or implement runtime/schema/content/protocol.
- [x] Original delivery PR #187 passed exact-head repository CI and squash-merged.
- [x] Identified the post-merge factual defect: E1 source year `2001` is wrong.
- [x] Corrected E1 provenance to **2002** without changing the substantive promotion classification.
- [ ] Open bounded post-merge repair PR, perform full exact-head self-review and fresh documentation CI.
- [ ] Merge the repaired exact head only when all gates pass, then archive the complete recovered task and release ownership.

## Key evidence findings retained

### B4 progression

- target-era progression vocabulary is strong;
- exact XP/skill arithmetic remains insufficiently proven for July 28;
- nonbinding recommendation: formula-neutral authoritative facts + explicit ruleset revision may permit durable ownership work while exact arithmetic remains a SIM/ruleset/parity-fixture blocker, unless later evidence proves representation/atomicity depends on the formula.

### B5 promotion

- level >=20 + Premium is `DERIVED / very strong primary continuity`;
- corrected earliest cited promotion source is official **2002** release material, not 2001;
- exact 20,000 gp and Premium-lapse July-28 edges remain unresolved;
- durable promotion achievement versus entitlement-derived benefit activation remains an owner decision candidate.

### B6 death/PvP

- base death/blessing family has strong continuity;
- official history proves material world/profile differences;
- full target Twist/fair-fight/skull/Death Redemption edge matrix remains unresolved.

### B7 offline training

- timer/pool state machine has strong primary continuity;
- exact effectiveness/rounding/modifier behavior remains unresolved.

### B8 modern build/progression

- Wheel/Promotion Points are strong character-build state candidates;
- Weapon Proficiency Progress is explicitly character-bound/non-transferable;
- Character Bazaar transfer contract explicitly classifies charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots as character-specific;
- definition/formula/migration semantics and physical sub-aggregate placement remain downstream questions.

## Post-merge repair

### Finding

Merged report E1 said:

`Official 2001 release material introducing promoted vocations...`

The cited official page is dated **February 18, 2002**.

### Repair

Changed only the factual provenance wording:

`Official 2001 release material` -> `Official 2002 release material`.

The substantive evidence classification is unchanged:

- level >=20 + Premium core eligibility remains `DERIVED / very strong primary continuity`;
- exact 20,000 gp target continuity remains `UNKNOWN`;
- exact Premium-lapse target semantics remain `UNKNOWN`.

## Validation

### Original delivery exact-head CI

Final original delivery head `2d93f3211a1a27f0e88f08fd13612fae9cf50193`:

- Agent Governance `31538521240` / #858 — **success**;
- Dependency Review `31538521182` / #614 — **success**;
- CodeQL `31538521188` / #746 — **success**.

These checks prove the original head only and do not validate the repaired head.

### Post-merge repair focused validation

- source-date correction independently rechecked against official Tibia primary evidence;
- report remains nonbinding and the substantive promotion rule classification is unchanged;
- no B1-B3, GAME-CHAR status, DUR boundary, runtime/schema/content or external-repository semantics changed.

### Component/integration/E2E

`NOT_APPLICABLE` — paper-only evidence provenance repair.

### Repair exact-head CI

Pending repair PR final immutable head.

## Self-review

Pending repaired exact head. Review must verify the changed-file set is limited to the factual source-date repair plus recovered task metadata and that no semantic evidence classification drift was introduced.

## Independent review

- repair risk: low factual documentation correction;
- required: `NO` unless final diff unexpectedly changes accepted/high-risk semantics.

## Context checkpoint

```yaml
last_progress: PR #187 merged as dad86260b22744248cbe333eac0a650af919c993 with green repository CI but retained the already-reported E1 provenance date defect; same task recovered on a post-merge repair branch and report corrected from 2001 to 2002.
status: implementing
branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02-postmerge-repair
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 2
owner_action_required: null
blocker: null
next_action: Open the bounded post-merge repair PR, inspect the exact two-file diff, run self-review and fresh exact-head documentation CI, merge only if all gates pass, then archive the recovered task and release ownership.
```
