# OTV2-20260811-game-char-stage-b-b1-b3-evidence — archived

```yaml
task_id: OTV2-20260811-game-char-stage-b-b1-b3-evidence
title: Acquire GAME-CHAR Stage B naming deletion creation evidence
mode: COORDINATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
delivery_branch: docs/OTV2-20260811-game-char-stage-b-b1-b3-evidence
delivery_pr: 185
lifecycle_closeout_branch: docs/OTV2-20260811-game-char-stage-b-b1-b3-evidence-closeout
lifecycle_closeout_pr: 186
base_sha: 1411994c70abbf065273c0502c88413b61ca5ca0
final_head_sha: 44e6d6da4368870f436aa070bc7266a043963881
delivery_merge_sha: 60abba1188d5bfdeed53c468c243ed0fb0b01370
owner: released
created_at: 2026-08-11T23:05:00+02:00
completed_at: 2026-08-11T23:20:00+02:00
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
implementation_status: NOT_APPLICABLE
```

## Outcome

Acquired and reconciled historical primary evidence for Stage-B blockers B1-B3 against the accepted 2026-07-28 Reference target, preferring dated official Tibia/CipSoft evidence and preserving current-only rules as `UNKNOWN` where target continuity could not be established.

Canonical nonbinding report:

- `docs/architecture/GAME-CHAR-01_STAGE_B_B1_B3_EVIDENCE_ACQUISITION.md`.

Result:

- B1 naming: **PARTIALLY REDUCED**;
- B2 deletion/quota: **LARGELY UNCHANGED / TARGET UNKNOWN** beyond the strong 25-active-character continuity candidate;
- B3 creation/starter: **PARTIALLY REDUCED**;
- no Stage-B owner acceptance or implementation authority created.

## Acceptance criteria

- [x] Searched dated official primary sources for naming restrictions, uniqueness, rename and deleted-name reuse.
- [x] Searched dated official primary sources for deletion grace, quota interactions and undelete behavior; target-era proof for 60/30/recycling was not found.
- [x] Searched dated official primary sources for creation/starter flow and established Newhaven/Targuna chronology while preserving exact starter template gaps.
- [x] Attempted historical/archive discovery only with provenance requirements; no usable July-28 account-manual snapshot was resolved through the available search path.
- [x] Reconciled current official manuals only as post-target evidence and did not project them backward without continuity support.
- [x] Identified B1 and B3 claims that can move to strong continuity candidates while B2 remains largely target-UNKNOWN.
- [x] Did not accept Stage B or update current status/register/horizon.
- [x] Did not implement runtime/schema/content/protocol or write external repositories.
- [x] Performed final exact-head self-review and repository-required documentation CI before delivery merge.
- [x] Delivery PR #185 squash-merged and closeout task moved from active to archive.

## Excluded scope

This task did not:

- address B4-B8 formulas/promotion/death/offline/build ownership except where source chronology directly intersected B1-B3;
- define a physical Character schema, indexes, locks or migrations;
- accept an intentional Reference difference;
- use Canary/crystalserver/community implementation as Reference evidence authority;
- implement runtime/client/protocol/content/Platform/production behavior;
- modify external repositories or PR #162.

## Findings

### B1 naming

- official 2008 primary evidence explicitly establishes Tibia-wide/global character-name uniqueness;
- 2008 technical rules establish a long-lived name-format restriction family, including no more than three words and no special characters such as a hyphen;
- 2010 Character Name Change introduction plus current official product manual provide strong continuity for six-month old-name lookup/history after rename;
- current max-29/no-number/no-special-character rules remain current-only for exact target continuity;
- deleted-name release is not equivalent to rename history: current account manual says at least six months and official support warns availability can take months or years, so exact recycling remains unresolved.

B1 status: **PARTIALLY REDUCED**.

Still blocked:

- exact normalization/case/space/repertoire;
- exact 29-letter July-28 continuity;
- restricted/reserved pattern authority;
- exact deleted-name release timing/algorithm;
- complete rename collision/history semantics.

### B2 deletion/quota

- active-character limit 25 remains a strong `DERIVED` continuity candidate from official 2025 20→25 change plus current manual;
- current 60-day deletion, total 30 and undelete/quota rules are clearly documented now;
- no provenance-clear target-era primary snapshot/change chronology was found for those exact values despite targeted search;
- target continuity for 60 days, total 30, undelete interaction and deleted-name hold remains `UNKNOWN`;
- generic repeat search is stopped pending a genuinely new historical source/hypothesis.

B2 status: **LARGELY UNCHANGED / TARGET UNKNOWN** beyond the 25-active continuity candidate.

### B3 creation/starter

