# Shared NPC quest rewards

`content/quests/shared-reward-callbacks.json` is loaded by the Quest catalogue and
the real NPC talk dispatcher. Author records against
`shared-reward-callbacks.schema.json`; `qengine_rewards.validate_callbacks` checks
the cross-field constraints. `qengine_rewards.py --check` reproduces the source
compositions and the promoted native outfit catalogue from the committed
Crystal corpus, without network access.

The existing native source importer admits the additional physical Item
profiles used by those exchanges. Its pinned Canary XML/client appearance
inputs, canonical-fact checks and exact identity bindings are recorded in
`tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`.
`register_spell_families.py` retains that cohort and the cosmetics section during
ordinary native regeneration. New lanes must qualify additional referenced Item
identities through their owning content pipeline before claiming a playable
exchange; a canonical identity alone is insufficient native custody evidence.

Each record has these fields:

| Field | Meaning |
| --- | --- |
| `key` | Unique `oteryn:quest-reward/<name>`; the durable transition/callback name. |
| `quest` | Owning canonical Quest key. Existing tracks must belong to this quest. |
| `npc` | Canonical `oteryn:npc.<name>`, resolved through the current conversation. |
| `keywords` | One to eight exact, trimmed, lowercase ASCII player messages. |
| `topic`, `next_topic` | Current/committed conversation topic, unsigned 16-bit integers. |
| `storage` | One to eight `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as the declared track initial value (new callback tracks default to `-1`). `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
| `requires` | Optional, up to eight distinct read-only `{track, minimum, maximum}` prerequisites. Tracks must already be declared and cannot also occur in `storage`. Their original quest retains ownership. The writer locks them in sorted order with written tracks and rechecks their fresh values before any consumption or reward. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `give` | Optional/null `{item, count, contents?}`: one native reward root, count `1..100`, optionally up to twenty `{item, count}` children in a container. Native stack/container and weight rules still apply. The equipped main backpack needs room after the hand-in. |
| `outfit` | Optional/null `{outfit, addons}`. `outfit` is the canonical `oteryn:outfit.<name>`; `addons` is a bitmask `0..3`: base only `0`, first addon `1`, second addon `2`, both `3`. Base is always included. |
| `achievement` | Optional/null canonical `oteryn:achievement/<name>` from the admitted achievement catalogue. |
| `reply` | One to eight response lines, each at most 512 UTF-8 bytes. |
| `sources` | One to eight exact donor revision/path/hash citations. |

All effects share one session-generation-fenced Quest transaction and receipt.
Missing inputs, insufficient capacity, stale authority or incompatible cosmetics
refuse without partial effects. A replay returns the original committed result.
Existing account outfit unlocks retain their original immutable receipts and
satisfy subsequent character completions. Both sexes use the same outfit key;
the appearance owner selects the donor look using immutable creation sex. The
existing appearance protocol publishes the new catalogue and permits selection.

`shared-reward-offers.json` contains conversation-only records with `npc`,
`keywords`, `topic`, `next_topic`, `storage`, and `reply`. These records establish
confirmation topics and never apply storage effects. A reward's storage effects
apply only after the complete exchange commits.

The shared file is a registry, not an extra effect inside
`world-callbacks.json`. A lane adds its source-backed callback record and its
conversation offers to these registries during integration; it must preserve
the existing records and unique keys. Do not replace the shared generator's
outputs with a lane-only document.

Current compositions: `pirate-sabre`, `knight-sword`, `deepling-helmet`,
`hallowed-holy-water`, `outfox-helmet`, and `sam-backpack`, each prefixed by
`oteryn:quest-reward/`. The last is Sam's backpack hand-in and achievement;
Kroox's later armor delivery remains a separate quest stage.

L1 adds Warmaster Serpent/Tribal Crest hand-ins (Cael), Wayfarer Old Cape/Sedge
Hat hand-ins (Cranky Lizard Crone), and Yalaharian base/side-exclusive addons.
Palimuth keeps the Vampiric Crest: this is `present`, never `take` plus a new MINT.
Warmaster's completed New Frontier predicate preserves the parent quest's ownership.
Each independent addon has its own immutable once-only quest progress guard.

The two L1 base wardrobes live in `shared-outfit-callbacks.json`, generated by the
same producer from pinned action/startup scripts. Their `effects.outfit_reward`
uses the same OutfitReward shape and admitted grant mappings as NPC rewards.
The world callback writer composes these grants with its existing fenced USE
receipt and storage transaction; the Channel owner applies only map projections.
A world outfit reward requires nonempty, once-only storage guards and the current
native generation. Existing world callbacks can omit that participant.

