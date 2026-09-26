# OTV2-20260926-monster-authoring-schema-v1

```yaml
task_id: OTV2-20260926-monster-authoring-schema-v1
title: Admit the monster authoring schema as a v1 candidate
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 938
jira: KAN-16
base_sha: 3b578bc5b35e40fbff70fbe5758079fdbed6a3c0
head_sha: 71deb830ac927fd3a49757f76b1463ad55bf3a23
final_head_sha: 71deb830ac927fd3a49757f76b1463ad55bf3a23
final_head_frozen_at: null
owner: released
created_at: 2026-09-26
updated_at: 2026-09-26
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-monster-authoring-schema-v1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner asked to close the open numeric questions of the monster authoring proposal and
bring it to `main` as a candidate, following the Item Master Schema v1 precedent (#903).
Authority: direct owner request in this session (no #162 allocation comment is claimed);
product/runtime implementation stays unallocated.
Result: `docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` (CANDIDATE) and the
executable package under `tools/content-schema/monster-authoring/`.

## Architecture and source of truth

- Origin (PROVEN): proposal v2 at `3630e92e9fcee07274411e81d3193e603be1bc24`
  (`codex/monster-authoring-schema-20260926`). That branch is not written by this task.
- Changes against the origin (PROVEN by diff): schema IDs `proposal:2` -> `candidate:1`;
  `percent` loses `multipleOf` and precision moves to the semantic validator (D1);
  validator requires lowest-terms ratios (D2); templates renamed; 13 added verification cases.
- WorldProject/v2 mapping (DERIVED by inspection of `apps/game-server/src/content/project/v2.rs`
  at base): architecture document §5.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: authoring schema and offline Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `build_formal_schema.py` regenerates all five JSON files byte-identically.
- `verify_formal_schema.py`: 170/170 at the final head (117 from the first candidate, 50 from the
  owner-requested P2 repair `9368d4b`/`d9e69ca`/`8eab166`, 3 for D7); D1/D2 negative cases fail
  when their rules are removed (mutation-checked locally).
- `verify_source_coverage.py`: 242 paths accounted, 0 unclassified.
- Runtime, compiler admission and Tibia Global parity: UNKNOWN, not claimed.
- Test batch (owner request): 10 Canary monsters converted by `canary_batch.py`; 10/10 bundles
  validate; after owner decisions D5-D7 (pass_through default, quest event omission, race
  residue `{item, fluid_type}` from Canary/Crystal `dropCorpse`) 10/10 manifests resolve.
  Output is deterministic across reruns.

Jira: KAN-16 stays `W toku`; this candidate does not complete the aggregate Story.

## Closeout

- [x] Final head `71deb830ac927fd3a49757f76b1463ad55bf3a23` passed the required checks; the owner
  enabled auto-merge and Merge Queue squash-merged PR #938 as
  `c72925db52ee8cc19265b63d76e8e3ca863d93fc`.
- [x] The bounded P2 repair session (ChatGPT) released its ownership before the final
  reconciliation push; no force push or history rewrite occurred.
- [x] KAN-16 received the merge note; ownership released and task archived.
- Follow-up (owner request): second Canary batch, wiki/reference-date comparison and this
  archival run under `OTV2-20260926-monster-authoring-batch-2`.