- Newhaven production launch on 2025-10-21 is primary proof that it became the new entry point before target;
- official pre-release Newhaven materials plus production launch/current Newhaven evidence support `DERIVED` strong continuity for tutorial→Newhaven/vocation-selection flow and the no-casual-re-vocation/return model; teaser semantics are not direct target proof;
- March 2026 official chronology plus follow-up live fixes establish Targuna as the post-Newhaven level-8+ continuation before July 28;
- current Quick Start still says Thais Peninsula/Blue Valley, a verified stale-official-documentation conflict;
- current skip-tutorial level-2 rule and exact starter stats/items/home/citizenship remain target-`UNKNOWN`.

B3 status: **PARTIALLY REDUCED**.

Still blocked:

- exact target-era skip-tutorial level-2 continuity;
- exact creation input validation/error semantics;
- exact starter stats/items/template;
- home/citizenship/respawn details;
- precise account-state eligibility rules.

## Material delivery review repair

Delivery repair cycle 1 corrected overclassification of Newhaven teaser semantics from `PROVEN` to `DERIVED` continuity where appropriate. The production launch itself remains `PROVEN`; individual teaser-described flow details are not direct July-28 proof merely because Newhaven later launched.

## Evidence-acquisition stopping rule

This bounded pass reached diminishing returns for general primary-web search on B1-B3.

Further B1-B3 progress requires new evidence leverage such as:

1. provenance-clear archived official manual snapshots near July 28;
2. owner-provided historical official captures;
3. newly discovered rule-specific official change chronology;
4. controlled current observation only when continuity to July 28 is independently established.

Do not repeat generic searches for the same 60/30/skip-level-2 phrases without a new hypothesis.

## Validation

### Focused

- method: reconcile B1-B3 claims against the accepted July-28 evidence classifications, Stage-A semantics and no-schema/no-runtime boundaries;
- result: **PASS after delivery repair cycle 1**;
- unresolved evidence findings: B1/B2/B3 gaps listed above remain explicitly `UNKNOWN`/open rather than guessed.

### Component/integration

- result: `NOT_APPLICABLE` — nonbinding paper-only evidence analysis.

### E2E

- result: `NOT_APPLICABLE` — no executable/player-visible behavior changed.

### Delivery exact-head CI

Final delivery head `44e6d6da4368870f436aa070bc7266a043963881`:

- Agent Governance `31537120706` / #847 — **success**;
- Dependency Review `31537120707` / #605 — **success**;
- CodeQL `31537120702` / #735 — **success**.

### Delivery self-review

- exact head: `44e6d6da4368870f436aa070bc7266a043963881`;
- review id: `4910838001`;
- material findings before final freeze: `1`, repaired;
- final material findings: `0`;
- verdict: **PASS**.

### Independent review disposition

- required for delivery: `NO` under the trusted-base risk policy;
- reason: nonbinding paper-only evidence analysis with no executable security/protocol/persistence/production authority change after final self-review;
- result: `NOT_APPLICABLE`.

## PR and closeout

- delivery PR: #185;
- delivery changed files: exactly the active task record plus the B1-B3 nonbinding evidence report;
- delivery unresolved review threads at merge: `0`;
- delivery branch behind main at merge: `0`;
- delivery squash merge: `60abba1188d5bfdeed53c468c243ed0fb0b01370`;
- lifecycle closeout PR: #186;
- closeout intended diff: active task removal + complete archive record only;
- related PR #162: parallel/disjoint and untouched;
- ownership: released after terminal closeout.

## Boundaries preserved

- overall `GAME-CHAR-01` remains `PROPOSED / PLANNED / NOT_STARTED`;
- Stage A remains the accepted partial scope;
- Stage B remains unaccepted;
- accepted July-28 Reference target remains unchanged;
- final DUR-02 character schema remains blocked;
- no runtime/client/protocol/physical-schema/content/Platform/production/external-repository authority is created.

## Context checkpoint

```yaml
last_progress: B1-B3 evidence report delivered by PR #185 at exact head 44e6d6da4368870f436aa070bc7266a043963881 with self-review and all required CI PASS; closeout PR #186 preserves the full task record after review finding repair.
status: completed
delivery_branch: docs/OTV2-20260811-game-char-stage-b-b1-b3-evidence
delivery_pr: 185
final_head_sha: 44e6d6da4368870f436aa070bc7266a043963881
delivery_merge_sha: 60abba1188d5bfdeed53c468c243ed0fb0b01370
lifecycle_closeout_pr: 186
repair_cycles_for_delivery: 1
closeout_review_finding: abbreviated archive omitted canonical task scope/evidence; repaired by preserving complete task record plus terminal evidence
owner_action_required: null
blocker: none for lifecycle closeout; full GAME-CHAR Stage B remains evidence-blocked
next_action: after closeout merge, continue autonomous Stage-B evidence acquisition with B4/B5 and revisit B1-B3 only when genuinely new historical primary evidence appears.
```