L2 adds complete Battle Mage, Festive and Elementalist outfit conversations.
Battle Mage addons retain Secret Library ownership of their prerequisite; Festive
addons retain First Dragon ownership. Offers use the same prerequisites, while
the durable writer independently rechecks them under transaction locks.

The optional top-level `tracks` list declares missing read-only parent tracks
using `{key, quest, initial, min, max, sources}` (maximum256). Every declaration
must be used by a `requires` predicate and must never be written by a callback.
Sources are mandatory. Existing declarations must match the admitted original
owner/default/bounds exactly; conflicting ownership/defaults fail closed.
This adds catalogue facts only and never writes player progress. Missing donor
storage uses its declared initial, so a missing positive prerequisite refuses.

L3 composes Avar Tar's Demon base/shield/helmet with the original Annihilator,
Demon Helmet and Demon Oak owners. Ezebeth shares one Dark Trails/Glooth Engineer
base grant; six or more Oramond voting points prioritize addon2. The addon1 path
at six or more points also requires completed addon2, preserving source `elseif`
ordering without ambiguous eligible reward callbacks. Three to five points can
earn addon1 first. Oramond voting progress retains its original owner.

An optional `fee: {gold, include_bank}` composes the existing native fee owner
with the same Quest transaction. A price is1..1000000000000 gold units; it cannot
also consume coin identities through `take`. The retained native coin/change
facts come from the admitted generation. The writer checks the source price,
fresh carried tree, change capacity and the source bank policy under locks.
Carried-only prices never debit Bank. Missing inputs or any later grant refusal
roll back the entire payment, state, item and appearance transaction. A distinct
native audit cause binds the callback, exact price and bank policy; it retains
all bounded carried-source audit checks and the Type2 activation tuple.

`creation_sex` optionally requires immutable Character creation sex (`female` or
`male`). The fenced appearance owner reads it before any source value is paid.
Selected look, disguise and caller-supplied gender never establish this guard.

`participants` optionally names up to three distinct existing foreign quest
owners and their exact storage effects. The combined primary and participant
write set is limited to sixteen tracks, with no duplicates or overlap with readonly
prerequisites. A participant cannot invent or reassign a track. Its validated
transition contributes to its original owner's definition hash. Participants
cannot be submitted as independent quest transitions. The Character writer locks
the sorted union of source tracks and admits shared track/state capacity; all
participants, exact owner pins, source proof, primary stage, costs and rewards
commit at one revision or roll back together. Immutable participant proofs link
to the same-transaction source and primary Quest receipt. Original single-owner
SQL proof arms and row immutability remain in force, with a precise additional
participant proof arm. A participant never completes another quest.

`state_ops` optionally composes bounded `from` conditions (`any`, `between`,
`elapsed_at_least`) and existing native effects (`add`, `set_now`). Both primary
and foreign-owner participants support it. Elapsed checks preserve their clock
with add0;the database transaction clock authorizes value. Dialogue time never
authorizes a grant. Duplicate tracks,owner confusion and combined effects>16
fail closed. Source deadlines can normalize to start+exact elapsed duration.

A source-bound track declaration is read-only unless `writable:true` is explicit.
Every writer must match its declared owner;existing native metadata must still
match exactly. This permits missing shared counters without reassigning tracks.
Immutable source/participant proofs validate exact operation conditions/results,
and all state,costs,grants and original owner pins commit or roll back together.

Optional `requires_premium` uses the existing native premium Current authority in
the fenced transaction; an absent owner refuses before costs. Optional
`requires_outfits` (at most8 distinct outfit identities) reuses the normal
starter-or-compatible-earned account appearance proof in that transaction,
before inventory or value effects. Empty/default fields preserve legacy plans.

`requires_actor_conditions` is a bounded unique list of source Fire/Drunk kinds.
The command admission samples the existing runtime/player condition owners under
their normal locks. An opaque proof binds that sample to the exact command,
callback, current item fence and native generation. SQL reward application
refuses absent/mismatched evidence before value participants and does not lock
the runtime while holding SQL locks. This models source greeting prerequisites
conservatively at each value action; no storage flag substitutes for a condition.
`forbids_addons` reuses the compatible earned/starter appearance owner for each
requested addon bit. Unknown identities or unsupported bits refuse. Both optional
fields omit empty values to preserve historical callback plan serialization.
# Shared NPC quest rewards

