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
| `storage` | One to sixteen `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as the declared track initial value (new callback tracks default to `-1`). `preserve: true` keeps that predicate's value. Unless explicitly costed and repeatable, at least one changed value must leave its accepted range, preventing immediate repeat grants. |
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

## Costed repeats and absolute lifetimes

The owner-authorized NPCFIX composition reuses QENGINE2 `9d3c3869f` and
QUESTL2 `7e16b64ee`. Storage plans support sixteen effects, matching the shared
QuestState bound. `repeatable: true` requires consumed inputs and an item output;
its immutable MINT identity includes the recipe, session and command occurrence.
Exact command replay consumes and mints nothing further. Ordinary callbacks keep
their existing once-only guard and claim identity. Keyword matching remains exact.

A repeat may explicitly set `achievement_once: true` with an achievement. The
existing account achievement owner retains the first provenance; the stable
Character/recipe event key makes later paid conversions idempotent for that
achievement. The flag cannot authorize a free or outfit repeat. All consumption,
MINT, receipt and achievement writes remain one fenced transaction.

An output may declare `decay: {duration_millis, target}`. The runtime requires an
exact match to its admitted native Item definition. Only a single non-stopping
absolute-deadline item is supported; charged items, stacked timed outputs and
timed nested reward contents remain rejected by this composition. The existing
absolute Item owner stores the deadline in the original reward transaction, and
the database binds it to the exact MINT, generation, duration and target. Carried
moves preserve that immutable schedule. No timer or Item definition is fabricated
from a dialogue line.

`npcfix_rewards.py` reads the pinned repository corpus for Al Dee, A Weakened
Forest Fury and A Wandering Soul. Every generated callback has its donor path,
line and digest. The cage acceptance uses the existing issued-cage marker once;
the source replacement dialogue says to return later and does not mint another
free cage. Al Dee's receipt-only internal track does not invent mission progress.
Wandering Soul preserves the Blood Brothers mission track while consuming the
empty crystal, creating its seven-day charged form and earning Ghostwhisperer
once. Conditional voices use bounded read-only carried evidence, including
nested bags and equipment, and never authorize a reward.
