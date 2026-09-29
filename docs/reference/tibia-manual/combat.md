# Tibia manual notes: combat

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=combat>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `83f6b9e60101e0be404cf2d49402d69941c47a83b95ffb8d6760042ad529610b`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §combat <heading>, capture 2026-09-28`.

## 5.3 Combat (overview)

- [both] Section extends the basic combat intro from controls; covers combat mechanics, logout block, PvP rules and party mode.

## 5.3.1 Combat Rounds

- [server] Attacker strikes a chosen target repeatedly at a fixed interval once combat starts ("steady" attack cadence); only a single target can be actively attacked at once.
- [server] Defenders always get a chance to mitigate an incoming attack, even without retaliating.
- [server] Hard cap: no more than 2 attacks can be blocked concurrently (i.e. against >2 simultaneous attackers, blocking chance effectively caps out).
- [server] Hit resolution: each swing computes an attack value vs the defender's defence value (both derived from skills, equipment, and a random factor); attack value > defence value = hit lands, damage number displayed.
- [client] Visual feedback: damage number on hit; puff-of-smoke effect on a successful block; spark/flash effect when armor absorbs/reduces a blow.

## 5.3.2 Damage Types

- [server] 7 damage types: fire, ice, energy, earth, death, holy, physical.
- [server] Creatures have varying sensitivity/resistance per damage type — matching damage type to target weakness increases kill efficiency (qualitative, no numbers given here).
- [server] Sorcerer specialty: energy + fire, plus the strongest death-type rune ("Sudden Death").
- [server] Druid specialty: ice + earth.
- [server] Paladin: sole vocation with holy-type spells.
- [server] Knight/Monk: mainly physical, but weapon choice can add fire/ice/energy/earth damage.

## 5.3.3 Skills and Equipment

- [server] Attack-value skill used = whichever skill matches the currently wielded weapon; unarmed or using a weapon restricted to another vocation both fall back to Fist Fighting.
- [server] Armor (legs, helmet, etc.) reduces incoming hit damage; shield bonus feeds into block chance calculation.
- [server] Some clothing/amulets give damage-type-specific reduction.
- [server] Monk-only "mantra" gear slot: functions like armor but reduces direct elemental damage across Fire/Earth/Ice/Energy/Holy/Death/Life-Drain/Mana-Drain; mantra reduction stacks additively like armor; does NOT reduce damage-over-time (condition) ticks; reduction is applied after other resistances.
- [server] One-handed weapons may carry a defence modifier that changes total defence value when a shield is also equipped in the offhand.
- [server] Attack value drops if the wielded weapon requires more skill/experience than the character currently has ("too inexperienced").
- [server] Weapons can be enchanted with magic stones (crafted/sold by sorcerers and druids) to add an elemental damage type to that weapon.
- [server] Monk "Elemental Bond" weapon attribute: sets which element (Physical/Earth/Energy) the monk's offensive spells deal when wielding that weapon; without such a weapon equipped, monk offensive spells deal only half damage, as pure Physical. Standard (non-spell) attacks are unaffected by Elemental Bond either way.

## 5.3.4 Weapon Proficiency

- [client] Accessed via weapon context menu or a customizable status-bar button; shows per-weapon proficiency tree, level progress, Mastery progress, and chosen perks.
- [server] Each weapon's proficiency tree has 1–7 levels; each level-up offers 1–3 selectable perks, but only 1 perk per level may be active at a time.
- [server] An unfilled perk slot (never chosen) can be freely picked from anywhere; once chosen, changing/removing it requires standing in a protection zone.
- **Modifying Perks**
- [server] Perk modification (only in a PZ) rerolls the current perk to a new random one, costs "dust".
- [server] Max 2 perks modifiable per weapon: 1st unlocks at proficiency level 3, 2nd unlocks at Mastery.
- [server] A modified perk has a rank (1–10); rank increases by spending dust one step at a time, or instantly maxed via a "Lunar Ascension Orb".
- [server] Reshaping a modified perk offers 3 new perk options at the same current rank; player may decline and keep the existing one.
- [server] Clearing a modified perk restores it to unmodified state, allowing a different modification choice later.
- **Proficiency Progress**
- [server] Progress earned by defeating monsters while that weapon is equipped; amount follows Bestiary-style rules (all damage-contributing players get credit).
- [server] Per-kill progress range: 1 point (trivial monster) up to 175 points (very hard monster); bosses grant up to 1,000 points, scaled by boss tier.
- [server] Progress is per-character and non-transferable.
- [server] Level thresholds scale steeply: level 1 needs 1,750 progress; level 7 needs 20,000,000 progress.
- [server] Two catalyst items (Proficiency Catalyst, Greater Proficiency Catalyst) accelerate progress gain.
- [server] After max proficiency level, further progress goes toward "Mastery", requiring progress equal to 2 additional levels' worth beyond the weapon's top level; Mastery grants no gameplay bonus, only cosmetic titles/achievements.

## 5.3.5 Critical Hits, Life Leech and Mana Leech

- [server] 3 chargeable equipment effects: Critical Hit (chance of bonus extra damage on a hit), Life Leech (chance to recover HP = a % of damage dealt), Mana Leech (chance to recover mana = a % of damage dealt).
- [server] Charges last only a limited duration; each effect type shows via a distinct item colour while charged.
- [server] Per effect, 3 independently tunable parameters exist: duration, trigger chance, intensity (magnitude).
- [server] These leech/crit effects also apply to damage dealt via spells and runes, not just melee/ranged attacks.

## 5.3.6 Distance Fighting

- [server] Ranged attacks fire/throw at a regular interval, same cadence model as melee (5.3.1).
- [server] Ammo/weapon use requires meeting a minimum character level; below it, the weapon can't be used.
- [server] Hit chance scales with distance to target: farther = lower hit chance (inverse relationship, no formula given).
- [server] Point-blank penalty: firing at an adjacent target also incurs a to-hit penalty (distance fighters are NOT best at melee range).
- [server] Bows/crossbows are two-handed, so unusable together with a shield — poor melee fallback.
- [server] Recommended for melee emergencies: switch to a one-handed throwing weapon + shield instead of bow/crossbow.
- [server] Thrown weapons can be consumed/vanish on use (not always retrievable).

## 5.3.7 Protection Zones

- [server] On non-optional-PvP worlds, PvP becomes legal once both characters are level ≥8, except inside protection zones (PZs), which block PvP everywhere.
- [client] PZ presence is shown by an icon in the condition indicator field.
- [server] Inside a PZ: character cannot be attacked from outside, and cannot perform any offensive action themselves.
- [server] HP/mana regeneration is disabled in PZs (food is still consumed normally, i.e. hunger still decreases).
- [server] Movement exception: in PZs, characters CAN walk onto fields already occupied by another character (except a few special field types) — this exception applies on all game worlds.
- [server] Typical PZ locations: temples, depots, ships, private houses; Ankrahmun pyramids are PZs that a character under a protection-zone-block cannot enter.
- [server] No protection-zone-block is applied for attacking a party member, or for retaliating after being attacked first.
- [server] Special PZ case — swimming/water areas: entering water clears all special conditions; "Invisible", "Creature Illusion", "Chameleon" spells and stealth rings are all disabled in water; safe-trade option is unusable in water.
- [server] Characters can freely enter/stay in PZs indefinitely as long as no protection-zone-block is active; can be pushed around inside a PZ by other players, but cannot be pushed OUT of a PZ.

## 5.3.8 Combat Strategy Guide

- [both] Know Your Enemy: study creature strengths/weaknesses via bestiary/library before engaging unfamiliar monsters (design/UX guidance, not a hard rule).
- [both] Know Your Surroundings: keep situational awareness of terrain/escape routes.
- [both] Cooperate with Other Players: grouped/coordinated players are disproportionately stronger than solo play; players can rope each other through holes, carry gear for each other, and recover a dead ally's dropped loot.
- [server] Avoid Getting Cornered: hard mechanical cap — max 2 simultaneous blockable attackers regardless of Shielding skill (reaffirms 5.1.5/5.3.1); pushing an opponent away is possible but not guaranteed to work on all creature types, and the push effect is temporary (opponent will reposition).
- [both] Plan Your Attacks: focus-fire one target at a time before engaging the next; a "heavily wounded" enemy still fights at full offensive capacity (no damage-based accuracy/power falloff implied).
- [both] Have a Backup Plan Ready: keep healing resources on hand; retreat is a valid tactic ("living to fight another day").

## 5.3.9 Prey Dialog

- [server] Prey requires having left the starter islands (Rookgaard or Newhaven) to activate.
- [client] "Prey Creatures" tab shows 9 candidate creatures per slot to choose from; can reroll the list.
- [server] Free list reroll: 1 per 20 hours; extra rerolls purchasable with gold (price scales with character level).
- [server] Spending 5 Prey Wildcards lets the player freely pick any eligible creature as Prey (bypassing the 9-option roll).
- [server] Prey Wildcards (Store, Tibia Coins) can: reroll the active prey's bonus, lock the active prey, or free-select a prey creature.
- [server] Prey bonus types: damage boost, damage reduction, bonus XP, improved loot — randomly assigned; rerolling a bonus via Wildcard guarantees a value ≥ current (never worse), unless already at max.
- [server] Prey duration: 2 hours of hunting time, only ticking down while actively hunting (not real-time); a fresh list roll or bonus roll resets the timer to 2h.
- [server] Automatic Bonus Reroll (opt-in toggle): auto-rerolls bonus (and thus extends hunting time by 2h) just before expiry; consumes 1 Prey Wildcard per trigger; auto-disables when Wildcards run out.
- [server] Lock Prey (opt-in toggle): auto-extends the same prey+bonus by 2h on expiry instead of rerolling; consumes 5 Prey Wildcards per trigger; auto-disables when insufficient Wildcards remain.
- [server] Prey slots: 3 max. Slot 1 = everyone; Slot 2 = free for Premium, purchasable permanently for Free accounts via Store; Slot 3 = purchasable permanently by anyone via Store.

## 5.3.10 Task Board

**a) Bounty Tasks**
- [server] Player selects 1 of 3 offered Bounty tasks: kill N of a specific creature type; reward = XP + reroll tokens + Bounty Points.
- [server] 4 difficulty tiers: Beginner (easy only), Adept (easy+medium mix), Expert (medium+hard), Master (hard+challenging). Difficulty change takes effect only on next task selection, not the current one.
- [client] Preferred List lets players bias/filter which creature types can roll as tasks.
- [server] Reroll Tokens: 1 free per day, cap of 10 held at once; consumed to reroll current task options.
- [server] Silver/Gold Bounty tasks occasionally appear — rarer, higher reward.
- [server] Bounty Talisman: bought from any jeweller NPC for 5,000 gp; upgraded with earned Bounty Points into permanent, character-bound bonuses: damage vs. task creatures, life leech, extra loot, extra Bestiary-progress chance. Bonuses only apply while the Talisman is equipped AND fighting creatures from the character's currently active Bounty Tasks.

**b) Weekly Tasks**
- [server] Base allotment: 6 Kill Tasks + 6 Delivery Tasks per week; +3/+3 more with a "Permanent Weekly Task Expansion" (Store purchase).
- [server] Rewards granted every Monday after server save.
- [server] Reward currencies per completed task: XP + Hunting Task Points + 1 Soulseal.
- [server] Hunting Task Points per task: 25 (Kill Task), 75 (Delivery Task).
- [server] Point multiplier tiers by total weekly tasks completed: ≥4 tasks = ×2, ≥8 = ×3, ≥12 = ×5, ≥16 = ×8.
- [server] Hunting Task Points spend in the Hunting Task Shop for outfits, mounts, items, and Wheel-of-Destiny promotion points.
- [server] Soulseals usable at the Soulpit: click the obelisk then your character to open a menu to solo-challenge a chosen creature.

**c) Hunting Task Shop**
- [client] Third tab of Task Board; trades Hunting Task Points for outfits/mounts/trophies/house decorations, and can convert points into Wheel of Destiny promotion points.

## 5.3.11 Player Killing

- [server] PvP legality/restrictions vary by world type: Hardcore PvP = no restrictions; Open PvP = restrictions via skull system; Optional PvP = only via mutual guild-war consent.
- [server] Same-party/guild members cannot deal damage to each other except under the "Friendly Fire" exception (5.3.11.h).
- [server] 10-second post-login PvP immunity: cannot attack others in this window, but CAN defend if attacked first.
- [server] PvP damage penalty: attacking a non-black-skulled character deals only 50% of normal (PvE-equivalent) damage.

**a) Ok Kills vs. Unjustified Kills vs. Assisted Kills**
- [server] All kills are auto-logged/evaluated by the skull system.
- [server] Killing a marked character (white/red/black skull victims, or yellow/orange skull victims who attacked you first) never counts against the killer.
- [server] Killing unmarked characters is capped ("unjustified kills") per rolling time windows before triggering red/black skull (see 5.3.12.b).
- [server] Assisted kill: character dealt no direct damage but helped the kill land within the 5 minutes before the victim's death, via paralysis or being part of a "trap" (occupying/blocking all tiles around the victim, e.g. via standing there or a magic wall spell). Assisted kills still count toward the skull system but fill the "kill counter" more slowly than direct damage. Guild-war kills against guildmates never count as assisted.

**b) Hardcore PvP Worlds**
- [server] No PK restrictions at all; killing is explicitly encouraged.
- [server] "Cancel Invisibility" spell can destroy worn stealth rings on these worlds.
- [server] Blocking another character (can't walk through) is NOT treated as an attack here (since you can't walk through characters at all on this world type).
- [server] Secure mode is always off; expert mode locked to "red fist".
- [server] Harmful magic fields always damage anyone standing on them, including the caster.

**c) Retro Hardcore PvP Worlds**
- [server] No PK restrictions; PvP kills DO grant XP here if the victim is roughly the same level or higher (exception to the general "PvP grants no XP" rule in 5.1.1).
- [server] Same stealth-ring-destroy behavior for "Cancel Invisibility" as Hardcore.
- [server] Secure mode defaults to red-fist but is player-changeable; expert mode toggle unavailable entirely.
- [server] Harmful magic fields damage everyone standing on them (including caster) but ONLY for the first 5 seconds after the field is created.
- [server] PvP blessing "Twist of Fate" is unavailable on this world type.
- [server] Blessing loss % differs here: 6.31% per blessing (vs 8% standard) — see characters 5.1.11.

**d) Open PvP Worlds**
- [server] Kills split into "ok" (marked victim) vs "unjustified" (unmarked victim); unlimited ok kills, capped unjustified kills before sanctions (skull marks).
- [server] Kill-credit cap: a kill counts fully (for skull-point purposes) toward at most 5 damage-contributing characters (measured over the last 5 minutes before death); if more than 5 participated in an unjustified kill, each contributor's individual unjustified-point gain is reduced.

**e) Optional PvP Worlds**
- [server] PvP only possible between characters actively in a mutual guild war.
- [server] War participants cannot walk through each other or their summons.
- [server] Harmful magic fields do NOT damage their own caster on this world type (exception vs. hardcore/retro-hardcore).
- [server] Weapon skill and shielding skill do NOT progress from attacking enemies/summons during an optional-world guild war (no skill training from guild-war combat).

**f) Blocking**
- [server] On Optional/Open PvP worlds, characters normally walk through each other; certain PvP states cause blocking instead, governed by expert mode: Dove = never blocks; White Hand = blocks only characters that were aggressive toward you or your party/guild; Yellow Hand = blocks all skulled characters; Red Fist = blocks everyone.
- [server] Party/guild members never block each other regardless of expert mode.
- [server] Blocking another character always counts as an attack on them (triggers marking/skull consequences).

**g) Join Aggression**
- [server] Right-click a party/guild member in a fight → "Join Aggression": immediately adopts the same PvP relationship (aggressor status) against every character that member is fighting, without needing prior individual aggression — e.g. your AoE spell or magic wall will now hit/block all of them too.
- [server] This is treated exactly as if you had individually attacked every one of those characters yourself (full skull/marking consequences apply).
- [server] With secure mode active, Join Aggression only succeeds if ALL the targets you'd be joining against are valid/justified targets under your current expert-mode settings.

**h) Friendly Fire**
- [server] Default: same-party/guild members can't damage/block each other.
- [server] Exception: if you and a party/guild ally are both actively fighting the same enemy, you CAN then damage/block each other too (e.g. via an overlapping AoE spell).
- [server] Even after accidentally hitting only a guildmate this way, you can still enter protection zones afterward (no PZ-block penalty from friendly fire).

## 5.3.12 Player Killing Related Symbols and Unjustified Points

**a) Unjustified Points**
- [server] An unjustified kill has a "value"; contributors to bringing the victim down split a share of that value into their personal unjustified-points bar.
- [server] Direct damage dealers get the largest share; blocking, debuffing the victim, and healing the attacker also contribute (additively) to a contributor's share — so a player who both damaged and blocked gets more than one who only damaged.

**b) Skull Marks**
- [server] Skulls exist only on Open PvP worlds.
- [server] An unmarked character cannot be attacked by someone in secure mode.
- [server] Attacking/hurting an unmarked character marks the ATTACKER with a visible skull (not the victim).
- [server] Attacking/killing an already-marked character does not itself mark the attacker.
- [server] 5 skull colours:
  - White: attacker recently attacked/killed an unmarked character; persists as long as the linked logout block is active; further offensive acts while marked extend the mark, logout block, and protection-zone-block durations together.
  - Red: triggered when the unjustified-points bar fills from repeated unjustified kills/assists. Thresholds (examples given as rule-of-thumb): 3–5 unmarked-character kills/24h, 5–9/7 days, or 10–19/30 days. Lasts 30 days, resets to a fresh 30 days on any further unjustified points gained during that window. Adds a full-item-loss-on-death penalty (bypasses all blessings and Amulet of Loss).
  - Black: escalation from Red after further unjustified points fill the bar again. Thresholds (rule-of-thumb): 6+ /24h, 10+/7 days, or 20+/30 days. Lasts 45 days, resets to 45 on further unjustified points. Also full item loss on death (like red); additionally cannot attack any unmarked character at all; cannot select "Red Fist" expert mode; deals 100% (not 50%) damage in PvP fights; on death, respawns at temple with exactly 40 HP and 0 mana.
  - Yellow: private mark (visible only to you), shown when a marked character attacked/damaged you first (so you're defending yourself); killing a yellow-skulled attacker doesn't count as a bad kill; duration/extension rules mirror White skull.
  - Orange: private mark (visible only to you and the marked party), shown on characters who killed you unjustified; lasts 7 days, or until you avenge that specific kill; killing an orange-skulled character doesn't count as unjustified for you, but attacking one still marks you Yellow toward them and applies the usual protection-zone-block. If killed multiple times by the same attacker within 7 days, the orange mark persists until every one of those kills is avenged (or all age past 7 days).
- [server] Note: red/black-skulled attacker who kills someone else and reduces that victim's yellow-skull relationship — killing counts noted in manual around chain effects (a red skull turning black mid-fight prevents further attacks on previously-attacked targets without granting them a yellow skull).

**c) Logout Block**
- [client] Shown via an icon in the condition indicator (below inventory).
- [server] Triggers: attacking a character/creature, being attacked, dealing damage, taking damage (any source), or casting offensive spells/runes.
- [server] Standard logout block duration: 60 seconds, refreshed (reset to 60s) each time a triggering event occurs.
- [server] Client can still be closed / computer turned off while blocked, but the character remains in the game world and vulnerable (not a safe way to "log out").
- [server] Protection-zone block: a stricter companion to the logout block specifically from PvP combat — bars entering PZs in addition to bars on logging out; same 60s duration/refresh model, tied to "involved in violence" events.
- [server] Casting certain offensive spells (e.g. field runes) triggers a protection-zone-block even without hitting anyone; also a standard 60s duration, refreshed only if the caster then gets involved in actual combat.
- [server] Killing another character: 15-minute hard logout/PZ-entry block for the killer; if further violence occurs during the final minute of that window, the block extends again (can chain to a very long block).

**d) Summons**
- [client] Summons show a distinguishing "own summon" vs "other's summon" icon (little animal head) to help identify friend/foe in chaotic PvP.
- [server] Summons belonging to your party/guild are protected the same as their owning characters (cannot be attacked).

**e) Frames**
- [client] Coloured character/summon frames indicate PvP relationship: Yellow = your own character/summons while actively in PvP, and anyone you've attacked or who's attacked you; Orange = characters/summons who attacked a member of your guild/party; Brown = characters/summons in a PvP situation you're not part of (e.g. two unrelated guilds fighting nearby).

## 5.3.13 Party Mode

- [server] Party members auto-join a shared party chat channel; area spells won't damage party members; parties unlock shared XP and party-exclusive spells.

**a) Forming a Party**
- [server] Invite via right-click → "Invite to Party"; invitee accepts via right-click → "Join XXX's Party".
- [server] Inviter automatically becomes leader; all subsequent joiners become plain members.
- [client] Shield icon under nameplate encodes party-status role (leader / member / invited-leader / pending-invitee / other-member variants).
- [server] A character can be in only one party at a time; only the leader can invite; leader can transfer leadership to any member via "Pass Leadership".

**b) Leaving a Party**
- [server] Leave via right-click self → "Leave Party"; blocked while under a logout block.
- [server] Leaving immediately clears the shield icon.
- [server] Logging out or disconnecting auto-removes a member from the party.
- [server] If the leader leaves/disconnects, leadership auto-transfers to the earliest-invited remaining member (chain continues if that member also leaves).

**c) Shared Experience**
- [server] Toggled on by the leader via context menu.
- [server] Base shared-XP bonus: +20%, only for kills that would normally yield ≥20 XP.
- [server] Bonus scales with number of distinct vocations participating in the kill: 2 vocations = +30%, 3 vocations = +60%, 4+ vocations = +100%.
- [server] Activation requirements: leader must have no battle sign (not red/black skulled state implied); level spread requirement — lowest party member's level must be ≥ two-thirds of the highest member's level (example: level 40 can share with level 60 but not level 20; level 200 can share with level 300); distance requirement — every member within 30 tiles of the leader (including across floors up/down); activity requirement — every member must have recently either healed a party member or attacked an aggressive monster.
- [client] A status icon under the nameplate shows whether shared-XP conditions are currently satisfied (2 variants: active / a member failing conditions, shown with a flashing icon for whoever fails).
- [server] Distribution: XP from a kill splits evenly across all qualifying members; uneven splits round UP to the next integer; a member who receives less/no XP due to insufficient stamina has their share removed from the pool without affecting others' shares; members whose received XP share ≥ their own level also gain soul points from that kill (ties to characters 5.1.4 soul mechanic); if a party member has a summon, the summon takes its own XP share first, before the remaining shared-XP split is calculated among characters.
- [server] Deactivation: if any condition fails, shared XP turns off for the whole party until all members meet requirements again; if deactivated specifically because the top-level member leveled up, it stays off until the lowest-level member catches back up to the two-thirds threshold.
- [server] While deactivated, kill XP is distributed by proportional damage dealt instead of an even split.

## 5.3.14 Tibiadrome

- [server] Premium-only PvE practice zone; no XP/skill/item loss from dying there; rewards special potions. Free accounts may spectate from the roof only.
- [server] Locations: Ankrahmun, Kazordoon, Thais, Edron, Rathleton; entered via a demonhead-marked teleporter.
- [server] Group size: 1–5 players, each ≥ level 50.
- [server] Rotation cycle: new rotation every 2 weeks; players fight battles to earn drome points/potions within a rotation.
- [server] A battle = unlimited sequential monster waves (increasing difficulty) plus combat modifiers; each wave has a 120-second time limit.
- [server] No death penalties (XP/skill/items/blessings) for dying in the drome.
- [server] Mana/health potions, arrows, bolts, and runes are NOT consumed when used in the drome; other consumables (rings, amulets, food, and imbuement charge-time) ARE consumed normally.
- [server] No skill or XP gain from drome fighting; drome fights don't drain stamina.
- [server] Soul-point-costing spells are disabled in the drome.
- [server] Only one group can occupy/fight in a given drome instance at a time; no PvP possible inside.
- [server] Familiars and "Chivalrous Challenge" are usable in the drome.
- [client] Leaderboards (website + in-game, near entrance) show current-rotation standings; a separate Highscores list tracks lifetime cumulative drome levels reached.
- [server] Character trading preserves drome level/starting level, but the character is removed from the leaderboard for the duration of the auction/trade; if a rotation ends before re-listing, that rotation's points/prizes are forfeited.
- [server] Starting level determines which wave a run begins at; beating a wave increases both drome level and starting level by 1. In a group, the run starts at the lowest participant's starting level.
- [server] Rotation end: drome level resets to 0 for everyone; starting level is reduced by 5 (floor 0). Players may also voluntarily lower their starting level by 5 via an NPC (repeatable down to 0).
- [server] Run start: all participants stand on designated tiles, one pulls a lever, teleport occurs, then an 8-second prep phase before the fight starts.
- [server] Monster count by group size: 1→4, 2→8, 3→11, 4→14, 5→17.
- [server] Monster pool (drome-exclusive, 5 types, each a distinct role): Murmillion (tanky melee), Scissorion (high single-target melee damage), Hoodinion (ranged AoE + damage spikes), Mearidion (ranged, consistent single-target), Domestikion (support melee, buffs other monsters' damage).
- [server] Combat modifiers (randomly selected per round), with effects:
  - Somersault: melee monsters have 15% chance to teleport to the farthest player.
  - Going Down With Me: monster death triggers an AoE hitting the last hitter + nearby players for 40% of their CURRENT HP.
  - Exploding Corpses: monster death AoE hits nearby players for 25% of their CURRENT HP.
  - That Escalated Quickly: monsters below 25% HP get stronger, as if 5 wave-levels higher.
  - The Floor is Lava: every 15s, 100 random tiles marked; after 3s deal 60% of standing players' CURRENT HP.
  - Beam me Up!: every 15s, 100 random tiles marked; after 3s teleport standing players to a random drome position.
  - Tanked Up: every 15s, 100 random tiles marked; after 3s affected players become "super drunk" for 10s, taking +10% damage while drunk.
  - Sown Sorrow: seed spawns every 20/17/14/11/8s (scaling inversely with participant count); unstepped-on for 6s → explodes, fears all players 2s; feared players take +15% damage.
  - Bad Roots: seed spawns every 20/17/14/11/8s (same scaling); unstepped-on for 6s → explodes, roots all players 3s; rooted players take +25% damage.
- [server] Battle ends when: a wave isn't cleared within 120s, the whole team dies, or the team exits via the teleporter. If one member of a group leaves, remaining members continue.
- [server] Post-battle: players relocated to entry hall; HP/mana reset to their pre-entry values; starting level reduced by 5.
- [server] Cooldown: all participants blocked from further drome fights for 60 minutes after a battle ends.
- [server] Rotation-end rewards: drome points = drome level × 10 (e.g. level 25 → 250 points), spendable on cosmetics (outfits/mounts/decorations, mostly non-tradeable) via entry-hall NPC; eligibility requires clearing at least 1 wave during the rotation.
- [server] Top-5 finishers in a rotation get a temporary title lasting until the next rotation ends.
- [server] Potion rewards by rank: 1st–5th = 3 potions; 6th–10th = 2; 11th–20th = 1; remaining potions (equal to 20% of remaining participants) are raffled among rank 21+, weighted by relative drome level (higher level = better odds).

## Open questions for Oteryn

- No numeric hit-chance formula given for attack value vs defence value (5.3.1) — needed for server combat resolution.
- No numeric distance-to-hit-chance curve for Distance Fighting (5.3.6), nor the exact point-blank penalty magnitude.
- No numeric mantra reduction values or stacking cap (5.3.3).
- No numeric base success-chance for Weapon Proficiency perk rolls, or dust costs for rank-ups/modifications (5.3.4).
- No exact chance/intensity numeric ranges for Critical Hit / Life Leech / Mana Leech tiers (5.3.5) — only "3 tunable parameters" described.
- Red/black skull point thresholds are explicitly called "rule of thumb" ranges (e.g. 3–5, 5–9, 10–19) rather than exact fixed numbers — exact server threshold values are not given and must be sourced elsewhere or decided.
- Unjustified-points share weighting (damage vs block vs debuff vs heal) is described as additive but with no relative weight numbers (5.3.12.a).
- No cooldown durations (seconds) given anywhere for individual spells/spell groups (5.3.1 general combat rounds section defers this).
- Party shared-XP: rounding rule for split is given (round up), but no tie-break rule when the stamina-reduced member's leftover fractional share must be redistributed.
- Drome: no numeric details on wave-to-wave monster difficulty scaling within a battle, nor exact HP/damage values for drome-exclusive monsters.
