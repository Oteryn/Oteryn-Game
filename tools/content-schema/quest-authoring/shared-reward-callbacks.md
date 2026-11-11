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
| `storage` | One to sixteen `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as `-1`. `preserve: true` keeps that predicate's value. An ordinary reward changes a value outside its accepted range. Explicit paid repeatable item sales preserve every storage guard and require a positive fee; unpaid grants remain once-only. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `give` | Optional/null `{item, count, contents?, key_number?}`: one native reward root, count `1..100`, optionally up to twenty `{item, count}` children in a container. Native stack/container and weight rules still apply. The equipped main backpack needs room after the hand-in. |
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

Startup selector projection uses the pinned, normalized
`world-authoring/samples/quest-access-bindings.json` capture. The server adds a
missing UID/AID only at the exact source position and appearance, outside Houses,
and only when one top-level entry has that appearance. Existing conflicting
attributes remain authoritative. Projected bindings prevent pickup and participate
in the normal callback selector checks. Binary map placements stay unchanged;
`test_quest_access_bindings.py` verifies that every captured selector has one
unambiguous source object, and the World unit tests cover projection, idempotence,
wrong position/appearance, Houses and conflicting native attributes.

The finite exchange-item provider preserves an already admitted record when its
identity, admission and known physical custody facts agree. It refuses conflicting
custody and does not replace another provider's attributes or source semantics.

Exact prices use the optional `fee` participant (`gold`, `include_bank`). The native
fee owner and reward MINT share the existing fenced Quest transaction and receipt.
Reused from qengine2 commit33b640335; migration1096 avoids this branch's existing
Corruptor migration1000.

`give.key_number` binds one non-container native KEY root to its nonzero u16
number. It cannot be attached to a different Item classification.
`repeatable: true` is available only for a positive-fee item sale with preserved
guards, no outfit and no achievement; each new command pays, while replay
returns the existing receipt. Each paid repeatable MINT claim includes the exact
GameSession/command identity; its deferred source proof requires that same
identity and a positive same-transaction payment. Ordinary unpaid claims retain
their canonical once-only identity.

`completes: true` commits the owning Quest completion flag with the transition.
It is unavailable on repeatable sales. `minimum_level` optionally requires the
Character owner's durable level, checked under the same character lock before
any payment or item consumption. `required_claims` optionally names up to eight
unique canonical RewardClaim keys; each must have an immutable earned receipt
for this character. These predicates never mint the claimed rewards again.
Fibula uses this terminal participant for the chosen level-50 adaptation after
eight native cavern kills and all six original source cache claims.
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
| `storage` | One to sixteen `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as the declared track initial value (new callback tracks default to `-1`). `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
| `requires` | Optional, up to eight distinct read-only `{track, minimum, maximum}` prerequisites. Tracks must already be declared and cannot also occur in `storage`. Their original quest retains ownership. The writer locks them in sorted order with written tracks and rechecks their fresh values before any consumption or reward. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `give` | Optional/null `{item, count, contents?, additional?, text?}`: one native reward root, count `1..100`, optionally up to twenty `{item, count}` children in a container. Native stack/container and weight rules still apply. The equipped main backpack needs room after the hand-in. |
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
write set is limited to sixteen tracks (each foreign participant retains its eight-track bound), with no duplicates or overlap with readonly
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
authorizes a grant. Duplicate tracks,owner confusion and combined effects>8
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
| `storage` | One to sixteen `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as the declared track initial value (new callback tracks default to `-1`). `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
| `requires` | Optional, up to eight distinct read-only `{track, minimum, maximum}` prerequisites. Tracks must already be declared and cannot also occur in `storage`. Their original quest retains ownership. The writer locks them in sorted order with written tracks and rechecks their fresh values before any consumption or reward. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `give` | Optional/null `{item, count, contents?, additional?, text?}`: one native reward root, count `1..100`, optionally up to twenty `{item, count}` children in a container. Native stack/container and weight rules still apply. The equipped main backpack needs room after the hand-in. |
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
write set is limited to sixteen tracks (each foreign participant retains its eight-track bound), with no duplicates or overlap with readonly
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
authorizes a grant. Duplicate tracks,owner confusion and combined effects>8
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
# Shared NPC quest rewards

`content/quests/shared-reward-callbacks.json` is loaded by the Quest catalogue and
the real NPC talk dispatcher. The shared transaction and source records are reused
from the engine lane at `5a6de28c142de185594217ce1e5cbc7f4b6e6e4e`.
Author records against `shared-reward-callbacks.schema.json`; the Rust loader also
checks bounded quantities, exact predicates, duplicate identities and once-only
effects. Authored bindings use the separate registry described below.

Every referenced Item and cosmetic must resolve in the admitted native generation.
A canonical identity alone does not prove physical custody facts or a compatible
outfit grant. The shared owner refuses unavailable native facts and never invents
item profiles, weight, stack limits, cosmetics or appearance entitlements.

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
`world-callbacks.json`. A lane integrates source-backed callback records and
conversation offers through the upstream source producer; it must preserve
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

The two L1 base wardrobes live in `shared-outfit-callbacks.json`, reused from the
engine lane's producer of pinned action/startup scripts. Their `effects.outfit_reward`
uses the same OutfitReward shape and admitted grant mappings as NPC rewards.
The world callback writer composes these grants with its existing fenced USE
receipt and storage transaction; the Channel owner applies only map projections.
A world outfit reward requires nonempty, once-only storage guards and the current
native generation. Existing world callbacks can omit that participant.

L2 adds complete Battle Mage, Festive and Elementalist outfit conversations.
Battle Mage addons retain Secret Library ownership of their prerequisite; Festive
addons retain First Dragon ownership. Offers use the same prerequisites, while
the durable writer independently rechecks them under transaction locks.

`requirements.owned_cosmetics` is an optional bounded list of up to eight unique
canonical outfits. Each entry uses `{ "outfit": "oteryn:outfit.fiend_slayer",
"addons": 1 }`: the base entitlement is required, plus every selected addon bit.
The Character-fenced writer locks the existing account grant owner, reads creation
sex, and resolves starter or earned facts through the current native catalogue's
explicit grant/provenance mappings. An inventory token, a grant-key spelling or
an acquisition-stage counter is never ownership. These checks consume nothing
and do not modify a prerequisite quest. Missing native content refuses the grant.
Omitted lists retain the old serialized callback shape and behavior.

Selected consumptive outfits explicitly map the existing item-action grant
identities and provenance to admitted cosmetic targets. This lets a quest base
and an item-earned addon satisfy the same canonical ownership predicate while
retaining the independent receipt and consumption owners. The producer declares
the exact look pairs; it does not infer authority from names or numeric suffixes.

World callbacks can declare the same bounded `requirements` as NPC rewards.
The fenced writer checks current level, Current Premium authority, completed
prerequisite quests and explicit cosmetic ownership before committing any
world effect. Legacy callers without Premium authority refuse Premium-gated
callbacks. Canonical source citations include the definition path and SHA256;
these describe selected Oteryn adaptations rather than donor script locations.

World callbacks may declare `completes: true` only with a stateful, once-only
progress write. The existing fenced writer commits completion with its receipt;
read-only predicates cannot acquire completion or a generic mutation route.

`native-world-callback-bindings.json` supplies bounded exact-selector aliases
when a donor UID is absent from the admitted map. Every binding names a callback,
its complete expected original UID selector, an explicit native position and
appearance, and source evidence. Missing targets, changed original selectors,
duplicates and malformed selectors fail the load. Only the selector and attached
provenance change; quest guards, effects and transport remain unchanged.

World callbacks may declare a top-level `item_reward` using the same item/count
shape as NPC `give`. `contents` places children inside the reward container;
`backpack_outputs` instead places at most19 additional outputs directly in the
main backpack. These layouts are mutually exclusive. All outputs, capacity,
weight, custody proofs and quest completion share one fenced MINT transaction.
World rewards require a stateful once-only USE callback and admitted native
item/weight facts. Default absent fields preserve legacy callback serialization.

An optional world `activation` track/range routes a selected overlay only during
its active personal phase, leaving unrelated original object interactions
available otherwise. It must exactly match a non-preserved storage write guard;
the fenced writer always rechecks that guard. The admitted quest copy chooses
routing only and cannot authorize any reward or bypass durable admission.

`requirements.owned_items` checks at most8 unique item definitions/counts against
current live carried custody (including closed nested containers), without
consuming or minting anything. Optional `map_source` is the native placement key:
a personally completed MapBase pickup of that definition/source in the same
World must exist in the immutable map-MINT and matching item-loop operation5
receipts. A stranded MINT or scripted Ground relocation is insufficient. Pickup
milestones survive ordinary stack merges; carried quantity is checked afresh.

## Authored completion rewards

`content/quests/chosen-reward-callbacks.json` uses the same schema and the same
fenced transaction as the source registry. Keys are unique across both registries;
the combined bound remains 256. `experience` is optional, positive and bounded by
QUESTGATE0-RL-05; `completes` defaults to false. They lower to the existing Quest
transition fields. The quest XP obligation is earned with the item grant and
completion, then consumed by the existing progression owner during quest refresh.
A rejected hand-in or full/missing backpack rolls back all participants.

The selected hunting campaign requires the Snapper, Deathbine and Many reports
and the first source rank in their ordered chosen phases before phase 10 can
advance to the terminal phase 11. The authored registry is generated by
`chosen_campaign_rewards.py`, which checks the canonical chosen recipe.
Its authored 5,000-gold payout uses the normal 100-gold platinum denomination:
fifty platinum coins, separate from the original source task payouts. The 10,000
XP is also additional to source rewards. This is a delivery binding for the
chosen currency value, not a change to the source tasks or their repeat limits.

`required_receipts` optionally supplies up to eight distinct
`{transition, track, value}` predicates. Each transition and track must belong to
the same canonical quest, and the expected value must fit the declared track.
The fenced writer checks that this character has an immutable receipt whose
`values_after` contains the exact track/value. A failed boss visit's rearm receipt
cannot satisfy a successful report. This predicate does not rewrite progress,
copy account achievements, or rely on a session's cached history. Its lookup uses
the `(character_id, quest_key, transition_key)` index under the Character root lock.

Source task repetitions can legitimately rearm a completed boss. The hunting
campaign therefore checks the three successful report receipts, retaining the
source rearm behavior. Its rank and once-only completion flag remain live storage
predicates. The complete reward plan, including historical predicates and value
outputs, is sealed into the quest definition hash by a non-executable marker.

`give.backpack_outputs` is an optional list of up to19 additional direct backpack
outputs. It is mutually exclusive with `give.contents`; the root plus every
additional output must fit together. It reuses the existing MINT reservation,
stable output identities, one-item audits and complete deferred custody proof.
The layout is bound into the request, and quest source proof compares every
output. Empty layouts preserve legacy request bindings. Split non-stackable
quantities into separate one-item outputs and stack quantities at proven caps.

Executable authored reward references close native Item admission during normal
regeneration. Unknown canonical stack facts can borrow pinned client custody
facts through `authored_quest_item_custody.py`; canonical records, accepted holds,
non-custody facts and stateful instance-owner requirements remain authoritative.

Authored death objectives may declare `guardian: {position, respawn_ms}` when the
chosen programme requires a native creature absent from the retained spawn
manifest. The position must lie inside the bounded objective chamber and the
respawn delay must be60000..3600000ms. Admission uses one shared native actor
slot per objective (at most32), native creature policy, real walkability and
occupancy checks. Only a present player with an active objective requests the
slot. Refresh cannot duplicate a living guardian, and pinned corpse projection
prevents retirement. This creates no quest credit; native death capture and the
existing fenced progress writer remain authoritative. Omit `guardian` to retain
an existing world spawn without changing its sealed definition.

Bounded scene edits may include `relocations: [{from: [Position], to: Position}]`.
At most32 distinct origin cells and32 current actors may participate. Origins
must be edited scene cells on the landing floor; the landing must remain outside
the edited cells. The World movement owner qualifies a known walkable landing
without a house, protection zone, teleport or floor change. A shared landing
may already contain actors, as required by forced source bridge transport.
Exact actor identity, position revision and existing summon-lifecycle checks
remain in the movement owner. A late refusal or rejected map projection restores
all carrier state; players receive the usual movement/environment updates.
Omitting relocations preserves existing source scene definitions. Native ordinal
lever bindings accept both declared lever appearances only when reversible,
while position, ordinal and any source attributes remain mandatory.

A world callback `item_on_target` can set `reusable: true` for a reusable carried
tool. It requires `count: 1` and no inventory `replacement`; a target replacement
remains supported. The existing fenced quest transaction locks the actual
carried instance and its custody location before committing the quest receipt.
It neither burns nor mints that tool. The omitted/false default preserves the
serialized shape and consumption behavior of existing callback receipts.

Reusable-tool contract reused from questpc4 commit `d168fabd39622ddccef6cf280e55e67775bf613c`; unrelated counter/quest changes are not duplicated.

## M09 independent item outputs

`give.additional` (also `effects.item_reward.additional` for world USE) contains
up to nineteen ordinary `{item, count}` outputs placed directly in the equipped
main backpack. They share the primary output's once-only claim, frozen intent,
reservation and SQL transaction. They are not children of the primary item and
do not require an invented reward container. Existing `contents` remain nested
children and keep their original deterministic identities and fluid proofs.

The complete output plan must fit both remaining backpack entries and carrying
weight before reserving or consuming the claim. A failed output rolls back all
outputs and quest effects. Sibling fluid subtypes, numbered keys and authored
text are refused; the existing primary/nested owners keep those metadata paths.
The internal equivalent is `RewardClaimMintRequest.additional_items`, with
ordinary `RewardClaimMintContent` entries and no metadata modifiers. Migration
2100 freezes and proves the complete sibling plan, including each audit event
and direct placement. Empty sibling lists preserve legacy intent hashes.
