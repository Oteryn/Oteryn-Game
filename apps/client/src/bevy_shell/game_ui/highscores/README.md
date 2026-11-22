# Highscores world selection

The named choices use the world IDs and display names already supplied to the
character picker. Only available character rows contribute; choices are sorted
and deduplicated by world ID. They remain through admission and are cleared when
the character scope ends. Selecting a name uses the existing capability-9
`HighscoresQuery.world` and resets pagination through the retained request owner.
Unknown worlds remain accessible by UUID; unknown result IDs display as UUIDs.
This is a partial catalogue of known worlds, not an invented global world list.

Target: `docs/reference/tibia-manual/interface.md:218` describes vocation, world,
world-type and skill filters (retained Global manual evidence). OTClient reference:
`edubart/otclient` at `dab86e610991092fe8b2471f7de55a7c986b3cd2`,
`modules/client_entergame/characterlist.lua:240` retains server-supplied world
names. That revision has no highscores module. No OTClient code was copied.

NEEDS-CONTRACT: the accepted `HighscoresQueryV1` selects one optional World UUID;
`HighscoresResultV1` echoes it, but neither supplies names or world types.
`DirectoryWorld` supplies a name but no world type, and the selector only receives
worlds represented by account characters. Full parity requires a new authoritative
`HighscoresWorldsV1` catalogue with `world_id`, `display_name` and `world_type`,
plus a `world_type` selector on `HighscoresQueryV1` with defined cross-world ranking,
pagination and own-rank semantics. No new message, capability or server data is
introduced here. The window states that world types are unavailable.
