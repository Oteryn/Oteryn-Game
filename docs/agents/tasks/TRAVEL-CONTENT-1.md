# TRAVEL-CONTENT-1

```yaml
task_id: TRAVEL-CONTENT-1
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: content-prep/travel-content-1
pr: https://github.com/Oteryn/Oteryn-Game/pull/2235
base_sha: 64d5e4dab918ed0274dd9b1479b6218887b644bb
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - docs/reference/content-prep/travel-content-1/sources.json
  - docs/reference/content-prep/travel-content-1/README.md
  - docs/agents/tasks/TRAVEL-CONTENT-1.md
  - docs/reference/content-prep/travel-content-1/target-files/content/services/travel/admission-evidence/evidence-00000-00194.json
  - docs/reference/content-prep/travel-content-1/target-files/content/services/travel/admission-evidence/index.json
  - docs/reference/content-prep/travel-content-1/target-files/content/services/travel/admission-evidence/services/evidence-00000-00055.json
  - docs/reference/content-prep/travel-content-1/target-files/content/services/travel/admission-evidence/services/index.json
  - docs/reference/content-prep/travel-content-1/service-apply-plan.json
  - docs/reference/content-prep/travel-content-1/research-audit.json
```

## Outcome

Current Reference evidence is prepared for NPC-WORLD-1 #2232. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../reference/content-prep/travel-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../reference/content-prep/travel-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Publication requires the commands and results retained in the canonical draft PR. Local pre-publication checks are run on each complete candidate. Exact-head CI remains a separate check after publication.

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
