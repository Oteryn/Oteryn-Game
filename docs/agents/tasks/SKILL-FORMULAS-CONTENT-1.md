# SKILL-FORMULAS-CONTENT-1

```yaml
task_id: SKILL-FORMULAS-CONTENT-1
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: content-prep/skill-formulas-content-1
pr: https://github.com/Oteryn/Oteryn-Game/pull/2242
base_sha: 64d5e4dab918ed0274dd9b1479b6218887b644bb
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - rulesets/character/skills/reference-evidence.json
  - rulesets/character/skills/index.json
  - docs/reference/crit-leech-skill-formulas.md
  - docs/reference/content-prep/skill-formulas-content-1/sources.json
  - docs/reference/content-prep/skill-formulas-content-1/README.md
  - docs/agents/tasks/SKILL-FORMULAS-CONTENT-1.md
  - docs/reference/content-prep/skill-formulas-content-1/research-audit.json
```

## Outcome

Current Reference evidence is prepared for LOOP-DAMAGE-1 #2195. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../reference/content-prep/skill-formulas-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../reference/content-prep/skill-formulas-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Publication requires the commands and results retained in the canonical draft PR. Local pre-publication checks are run on each complete candidate. Exact-head CI remains a separate check after publication.

## Self-review

The implementing agent reviewed owned paths, provenance, factual field boundaries, family shard coverage, classification counts, source links and payload digests. Gameplay-sensitive community formulas remain held.

## Independent review

No external paid review triggered. Data-only draft Reference packets do not authorize gameplay changes or protected integration.

## Reference delivery boundary

The live official qualitative critical/leech behavior and the complete community skill-advance candidates, constants and vocation multipliers are retained. Numeric critical, multi-target leech, rounding and skill formulas lack official or controlled evidence and remain held. No OTS implementation is promoted into proof.

Task status `ready` refers only to the data/docs handoff. The PR remains draft; runtime admission and protected integration remain blocked. Complete candidate validation and exact remote-blob verification are recorded in the canonical draft PR.

## Full outstanding-field source sweep —2026-10-18

combat.critical/intrinsic_chance; combat.critical/intrinsic_extra_damage.

Only explicit sourced values are selected. Wiki omissions and unsupported exact gameplay formulas remain held. Primary news retrieved from indexed official publications is labelled with the direct HTTP403 limitation. Runtime activation and integration remain blocked.
