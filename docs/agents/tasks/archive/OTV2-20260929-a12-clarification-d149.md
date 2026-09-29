# OTV2-20260929-a12-clarification-d149

```yaml
task_id: OTV2-20260929-a12-clarification-d149
title: "A12 clarifications: D149 and binding evidence"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1245
base_sha: cf7777d7
head_sha: f60efa612a2c9fa25499f5ba52b96bc104e384e2
final_head_sha: f60efa612a2c9fa25499f5ba52b96bc104e384e2
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29T18:10:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md
  - docs/agents/tasks/active/OTV2-20260929-a12-clarification-d149.md
  - docs/agents/tasks/archive/OTV2-20260929-a12-item-identity-tibia-id.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records two follow-ups to A12 (#1237, `8af7f88e`), both answering the coordinator's
request on #162 (5893830130). The ruling is 5894110248.

- **Owner decision D149:** the 4,590 OT records with no CipSoft appearance are removed from
  authored content. Their old keys get `retired_without_successor`.
- **Architect clarifications:**
  - §4.2 defines `EXACT` evidence: the appearance reference plus CipSoft record continuity. A
    differing OT name alone does not block `EXACT`.
  - A repurposed id becomes `CONFLICT`.
  - §5 "no binding lost" means every binding row survives with a disposition.

The task also adds review details to the #1237 task record, which the coordinator archived in #1250. No code, content or key change is made.

## Architecture and source of truth

- `PROVEN`: A12 on `main@cf7777d7`; the coordinator request 5893830130; the G4 decision.
- `UNKNOWN`: none added.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. ITEM-ID-1 carries the identity and migration review.

## Acceptance criteria

- [ ] The amendment is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, content, keys, bindings and migrations.

## Finding dispositions

The Codex review of `ca8cece` raised four P1 findings. All were accepted and fixed in the repair
commit.

- 4135778774, continuity for retired ids: the comparison file is the last admitted file that
  contains the id. Its removal is proven by absence from every later manifest.
- 4135778781, `CONFLICT` as a binding: every source row is kept as a crosswalk evidence row with a
  disposition. Only `EXACT` and `ACCEPTED_ALIAS` emit target-bearing G4 bindings, which matches the
  current binding schema and importer.
- 4135778789, requalification: every crosswalk row is re-derived from its own §4.2 evidence. The
  manifests now carry per-id appearance record digests, so continuity can be proven.
- 4135778798, D149 history: each D149 key keeps a tombstone (its last authored definition and
  digest) in a historical archive. Durable rows resolve to it, and live materialization fails
  closed for that item only.

The owner-requested re-review of `7a594cf` raised three P1 findings. All were accepted and fixed
in the second repair commit.

- 4135816153, record evolution: continuity compares an identity projection digest (object class
  and CipSoft name). Flag, sprite and other field changes become evolution notes.
- 4135816165, alias requalification: `ACCEPTED_ALIAS` needs its G4 rule 7 duplicate proof and
  acceptance provenance. Without them the row becomes `AMBIGUOUS`, and no acceptance is invented.
- 4135816175, tombstone live path: ITEM-ID-1 must test that a tombstoned key never materializes as
  a live item, the row stays byte-identical, and the event is reported.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff --check`

## Context checkpoint

```yaml
last_progress: protected-integrated as 2065ad86; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: f60efa612a2c9fa25499f5ba52b96bc104e384e2
pr: 1245
owner_action_required: null
blocker: null
next_action: null
```

## Closeout

- PR #1245 merged via Merge Queue: final head `f60efa612a2c9fa25499f5ba52b96bc104e384e2`, merge commit `2065ad86b411a8ed0c4b1953999acb41ca119492`.
- Protected-main readback: all 3 files the merge changed are byte-identical between the final head and the merge commit.
- Ownership released on merge. The record stayed in `tasks/active/` and turned `Agent governance` on `main` red; archived by the Work coordinator in a P0 archive batch.
