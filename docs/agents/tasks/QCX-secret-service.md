# qcx52 Secret Service Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Canonical quest
`oteryn:quest.secret_service_quest`. The Global comparison uses the fetched
`qcx-facts/secret-service.md` at `460e1d8c2c0e88c438ae5e54910db00822dc0129`.
The two revisions are
[Secret Service Quest (20024/1096988)](https://tibia.fandom.com/?curid=20024&oldid=1096988)
and [Secret Service Quest/Spoiler (20260/1078374)](https://tibia.fandom.com/?curid=20260&oldid=1078374).
The donor is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. Paths and
line numbers below refer to that revision unless otherwise specified.

## Comparison

The quest definition already records level 50 and Premium. Its dormant
chosen recipe only models TBI and names Black Knight as the final target.
Global offers TBI, AVIN or CGB and locks the choice on recruitment. All
three culminate in Mad Technomancer, with the beard as the hand-in.
The definition has no native storyline bindings. Crystal's join confirmation writes a common questline value and
the first mission writes the faction-specific track. That leaves a
window between those two commands where the donor has no persistent
record of which recruiter accepted the player. The imported mission
and door records are source projections. The active and held world callback
directories, and the quest NPC callbacks and offers on the base, have no
Secret Service binding.

| Route | Global sequence | Difference on base |
| --- | --- | --- |
| TBI 1–3 | Light the Venore shipyard with a Fire Bug, wait for any prior seven-minute fire to end. Retrieve the Black Knight parchment via key 5010. Face two Pirate Buccaneers in Port Hope and take the chest scroll. | No dock fire or ambush binding. Crystal's fire callback has a seven-minute event that removes the tool instead of the fire and lacks Global's explicit shared lockout (`data-global/scripts/quests/others/actions_fire_bug.lua:144-155`). Parchment and scroll claims have no map selectors on the sampled raw cells. |
| TBI 4–6 | Retrieve Intelligence Reports from the sealed Hellgate corpse route via key 3012. Plant Chester's Suspicious Signet Ring in the Ice Witch Tower. Use the Amazon disguise in the Svargrond tavern cellar, crowbar the cask within roughly 20 seconds and face two Barbarian Skullhunters. Chester replaces the kit for 1,000 gp. | No Secret Service progress gates, ring placement, disguise, crowbar ambush or replacement offer. The corpse claim lacks its selector. Crystal checks AID 12566 and Amazon look type 137 before spawning the two Skullhunters (`data-global/scripts/lib/register_actions.lua:955–965`). The existing crowbar handler for AID 40041 serves The Ape City. Buddel travel via Barbarian Test Quest is optional. |
| AVIN 1–3 | Deliver Uncle's Secret Letter after Gamel's two Bandits. Take File AH-X17L89 in Thais castle. Deliver the 85 oz beer barrel to Karl after two Amazons in the Carlin sewers. | No letter, greeting ambush, quest drawer selector, beer hand-in or sewer ambush binding. |
| AVIN 4–6 | Earn personal most-damage credit on Black Knight, take the signet ring in the Isle of the Kings crypts and deliver Uncle's weapons to Freezhild. | Crystal credits damaging players broadly (`data-global/scripts/quests/secret_service/creaturescripts_black_knight_kill.lua:3-9`). No exact AVIN kill-credit binding, crypt claim selector or Freezhild hand-in. White Raven Monastery mission 1 and first-visit Costello dialogue concern route access. |
| CGB 1–3 | Take the Nature Magic Spellbook in the Edron academy, take the Rotten Heart of a Tree at the Black Knight Villa, and use Emma's Rust Bugs on the Ironhouse keyhole to leave a trail. | No native claim selectors for the spellbook or tree. Missing slot-specific rust bugs use and trail. Crystal's rust-bug callback identifies unique 22005 and creates four trail items (`data-global/scripts/quests/secret_service/actions_rust_bugs.lua:2-15`). |
| CGB 4–6 | Get ship plans at the Venore shipyard, suspicious documents past Dark Cathedral's lever teleporter, then the book from the White Raven Monastery crypt. The monastery access quest can be done during CGB. | Ship plans, documents and book have content claims with no corresponding raw selectors. No Secret Service mission or hand-in binding. |
| All 7 | Pull the Kazordoon lever and obtain a Technomancer Beard. Every player seeking one pulls the lever. A purchased or shared beard also works, and completed agents retain room access. TBI can ask to restart the mission. | No active Secret Service lever, per-player loot eligibility, completion access, restart or native final NPC exchange. Crystal's lever toggles an AID 12574 lever and spawns the boss and adds without checking a player mission (`actions_lever.lua:1-36`). Crystal's beard is item 396, with guaranteed loot (`data-global/monster/bosses/mad_technomancer.lua:69-70`). |

The raw B3 placements contain all ten imported chest/corpse appearances at
their claim coordinates, each without the claimed top-level UID. Crystal
confirms all ten claim selectors and one-item counts in
`data-global/startup/tables/chest.lua:2481–2550`. The imported one-item
claims are as follows. Numeric IDs identify the repo or
donor item and placement, while the pinned Global spoiler identifies them
by name and location.

| Mission item | Repo item | UID | Coordinate |
| --- | ---: | ---: | --- |
| AVIN file | 403 | 6243 | 32311,32177,5 |
| AVIN family signet ring | 406 | 6244 | 32156,31954,13 |
| CGB magic spellbook | 648 | 6245 | 33271,31839,3 |
| CGB ship plans | 399 | 6246 | 32907,32012,6 |
| CGB suspicious documents | 400 | 6247 | 32598,32380,10 |
| CGB crypt book | 401 | 6248 | 32180,31930,11 |
| CGB rotten heart | 652 | 6249 | 32876,31958,11 |
| TBI parchment (import key `lottery_ticket`, written text held) | 5956 | 6250 | 32872,31958,11 |
| TBI Port Hope scroll | 5952 | 6251 | 32643,32733,7 |
| TBI Intelligence Reports | 348 | 6252 | 32773,31582,11 |

Crystal also confirms the eleven source door coordinates, including the
final-room door at 32537,31897,13, in
`data-global/startup/tables/door_quest.lua:560–599,760–763`. The route
keys 3012 and 5010 have donor door coordinates in
`data-global/startup/tables/door_key.lua:70–76,168–171`.

The global item rewards are a sealed package and its faction tool. Crystal
hands out one package and exchanges it for one tool. Its numeric mapping
is 897 → 9598 (TBI), 899 → 9594 (AVIN), 898 → 9596 (CGB). The repo's
three package item records are non-materializable. No package opening
recipe is registered. The Global page lists Secret Agent and one faction
achievement, with no XP or outfit reward listed. The dormant recipe omits
Secret Agent and the AVIN/CGB faction achievements. Crystal grants Secret
Agent at AVIN/CGB recruitment and Top AVIN/CGB Agent at hand-in. Chester
grants a package and speaks of the title but grants neither achievement
in his completion handler.
[`actions_tools.lua:1–22`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/secret_service/actions_tools.lua#L1-L22)
[`chester_kahs.lua:154–160`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/chester_kahs.lua#L154-L160)
[`uncle.lua:106–109`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/uncle.lua#L106-L109)
[`emma.lua:118–121`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/emma.lua#L118-L121)

## Entry slice

The existing NPC offer and atomic quest reward callback owners handle
`join` and `yes` at Chester Kahs, Uncle and Emma. The offer checks level 50.
The confirmation rechecks level and current Premium and writes the first
questline value together with a durable faction selection. Local faction
values are 1 for TBI, 2 for AVIN and 3 for CGB. A started questline or a
selected faction refuses subsequent recruitment. Crystal places the Secret
Agent award at AVIN and CGB recruitment. The pinned Global page establishes
the reward for TBI too, while its exact timing remains open.

The placement generator scoped the three recruiters at the retained
candidate coordinates, which agree with Crystal
`data-global/world/world-npc.xml:210–211,609–610,846–847`.
Chester is at 32349,32183,6, Uncle at 32952,32039,4, and Emma at
32384,31781,6. Emma and Uncle occupy source-backed solid-object
contacts. The compiler applies its existing contact policy at those two
cells. Decoding the pin-checked bundle confirmed exactly these three
placements.

## Blockers

- BLOCKER AVIN mission 1 NPC greeting cannot atomically spawn two Bandits via the NPC rule owner data-global/npc/gamel.lua:59–69
- BLOCKER TBI dock fire atomic shared seven-minute created-fire lockout and retirement through carried Fire Bug use data-global/scripts/quests/others/actions_fire_bug.lua:144–155
- BLOCKER All mission 7 lever contribution to guaranteed per-player loot and continued access after completion data-global/scripts/quests/secret_service/actions_lever.lua:25–36

The sampled B3 source regions also lack the donor AIDs for the signet
ring (12563), Port Hope ambush (12571), Svargrond cask (12566), and
Technomancer lever (12574), and the Ironhouse unique 22005. The donor
registrations live in `data-global/scripts/quests/secret_service`,
with the cask registration in `data-global/scripts/lib/register_actions.lua:955`.

## OPEN-FACTS

Exact Global item and placement IDs, exact door thresholds and action
selectors, AVIN Bandit and TBI ambush death-credit attribution, shared
lever timing/reset conditions, and TBI's Secret Agent award timing.
The spoiler gives only an approximate disguise duration. No numerical XP
award or permanent outfit is stated. The equipment table describes tool
functions but omits their jam probabilities and duration.

## PARTIAL

Recruitment is a separate slice. The 21 mission steps, progress-gated
claims and selectors, access and action callbacks, written parchment,
package admission/opening, mission 7 hand-ins and faction achievements
remain pending. The dormant TBI approximation, its erroneous final
target, and the CGB male recruitment opener are also pending. No plain
claim was activated independently of its mission gate.

## Validation

Validation target `8156fc87b5c4779ee06dbfa05cfa2a65e3b0fa39`.
Formatting, dispatch library Clippy with `-D warnings`, and all 44
quest-authoring check commands exited zero. NPC placement conversion
matched the tracked shards. The World compiler pin check passed and the
bundle's NPC frame contained the three exact source placements. The
native QuestState catalogue loaded all three confirmations with the
level, Premium and faction guards.
