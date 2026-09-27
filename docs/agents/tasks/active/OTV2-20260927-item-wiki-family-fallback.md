# OTV2-20260927-item-wiki-family-fallback

```yaml
task_id: OTV2-20260927-item-wiki-family-fallback
title: Classify unresolved engine Items from pinned English TibiaWiki evidence
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1040
base_sha: bab42d5c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-27T00:00:00Z
updated_at: 2026-09-27T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-census/item_wiki_family_capture.py
  - imports/tibiawiki/facts/items-family-fallback.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - tools/content-schema/item-authoring/engine_items.py
  - tools/content-schema/item-authoring/build_formal_schema.py
  - tools/content-schema/item-authoring/item.schema.json
  - tools/content-schema/item-authoring/population_census.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/active/OTV2-20260927-item-wiki-family-fallback.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

The owner asked to classify, from TibiaWiki, as many as possible of the engine Items the
converter leaves `family_profile_unresolved`: 1,827 at Crystal `ff7ede5` and 1,746 at
Canary `47dfd51f`. About 96% of them carry no `primarytype`/`type`/`slot`/`weapontype`
attribute in `items.xml`. A result counts only when it is proven. The rest stays
unresolved and is listed for the owner.

## Architecture and source of truth

- PROVEN: `imports/tibiawiki/index.json` scopes the directory to TibiaWiki source
  mappings/census/reimport metadata; source ids never become gameplay identity.
- PROVEN: `engine_items.PRIMARYTYPE_PROFILE` is already an English-TibiaWiki-vocabulary
  crosswalk to the 22 profiles in `profile-catalog.json`.
- PROVEN: `tibiawiki.com.br` answers this environment with a Cloudflare challenge
  (HTTP 403). `tibia.fandom.com` (English TibiaWiki) answers the MediaWiki API normally.
- DERIVED: the snapshot stores only facts: page id, revision id and sha1, content
  SHA-256, title, URL, and the observed `primarytype`/`objectclass` value. It is keyed by
  the Oteryn registry key, so one file serves both engines.
- UNKNOWN: whether BR TibiaWiki category membership would resolve more of the
  disambiguation and no-infobox cases. That needs an environment where BR is reachable.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is an offline content-authoring converter/census plus import
evidence. It changes no protocol, persistence, session fence, production or runtime.

## Acceptance criteria

- [x] Capture tool `tools/content-census/item_wiki_family_capture.py` (manual, network)
  writes a deterministic snapshot with no wikitext bodies. A record is written only when
  the admitted mapping resolves it to exactly one profile. For a disambiguation page,
  every candidate must agree.
- [x] Admitted mapping:
  - `decorations` and `lamps` are admitted as aliases of existing entries.
  - `WIKI_OBJECTCLASS_PROFILE` admits only `imbuement scrolls` to `progression_material`.
  - Broad buckets never resolve, and neither do `fireworks`, `blessing charms`,
    `clothing accessories` or `tools (objects)`. "(Objects)" names map objects.
- [x] Loader `load_wiki_family_fallback` fails closed on duplicate or unknown keys, a
  digest mismatch, an unknown registry key, an unadmitted value, and divergent
  disambiguation candidates.
- [x] The converter applies the fallback only after engine classification and after
  non-item/immovable routing, and only when the Item's own name was the matched name.
  It records `family_profile_basis` and `family_profile_evidence`, which the schema and
  validator accept.
- [x] Both censuses are regenerated with 0 validator errors, and
  `--check --self-check` passes on both engines. Routing is unchanged from the baseline:
  25,656 / 25,193.
- [ ] The required checks pass on the frozen PR head.

## Excluded scope

- No new profile or family.
- No owner decision taken on Blessing Charms (5: phoenix, solitude, spiritual, twin sun
  and unity charm) or Clothing Accessories (1: old rag).
- No fuzzy matching and no BR TibiaWiki capture.
- Nothing for the 1,409 new 15.30 appearance ids, which have no engine server data.
- No runtime, Rust importer or client change.

## Implementation / findings

Results:

| | Crystal | Canary |
|---|---|---|
| `family_profile_unresolved` | 1,827 → 934 | 1,746 → 893 |
| fully resolved | 9,773 → 10,665 | 9,476 → 10,328 |
| resolved by wiki fallback | 893 | 853 |
| Delivery Task eligible | 384 → 433 | 364 → 401 |

The snapshot has 946 registry-key records: 885 direct and 61 disambiguation-with-agreement.

Of 1,065 unique names, 308 remain unresolved:

| Reason | Names |
|---|---|
| No wiki page | 133 |
| Empty `primarytype` | 44 |
| `objectclass` forbidden bucket | 33 |
| `primarytype` forbidden bucket | 28 |
| Disambiguation candidates disagree | 25 |
| No infobox | 18 |
| Owner decision pending | 8 |
| Disambiguation page with no candidates | 7 |
| Multiple candidate pages disagree | 4 |
| Generic placeholder name | 5 |
| Multiple candidate pages, one unresolved | 3 |

The first implementation also aliased `tools (objects)` to `tools`. Checking against the
engine rows showed that this turned 276 immovable world objects (niche, buoy, buoy line,
parasol and others) into portable tools. The alias was removed, its test was inverted and
the censuses were regenerated.

A follow-up (this pass) replaced the small-word title-case/first-letter-case guess with an
exact, case-insensitive index of every main-namespace TibiaWiki title (29,012 titles),
plus the `" (item)"`/`" (object)"` disambiguating-suffix candidates, and fixed a fallthrough
bug that stopped an infobox with an admitted `objectclass` from ever being tried whenever
`primarytype` was present at all (even empty or unadmitted). Result: 946 registry-key
records (was 881; 885 direct, 61 disambiguation), recovering 65 more Crystal and 61 more
Canary Items, with routing unchanged (`routed_non_item` still 25,656 / 25,193) and no
widening of the admitted mapping. Example recoveries: "amber with a bug" (Crystal 32624,
the exact page title the old title-case guess missed), "slime" (resolves via its "Slime
(Object)" suffix page — the bare "Slime" title is a creature, not the item), and names like
"tortoise egg from nargor" whose `primarytype` was empty but whose `objectclass` is now
tried and resolves (Food).

## Validation

All of the following PASS:

- `verify_formal_schema.py`: 242 checks.
- `test_engine_items.py`: 344 checks.
- `test_lower_promotion_packet.py`: 55 checks.
- `build_formal_schema.py`: byte-identical output.
- `population_census.py --check --self-check` for Crystal and Canary.
- `validate_item_master_schema.py`.
- `ruff check` and `ruff format --check`.
- The content-tree migration test and validator.
- The G4 Crystal bindings self-test and `--check`.
- `validate_governance.py`.
- The content-routing and PR-lane classifier tests.

## Self-review

- exact head: pending
- method/reviewer: implementing session, plus a read-only adversarial review of the
  converter, loader, schema, tests and capture tool
- material findings: the `tools (objects)` alias (fixed above). The read-only review found
  nothing blocking. Two suggestions were adopted: `test_engine_items.py` now loads the
  committed snapshot fail-closed in CI, and the schema makes the direct and disambiguation
  evidence shapes mutually exclusive.
- verdict: PASS on the local candidate. CI checks the exact remote head.

## Independent review

- required: NO. This is offline authoring tooling and import evidence. No gameplay
  identity, protocol, persistence or production surface is touched.
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none known
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: fallback implemented, tools (objects) alias removed, rebased on main bab42d5c, all local checks pass
status: implementing
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: 1040
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: decide Blessing Charms and Clothing Accessories families (optional)
blocker: null
next_action: finish read-only review, publish PR, drive CI to green
```
