# OTV2-20260927-npc-authoring-schema-v1

```yaml
task_id: OTV2-20260927-npc-authoring-schema-v1
title: NPC authoring schema v1, Canary/Crystal converter, source diff, Fandom comparison and readiness census
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 975
jira: KAN-16
base_sha: a822326c9cf4607100e58bbc3673748f3fa299bb
head_sha: 91472e3a3832f3425ca5bfbf14b4e9b97bd1816c
final_head_sha: 91472e3a3832f3425ca5bfbf14b4e9b97bd1816c
final_head_frozen_at: null
owner: released
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-npc-authoring-schema-v1.md
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/npc-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request: migrate NPCs from Crystal and Canary into Oteryn structures and enrich them from
TibiaWiki BR and Fandom. Owner decisions in this session (recorded as D1–D3 in the schema doc):
first step is schema + converter + census without `content/` writes or native identity (D1);
Canary and Crystal are equal sources with no automatic winner (D2); Fandom now, TibiaWiki BR later
because it answers HTTP 403 (Cloudflare) from the build container (D3).
Authority: direct owner request in this session; promotion into `content/` stays unallocated.

## Architecture and source of truth

- `docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md` (this task) — companion of
  `OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md` and
  `OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md`. PROVEN by the validator and tests.
- Sources: `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`,
  `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`, tibia.fandom.com snapshot
  (revision ids in the compare evidence). Evidence class `OTS_HYPOTHESIS_ONLY`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `convert.py` converts all 1,036 Canary and 1,119 Crystal NPC files; 2,155/2,155 bundles pass
  `validate_npc.py`; two conversions are byte-identical (`bundle_digest` in each census).
- Census (`samples/census-*.json`): Canary 383 static-complete, 546 static-services, 35 scripted,
  65 unplaced, 7 not loadable; Crystal 215 / 745 / 39 / 116 / 4.
- Source diff (`samples/source-diff-*.json`): 1,026 pairs, 67 conflicting, 953 complementary,
  1 identical; 10 Canary-only, 93 Crystal-only NPCs.
- Fandom comparison (`samples/fandom-compare-*.json`): Canary 955 NPCs joined; trade buy prices
  5,089 match / 229 mismatch, travel 86 / 22, positions 807 within 3 sqm / 92 mismatch.
- `python -m unittest test_npc_authoring.py` and `python wiki_fandom.py self-test` pass offline.
- No committed bundle carries Tibia text (validator rule; LICENSE-ASSETS.md).

## Next action

Owner review of the schema doc and open decisions O1–O7; a later task promotes the
`STATIC_COMPLETE` / `STATIC_SERVICES` population into `content/` once O1, O2 and O5 are decided.

## Completion

- PR: #975
- Exact PR head: `91472e3a3832f3425ca5bfbf14b4e9b97bd1816c`
- Merge commit: `f31c0d65be86bafe924a0706cf821ca767305516`
- Merged at: `2026-09-27T11:10:55Z`
