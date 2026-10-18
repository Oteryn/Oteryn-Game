# RAID-CONTENT-1

```yaml
task_id: RAID-CONTENT-1
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: content-prep/raid-content-1
pr: https://github.com/Oteryn/Oteryn-Game/pull/2243
base_sha: 64d5e4dab918ed0274dd9b1479b6218887b644bb
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - content/encounters/raids/index.json
  - docs/reference/content-prep/raid-content-1/sources.json
  - docs/reference/content-prep/raid-content-1/README.md
  - docs/agents/tasks/RAID-CONTENT-1.md
  - content/encounters/raids/evidence-00000-00121.json
  - docs/reference/content-prep/raid-content-1/research-audit.json
```

## Outcome

Current Reference evidence is prepared for EVENTS-RAID-1 #2177. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../reference/content-prep/raid-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../reference/content-prep/raid-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Publication requires the commands and results retained in the canonical draft PR. Local pre-publication checks are run on each complete candidate. Exact-head CI remains a separate check after publication.

## Self-review

The implementing agent reviewed owned paths, provenance, factual field boundaries, family shard coverage, classification counts, source links and payload digests. Gameplay-sensitive community formulas remain held.

## Independent review

No external paid review triggered. Data-only draft Reference packets do not authorize gameplay changes or protected integration.

## Reference delivery boundary

There are 122 actual source raid descriptions, including explicitly historical/deprecated entries. Four general mechanics sections and two nested map headings are excluded from the raid family. Listed creatures, 119 resolved boss classifications, broadcasts, count-bearing observations and explicit seasonal schedules are captured. Approximate counts/ranges remain qualified; message times alone do not prove exact spawn composition. Undocumented per-raid intervals and counts remain held.

Task status `ready` refers only to the data/docs handoff. The PR remains draft; runtime admission and protected integration remain blocked. Complete candidate validation and exact remote-blob verification are recorded in the canonical draft PR.

## Supplemental source completion — 2026-10-18

All 122 actual raid descriptions remain covered. Supplemental structured evidence gives the explicit Necromancer waves (1, then 5, then 4 Liches), Draconia spatial group composition, five Ghostlands waves with two unannounced, and source-qualified Terramite counts. Spatial groups are not inferred to be temporal waves. Approximate counts remain approximate; undocumented exact spawn configurations and intervals remain held.

This authoring successor replaces the previous reference candidate only after full local validation and exact remote-byte verification. The previous CI results do not qualify a new head. Draft-only publication; no ready transition, merge or runtime admission.

### Final source-gap sweep

Linked Nimmersatt quest sources now establish Dragon Bride as the sole creature type and a source-stated six-month recurrence (exact dates unproven). Explicit isboss=no flags resolve the Monks and Warlocks source-listed creature set.

## Full outstanding-field source sweep —2026-10-18

oteryn:raid.northern_darama_raids.djinn_raid/creatures.

Only explicit sourced values are selected. Wiki omissions and unsupported exact gameplay formulas remain held. Primary news retrieved from indexed official publications is labelled with the direct HTTP403 limitation. Runtime activation and integration remain blocked.
