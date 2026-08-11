# OTV2-20260811-game-char-stage-b-evidence-delta-02

```yaml
task_id: OTV2-20260811-game-char-stage-b-evidence-delta-02
title: Reduce GAME-CHAR Stage B B4-B8 evidence blockers with primary-source delta
mode: COORDINATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02
pr: 187
base_sha: 1411994c70abbf065273c0502c88413b61ca5ca0
reconciled_main_sha: 2ebce3d657f2f844883ef0a5f1a903adbf410984
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-11T23:12:00+02:00
updated_at: 2026-08-11T23:34:00+02:00
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
  - honest reduction of GAME-CHAR Stage-B B4-B8 UNKNOWN/CONFLICT evidence
  - eventual Stage-B owner decision package
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Produce one nonbinding B4-B8 evidence delta against the accepted 2026-07-28 Reference target, strengthening progression/promotion/death/offline/build ownership evidence without rewriting historical dossiers, duplicating B1-B3 acquisition or accepting Stage B.

## Source of truth

- `PROVEN`: task started from `main@1411994c70abbf065273c0502c88413b61ca5ca0`.
- `PROVEN`: PR #185 merged B1-B3 evidence as `60abba1188d5bfdeed53c468c243ed0fb0b01370`; PR #186 lifecycle-closed it as `2ebce3d657f2f844883ef0a5f1a903adbf410984`.
- `PROVEN`: this branch was reconciled non-destructively with both main advances through merge commits `40c4dcbcfad11dcb56d602b4d76487c8327eaa5a` and `6e9c3333ed2c5b0b0f495a5c6e009bb27d5babde`; compare after #186 showed `behind_by=0` and exactly the two declared owned paths.
- `PROVEN`: first Reference target is owner-accepted as Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary.
- `PROVEN`: prior Stage-B reconciliation, #185 B1-B3 evidence and this delta remain nonbinding evidence analysis; overall `GAME-CHAR-01` remains `PROPOSED / PLANNED / NOT_STARTED`.
- `PROVEN`: open PR #162 owns disjoint CI/repository-governance paths and is out of scope.

## Acceptance criteria

- [x] Added a separate B4-B8 evidence delta; historical #183 and #185 reports remain unchanged.
- [x] Strengthened B4-B8 evidence using official primary sources for promotion, offline training, death/blessing families, Wheel, Weapon Proficiency, Character Bazaar transfer semantics and Animus.
- [x] Preserved `UNKNOWN`/`CONFLICT` where target continuity remains insufficient.
- [x] Separated semantic ownership/schema questions from exact formula/value implementation blockers as a nonbinding recommendation.
- [x] Established explicit/strong character-specific evidence for Weapon Proficiency progress, charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots without claiming one giant physical Character row.
- [x] Preserved Platform entitlement, GAME-ITEM/DUR-03 value/conservation and ruleset/content-definition boundaries.
- [x] Did not accept Stage B or overall GAME-CHAR.
- [x] Did not update current status/register/horizon.
- [x] Did not modify runtime, physical schema, protocol, content, Platform, Canary or external repositories.
- [x] Reconciled parallel #185/#186 without path conflict.
- [x] Self-review repair cycle 1 removed duplicated B1/B2 analysis from the draft delta and PR body; B1-B3 now consume only the canonical #185 report.
- [ ] Perform full exact-head self-review and repository-required documentation CI before merge.

## Key findings

### B4 progression

- target-era progression vocabulary is already strong;
- exact XP/skill arithmetic remains insufficiently proven for July 28;
- **recommendation:** formula-neutral authoritative facts + ruleset revision should allow durable ownership work to proceed while exact arithmetic remains a SIM/ruleset/parity-fixture blocker, unless later evidence proves representation/atomicity depends on the formula.

### B5 promotion

- level >=20 + Premium is `DERIVED / very strong primary continuity`;
- 20,000 gp and Premium-lapse target edges remain unresolved;
- durable promotion achievement versus entitlement-derived benefit activation remains an owner decision candidate.

### B6 death/PvP

- base death/blessing family has strong continuity;
- official history proves material world/profile differences;
- full target Twist/fair-fight/skull/Death Redemption edge matrix remains unresolved;
- **recommendation:** do not force one universal world-independent Character death formula before GAME-CHANNEL/world-profile scope is accepted.

### B7 offline training

- 10-minute activation, 12-hour maximum and 1:1 counter drain/refill state machine have strong primary continuity;
- exact effectiveness/rounding/modifier behavior remains unresolved;
- semantic counter/capability ownership is near freeze-ready while arithmetic remains ruleset evidence-blocked.

### B8 modern build/progression

- Wheel/Promotion Points are strong character-build state candidates;
- Weapon Proficiency Progress is explicitly character-bound/non-transferable;
- official Bazaar transfer contract explicitly classifies charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots as character-specific;
- Animus Mastery is strongly aligned with character-specific progression;
- definition/formula/migration semantics and physical sub-aggregate placement remain downstream questions.

## Review repair

### Repair cycle 1

Finding before final freeze:

- draft delta still repeated B1/B2 evidence even though PR #185 had already delivered the canonical B1-B3 acquisition.

Repair:

- rewrote `GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md` to scope strictly to B4-B8;
- changed PR #187 metadata to consume #185 for B1-B3 rather than duplicate it;
- preserved all B4-B8 evidence findings and no accepted semantics changed.

Repair budget used: `1/3`.

## Validation

### Focused

- reconciled classifications against accepted Reference evidence hierarchy, Stage A ownership boundaries, prior #183 and canonical #185;
- result: **PASS after repair cycle 1**; no Stage-B acceptance, schema/runtime authority or B1-B3 duplicate authority remains.

### Component/integration/E2E

`NOT_APPLICABLE` — nonbinding paper-only evidence analysis only.

### Exact-head CI

Pending final immutable PR head after this bookkeeping commit.

## Self-review

Pending final immutable PR head.

## Independent review

- required: `NO` unless final diff unexpectedly changes accepted security/protocol/durable-data/production authority; intended final scope is nonbinding paper-only evidence analysis.

## Context checkpoint

```yaml
last_progress: PR #187 is now strictly B4-B8, reconciled with lifecycle-closed #185/#186; repair cycle 1 removed duplicate B1/B2 scope before final freeze.
status: validating
branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02
pr: 187
final_head_sha: null
final_head_frozen_at: null
ci_checks_for_current_head: 0
repair_cycles_for_current_gate: 1
owner_action_required: null
blocker: null
next_action: Freeze final PR head, perform full-diff self-review, run exact-head documentation CI, merge/archive only if all gates pass, then continue from combined #183/#185/#187 evidence toward Stage-B closure.
```
