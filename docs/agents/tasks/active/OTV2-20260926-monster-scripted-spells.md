# OTV2-20260926-monster-scripted-spells

```yaml
task_id: OTV2-20260926-monster-scripted-spells
title: Convert registered Canary monster spell scripts (D11/D12)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 955
jira: KAN-16
base_sha: 139202f1aa790e573c0ae5d41f4e529fdc503016
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-26
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-monster-scripted-spells.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request: step 2 of the monster import plan. Implement the D11/D12 schema extensions and
convert registered Canary spell scripts (P1-P3) into shared Abilities, so the spells stop
blocking monsters. P4 custom logic stays unresolved for native behaviours (D13).
Authority: direct owner request in this session; product/runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- Schema: `area.matrix`, fixed-tick DoT, `attribute_modifiers`, `Ability.variants`,
  `Ability.chain`, Formula `caster_magnitude`, schedule `magnitude`/`range_tiles`;
  `verify_formal_schema.py` 197/197 with each new negative case failing for its own reason.
- `spell_scripts.py` evaluates registered spells; `spell_census.py` output is unchanged after its
  classification moved into that module.
- `population_census.py` without wiki adoption: 1,315 of 1,656 fully resolved (1,103 before).
- Owner request (population step 1): D15 wiki values applied across the population. With the
  adoption 1,298 fully resolved, 352 blocked, 6 not converted, 0 structure-invalid; 988 resolved
  monsters carry adopted values. Bundles are written outside the repository with `--bundles`; the
  committed index holds one SHA-256 per bundle. Batch 1 10/10, batch 2 9/10 manifests resolve.
- `wiki_compare.py --population` parses thousands separators and marks `?`/`~` values as
  WIKI_UNCERTAIN (never adopted) and non-numeric values as WIKI_UNPARSED.
