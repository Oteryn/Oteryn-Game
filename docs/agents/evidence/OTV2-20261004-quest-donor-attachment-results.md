# Local Quest donor refinements — 2026-10-04

Local AUTHORING only on `codex/quest-data-completion-80-20261003`,
HEAD `88d63a18b44123ce342008cf52bcdb427c909ec2`. No PR, commit, push,
merge, deployment or Remote Desktop use. Five subagents handled associations,
NPC dialogue, conditions, finite requirement corrections and independent review.

## Actual product changes

**108 of 352 canonical definitions changed in this batch.** 105 chosen Quest
recipes now expose qualified Source records through closed, typed
`oteryn_recipe.donor_source_data` references. Three other recipes receive the
finite minimum-level corrections below. This is data enrichment, not 108 newly
complete or playable quests.

|Source work|Generated coverage|Canonical attachments|
|---|---|---|
|Lexical Lua truthiness|636 guard projections:491 full predicates,145 partial predicates,673 bound variable reads;75 Quest owners|Included with existing clock/position guards in628 condition references|
|NPC progress/dialogue|All80 existing Quest definitions with NPC progress occurrences:633 files,4,379 occurrences,4,290 exact AST-write joins|4,290 exact occurrence/context references|
|Component associations|17 of248 components associated with7 Quest keys by32 caller/symbol proofs;15 additional components beyond earlier Brokul pair|36 references covering18 Quest/component relations and their actual typed component definitions|
|Guarded rewards|Existing3 Source call projections retain7 guarded reward choices and fallback branches|3 reward references|

NPC data retain746 function contexts,135 Storage-alias joins and33 explicitly
marked legacy global helper calls. Guard/argument AST references reuse the retained
captures; contexts are shared rather than copying whole programs. Function siblings
are context evidence, not a reachable execution sequence. Symbol/caller associations
do not establish event dispatch, equivalent storage domains or complete callbacks.

Already cached TibiaWiki BR source specifications correct the chosen minimum
levels: **Battle Mage0→250, Falconer0→100, Makeshift Warrior0→100**.
The original Source minima remain unknown; no original Source hold is silently
cleared. Exact entry/revision/body hashes and pre-change recipe fences qualify these
three changes. No external wiki research or donor download occurred.

## Schemas and regeneration

`quest_donor_attachment.schema.json` closes reference categories, projection scope,
Quest identity, packet/record hashes and non-admission flags. A derived completion
schema adds only this optional field and preserves every original constraint of
the pinned completion schema. The pinned compiler and Source-fix guards are intact.

`quest_donor_source_authoring.py` reproduces the conditions, association and all-NPC
packets under `samples/donor-source/refinements/`. The ordinary Quest tree generator
attaches them after the original Source-core checks. The binding packets were
regenerated against the changed canonical shard hashes. Qualified packet hashes
are checked before references are emitted; altered reward/refinement/component
packets cannot reuse stale qualification results.

The files are local repository data. Server runtime loading, Quest execution and
Native dispatch are not qualified by this work.

## Verification

Independent canonical readback resolves every attached packet/record hash. Removing
only the new attachments and reversing exactly the three chosen level corrections
and their notes reproduces **all four original canonical shard hashes exactly**.
All25 pinned compiler hashes remain exact. All352 original Source/authored cores,
readiness values, missing-data holds and Native states are unchanged.

All required authoring checks pass:729 unit tests (3 optional fixtures skipped),
262 schema cases, both RewardClaim suites (13+6 tests), exact Source archive/AST
and refinement replay, binding/bundle/tree checks. The initial full run exposed
stale derived rollout and NPC-capture input hashes after canonical changes; those
outputs were regenerated and every remaining check then passed. No implementation
changed after the passing unit run. Governance, repository policy and
`git diff --check` pass; these are local checks, not frozen-candidate VALIDATE
or production admission. Local logs are under `/workspace/quest-components-next/root/`.

## Concrete remaining data work

**89 NPC target matches remain unresolved**, with exact Source-line candidates:

|Quest key suffix|Occurrences|
|---|---:|
|outfit_and_addon_quests|8|
|rottin_wood_and_the_married_men_quest|16|
|the_inquisition_quest|3|
|the_isle_of_evil_quest|24|
|the_new_frontier_quest|4|
|the_paradox_tower_quest|6|
|unnatural_selection_quest|28|

These are target-expression/alias correlations; all633 referenced NPC source files
are present. They are not89 absent quests or absent NPC files.

231 component files lack a proven Quest association in this bounded pass. Some are
shared encounters or world logic, so do not count them as231 broken quests.
145 truthiness guards retain unknown sibling expressions.89 guard records lack an
authoritative canonical Quest owner. Original opaque statements, API effects and
runtime bindings still require additional concrete mapping.

Original readiness remains **42 definition_ready /242 waiting_data /
68 waiting_native_bindings**. Definition readiness is not gameplay completeness.
No new whole-Quest completion count or80% semantic completion is established.

For the architect/coordinator, retain only actual owner decisions: accepted runtime
owners/bindings for Quest/NPC/Interaction execution; the existing Medusa written-text
carrier question; and the two Ornamented Shield charge conflicts already preserved
by D277. Ordinary Source aliases, missing associations and remaining condition
transcription are authoring tasks, not automatic architecture blockers. The finite
chosen minima corrections do not supersede original Source requirement decisions.
