# SHARED-VITALS-CONTENT-1

```yaml
task_id: SHARED-VITALS-CONTENT-1
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: content-prep/shared-vitals-content-1
pr: https://github.com/Oteryn/Oteryn-Game/pull/2241
base_sha: 64d5e4dab918ed0274dd9b1479b6218887b644bb
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - rulesets/party/shared-experience/reference-evidence.json
  - rulesets/party/shared-experience/index.json
  - rulesets/character/stamina/reference-evidence.json
  - rulesets/character/stamina/index.json
  - rulesets/character/soul/reference-evidence.json
  - rulesets/character/soul/index.json
  - docs/reference/content-prep/shared-vitals-content-1/regeneration.json
  - docs/reference/content-prep/shared-vitals-content-1/food-durations.json
  - docs/reference/shared-experience-regeneration.md
  - docs/reference/content-prep/shared-vitals-content-1/sources.json
  - docs/reference/content-prep/shared-vitals-content-1/README.md
  - docs/agents/tasks/SHARED-VITALS-CONTENT-1.md
  - docs/reference/content-prep/shared-vitals-content-1/research-audit.json
```

## Outcome

Current Reference evidence is prepared for PARTY-XP-1 #2172; LOOP-VITALS-1 #2196; BED-REGEN-1 #2148. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../reference/content-prep/shared-vitals-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../reference/content-prep/shared-vitals-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Publication requires the commands and results retained in the canonical draft PR. Local pre-publication checks are run on each complete candidate. Exact-head CI remains a separate check after publication.

## Self-review

The implementing agent reviewed owned paths, provenance, factual field boundaries, family shard coverage, classification counts, source links and payload digests. Gameplay-sensitive community formulas remain held.

## Independent review

No external paid review triggered. Data-only draft Reference packets do not authorize gameplay changes or protected integration.

## Reference delivery boundary

Current official shared-XP eligibility/bonuses, stamina and soul rules are recorded. Five vocation/promotion HP/MP observations are structured numerically; 75 own-ID food timers are retained. Exact community-only regeneration, food and activity/drain numbers remain UNKNOWN for gameplay. Current official promotion-based soul caps and party bonuses take precedence, while Wiki disagreements remain recorded.

Task status `ready` refers only to the data/docs handoff. The PR remains draft; runtime admission and protected integration remain blocked. Complete candidate validation and exact remote-blob verification are recorded in the canonical draft PR.

## Complete food-source successor — 2026-10-18

The current rendered Food page contains 146 regenerative rows and 42 separately listed non-regenerative items. Every regenerative row is reconciled to its own current item-page revision. The duration packet now contains 140 existing-native-ID records and six explicitly source-only records (four IDs absent from native definitions and two multi-state cake pages). 143 numeric nutrition durations are retained from own-item Wiki data. The owner selected three additional DERIVED reference defaults: Cake (Item)240s, Cake with a Cherry240s, and Cheese Cookie24s copied from ordinary Cookie3598. All146 reference rows now have numeric durations; these three defaults are not exact-item source measurements. Five ordinary foods were spot-checked from an explicit hungry baseline using one item and a fresh official Food tooltip: brown mushroom 264 s, blueberry 12 s, cherry 12 s, raspberry 12 s, banana 96 s. These five checks agree with Wiki; no other row is claimed individually measured. The 42 non-regenerative names are retained separately without inventing zero-duration values. HP/MP regeneration rates, the food cap and other behaviour-sensitive formulas retain their existing evidence holds. No original binary, screenshot, character identity or balance is published.

Owned documentation returned to AUTHORING for a single expected-head successor. The inherited shared-experience index overlap with #2203 is untouched and excluded from mutation additions. Fresh complete-packet validation and exact-head CI are required; draft only, no merge or runtime activation.

## Owner-selected Food defaults —2026-10-18

The owner explicitly set both movable event-cake records to240seconds and requested an analogue for Cheese Cookie. Ordinary Cookie3598 supplies the selected24-second default. The exact-item source values remain unspecified and no additional controlled measurement is claimed. These three fields are DERIVED owner decisions, not PROVEN Tibia mechanics. The prior frozen draft remains unchanged until successor publication.

## Full outstanding-field source sweep —2026-10-18

New official unified intervals; twenty obsolete community timer candidates explicitly held as historical.

Only explicit sourced values are selected. Wiki omissions and unsupported exact gameplay formulas remain held. Primary news retrieved from indexed official publications is labelled with the direct HTTP403 limitation. Runtime activation and integration remain blocked.
