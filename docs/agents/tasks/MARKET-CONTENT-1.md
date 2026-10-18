# MARKET-CONTENT-1

```yaml
task_id: MARKET-CONTENT-1
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: content-prep/market-content-1
pr: https://github.com/Oteryn/Oteryn-Game/pull/2234
base_sha: 64d5e4dab918ed0274dd9b1479b6218887b644bb
owner: Codex / current owner-authorized content preparation session
created_at: 2026-10-18
execution_policy: continuous_progress
owned_paths:
  - docs/reference/content-prep/market-content-1/listing-restrictions.json
  - docs/reference/content-prep/market-content-1/sources.json
  - docs/reference/content-prep/market-content-1/README.md
  - docs/agents/tasks/MARKET-CONTENT-1.md
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/evidence-00000-00499.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/evidence-00500-00999.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/evidence-01000-01499.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/evidence-01500-01999.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/evidence-02000-02075.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/index.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/services/evidence-00000-00323.json
  - docs/reference/content-prep/market-content-1/target-files/content/services/trade/market-evidence/services/index.json
  - docs/reference/content-prep/market-content-1/service-apply-plan.json
  - docs/reference/content-prep/market-content-1/research-audit.json
```

## Outcome

Current Reference evidence is prepared for MARKET-1 #2229; DEPOT-1 #2156 indirectly. This draft is not a runtime activation or a claim that all facts are proven. The owner authorized current-at-fetch evidence instead of the historical programme cut, data/docs only, one new branch and draft PR per task, no merge.

## Architecture and source of truth

Evidence follows the existing full-game tree placement. Source records are pinned to page revision IDs or immutable repository SHA-256 inputs in [the source registry](../../reference/content-prep/market-content-1/sources.json). Native wiring belongs to CP. The evidence envelope is not a newly accepted runtime schema.

## High-risk authority/recovery qualification

NOT_APPLICABLE: data and documentation only; no production, session, identity, protocol, storage, value mutation, or merge authority.

## Acceptance criteria

- Every inspected field is sourced or explicitly UNKNOWN/CONFLICT.
- Existing runtime candidate routes/offers and protected/open-PR files are preserved.
- Changed candidates pass full-tree, materialized-tree, governance and the independent source/coverage audit before publication.

## Implementation / findings

See [the task evidence](../../reference/content-prep/market-content-1/README.md). Remaining source gaps are carried in UNKNOWN/CONFLICT facts and grouped in the draft PR. Canonical live branch, frozen SHA, PR and CI state are recorded on GitHub after the atomic candidate is created; this source record does not self-reference its future SHA.

## Validation

Publication requires the commands and results retained in the canonical draft PR. Local pre-publication checks are run on each complete candidate. Exact-head CI remains a separate check after publication.

## Self-review

The implementing agent reviewed owned paths, provenance, factual field boundaries, family shard coverage, classification counts, source links and payload digests. Gameplay-sensitive community formulas remain held.

## Independent review

No external paid review triggered. Data-only draft Reference packets do not authorize gameplay changes or protected integration.

## Service placement constraint

Service evidence is staged under docs/reference with an explicit apply plan. World Bundle pin inputs cannot be updated under the owner's current scope. Existing native Service declarations are preserved exactly; canonical placement and qualified regeneration are the coordinator's integration step.

## Reference delivery boundary

All 324 original trade services and 2076 distinct offered Item identities are covered. Current own-item Wiki observations and the Market rules are preserved; 1868 items also have positive raw category/market metadata from the pinned official 15.30 client asset. That asset is explicitly not represented as a fresh live-client capture. Missing flags never become false. Global instance exclusions are linked on every item; unidentified current filters and exhaustive item-instance restrictions remain held.

Task status `ready` refers only to the data/docs handoff. The PR remains draft; runtime admission and protected integration remain blocked. Complete candidate validation and exact remote-blob verification are recorded in the canonical draft PR.

## Supplemental source completion — 2026-10-18

All 324 original trade services and 2076 distinct offered Item identities remain covered. Fresh read-only extraction from the owner-installed CipSoft client 15.33.9d148f (appearance SHA256 501594e860be8a531ca465ae6e7638da05e3a4d5cef2d038d4f2321ae6729f5f) supplies 1868 positive category/display observations, including trade-as/show-as identity and level/profession filters where explicitly present. A market submessage is not proof of server listing permission. Current own-ID Wiki tradeability, explicit non-portable exclusions and inapplicable categories are distinguished. Complete state-dependent listing predicates remain held where unavailable.

This authoring successor replaces the previous reference candidate only after full local validation and exact remote-byte verification. The previous CI results do not qualify a new head. Draft-only publication; no ready transition, merge or runtime admission.
