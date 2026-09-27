# OTV2-20260927-monster-behaviour-patterns

```yaml
task_id: OTV2-20260927-monster-behaviour-patterns
title: Wiki ability scenes, spell behaviour patterns and plain-combat fields for Canary monsters (D18, D19)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-monster-behaviour-patterns.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner requests after PR #955: steps 2 and 3 of the population plan (wiki ability scene comparison,
grouping of custom-logic spell scripts into shared behaviours), then owner decisions D18 (accept the
19 behaviour patterns; boss logic goes to Encounters) and D19 (plain-combat schema fields), the armor
and shield mitigation field, and data forms for the summon, ally-healing, item-removal and path-trail
patterns.
Authority: direct owner requests in this session; product/runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `wiki_scenes.py`: scenes of 1,029 monsters compared with the Canary conversion rebuilt with the
  engine area rules; evidence only, nothing adopted (D18).
- `samples/p4-behaviour-patterns-canary-47dfd51f.json`: 93 blocking spell scripts in 19 patterns
  (model-assisted, evidence lines); the converter tags blocking rows with their pattern.
- Schema: damage `mitigated_by`; operations `remove_condition`, `remove_items`, `summon_creature`;
  damage/heal `affects`; Ability `path_requirement`; presentation `path_asset_binding`;
  `verify_formal_schema.py` 222/222.
- Converter: engine-faithful `setParameter` binding (undefined constant = 0), `spell_probes.py`
  behaviour probes, `RegisterPrimalPackBeast` omission, race-less Bestiary class from agreeing wiki.
- `population_census.py`: 1,350 of 1,656 fully resolved (1,298 on PR #955), 0 structure-invalid.
  Batch 1 10/10, batch 2 9/10 manifests resolve; source coverage 242/242.
