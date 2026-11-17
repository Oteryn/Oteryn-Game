# General creature objectives

`quest::creature_objectives::KillCounter` is the declaration owner. The existing
`authored_bindings::Trigger::Kill` schema consumes it; the old
`authored_bindings::kills::KillCounter` path remains a re-export. Content lanes
supply these bindings for Blood Brothers, Cults of Tibia, Dangerous Depths,
Ferumbras' Ascension, Forgotten Knowledge, No Rest for the Wicked, Rotten Blood,
Rottin Wood, Soul War, Dream Courts, First Dragon, Order of the Lion, Rookie
Guard and Wrath of the Emperor. There is no quest-name allowlist.

A declaration binds one creature type to a character counter, an active stage
and a maximum. Multiple declarations can share a counter for multiple creature
types. `advance_gate_to` uses the Quest progression owner's threshold effect;
the increment and advancement share the same locked original state and receipt.
The native lethal queue freezes eligibility before settlement. Durable Quest
receipts deduplicate death occurrences and current session-generation fences
remain mandatory at mutation.

`credit` selects `most_damage_party` (the default), `most_damage`, `last_hit`, or
`damaging_players`. Personal policies carry owner-issued death evidence binding
the native scope/generation, character, session, lease, creature and death area.
An arbitrary command or death occurrence cannot supply that evidence. Shared
party credit uses the existing frozen party-membership owner, independently of
XP amounts. Damaging-player credit reads the physical Player damage map, not
summon-master reward attribution. No minimum damage percentage is imposed.

`BoundQuest.creature_removals` binds despawn declarations to any declared
noninitial track value, including NPC and world callback stages.

`remove_on_advance` on a kill counter is a bounded list of creature keys and inclusive native
areas. It requires `advance_gate_to`. Removal reconciles from the persisted
advanced gate and capped counter during quest admission/refresh, including after
restart. Only matching live creature projections in those areas are removed;
this uses the administrative despawn owner and creates no kill, corpse or loot.
Repeated reconciliation converges without another counter increment.

## Source evidence

Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
These are OTS implementation hypotheses used for compatibility, not official
CipSoft proof.

* `data/libs/functions/functions.lua:1064-1072`: “local participants =
  Participants(player, true)” and “func(creature, participant)”.
* `data/libs/functions/party.lua:129-139`: absent party or inactive required
  shared experience returns “{ player }”; otherwise members plus leader.
* `data/libs/functions/functions.lua:1075-1083`: “for key, value in
  pairs(creature:getDamageMap())” selects existing Player objects without a
  percentage threshold.
* `data-global/scripts/quests/the_rookie_guard/mission03_rational_request.lua:7-13`:
  “onDeathForParty(creature, mostDamageKiller” gates Mission03 at 1 and increments
  RatKills below 5. The production proof binding preserves that NPC-owned mission
  stage; reaching five kills does not invent an automatic NPC conversation.

The Rookie Guard native rat binding is the sole new production quest declaration.
Other quest content remains the responsibility of its content lane.
