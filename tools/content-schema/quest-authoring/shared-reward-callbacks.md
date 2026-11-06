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
| `storage` | One to eight `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as `-1`. `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
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
