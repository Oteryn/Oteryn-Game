# QCX50 Wrath of the Emperor Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global source is the fetched
`qcx-facts/wrath-of-the-emperor.md` at `460e1d8c2c0e88c438ae5e54910db00822dc0129`.
The base page establishes requirements and advertised rewards. The spoiler
establishes the mission order, interactions, and achievements.
[TibiaWiki quest, curid 46607 oldid 1103764](https://tibia.fandom.com/?curid=46607&oldid=1103764)
[TibiaWiki spoiler, curid 46608 oldid 1103771](https://tibia.fandom.com/?curid=46608&oldid=1103771)

Crystal evidence is pinned at `201189f871d23597b82caf88fd611454a2ddefb8`.
Lua paths below are relative to `data-global/scripts/quests/wrath_of_the_emperor/`
unless otherwise indicated. The retained repository audit references are
`docs/reference/QUESTPC8-CONTRACT-NOTES.md` and the four
`docs/reference/wrath-*-validation.json` files.
[Crystal quest scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/wrath_of_the_emperor)

## Parity audit

The canonical definition carries premium and a null minimum level. Its disabled
chosen recipe uses level 0, has no prerequisites, and represents an abbreviated
route. The global level 80 gate belongs to the Mission 8 expertise door. The
recommended level 130+ is informational. The spoiler lists a shovel, rope,
rake, pick, obsidian knife, 15,000 gp, three nails, wood, and 80 oz of free
capacity for the crate. The global path also requires The New
Frontier through receipt of Mortal Kombat, Children of the Revolution completion
plus Zzuppliezz, and The Ape City with Mission 9 started. The Zalamon opening on
base checks only premium and local progress. Crystal checks Children completion
at questline 21 when Zalamon offers the opening (`data-global/npc/zalamon.lua:166`).
The spoiler explicitly places the daily task at Mission 5 and requires finishing
Strike if accepted first.

| Mission | Comparison with the native route |
| --- | --- |
| 1 Catering the Lion's Den | Zalamon consumes nails 953 ×3 and wood 5901 ×1 for marked crate 11328 ×1. Base uses existing authored entry and crate keywords. Shovel entry, crate disguise, detection, jail and Zumtah's escape, relay switches, and Chartan handoff remain unverified or unbound. Global lights last 30 seconds and the wall 10 seconds. Crystal uses 20-second timers in the light script, including the wall restoration. The spoiler describes both the two-person relay and a timed solo workaround. |
| 2 First Contact | The definition describes restoring the teleport, but the chosen runtime lacks Chartan's report and the complete sacred earth, clay, bowl, water, coal and basin chain. The global route uses reusable rake and knife, with shovel/pick alternatives. Crystal identifies the final basin with action ID 8025 and resets the water visualization after 60 seconds (lines 40–55). |
| 3 The Keeper | Base has a chosen objective and source transition evidence. Global poison exhaustion spawns The Keeper, corpse use yields the tail, and the active poison field deals 8,000 damage. The live encounter and tail/report chain remain unbound. Crystal uses poison 11364 on action ID 8026 and gives tail 11367 from corpse 11360. |
| 4 Sacrament of the Snake | Global parts Cusp, Emerald and Shaft cost 5,000 gp each. The player must assemble the sceptre using the bottom-floor switch personally, then hand it to Zalamon. The chosen definition covers the objective, with no complete native offer, assembly and report chain. Crystal registers assembly on UID 1053 and exchanges 11368, 11369 and 11370 for Snake Sceptre 11371. |
| 5 New in Town | Global supplies/Strike gate, Gate Guardian's one-time admission and Zlak handoff are absent from the chosen route. Re-entry uses the permanent city routes after the handoff. |
| 6–7 The Office Job and A Noble Cause | Native death objectives use the correct counts of four Magistratus and six Nobles. They run together in a single bounded chosen stage and report. Global has separate Zlak assignments and reports. Crystal uses party death attribution, while the chosen route credits the present principal. |
| 8 Uninvited Guests | The chosen route skips the level 80 expertise door, palace traversal and Zizzle report. Crystal's lever uses UIDs 3184 and 3185. |
| 9 The Sleeping Dragon | The source NPC placement and native linear dream finale award Wayfarer. The preceding chosen hunt report sets the potion marker directly. The global Zizzle potion/paper handout, actual potion use, continued dream navigation, charm after the dream and repeat Scale-of-Corruption entry remain incomplete. Crystal identifies potion 11372, paper 11426, scale 11673 and charm 10343. |
| 10 A Message of Freedom | The chosen side objective checks four literal appearance-11427 crystals in clockwise order while carrying Snake Sceptre 11371. Global calls for Replica of the Sceptre 11362 from Zizzle and pillar use, boss summon, defeat and timed remains use in each room. Crystal uses boss UIDs 3193–3196, absent from these native selectors. |
| 11 Payback Time | Global Awareness briefing, four successive Zalamon forms, replica on final remains, Godslayer, and loss of the Zalamon NPC route with task handoff to Chartan are outside the chosen native route. |
| 12 Just Rewards | Global requires Awareness then Izsh, both backpacks and one armor choice before the wardrobe. The retained outfit callback uses wardrobe appearance 11431 at (33073,31173,8), donor UID 3200, Mission12=0 and the base Wayfarer entitlement with zero addons. The callback does not check the two backpacks or armor choice. The quest state records completion as `NOT_LOWERED_MULTI_TRACK`. |

Wrath has no entries in the `world-callbacks-held/` batch files. The quest
bindings above are in `authored-world-callbacks.json`,
`chosen-reward-callbacks.json`, `authored-death-objectives.json` and
`shared-outfit-callbacks.json`. The four crystal selectors use floor 15 and
ordinals 1, 1, 2, 1 at (33049,31089), (33095,31091), (33096,31114), and
(33049,31115). The donor dialogue document and mission-state
callbacks preserve transitions beyond the selected live routes.

## Reward and access census

- Spiritual Charm 10343 ×1 follows the dream. Crystal implements the separate
  grant at `data-global/npc/a_sleeping_dragon.lua:203–222`. The retained report
  after the dream awards no item. Charm 10343 is in the shared item catalogue
  with unknown stack class and materialization disabled. Wayfarer achievement
  is already live.
- Platinum Coin 3035 ×100 is imported for UID 6297 at (33073,31169,8).
  UID 6298 at (33074,31169,8) has a held container
  variant retains Crystal 3043 ×10, Red Dragon Scale 5882 ×10, Gold Ingot 9058 ×5,
  Black Pearl 3027 ×10, Blue Gem 3041 ×1, Black Jade Cobra 11695 ×1, Dragon
  Backpack 10326 ×1 and Jewelled Backpack 5801 ×1, inside donor container 2853.
  The global snapshot establishes the rewards, with no explicit nested layout.
- Armor candidates are Elite Draken Helmet 11689 ×1, Royal Draken Mail 11686 ×1,
  Royal Scale Robe 11687 ×1 and Merudri Scale Mail 50263 ×1. Crystal bindings
  use 6300, 6299, 6308 and 7825 respectively, with alternate Canary UIDs in the
  claim records. The imported armor claim does not prove a live four-way
  selection linked to the wardrobe. Native reward positions are (33080,31170,8)
  for the helmet, (33078,31170,8) for the mail, (33076,31170,8) for the robe,
  and (33074,31170,8) and (33079,31173,8) for alternate Merudri placements.
- Global access covers Rebel Camp, Razachai and Inner Sanctum with teleports.
  The complete gated teleport chain is pending. Donor coordinates and access
  checks are in `movements_teleports_access.lua:1–69`.
- The 10,000 XP in Crystal `data-global/npc/zalamon.lua:161` is paid on Children
  of the Revolution completion. The global Wrath snapshots state no XP amount.

## Blockers

BLOCKER Mission 1 item-look disguise with prison/escape restoration actions_mission01_crate.lua:1

BLOCKER Mission 1 shared light and wall timers with detection actions_mission01_lights.lua:20

BLOCKER Mission 3 poison-triggered Keeper encounter and corpse-tail handoff actions_mission03_the_keeper.lua:15

BLOCKER Mission 9 shared-item custody for Spiritual Charm 10343 data-global/npc/a_sleeping_dragon.lua:221

BLOCKER Mission 10 incarnation room lifecycle and transient remains credit actions_mission10_a_message_of_freedom_sceptre.lua:17

BLOCKER Mission 11 chained boss replacement and timed room reset creaturescripts_zalamon_kill.lua:38

## OPEN-FACTS

Global XP amount, numeric object bindings and item IDs, exact chest nesting,
and Keeper poison-use quota are unspecified by the global snapshot. Item IDs
and source UIDs above are donor mappings, not values asserted by TibiaWiki.

## PARTIAL

Remaining prerequisite bindings and Mission 8 door, Missions 1–5 beyond the
opening, separate hunt reports and global attribution, Missions 8–12 and the
end-to-end reward-room claims, outfit prerequisites and teleport chain.

## Entry gate change and regression

**Open dependency** — P1-3 tail. A fresh character needs the out-of-package Phantom Army acceptance and battlefield bindings to reach Children of the Revolution stage 20. This dependency is non-blocking for this package.

The Zalamon opening now requires the existing Children of the Revolution
questline to equal 21. The callback only reads the upstream track. The check
runs again in the locked quest writer, so a stale NPC view cannot authorize
entry. The crate hand-in continues to validate its existing Wrath state and
exact materials. The PostgreSQL fixture seeds the upstream completion through
the fenced quest writer. This test covers the Wrath gate and does not qualify
the upstream quest's terminal route.

PG via shard (pgsplit build files applied on
`9102bb7bda719bb45129f28b1d54097d343ac11a`): red, 0 passed and 1 failed in
49.59 s. The first `wrath mission` wire command returned Accepted when the
prerequisite was absent, where the regression expected Rejected.

PG via shard (pgsplit build files applied on
`c7c073ac755c13ccb2b4c9fdec84c7a22da2a050`): green, 1 passed and 0 failed
in 52.47 s. The same wire request is rejected without the upstream flag
and accepted after seeding it. The crate checks still pass for exact
materials, partial-hand-in refusal, repeat and cold replay.

- Route-less run using the same filter with `OTERYN_TEST_POSTGRES_ADMIN_URL`
  removed returned `Ok(())`, 1 passed and 0 failed in 0.00 s.
- Quest authoring passed all 44 check commands. The chosen reward generator's
  own `--check` also passed.
- World pin refresh retained
  `0676b1568f55a55fbe994f344a8e0c60debf766751d12849f56c19f60c93aae5`.
  The compiler's `pin-check . <temporary output directory>` passed.
- `cargo fmt --all -- --check` passed.
- Dispatch library Clippy passed with all features and `-D warnings`.
