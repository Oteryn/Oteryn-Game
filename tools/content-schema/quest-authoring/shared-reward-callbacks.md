# Shared NPC quest rewards

`content/quests/shared-reward-callbacks.json` is loaded by the Quest catalogue and
the real NPC talk dispatcher. Author records against
`shared-reward-callbacks.schema.json`. The Rust loader checks cross-field bounds,
track ownership and guards against repeatable grants.

`questpc3_npc_programs.py --check` reproduces this lane's source-backed NPC
programs and retains foreign registry records. Its source URLs pin Crystal
201189f. Generated dialogue is authored from those facts; donor code is neither
executed nor copied. Item rewards still require admitted native identity,
custody, capacity and MINT support; identity alone is insufficient evidence.

Each record has these fields:

| Field | Meaning |
| --- | --- |
| `key` | Unique `oteryn:quest-reward/<name>`; the durable transition/callback name. |
| `quest` | Owning canonical Quest key. Existing tracks must belong to this quest. |
| `npc` | Canonical `oteryn:npc.<name>`, resolved through the current conversation. |
| `keywords` | One to eight exact, trimmed, lowercase ASCII player messages. |
| `topic`, `next_topic` | Current/committed conversation topic, unsigned 16-bit integers. |
| `storage` | One to eight `{track, minimum, maximum, set, preserve?}` predicates/effects. Bounds are inclusive; missing progress reads as `-1`. `preserve: true` keeps that predicate's value. At least one changed value must leave its accepted range, preventing immediate repeat grants. |
| `experience` | Optional/null `1..100000000` XP, using the existing Quest XP obligation and transfer owner. The callback and executable transition must name the same amount; replay cannot transfer it twice. |
| `take` | Up to eight distinct `{item, count}` requirements, counts `1..65535`, aggregated across carried stacks, equipment and closed nested containers. Filled containers and items with keys, timers, text or imbuements refuse consumption. |
| `present` | Optional, up to eight distinct `{item,count}` carried requirements (`1..65535`). Uses the same bounded locked custody scan as `take`, including closed nested containers. It retains the original instances, quantities and receipts. |
| `equipped_slots` | Optional distinct occupied **durable** slots: HEAD=1, TORSO=2, LEGS=3, FEET=4, WEAPON=5, SHIELD=6, AMULET=7, RING=8, EXTRA=10. Backpack slot9 is excluded; donor and wire numbers are not durable keys. The current transaction checks the Character's equipment and live Item custody under the Character root lock. Combined with `present`, this can require a carried item and any occupied armor slot independently. |
| `health_loss` | Optional/null positive `u32` damage. The existing vitals owner stages source health loss (including mana shield and death), reserves its successor before the durable quest command, and installs it once after classification. Unknown outcomes retain the original pending successor; receipt reconciliation cannot repeat an installed effect. The amount extends the callback binding. |
| `minimum_level` | Optional/null positive `u32` level. NPC selection and read-only offers consult the current progression projection; the reward transaction independently checks locked Character progression before any effect. The gate is bound into the callback definition; an absent gate preserves its original hash. |
| `repeatable_check` | Optional boolean, false by default. Allows a source conversation to repeat a storage check. Every non-preserved predicate must accept exactly its unchanged next value. It forbids every physical debit, reward, XP, blessing, outfit, achievement and completion effect. Requirements and policy are bound into the transition digest. |
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
`keywords`, `topic`, `next_topic`, `storage`, and `reply`. These records may also carry `minimum_level`. They establish
confirmation topics and never apply storage effects. A reward's storage effects
apply only after the complete exchange commits.

The shared file is a registry, not an extra effect inside
`world-callbacks.json`. A lane adds its source-backed callback record and its
conversation offers to these registries during integration; it must preserve
the existing records and unique keys. Do not replace the shared generator's
outputs with a lane-only document.

The current lane program contains Santiago's initial greeting, five tutorial
confirmations, the exact three-leg/100-XP hand-in, and a native fish grant.
Zirella's delivered-firewood reward grants 50 XP. Return greetings restore source topics.
These callbacks do not qualify the whole tutorial: the remaining world
objects, occurrences, equipment, health lessons and wood delivery still need
their own executable bindings and PostgreSQL evidence. The reused optional item, outfit and achievement owners
are available to subsequent source compositions; no unconfigured reward is
inferred from dialogue text.
