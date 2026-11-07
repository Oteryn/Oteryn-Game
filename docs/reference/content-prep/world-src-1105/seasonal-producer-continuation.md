# WORLD-SRC seasonal producer completeness continuation

Fresh source-only follow-up to `seasonal-continuation.md`; no donor Lua VM,
Cargo build or production execution is claimed. Facts are `OTS_HYPOTHESIS_ONLY`.
The companion `seasonal-producer-continuation-sources.json` records 19 fresh
HTTP-200 raw fetches, exact URLs, SHA-256 and matching pinned Git blob SHA-1s.
Pins remain Crystal `201189f871d23597b82caf88fd611454a2ddefb8` and Canary
`47dfd51f45280a59a1d3e50ba7edd573d7234446`. The normal research User-Agent was
`OterynCompatibilityResearch/1.0`; no new wiki/HTML/CipSoft route was used.

## Base monster-spawn census

Every `/world/` XML ending `-monster.xml` in both complete pinned trees was
fetched and searched: six Crystal and six Canary files. This includes primary
`data-global/world/world-monster.xml` and
`data-otservbr-global/world/otservbr-monster.xml`, alternate packs, custom maps,
Winterlight island and both misguided-quest maps. None has a line matching
case-insensitive `halloween`, `mutated pumpkin`, `grynch`, `cake golem` or `santa`.
The JSON enumerates every path and empty hit list. No separate base-monster
producer for these actors was recovered. This bounded XML census does not assert
anything about arbitrary operator additions or uninspected binary attributes.

## Actual event-calendar consumer

Crystal `src/game/scheduling/events_scheduler.cpp:96–102` and Canary
`src/game/scheduling/events_scheduler.cpp:163–173` load
`data/json/eventscheduler/events.json`. Both freshly fetched files have the same
ten entries, named at lines 4,22,40,58,76,94,112,130,148,166: Forge Time,
Double Bestiary, Fast Exercise, 50% Loot Bonus, 50% Exp Bonus, Boss Cooldown
Reduction, Double Exp, Double Loot, Double Bosstiary and Fast Respawn. Their
dates are in 2024; neither supplies Christmas, Halloween or anniversary windows
or actor/raid activation. Crystal legacy `data/XML/events.xml:1–23` contains only
two example entries and Double Fiendish Weekend. `data/events/events.xml` in
both donors is a creature/player event-method callback registry, not a holiday
calendar. Those files cannot establish the missing holiday windows.

Separately, both primary `scripts/globalevents/others/raids_schedule.lua:21–22,43–48`
do establish the October 31 16:00 Halloween Hare request and process-local
attempt marker. The actual matching raid definition remains absent from the
previous complete XML/Lua raid-registry census. A known request time is a source
component, not executable spawn geography.

## Audit row mapping and remaining boundaries

- `OTERYN_GAME_EVENTS_GAP_AUDIT_2026-11-01.md:463`, **W05d Cake and calendar**:
  the omitted attack family is now resolved to Undefined by the donor engine
  trace in the earlier continuation. Correct HP execution still requires the
  protected native consumer; generated manifests can carry source-backed data,
  so a protected default manifest alone is not a valid blocker. Bridge item48227
  and the audit's 50 missing edible placements remain separate historical
  dependencies; this XML/calendar investigation does not resolve them.
- Audit line465, **W19 item actions**: independently placed Ruprecht and its22
  exact token exchanges are a source-backed component. Existing novelty/Christmas
  bundle actions do not supply Santa or Messenger of Santa positions, seasonal
  raid spawns or holiday availability.
- Audit line466, **Calendar / Halloween**: Hare's request date/time is known.
  Christmas/anniversary windows and matching executable holiday raid producers
  were not recovered from the actual scheduler JSON, base-monster XML or earlier
  activation/raid census. No annual Santa reset is inferred.

Cake STOP (five lines):
1. Cake's second attack is real Undefined HP damage, not Earth/Physical or an omitted attack.
2. Undefined preserves ordinary mitigation/absorbs but bypasses mana shield and armor/shield blocking as sourced in the earlier continuation.
3. The required native HP consumer remains in explicitly protected files.
4. Generated content can carry data; default-manifest protection is not the blocker.
5. Stop execution until that protected consumer can be changed; do not introduce an alternate HP writer or damage-family alias.

Halloween STOP (five lines):
1. October31 at16:00 requests Halloween Hare in both pinned scheduling scripts.
2. No matching executable raid definition was recovered in the complete inspected XML/Lua registries.
3. No Hare/Pumpkin spawn geometry/counts were recovered in any of the12 base monster XMLs.
4. The permitted wiki query/parse routes previously returned402, supplying no revision to resolve the missing event facts.
5. Stop the producer pending executable spawn/lifecycle data; do not infer positions, portal behavior or participation entitlements from creature definitions.

Christmas/calendar STOP (five lines):
1. Santa's permanent success flag and reachable normal reward groups are known; no annual reset is assumed.
2. Santa and Messenger of Santa still lack recovered actor coordinates/activation in the NPC/activation census; Ruprecht is separately placed.
3. Empty reward selection is established, but its exact donor runtime error/mint timing remains unverified.
4. No Grynch base spawn, executable seasonal raid geometry/counts/lifecycle, or Christmas/anniversary event window was recovered in the inspected registries/calendars.
5. Stop those producers pending these exact facts; Ruprecht's independently sourced token exchange does not complete them.
