# OTV2-20260928-item-nonitem-routing

```yaml
task_id: OTV2-20260928-item-nonitem-routing
title: Fluid/placeholder/appearance-title/actualname non-Item routing; availability
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1105
base_sha: 800e3eb6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/engine_items.py
  - tools/content-schema/item-authoring/test_engine_items.py
  - tools/content-schema/item-authoring/README.md
  - tools/content-schema/item-authoring/samples/population-crystal-ff7ede5.json
  - tools/content-schema/item-authoring/samples/population-canary-47dfd51f.json
  - tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json
  - tools/content-census/item_wiki_family_capture.py
  - imports/tibiawiki/facts/items-family-fallback.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260928-item-wrap-target-corpses.md
  - docs/agents/tasks/active/OTV2-20260928-item-nonitem-routing.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Outcome

Ported onto `main` after PR #1086 (wrap-target/dead-item, archived separately). Six
lowest-priority non-Item/name-join rules, applied in this order, each only once every
higher-priority classifier has already failed (proven; see Implementation table):

1. **Fluid types with no appearance** (owner `Fluid`, reason
   `fluid_type_without_appearance`): items.xml ids 1-20 name a `Fluids_t` enum value
   with no `appearances.dat` object.
2. **Late placeholder names** (reason `appearance_placeholder_slot`): `old tibia item`,
   `unknown item`, `unknow`, `event item`, `unknown corpse`, applied last.
3. **Appearance-title wiki join** (`match_basis: "appearance_title"`): a key's
   `appearances.dat` name (which can differ from a blanket items.xml range label, e.g.
   Crystal `23577-23667` all named `weapon of mayhem` but carrying real per-id client
   names like "slayer of mayhem") is looked up in the existing title index. Resolves
   45 of the 46 remaining "weapon of mayhem" ids (task (d)).
4. **Availability from TibiaWiki `status`**: every wiki-matched record now also carries
   an optional top-level `availability` field (`{status, evidence}`). Purely additive.
5. **Exact `actualname` join** (`match_basis: "actualname"`): a key's items.xml or
   appearance name is looked up in an index of every Infobox Object page's own
   `actualname` field; resolves only when every page sharing it agrees.
6. **No-client-appearance last resort** (owner `WorldObject`, reason
   `no_client_appearance`): an item with no `appearances.dat` object at all (an id gap
   inside a blanket items.xml range) routes non-Item.

Full rule text/evidence: `docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`
§5e-5h.

**Priority fix (found while implementing 5):** a generic engine name already correctly
handled by wrap-target/dead-item can coincidentally collide with an unrelated wiki
page's `actualname` -- "dead rat"/"dead goblin" both hit unrelated Oramond-quest pages
sharing that literal actualname, silently overriding the already-correct dead-item
answer. Fixed by ranking `appearance_title`/`actualname` evidence *below*
wrap-target/dead-item (itemid/title stay above: exact joins, no collision risk). Both
ids keep their original outcome; the invariant proof (zero unsafe diffs) covers this.

Also fixed a capture-tool bug: `collect_unresolved`'s "does this id need a wiki lookup"
probe must use only classifiers that outrank the wiki fallback, never wrap-target/
dead-item or the routes above. Fixed via `sources["skip_post_wiki_fallback_routes"]`.

A wiki `world_object` routing rule was implemented, investigated (zero routed items)
and removed per owner review (playable-first doctrine).

**(d) "weapon of mayhem"**, task (d)'s subject: 45/46 remaining ids resolve via rule 3
(their real per-id `appearances.dat` names, e.g. "slayer of mayhem", are what TibiaWiki
documents, not the blanket items.xml label); the 46th has no client appearance and
routes via rule 6.

**Owner decision (2026-09-28):** the 10.94 Carving/Mayhem/Remedy weapons (TibiaWiki
`status = unavailable`, merged into "of Destruction" in the 2017 Winter Update) are kept
as ordinary Items with their real weapon family -- recorded truthfully via
`availability`, never excluded/rerouted.

## Architecture and source of truth

- PROVEN: both engines' items.xml ids 1-20 are exactly the `Fluids_t` enum names,
  byte-identical in `research_clones/{canary,crystal}-full/src/utils/utils_definitions.hpp`;
  none has an `appearances.dat` object.
- PROVEN (weapon of mayhem): "X of Mayhem" pages' `itemid`s fall in the "weapon of
  carving" range; `list=search` for ids in 23577-23667 returns zero hits; live-fetched
  appearance-named pages (e.g. "Slayer of Mayhem") confirm documentation under those
  names instead.
- PROVEN (TibiaWiki `status` default): `Infobox Object` forwards `status` to `Status
  Messagebox`, whose `{{#if:{{{1|}}}|...}}` renders nothing when absent/empty -- from
  the live template source; no implicit "active" default.
- PROVEN (priority collision): "dead rat"/"dead goblin" actualname candidates ("Dead
  Rat (Oramond)", three "Dead Goblin (...) Quest" pages) are unrelated quest pages
  sharing only the literal actualname string.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring tooling only.

## Acceptance criteria

- [x] Fluid/late-placeholder/no-client-appearance routes and the appearance-title/
  actualname wiki-evidence joins wired in proven-safe priority order (itemid/title
  above wrap-target/dead-item; appearance_title/actualname below).
- [x] `sources["skip_post_wiki_fallback_routes"]` gates every post-wiki-fallback rule;
  test added.
- [x] `availability` wired end-to-end (capture, loader, `convert_item`, schema,
  idempotent); absence means "not asserted".
- [x] Tests added incl. required negatives: already-resolved items never rerouted; a
  name-join match on an item wrap-target/dead-item already resolved never overrides it;
  loader rejects unknown `match_basis`/`availability.status`.
- [x] Recapture rerun; digests regenerated; censuses + promotion packet regenerated,
  `--check --self-check` passes; formal doc updated.
- [x] Changed-id invariant proven against current `main`: exactly 225 ids/engine
  change, every one from `family_profile_unresolved`; zero unsafe diffs.
- [x] Wiki `world_object` routing found to route zero items, removed.
- [x] Owner decision recorded: Carving/Mayhem/Remedy weapons keep `status =
  unavailable`, never excluded/rerouted.

## Excluded scope

- No `WORLD_OBJECT_PRIMARYTYPES` value added; no wiki `world_object` routing exists
  (removed; see Outcome).
- Does not touch the Delivery Task rule, delivery-list membership, or the wrap-target/
  dead-item rules themselves (PR #1086) -- only their relative priority.
- No `verify_formal_schema.py` negative for `match_basis`/`availability.status`: that
  pattern does not exist there for any enum (242 checks unchanged).

## Implementation / findings

| | Crystal `ff7ede5` | Canary `47dfd51` |
|---|---|---|
| `family_profile_unresolved` (before task) | 369 | 328 |
| `family_profile_unresolved` (final) | 144 | 103 |
| `routed_non_item` (before task) | 25,670 | 25,207 |
| `routed_non_item` (final) | 25,833 | 25,370 |
| `fluid_type_without_appearance` | 20 | 20 |
| `appearance_placeholder_slot` (increase) | +38 | +38 |
| `appearance_title` resolved | 45 | 45 |
| `actualname` resolved | 17 | 17 |
| `no_client_appearance` routed | 105 | 105 |
| items w/ `availability` | 113 | 113 |

`availability.status` observed (both engines, identical): `unavailable` 105 (incl. all
45 `appearance_title` Mayhem/Carving/Remedy weapons), `event` 5, `unobtainable` 2,
`ts-only` 1; `deprecated` not observed. `appearance_title` resolves all 45 to
`weapon_melee`/`weapon_distance`. `actualname` resolves "tic-tac-toe token" (2 ids ->
`event_collectible`) and "stone" (15 ids -> `material_valuable`, nine "Stone (...)"
pages).

Every other outcome/blocker count is byte-identical before/after -- confirmed by
diffing every id's full outcome against current `main`, both engines: exactly 225
ids/engine change (20+38+45+17+105), every one previously unresolved, zero unsafe
diffs. This directly caught and let us fix the "dead rat"/"dead goblin" collision.

## Validation

### Focused

- command/run: `python test_engine_items.py`
- result: PASS (537 checks)

### Component/integration

- command/run: `build_formal_schema.py` (x2, no diff); `verify_formal_schema.py`;
  `test_lower_promotion_packet.py`; `ruff check`/`format --check` on owned files;
  `validate_governance.py`
- result: PASS (idempotent; 242 checks; owned files ruff-clean; governance passes). A
  pre-existing E402 in `verify_formal_schema.py` (not an owned path, present unchanged
  on `main` before this task) is out of scope, untouched.

### E2E

- scenario: changed-id invariant re-run against a fresh `main` (`de431db7`) worktree,
  both engines; `population_census.py --check --self-check`; `lower_promotion_packet.py
  --check --self-check`; a full network recapture
- result: PASS both engines + packet; zero unsafe diffs; recapture additive-only (all
  1,415 pre-existing keys byte-identical, 67 new added, 0 dropped)

### Exact-head CI

- final head: `NOT_APPLICABLE` (local worktree task, no PR opened in this interaction)
- trigger source: `NOT_APPLICABLE`
- workflow/run/job: `NOT_APPLICABLE`
- runner assignment: `NOT_APPLICABLE`
- classification: `NOT_APPLICABLE`
- result: `NOT_APPLICABLE`

## Self-review

- exact head: `8deb61f0` (local, not pushed/frozen)
- method/reviewer: implementing session
- material findings: wiki `world_object` routing removed (zero items routed); capture
  probe bug found and fixed; a real "dead rat"/"dead goblin" false-positive collision in
  the `actualname` join found via the changed-id invariant proof and fixed by ranking
  `appearance_title`/`actualname` below wrap-target/dead-item
- verdict: pass

## Independent review

- required: NO; owner-decided family mapping, same class as the prior wrap-target task
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending; unresolved review threads: `NOT_APPLICABLE` (no PR)
- related/superseded PRs: supersedes the extension work on `item-nonitem-routing`
  (rebuilt on merged PR #1086)
- protected auto-merge: `NOT_APPLICABLE`; merge commit/result: pending; ownership
  release: pending

## Context checkpoint

```yaml
last_progress: b/c/e/f/g/h implemented; fixed a name-join priority collision found via the invariant proof (dead rat/goblin); recapture, censuses, packet regenerated; full check list green; committed locally to item-nonitem-routing-v2 at 8deb61f0
status: validating
branch: claude/compassionate-albattani-s29syw
head_sha: null
pr: 1105
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
owner_action_required: null
blocker: null
next_action: required checks, Merge Queue, archive record
```
