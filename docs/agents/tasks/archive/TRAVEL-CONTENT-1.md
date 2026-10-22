<!-- Current completion: actual target fields have been populated with DERIVED Oteryn working values. Original Global UNKNOWN/CONFLICT observations are archived, not reclassified as official proof. Earlier source-only sections below are historical context. -->

# TRAVEL-CONTENT-1

```yaml
task_id: TRAVEL-CONTENT-1
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/travel-content-1
pr: 2291
supersedes_pr: 2235
base_sha: ad227b2723c4df7b853e8976545ba210cb2e5eb0
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
updated_at: 2026-10-22
execution_policy: continuous_progress
owned_paths:
  - docs/reference/content-prep/travel-content-1/**
  - docs/agents/tasks/TRAVEL-CONTENT-1.md
  - docs/agents/tasks/archive/TRAVEL-CONTENT-1.md
```

## Outcome

Current Reference evidence is prepared for NPC-WORLD-1 #2232. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../../reference/content-prep/travel-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../../reference/content-prep/travel-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Results on the candidate tree (2026-10-22):

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `python tools/repository/validate_repository_policy.py`: pass
- every JSON file under `docs/reference/content-prep/travel-content-1/` parses and the apply-plan SHA-256 values match: pass
- `git diff --check`: pass

## Self-review

The implementing agent reviewed owned paths, provenance, factual field boundaries, family shard coverage, classification counts, source links and payload digests. Gameplay-sensitive community formulas remain held.

## Independent review

No external paid review triggered. Data-only draft Reference packets do not authorize gameplay changes or protected integration.

## Service placement constraint

Service evidence is staged under docs/reference with an explicit apply plan. World Bundle pin inputs cannot be updated under the owner's current scope. Existing native Service declarations are preserved exactly; canonical placement and qualified regeneration are the coordinator's integration step.

## Reference delivery boundary

All 56 original travel services and 195 route declarations are covered. Current NPC fares, directional carpet fares, explicit quest conditions, Free mainland boat eligibility and Premium carpet rules are extracted separately. Native historical candidates remain visible. Multiple NPC placements, omitted/obsolete route keywords and undocumented arrival tiles remain explicitly held. An NPC standing tile is never substituted for a destination arrival tile.

Task status `ready` refers only to the data/docs handoff. The PR remains draft; runtime admission and protected integration remain blocked. Complete candidate validation and exact remote-blob verification are recorded in the canonical draft PR.

## Supplemental source completion — 2026-10-18

All 56 native travel services and 195 routes remain covered. Supplemental current sources establish Premium-city access prerequisites, Gray Island fares, the 24-hour honeymoon window, local-ferry transport, placement-specific Junkar fares/world-change conditions and Hawkhurst/Tonar/Gnomprona access unlocks. Source-described NPC standing tiles are never used as arrival tiles. Four Junkar table/NPC fare disagreements remain explicit CONFLICT. Quest-start level 250 for Ingol is retained separately from an unproven post-unlock ferry level check. Historical Kendra anniversary travel is marked historical.

This authoring successor replaces the previous reference candidate only after full local validation and exact remote-byte verification. The previous CI results do not qualify a new head. Draft-only publication; no ready transition, merge or runtime admission.

### Final source-gap sweep

The final gap sweep also checks secondary NPC professions and resolves 18 ferry transport fields and 10 source-named origin areas. Exact floors/arrival tiles remain independently held.

## Full outstanding-field source sweep — 2026-10-18

oteryn:service.travel.captain_max_calassa/liberty_bay/origin; oteryn:service.travel.captain_seahorse/krailos/fare_gold; oteryn:service.travel.captain_waverider_island/liberty_bay/origin; oteryn:service.travel.gewen/marapur/fare_gold; oteryn:service.travel.gewen/marapur/quest_requirements; oteryn:service.travel.iyad/marapur/fare_gold; oteryn:service.travel.iyad/marapur/quest_requirements; oteryn:service.travel.melian/marapur/fare_gold; oteryn:service.travel.melian/marapur/quest_requirements; oteryn:service.travel.pino/marapur/fare_gold; oteryn:service.travel.pino/marapur/quest_requirements; oteryn:service.travel.tanyt/marapur/fare_gold; oteryn:service.travel.tanyt/marapur/quest_requirements; oteryn:service.travel.urks_the_mute/cormaya/fare_gold; oteryn:service.travel.ziyad/darama/fare_gold; oteryn:service.travel.ziyad/darashia/fare_gold; oteryn:service.travel.ziyad/edron/fare_gold; oteryn:service.travel.ziyad/femor_hills/fare_gold; oteryn:service.travel.ziyad/hills/fare_gold; oteryn:service.travel.ziyad/issavi/fare_gold; oteryn:service.travel.ziyad/kazor/fare_gold; oteryn:service.travel.ziyad/kazordoon/fare_gold; oteryn:service.travel.ziyad/svargrond/fare_gold.

Only explicit sourced values are selected. Wiki omissions and unsupported exact gameplay formulas remain held. Primary news retrieved from indexed official publications is labelled with the direct HTTP 403 limitation. Runtime activation and integration remain blocked.

## Working-data completion — 2026-10-19

Populated 639 previously held fields directly in target records; 0 active unfilled fields. See [parameter register](../../../reference/content-prep/travel-content-1/oteryn-parameters.json). Runtime admission and protected integration remain blocked.

## Closeout

Superseded draft #2235 by #2291, rebuilt on current `main` with the nested `target-files/` mirror flattened. Data/docs only; no runtime admission. Merge commit/result: squash merge of #2291. Review state: recorded on #2291.