`content/quests/shared-reward-callbacks.json` is loaded by the Quest catalogue and
the real NPC talk dispatcher. Author records against
`shared-reward-callbacks.schema.json`; `qengine_rewards.validate_callbacks` checks
the cross-field constraints. `qengine_rewards.py --check` reproduces the source
compositions and the promoted native outfit catalogue from the committed
Crystal corpus, without network access.

The existing native source importer admits the additional physical Item
profiles used by those exchanges. Its pinned Canary XML/client appearance
inputs, canonical-fact checks and exact identity bindings are recorded in
`tools/content-schema/native-gameplay/qengine-reward-item-profiles.json`.
`register_spell_families.py` retains that cohort and the cosmetics section during
ordinary native regeneration. New lanes must qualify additional referenced Item
identities through their owning content pipeline before claiming a playable
exchange; a canonical identity alone is insufficient native custody evidence.

Each record has these fields:

| Field | Meaning |
| --- | --- |
| `key` | Unique `oteryn:quest-reward/<name>`; the durable transition/callback name. |
| `quest` | Owning canonical Quest key. Existing tracks must belong to this quest. |
| `npc` | Canonical `oteryn:npc.<name>`, resolved through the current conversation. |
| `keywords` | One to eight exact, trimmed, lowercase ASCII player messages. |
| `topic`, `next_topic` | Current/committed conversation topic, unsigned 16-bit integers. |
| `storage` | One to eight `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as the declared track initial value (new callback tracks default to `-1`). `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
| `requires` | Optional, up to eight distinct read-only `{track, minimum, maximum}` prerequisites. Tracks must already be declared and cannot also occur in `storage`. Their original quest retains ownership. The writer locks them in sorted order with written tracks and rechecks their fresh values before any consumption or reward. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `give` | Optional/null `{item, count, contents?}`: one native reward root, count `1..100`, optionally up to twenty `{item, count}` children in a container. Native stack/container and weight rules still apply. The equipped main backpack needs room after the hand-in. |
| `outfit` | Optional/null `{outfit, addons}`. `outfit` is the canonical `oteryn:outfit.<name>`; `addons` is a bitmask `0..3`: base only `0`, first addon `1`, second addon `2`, both `3`. Base is always included. |
| `achievement` | Optional/null canonical `oteryn:achievement/<name>` from the admitted achievement catalogue. |
| `reply` | One to eight response lines, each at most 512 UTF-8 bytes. |
| `sources` | One to eight exact donor revision/path/hash citations. |

All effects share one session-generation-fenced Quest transaction and receipt.
Missing inputs, insufficient capacity, stale authority or incompatible cosmetics
refuse without partial effects. A replay returns the original committed result.
Existing account outfit unlocks retain their original immutable receipts and
satisfy subsequent character completions. Both sexes use the same outfit key;
the appearance owner selects the donor look using immutable creation sex. The
existing appearance protocol publishes the new catalogue and permits selection.

`shared-reward-offers.json` contains conversation-only records with `npc`,
`keywords`, `topic`, `next_topic`, `storage`, and `reply`. These records establish
confirmation topics and never apply storage effects. A reward's storage effects
apply only after the complete exchange commits.

The shared file is a registry, not an extra effect inside
`world-callbacks.json`. A lane adds its source-backed callback record and its
conversation offers to these registries during integration; it must preserve
the existing records and unique keys. Do not replace the shared generator's
outputs with a lane-only document.

Current compositions: `pirate-sabre`, `knight-sword`, `deepling-helmet`,
`hallowed-holy-water`, `outfox-helmet`, and `sam-backpack`, each prefixed by
`oteryn:quest-reward/`. The last is Sam's backpack hand-in and achievement;
Kroox's later armor delivery remains a separate quest stage.

L1 adds Warmaster Serpent/Tribal Crest hand-ins (Cael), Wayfarer Old Cape/Sedge
Hat hand-ins (Cranky Lizard Crone), and Yalaharian base/side-exclusive addons.
Palimuth keeps the Vampiric Crest: this is `present`, never `take` plus a new MINT.
Warmaster's completed New Frontier predicate preserves the parent quest's ownership.
Each independent addon has its own immutable once-only quest progress guard.

The two L1 base wardrobes live in `shared-outfit-callbacks.json`, generated by the
same producer from pinned action/startup scripts. Their `effects.outfit_reward`
uses the same OutfitReward shape and admitted grant mappings as NPC rewards.
The world callback writer composes these grants with its existing fenced USE
receipt and storage transaction; the Channel owner applies only map projections.
A world outfit reward requires nonempty, once-only storage guards and the current
native generation. Existing world callbacks can omit that participant.

