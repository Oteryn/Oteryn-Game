# Cyclopedia Discovery V1

MAPDISC. Optional capability 527 requires 57 `CYCLOPEDIA_MAP_V1`. Like 57 it is in no static
offer set: the Cyclopedia offer hook inserts it, after 57, only when the World has admitted
Cyclopedia content. Command 6262 `CYCLOPEDIA_DISCOVERY_QUERY` reads the admitted Character's own Cyclopedia
map discovery data and the Measuring Tibia quest progress the same discoveries feed.

Discovery itself is not a command: the accepted movement path finds points of interest when
the Character enters a subarea (`OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03`).
Each advancing discovery commits one Character Cyclopedia receipt; in that transaction the
Measuring Tibia row (migration 2874) is updated. A replayed step returns its stored receipt and
writes nothing.

## Request

Empty (0 bytes). It cannot name another Character. An unselected capability, missing
Cyclopedia content, a stale session/connection/lease or a non-empty payload is rejected with
an empty result.

## Reply

Crystal 201189f8 `protocolgame.cpp:4093-4116` (`sendCyclopediaMapDiscoveryData`) with
`discovery_system.lua:491-526`, carrying the admitted bundle's `map_area` ids as `u32`, then the
Measuring Tibia block. All integers little-endian. Exact length, at most 6157 bytes:

- `mains:u16`, then per main area that has at least one subarea, ascending id:
  `area:u32`, `status:u8` (3 when every subarea is discovered, otherwise 1),
  `progress:u8` (`floor(discovered * 100 / subareas)`, 100 exactly when status is 3);
- `discovered:u16`, then that many ascending `subarea:u32`;
- `discoverable:u16`, then that many ascending `subarea:u32`, disjoint from `discovered`;
- `discovered_subareas:u16`, `completed_areas:u16`, `main_areas:u16`, `stage:u8`.

The three lists together hold at most 1024 entries. `main_areas` counts every main area of the
bundle, including ones without subareas (Crystal `discovery_system.lua:528-538`).
`completed_areas <= main_areas`. The stage follows Crystal `grantMilestones`
(`discovery_system.lua:540-563`): 0 before the first complete main area, 1 after it, 2 at 50%
and 3 at 100% of `main_areas`. A reached stage never regresses; the counts are those of the
latest advancing discovery, or zero with the current `main_areas` before the first one.

The Discoverer outfit and addons, the achievements "Widely Travelled" and "Measuring the World"
and the quest log lines are not granted by this capability.
