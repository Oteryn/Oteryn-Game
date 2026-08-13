# OTV2-20260811-game-char-stage-b-evidence-delta-02 — archived

```yaml
task_id: OTV2-20260811-game-char-stage-b-evidence-delta-02
title: Reduce GAME-CHAR Stage B B4-B8 evidence blockers with primary-source delta
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/OTV2-20260811-game-char-stage-b-evidence-delta-02
delivery_pr: 187
base_sha: 1411994c70abbf065273c0502c88413b61ca5ca0
reconciled_main_sha: 2ebce3d657f2f844883ef0a5f1a903adbf410984
final_head_sha: 2d93f3211a1a27f0e88f08fd13612fae9cf50193
delivery_merge_sha: dad86260b22744248cbe333eac0a650af919c993
lifecycle_closeout_pr: 188
owner: released
created_at: 2026-08-11T23:12:00+02:00
completed_at: 2026-08-11T23:39:00+02:00
execution_budget_minutes: 60
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
implementation_status: NOT_APPLICABLE
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
external_repositories: []
```

## Outcome

Delivered one nonbinding B4-B8 evidence delta against the accepted 2026-07-28 Reference target:

- `docs/architecture/GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md`.

The package reduces B4 progression, B5 promotion, B6 death/PvP, B7 offline-training and B8 modern build/progression uncertainty without accepting Stage B, changing current status/register/horizon, or authorizing runtime/schema work.

The canonical B1-B3 evidence remains the separate #185 package and was not duplicated in the final delta.

## Source of truth

- `PROVEN`: task started from `main@1411994c70abbf065273c0502c88413b61ca5ca0`.
- `PROVEN`: first Reference target is owner-accepted as Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary.
- `PROVEN`: PR #185 merged B1-B3 evidence as `60abba1188d5bfdeed53c468c243ed0fb0b01370`; PR #186 lifecycle-closed it as `2ebce3d657f2f844883ef0a5f1a903adbf410984`.
- `PROVEN`: delivery branch was non-destructively reconciled with both main advances via merge commits `40c4dcbcfad11dcb56d602b4d76487c8327eaa5a` and `6e9c3333ed2c5b0b0f495a5c6e009bb27d5babde`; final compare before merge had `behind_by=0` and exactly the two declared files.
- `PROVEN`: overall `GAME-CHAR-01` remains `PROPOSED / PLANNED / NOT_STARTED`.
- `PROVEN`: PR #162 remained disjoint and untouched.

## Acceptance criteria

- [x] Added a separate B4-B8 evidence delta; historical #183 and #185 reports remain unchanged.
- [x] Strengthened B4-B8 evidence with official primary sources for promotion, offline training, death/blessing families, Wheel, Weapon Proficiency, Character Bazaar transfer semantics and Animus.
- [x] Preserved `UNKNOWN`/`CONFLICT` where target continuity remains insufficient.
- [x] Separated semantic ownership/schema questions from exact formula/value implementation blockers as a nonbinding recommendation.
- [x] Established explicit/strong character-specific evidence for Weapon Proficiency progress, charms/charm points, Hunting Task Points and permanent Hunting Task/Prey slots without claiming one giant physical Character row.
- [x] Preserved Platform entitlement, GAME-ITEM/DUR-03 value/conservation and ruleset/content-definition boundaries.
- [x] Did not accept Stage B or overall GAME-CHAR.
- [x] Did not update current status/register/horizon.
- [x] Did not modify runtime, physical schema, protocol, content, Platform, Canary or external repositories.
- [x] Reconciled parallel #185/#186 without path conflict.
- [x] Repair cycle 1 removed duplicated B1-B3 scope from the draft delta and PR body.
- [x] Full exact-head self-review and repository-required documentation CI passed before merge.

## Key findings

### B4 progression

- target-era progression vocabulary is strong;
- exact XP/skill arithmetic remains insufficiently proven for July 28;
- recommendation: formula-neutral authoritative facts plus ruleset revision may support durable ownership work while exact arithmetic remains a SIM/ruleset/parity-fixture blocker, unless later evidence proves representation or atomicity depends on the formula.

