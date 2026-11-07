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
| `storage` | One to sixteen `{track, minimum, maximum, set, preserve?, set_if?}` predicates/effects. Bounds are inclusive; missing progress reads the admitted track initial value. `preserve: true` keeps that predicate's value. A non-repeatable callback must change a value outside its accepted range. |
| `repeatable` | Optional boolean, default false. Only explicit costed item exchanges may repeat: nonempty `take` or an explicit fee, non-null `give`, no outfit or achievement. Each paid command has its own MINT claim identity; replay retains its original receipt. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `fee` | Optional `{gold, include_bank}` source price (`1..1000000000000`). Uses the existing durable coin/change and eligible bank debit owner in the quest transaction; a recipe cannot also consume coin denominations through `take`. |
| `requires` | Up to eight read-only parent quest track ranges. Their original quest retains ownership. The writer checks them under its root and sorted track locks. Explicit source track declarations can admit a prerequisite without inventing a completion write. |
| `close_after` | Optional boolean. Concludes this conversation after a known success or refusal, including receipt reconciliation; it cannot close a newer conversation context. |
| `failure_reply` | Optional source response lines on a known refused exchange. An uncertain transaction retains its pending receipt and response policy. |
| `minimum_level`, `experience` | Optional positive level guard and quest XP reward. The level is rechecked under the character lock; XP uses the existing durable quest obligation and live refresh. |
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
Kroox's later permission grant is `sam-kroox`: it updates the NPC and mine-door
tracks together and does not mint armor. The armor remains a mine chest claim.

`questl2_rewards.py` adds Dorian's seven exact deliveries (`thieves-tusks`,
`thieves-vase`, `thieves-invitation`, `thieves-bridge`, `thieves-goblet`,
`thieves-fish`, `thieves-letter`) and Hairycles' six hand-ins (`ape-moss`,
`ape-syrup`, `ape-parchment`, `ape-egg`, `ape-mushroom`, `ape-hair`). Offers bind
each confirmation to its source topic and storage predicates. These are stage
definitions, not a whole-quest qualification claim. The witches' cap retains
its source 24-hour temporal facts; the existing reward owner refuses instances
with persisted timed state, so its timed hand-in is not qualified by ordinary-item tests.

Jorge's `token-abacus` composition exercises a costed repeat through the same
owner. The source token shop has no mission storage gate, so a preserved internal
receipt track stays at `-1` for each purchase. The carried-token cost admits each
purchase. A repeat's claim key binds the recipe, session and command ordinal;
the database proves that identity against its same-transaction consumption and
quest receipt. Ordinary once-only callbacks retain their recipe claim identity.
The complete 37-souvenir menu is parsed from the pinned Jorge table, with
per-recipe file, line and SHA citations. Three PostgreSQL shards drive every
recipe through one admitted player's NPC conversation and fenced item owner,
including exact-command replay and insufficient-token refusal. The Abacus
case additionally exercises a second paid occurrence. The native importer
preserves accepted souvenir weights, readability and equipment facts when
promoting inert canonical profiles.
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

Exact conditional storage replacement uses `set_if`: the fenced writer replaces
only that locked value with `set`, retaining every other value. It cannot be
combined with `preserve` or a native rule, and the exact comparison must lie
inside the accepted range. NPC rewards and world callbacks lower to the same
closed QuestState effect. Existing records omit the field and retain their
previous intent hashes.

A returning `step_in` callback may specify `failure_teleport`. Failure uses that
validated landing; omitting it retains the original approach-tile recovery.
Recovery grants no quest progress or quest receipt, and an unavailable landing cannot
invent a walkable tile. Source implementation reused from PC6 commits
`be812420e3c20d9dd8ae6c3c23a9c4acb942ceda` and
`c651c87d52acd0fcc56c5b426a1e9f27ab0b92ba`.

Donor justification: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`,
`data-global/scripts/quests/draconia/movement-escape.lua:11,16-19`, SHA256
`53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312`:
the missing-offering branch uses its authored recovery point; the successful
branch resets Dragonfetish only when its locked value is exactly one.

Native payment and prerequisite composition reuses QENGINE2 commit
`33b640335` (reviewed through `4ddf4bcff`). L2 migration 1142 retains the
source migration's deferred participant proofs while avoiding its original
1000 number, already used by the imported corruptor lifecycle migration.
The complete published payment/outfit examples remain in the shared producer;
L2 additions are composed through `questl2_rewards.py`.

Draconia's dragon greeting exchanges one brown mushroom (3723) for one dragon
fetish (3206), sets Dragonfetish to one, and always concludes focus. An existing
value of one refuses without payment. The canonical NPC key
`oteryn:npc.an_old_dragonlord` maps to donor file `an_old_dragon_lord.lua`;
Crystal revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, lines 61-80,
SHA256 `85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052`.
The full Draconia qualification also requires the offering and alternative exit
journeys; a greeting test alone does not qualify that entry.

`give.backpack_outputs` uses PC8's existing complete MINT output plan: up to
19 additional direct backpack entries accompany the root reward. It is mutually
exclusive with container contents. The whole set must fit the equipped backpack
and carried capacity; each item retains a native MINT audit and all outputs
commit with the same quest receipt. Absolute-deadline rewards currently require
a single root, so mixed timed peer outputs are rejected at authoring.

`bulk` declares an all-carried-input exchange. The authored one-input/root
reward pair is a template; the fenced writer resolves the full carried count,
rolls once per input with the native server RNG and freezes the exact result
in the immutable source plan. Root and peer stacks use the same complete MINT
owner. The database proves the exact retained outcome and rejects any remaining
carried matching input. A capacity refusal consumes nothing. This bounded form
requires distinct input/output keys and excludes unrelated value participants.

Finite world USE callbacks can declare `effects.item_reward` using the same
complete output layout as NPC rewards. The source transition must advance
bounded state; read-only, offering-consuming and item-on-target callbacks
cannot use this form. The original world occurrence, quest receipt and all
native output/audit participants commit together. Positive appearance-only
selectors resolve the actual server map stack for item-id registrations.