L2 adds complete Battle Mage, Festive and Elementalist outfit conversations.
Battle Mage addons retain Secret Library ownership of their prerequisite; Festive
addons retain First Dragon ownership. Offers use the same prerequisites, while
the durable writer independently rechecks them under transaction locks.

The optional top-level `tracks` list declares missing read-only parent tracks
using `{key, quest, initial, min, max, sources}` (maximum256). Every declaration
must be used by a `requires` predicate and must never be written by a callback.
Sources are mandatory. Existing declarations must match the admitted original
owner/default/bounds exactly; conflicting ownership/defaults fail closed.
This adds catalogue facts only and never writes player progress. Missing donor
storage uses its declared initial, so a missing positive prerequisite refuses.

L3 composes Avar Tar's Demon base/shield/helmet with the original Annihilator,
Demon Helmet and Demon Oak owners. Ezebeth shares one Dark Trails/Glooth Engineer
base grant; six or more Oramond voting points prioritize addon2. The addon1 path
at six or more points also requires completed addon2, preserving source `elseif`
ordering without ambiguous eligible reward callbacks. Three to five points can
earn addon1 first. Oramond voting progress retains its original owner.

An optional `fee: {gold, include_bank}` composes the existing native fee owner
with the same Quest transaction. A price is1..1000000000000 gold units; it cannot
also consume coin identities through `take`. The retained native coin/change
facts come from the admitted generation. The writer checks the source price,
fresh carried tree, change capacity and the source bank policy under locks.
Carried-only prices never debit Bank. Missing inputs or any later grant refusal
roll back the entire payment, state, item and appearance transaction. A distinct
native audit cause binds the callback, exact price and bank policy; it retains
all bounded carried-source audit checks and the Type2 activation tuple.

`creation_sex` optionally requires immutable Character creation sex (`female` or
`male`). The fenced appearance owner reads it before any source value is paid.
Selected look, disguise and caller-supplied gender never establish this guard.

`participants` optionally names up to three distinct existing foreign quest
owners and their exact storage effects. The combined primary and participant
write set is limited to sixteen tracks, with no duplicates or overlap with readonly
prerequisites. A participant cannot invent or reassign a track. Its validated
transition contributes to its original owner's definition hash. Participants
cannot be submitted as independent quest transitions. The Character writer locks
the sorted union of source tracks and admits shared track/state capacity; all
participants, exact owner pins, source proof, primary stage, costs and rewards
commit at one revision or roll back together. Immutable participant proofs link
to the same-transaction source and primary Quest receipt. Original single-owner
SQL proof arms and row immutability remain in force, with a precise additional
participant proof arm. A participant never completes another quest.

`state_ops` optionally composes bounded `from` conditions (`any`, `between`,
`elapsed_at_least`) and existing native effects (`add`, `set_now`). Both primary
and foreign-owner participants support it. Elapsed checks preserve their clock
with add0;the database transaction clock authorizes value. Dialogue time never
authorizes a grant. Duplicate tracks,owner confusion and combined effects>16
fail closed. Source deadlines can normalize to start+exact elapsed duration.

A source-bound track declaration is read-only unless `writable:true` is explicit.
Every writer must match its declared owner;existing native metadata must still
match exactly. This permits missing shared counters without reassigning tracks.
Immutable source/participant proofs validate exact operation conditions/results,
and all state,costs,grants and original owner pins commit or roll back together.

Optional `requires_premium` uses the existing native premium Current authority in
the fenced transaction; an absent owner refuses before costs. Optional
`requires_outfits` (at most8 distinct outfit identities) reuses the normal
starter-or-compatible-earned account appearance proof in that transaction,
before inventory or value effects. Empty/default fields preserve legacy plans.

Filled gift children may declare a positive `fluid_subtype` with count1; the
active native policy must prove a variable-fluid vessel. Intake and retained
evidence keep their closed item/count shape until exact fluid matching is
implemented. NPC requirements use `requires_premium`; world callbacks reuse
the complete MINT with their existing world reward admission. Missing optional
fields preserve existing JSON/receipt bindings.

Authored NPC story shards are merged with the generated donor packet before the
same closed validation runs. Duplicate callback keys, duplicated declarations,
foreign owners and the combined256-entry bounds still fail closed. The initial
shard is `content/quests/npc-callbacks/l6-stories.json`; editing it does not modify
the generated donor packet.

`completes` is an optional boolean, omitted when false. A true value lowers to
the existing owning Quest completion transition. Its source proof must match
the same transaction's Quest receipt and completed revision, enforced by
`1254_quest_reward_completion.sql`. It grants no outfit or item by itself.
