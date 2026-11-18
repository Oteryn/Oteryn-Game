# QCX78 — The Thieves Guild Quest

Quest `oteryn:quest.the_thieves_guild_quest`. Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

- Global facts from the 2026-11-17 capture in `qcx-facts/the-thieves-guild.md`. [Quest, curid 25436 and oldid 1090720](https://tibia.fandom.com/?curid=25436&oldid=1090720) · [Spoiler, curid 25437 and oldid 1086330](https://tibia.fandom.com/?curid=25437&oldid=1086330)
- Donor evidence uses Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. Paths below are relative to that tree. [Crystal source tree](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global)

## Comparison

| Part | Global and Crystal evidence | Base result |
| --- | --- | --- |
| Entry | Premium, minimum level 0, recommended 70+, Dorian greets and invites on `thieves`. Crystal `npc/dorian.lua:148–154,221–225`. | Definition requirements agree. Dorian’s NPC declaration is imported and its source binding is held from runtime. There is no quest-owned enrollment offer or callback, and the NPC is absent from the committed native NPC placements. Crystal’s enrollment callback has no explicit premium predicate. |
| 1. Ivory Poaching | Dorian requests and takes ten tusks. Crystal `npc/dorian.lua:61–70,156–161` uses item 3044. | Imported source journal and approximation retain the objective. Approximation leaves the tusk unresolved and no native Dorian turn-in is bound. |
| 2. Burglary | Dorian sells lock picks. Climb Sarina's vines, pick the chest, return the vase. Three to ten lock picks is a recommendation. The global Valuable Vase corresponds to canonical 7932, while Crystal’s Dorian takes 227 (Fine Vase). Crystal `npc/dorian.lua:71–82,163–168,226–254`, `startup/tables/item.lua:1780–1792`, `scripts/quests/the_thieves_guild_quest/actions_climbing_vine.lua:2–23`. | Dorian’s base trade service lists lock pick 7889 at 50 gp. The approximation collapses navigation and lock picking into collection and leaves vase unresolved. Vine actions 12501/12502 and chest selector 12503 are missing from their map entries. No quest-dispatched vine, burglary or hand-in route. |
| 3. Invitation | Oswald's invitation costs 1,000 gp, then Dorian takes it. Crystal `npc/oswald.lua:78–109` and `npc/dorian.lua:83–92,170–178` use item 7933. | Approximation names the invitation. It omits the price and turn-in, and the production NPC bindings are absent. |
| 4. Bridge Robbery | Snake Eye refers the player to Ahmet. Pay 1,000 gp for a deed, with a possible oasis Nomad kill. Collect a Theatre Script, pass Percybald's quiz, consume the kit for five minutes of dwarf disguise, exchange the deed for the painting with Nurik while disguised, then report. Crystal `npc/ahmet.lua:107–116,132–140`, `npc/percybald.lua:61–126`, `npc/nurik.lua:57–96`. Crystal checks RewardOasis for Ahmet's fee and uses a fixed quiz with an extra prompt. The global spoiler describes variable prompts and an optional Nomad request. | Approximation omits the mission. Native NPC route, kill credit, disguising and painting exchange are absent. The Theatre Script claim (8117) exists, but UID 6272 is missing from its chest. Prop-room door action 41721 is present at (32367,31782,8). Crystal uses kit 7865, deed 7866 and painting 7871. |
| 5. Enforcing Debts | Collect one Stolen Golden Goblet from the cave chest beyond the quest door and return it. Crystal `startup/tables/chest.lua:2637–2643`, `startup/tables/door_quest.lua:547–550`, `npc/dorian.lua:106–115,187–193`. | Door action 41715 is present. The item claim correctly names id 7870, but the approximation marks the goblet ambiguous, the map lacks chest UID 6273 and Dorian's route is missing. |
| 6. Fishnapping | Herbert sells the letter for 1,000 gp, Carina exchanges it and 5,000 gp for a necklace, Chantalle trades it for the key. Crystal uses letter 7940, necklace 7939, key 7934 and fish 7936. Alternatively rank-5 Arch Postman with Post Officer’s Hat 3576 equipped uses Chantalle (male) or Bertram (female). Use the upstairs bowl and return one fish. Crystal `npc/herbert.lua:60–75`, `npc/carina.lua:61–75`, `npc/chantalle.lua:61–88`, `npc/bertram.lua:61–69`, `npc/dorian.lua:116–125,194–200`. | Approximation omits both routes' detailed conditions and hand-in. One native callback consumes 7934 at mission state 2 and teleports inside. It selects action 51394, while the observed placement at (32359,32787,6) has no action and Crystal's script checks 41714. No quest-owned bowl acquisition, room-exit or hand-in binding. Crystal's `startup/tables/item.lua:4556–4559` places item 7936 directly, whereas the global route uses a bowl. |
| 7. Blackmail | Use Venore's Chest of Drawers for one Compromising Letter and return it. The old bed interaction was replaced. Crystal `startup/tables/chest.lua:2644–2650`, `npc/dorian.lua:126–134,201–210`. | Claim correctly uses item 7935 on the drawer, but UID 6274 is missing from the map. Approximation omits the mission and the NPC return is absent. |
| 8. Message | Flip six levers before they reset in about a minute, step through the teleport, post a note between the trophies, then report. Crystal `scripts/quests/the_thieves_guild_quest/actions_note.lua:12–26` consumes note 7873, places temporary wall object 7872 for five minutes and advances mission state. | Approximation describes posting only. No quest-bound lever set, teleport condition, note use or Dorian turn-in. The placement lacks action 12509 from Crystal `startup/tables/item.lua:1793–1796`. |
| Final rewards | One choice of Modified Crossbow 8021, Assassin Dagger 7404 or Spellbook of Warding 8073, count one, and Black Bert access. No XP or outfit reward listed. Dorian opens the reward room after the last hand-in. Crystal `startup/tables/chest.lua:2651–2670`, `startup/tables/door_quest.lua:551–554`, `npc/dorian.lua:211–219`. | The three placements share one once-per-character claim with correct rewards. The approximation lists only the dagger and Black Bert access. The room has action 41719, but no Dorian callback writes completion/door access. The three chest UIDs 6275–6277 are missing. Black Bert’s trade service exists, with no quest binding in its NPC declaration and no matching committed native NPC placement. Crystal’s shop also has no explicit completion check in `npc/black_bert.lua:158–204`. |
| Achievements | Amateur Actor and Master Thief. Neither named grant appears in the compared Crystal dialogue callbacks. | Both are defined in the achievement catalogue (Amateur Actor 52, Master Thief 51) and missing from the quest approximation and native delivery. The global quest names them without identifying exact trigger events. |

The exact quest key appears in one dispatcher world callback, Fishnapping’s key, and in no per-quest file under either world callback directory or the held directory. The native NPC callback and offer directories have no record owned by this quest.

## Correction

Restore UIDs 6272–6277 with the existing `quest_chest_bindings.py` capture and placement overlay. Each restored UID is checked against the existing claim's exact coordinate and appearance. The three final chests continue to share the same once-per-character claim. The PostgreSQL test reads each source selector from B3, uses a reduced approach inside the applicable room, then checks literal USE and the quantity in durable inventory.

## Engine and ownership gaps

- BLOCKER Burglary vines PvP-lock refusal on both vine endpoints requires a compatible shared teleport policy `data-global/scripts/quests/the_thieves_guild_quest/actions_climbing_vine.lua:13`.
- BLOCKER Bridge Robbery Nomad credit requires a shared death/party-credit producer scoped to this quest and oasis `data-global/scripts/quests/the_thieves_guild_quest/creaturescripts_nomad_kill.lua:2`.
- BLOCKER Bridge Robbery disguise requires consumable timed condition plus NPC outfit validation; the quest's retained item-disguise records use positional restoration `data-global/scripts/quests/the_thieves_guild_quest/actions_dwarf_disguise_kit.lua:1`.
- BLOCKER Fishnapping shortcut requires the combined equipped-head, rank and sex predicate on the key exchange `data-global/npc/chantalle.lua:85`.
- BLOCKER Message requires shared timer and world-object integration for the lever window and temporary wall note `data-global/scripts/quests/the_thieves_guild_quest/actions_note.lua:17`.

## OPEN-FACTS

- Exact achievement award events, Ahmet's live optional-branch predicate, Percybald's live prompt pool and selection, lockpick price/success policy, exact Cathedral reset durations, and the exact global bowl/door selectors are unspecified.
- The single 235 oasis bag from Crystal chest UID 6271 is absent from the global account. The legacy claim exists, but its placement and live role are unresolved. The approximately 2,210 gp of travel money is guidance.
- The wiki names rewards and mission objects. Protocol IDs and map coordinates in this audit are corroborated against the donor and canonical map or item data.

## PARTIAL

Reward placement correction only. Remaining work includes native enrollment, mission transitions and hand-ins, burglary placements, the prop-room and reward-room unlock writers, the Fishnapping door/bowl route, Cathedral mechanics, Black Bert access, both achievements, and alignment of the non-native six-job recipe with the eight missions and three reward choices.

## Verification

PG via shard (pgsplit build files applied on `afbdedc91` with the finalized regression in the working tree). All six cases reached dispatch on the original placements. Each returned `Accepted / NothingToUse` where the regression expected `Accepted / Committed`.

| PostgreSQL case | Original placements |
| --- | ---: |
| Assassin Dagger | 21.15 s |
| Compromising Letter | 10.23 s |
| Modified Crossbow | 9.81 s |
| Spellbook of Warding | 9.52 s |
| Stolen Golden Goblet | 9.15 s |
| Theatre Script | 9.64 s |
