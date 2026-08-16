# OTV2-20260811-game-char-delta-02-provenance-correction

```yaml
task_id: OTV2-20260811-game-char-delta-02-provenance-correction
title: Correct merged GAME-CHAR delta-02 promotion source date
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-game-char-delta-02-provenance-correction
pr: 191
base_sha: 5d14fbaace55a8e1789cb9b9194830b358bed827
head_sha: 489a9adc115a6fedc281594b54f6b4186b2462a9
final_head_sha: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T23:59:00+02:00
updated_at: 2026-08-16
execution_budget_minutes: 30
owned_paths:
  - docs/agents/tasks/active/OTV2-20260811-game-char-delta-02-provenance-correction.md
  - docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md
  - docs/agents/tasks/archive/OTV2-20260811-game-char-stage-b-evidence-delta-02.md
public_contracts: []
depends_on:
  - docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md
  - docs/agents/tasks/archive/OTV2-20260811-game-char-stage-b-evidence-delta-02.md
blocks:
  - exact-head documentation/governance validation on the final task-containing head
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
merge_authority: ARCHITECTURE_COORDINATOR_ONLY
implementation_authority: NONE
```

`head_sha` records the immediately preceding ancestry-synchronized checkpoint. The exact final task-containing head after this truth update is recorded externally in PR/review/check evidence rather than by creating a self-referential follow-up commit.

## Outcome

Correct one factual provenance error that survived delivery/closeout of the nonbinding B4-B8 evidence delta and preserve a durable erratum trail without reopening or changing its substantive Stage-B evidence classifications.

The correction is deliberately bounded to:

1. canonical report E1 wording `Official 2001 release material` -> `Official 2002 release material`;
2. a post-closeout erratum note in the already archived original #187/#188 task;
3. this correction task's own delivery/closeout evidence.

No original task is reactivated and no GAME-CHAR decision/status/runtime/schema/content/protocol/Platform/production authority changes.

## Source of truth

- `PROVEN`: original delivery PR #187 merged as `dad86260b22744248cbe333eac0a650af919c993` and lifecycle closeout PR #188 archived its task.
- `PROVEN`: canonical report E1 historically said `Official 2001 release material` for `https://www.tibia.com/news/?id=122&subtopic=newsarchive`.
- `PROVEN`: the official Tibia source is `The Update is here!`, dated **February 18, 2002**.
- `PROVEN`: the source still supports the substantive core promotion claim: Premium players at/above level 20 can receive promotion.
- `PROVEN`: level >=20 + Premium remains `DERIVED / very strong primary continuity`.
- `PROVEN`: exact 20,000 gp July-28 continuity remains `UNKNOWN`.
- `PROVEN`: exact Premium-lapse suspension/reactivation July-28 semantics remain `UNKNOWN`.
- `PROVEN`: initial repair PR #189 was closed without merge because parallel #188 had already archived the original task; merging #189 from the stale base risked reintroducing an active task record.
- `PROVEN`: PR #162 remains disjoint CI/repository-governance work and is out of scope.

## Acceptance criteria

- [x] Correct only the E1 factual year `2001 -> 2002` in the canonical evidence report.
- [x] Preserve every substantive B4-B8 classification/recommendation unchanged.
- [x] Add an auditable post-closeout erratum note to the archived original task referencing correction lifecycle PR #191.
- [x] Do not re-create or mutate the archived original task as active.
- [x] Do not change GAME-CHAR status, current status/register/horizon, runtime/schema/content/protocol/Platform/production semantics.
- [x] Keep external repositories and PR #162 untouched.
- [x] Reconcile the three correction-owned paths to `main@5d14fbaace55a8e1789cb9b9194830b358bed827` without semantic loss; synchronized checkpoint `489a9adc115a6fedc281594b54f6b4186b2462a9` had `behind_by=0` and exactly three paths.
- [ ] Perform full exact-head self-review and fresh repository documentation/governance CI on the final task-containing head.
- [ ] Require zero unresolved material threads and `behind_by=0` immediately before merge.
- [ ] Squash-merge PR #191 on the unchanged exact head.
- [ ] Archive this correction task after terminal completion.

## Correction proof

### Canonical report

E1 now reads `Official **2002** release material`. The final `main...head` report patch is exactly one replacement line. No other report meaning changes. In particular, the following remain unchanged:

- level >=20 + Premium core eligibility: `DERIVED / very strong primary continuity`;
- exact 20,000 gp July-28 continuity: `UNKNOWN`;
- exact Premium-lapse suspension/reactivation July-28 semantics: `UNKNOWN`;
- report status: `PRE-DECISION EVIDENCE DELTA / NOT ACCEPTED`;
- Stage-B and overall GAME-CHAR acceptance remain out of scope.

### Original archive

`docs/agents/tasks/archive/OTV2-20260811-game-char-stage-b-evidence-delta-02.md` remains `status: completed` and owner released. A new `Post-closeout provenance erratum — 2026-08-16` section points to correction task/PR #191, records the exact year correction and explicitly preserves the substantive classifications and original delivery/closeout evidence.

The final archive patch adds the erratum and updates only the archive's `last_progress` checkpoint to acknowledge that post-closeout audit trail. The original active task is not recreated.

## Excluded scope

This task does not re-review or accept the B4-B8 evidence package, change promotion semantics, alter Stage B, update owner decisions, implement code/schema/content, resolve unrelated evidence gaps, modify global status/register/horizon, touch PR #162 or authorize any runtime/production change.

## Validation

### Focused

- final report semantic delta: factual source-year provenance only (`2001 -> 2002`);
- original archive semantic delta: post-closeout erratum/audit note plus `last_progress` acknowledgement only;
- correction task: bookkeeping/validation truth only;
- final synchronized diff: exactly the three declared owned paths;
- runtime/component/integration E2E: `NOT_APPLICABLE` — factual paper-only provenance correction.

### Current-main synchronization

Completed at checkpoint `489a9adc115a6fedc281594b54f6b4186b2462a9` against `main@5d14fbaace55a8e1789cb9b9194830b358bed827`: `behind_by=0`, exactly three correction-owned paths. This task-truth update is the only later head movement and changes no report/archive semantics.

### Exact-head CI

Pending on the exact final task-containing head produced by this checkpoint update.

## Self-review

Pending on the exact final task-containing head. Independent review is `NOT_REQUIRED` because the final scope is a factual nonbinding provenance correction plus archive/task bookkeeping, with no security/protocol/persistence/production/authority change.

## PR and closeout

PR #191 remains draft until exact-head CI, self-review, zero material threads and final drift verification are clean. After delivery merge, this correction task will be lifecycle-archived in its own bounded closeout PR.

## Context checkpoint

```yaml
last_progress: three correction-owned paths were synchronized to final current main; report/year and archived erratum are frozen; this task-truth checkpoint is the final content commit before exact-head validation
status: validating
branch: docs/OTV2-20260811-game-char-delta-02-provenance-correction
pr: 191
head_sha: 489a9adc115a6fedc281594b54f6b4186b2462a9
final_head_sha: null
owner_action_required: null
blocker: exact-head self-review and repository CI
next_action: VALIDATE_FINAL_TASK_CONTAINING_HEAD_THEN_MERGE
```

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`
