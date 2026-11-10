# M38 Yselda contact anchor

The existing NPC declaration is `content/npcs/definitions/npcs-01000-01281.json`,
identity `oteryn:npc.yselda/definition-r1` (record257 at commit68d9ff751). Its exact BR page binding is52101.
The retained capture is
`imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json`, page Yselda,
revision353837, source SHA256
`da53912b608e44dcbfc6839c2f2fd12b5dbda6b3ea5ce70e32712bb4fb7e5eb3`:
<https://www.tibiawiki.com.br/wiki/Yselda?oldid=353837>.
It supplies the reference coordinate32619,32488,12 and no NPC speech.

The authoring adapter resolves an explicit native-contact opt-in against that
exact page binding and the existing target identity. It refuses a foreign target,
wrong page/name, duplicate/missing page, invalid revision/hash or coordinates.
Existing embedded metadata remains authoritative when present; malformed embedded
metadata cannot fall back to this capture. North remains the documented project
default because the capture supplies no direction. No new dialogue, quest stage,
walking profile or retail-placement equivalence is asserted.

The actual retained map holds this anchor as `NotWalkable`. The explicit
`non_walkable_contact` flag changes only NPC anchor admission. The NPC has no
permission to move the player through that ground; the original movement facts
remain unchanged. NoTile, NoGround, unclassified terrain, floor changes,
teleports, walls, solid objects, spawns and shared cells retain their ordinary
holds. The real-map test removes this one opt-in and requires NotWalkable again.
No geometry, palette, source asset, SQL schema or Node boot code changes.
