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
