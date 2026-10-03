# OTV2-20261001-monster-dependency-completion-round6

```yaml
task_id: OTV2-20261001-monster-dependency-completion-round6
title: Complete creature dependencies and source-qualified loot
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/monster-dependency-completion-round6-20261001
pr: null
base_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
head_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
final_head_sha: null
final_head_frozen_at: null
owner: codex-user-directed-monster-dependency-completion
created_at: 2026-10-01T21:52:47.987936Z
updated_at: 2026-10-01T22:18:14.060485+00:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261001-monster-dependency-completion-round6.md
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/test_outfit_missing_registration.py
  - tools/content-schema/monster-authoring/samples/wiki-population-crystal-00ce02a5-2026-09-27.json
  - tools/content-schema/encounter-authoring/prepare_custom_spell_drafts.py
  - tools/content-schema/encounter-authoring/test_prepare_custom_spell_drafts.py
  - tools/content-schema/encounter-authoring/samples/round3-custom-spell-drafts.json
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Authorization and predecessor

Owner explicitly requests parallel completion of the remaining 32 Encounter-dependent
creatures and 15 unresolved-reference creatures, and continued completion of all
monster data. This local worktree applies the complete frozen Round5 patch; all 91
predecessor files were byte-checked before authoring. Immutable predecessor source,
actual 1660 bundles / 1612 native records and qualifications remain in
/workspace/monster-round5-output. New evidence writes to monster-round6-output.

## Disjoint implementation lanes

- Reference worker owns canary_batch and the new outfit-registration regression.
  Implement only a proven source-parity correction: exact pinned registry and C++
  semantics must prove a missing outfit target produces presentation without an
  appearance transformation. Retain ordinary resolved outfits and unknown-state
  rejection. Prepare other missing creatures and familiar spells with explicit
  source/contract boundaries; never invent an alias or placeholder Ability.
- Loot worker owns only the Crystal Wiki data sample: three newly browser-captured
  Loot Statistics records, highest applicable version, exact Item mappings,
  quantities/probabilities and separate low-confidence evidence. Preserve all
  unrelated record fields and stale-version exclusions.
- Encounter worker owns the existing custom-spell draft builder/tests/sample if
  integrating the proven Time Guardian lost-time core. Preserve floor15, two RNG
  draws,25 offsets, unowned count1 spawn and actual source guards. No generic Lua,
  false covers, arbitrary summon caps or automatic cohort admission.
- Root owns integration and this task record. Independent review owns no project
  paths. Further exact-path ownership must be allocated before edits.

## Qualification and boundaries

Build an explicit source/dependency graph for every 32 / 15 entry, identify actual
repairable converter defects, prepare missing actor/Ability/Encounter drafts,
and keep unsupported semantics explicit. Do not turn 28 shape-valid actor drafts
into admitted actors by suppressing their custom spell/event blockers.

Run affected regression tests and exact source/data checks. Regenerate the complete
population after final source freeze; compare actual native profiles and bindings
against that final data. Source-preservation checks do not establish complete
Global parity or production mechanics execution. Architecture and coordinator
questions remain in #162 as instructed; no remote publication or runtime activation
is selected by this local data/dependency continuation.

## Implemented source-qualified additions

- Crystal loot: Corrupted Ghost, Corrupted Skeleton and Goldhanded Cultist Bride
  gain 12 entries, using their captured Loot Statistics and existing accepted Item
  identities. Skeleton uses the highest observed version (15.12.c7d92c, 357 kills);
  two names seen only in its older statistics remain excluded. Four estimates have
  explicit low-confidence provenance. All 99 other sample records and non-loot
  data are preserved.
- Encounter preparation: Time Guardian gains an exact typed lost-time core for
  both forms. The immutable source ledger remains 40 spell identities / 43 source
  bindings. Four cores across three fight successors are prepared; actor closure,
  source form exchanges and native activation remain separately bounded.
- Dependency preparation: all 32 Creature / 22 Encounter relationships are traced;
  28 missing actor drafts, seven reference Creature drafts and three Familiar Spell
  drafts are prepared with explicit unresolved contracts. Shape validity does not
  establish runtime or dependency closure.
- XP review: all seven existing D47 official-priority differences retain their
  accepted values. Official news 8935 (2026-08-25) records the four Darklight XP
  reductions, and a current ordinary-HTTP official-library proxy agrees with the
  exact library values. Wiki was read through public Chrome/CDP captures.

Round5 is fully qualified and packaged as the immutable predecessor:
archive SHA256 3ce22e8f9c10f6d27c12fbaa5fc046b0465346b8fdca002071ebd43decf27480.
Chayenne correction is implemented under the existing presentation-only Effect
contract: the complete default-source registry proves Devovorga is absent; the
condition therefore fails before transformation. Keep the cast and ENERGYHIT,
including its original interval and chance. Ordinary resolved outfits are unchanged;
modified, partial or alternate source profiles retain the unresolved dependency.
The proof checks 5278 exact Git-blob files, 1655 literal registrations and the one
exact-pinned Primal helper, whose mandatory suffix cannot equal Devovorga.

Final local qualification is recorded in immutable external receipts under
/workspace/monster-round6-output: final-source-byte-freeze.json, current index/stage,
final-native-qualification.json, final-native-profile-qualification.json,
final-population-qualification.json, stats-loot-batch/summary.json and
final-independent-review.json. The complete patch is checked against a clean base
through a temporary index; the real Git index and predecessor remain unchanged.
The record remains implementing because no remote candidate/publication was selected.

The full remaining mitigation source scan finds no safe additional numeric fills:
618 pinned Canary actors and one Wiki-authored candidate remain without mitigation;
the 619 matching source files are a separate count. The only extra numeric Crystal
candidate retains its identity conflict. Similar variant names are not aliases.
The separate Phantasm (Weak) page describes HP65/XP1 but omits mitigation, so its
base Creature value is not adopted. Missing numeric facts remain unknown.