### B5 promotion

- level >=20 + Premium is `DERIVED / very strong primary continuity`;
- exact 20,000 gp target continuity and Premium-lapse target edges remain unresolved;
- durable promotion achievement versus entitlement-derived active benefits remains a future owner-decision candidate.

### B6 death/PvP

- base death/blessing family has strong continuity;
- official history proves material world/profile differences;
- full target Twist/fair-fight/skull/Death Redemption edge matrix remains unresolved;
- recommendation: do not force one universal world-independent Character death formula before world/profile policy selects the relevant PvP scope.

### B7 offline training

- 10-minute activation, 12-hour maximum and 1:1 pool drain/refill semantics have strong primary continuity;
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

- the draft delta still repeated B1/B2 evidence after PR #185 had become the canonical B1-B3 evidence report.

Repair:

- rewrote `GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md` to scope strictly to B4-B8;
- updated PR #187 metadata to consume #185 rather than duplicate it;
- no accepted semantics changed.

Repair budget used: `1/3`.

## Validation

### Focused

Reconciled classifications against the accepted Reference evidence hierarchy, Stage-A ownership boundaries, prior #183 and canonical #185.

Result: **PASS after repair cycle 1**; no Stage-B acceptance, schema/runtime authority, B1-B3 duplicate authority or aggregate-ownership leakage remained.

### Mandatory exact-head self-review

- exact head: `2d93f3211a1a27f0e88f08fd13612fae9cf50193`;
- PR review id: `4910969517`;
- reviewer: implementing/coordinating agent, explicitly **not independent**;
- final material findings: `0`;
- verdict: **PASS**.

### Exact-head CI

Final delivery head `2d93f3211a1a27f0e88f08fd13612fae9cf50193`:

- Agent Governance `31538521240` / generation #858 — **success**;
- Dependency Review `31538521182` / generation #614 — **success**;
- CodeQL `31538521188` / generation #746 — **success**.

### Component/integration/runtime E2E

`NOT_APPLICABLE` — nonbinding paper-only evidence analysis; no executable/player-visible runtime behavior changed.

### Independent review

- required: `NO` under the trusted-base risk policy;
- reason: nonbinding paper-only evidence analysis with no executable security/protocol/persistence/production authority change and no unresolved final material finding.

## Delivery PR and boundaries

- delivery PR: #187;
- final changed files: exactly two declared documentation paths;
- unresolved review threads at merge: `0`;
- branch behind main at merge: `0`;
- squash merge: `dad86260b22744248cbe333eac0a650af919c993`;
- PR #162 remained disjoint and untouched;
- external repositories untouched;
- runtime/schema/protocol/content/Platform/production authority: **NONE**.

## Downstream consequence

Combined #183 + #185 + #187 evidence is now sufficient to stop treating every remaining numeric/formula uncertainty as an undifferentiated Stage-B schema blocker. The next paper-only step should build a minimal Stage-B closure decision packet that separates:

1. semantic ownership/versioning/migration decisions that must be owner-accepted before final durable Character schema;
2. exact Reference policy values/formulas that may remain later ruleset/SIM/parity-fixture gates if persistence stays formula-neutral and migration-safe;
3. PvP/world-profile details that belong to their dedicated policy gate rather than one universal Character rule.

No such closure recommendation is accepted by this archive.

## Context checkpoint

```yaml
last_progress: B4-B8 evidence delta delivered by PR #187 with repair cycle 1, exact-head self-review PASS and all required exact-head CI PASS; task ready for lifecycle archive/ownership release.
status: completed
delivery_pr: 187
final_head_sha: 2d93f3211a1a27f0e88f08fd13612fae9cf50193
delivery_merge_sha: dad86260b22744248cbe333eac0a650af919c993
lifecycle_closeout_pr: 188
repair_cycles_for_current_gate: 1
ci_run_ids:
  - 31538521240
  - 31538521182
  - 31538521188
owner_action_required: null
blocker: null
next_action: NONE
```
