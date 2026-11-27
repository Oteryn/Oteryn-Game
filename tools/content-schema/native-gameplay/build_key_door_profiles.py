#!/usr/bin/env python3
"""Source-derived profiles for the M06/M01c key-door and level-door playable path.

Crystal 201189f data/libs/tables/doors.lua:4,9,265 supplies the identities.
Its startup/tables/door_level.lua:66 binds 6264 to Noble Armor's level-35 gate.
Panpipe's captured key-door gate at (32643,32128,8) also needs locked1631;
closed1632/open1633 are already supplied by the plain door provider.
Ghoul Room's key 3600 gate at (32506,32175,14) needs locked 5106;
the same provider already supplies closed 5107 and open 5108.
Desert Dungeon uses the existing5102/5103 pair with action1020 at32673,32100,8
(Crystal startup/tables/door_level.lua:9-15); reuse questpc4/5 profile closure.
Crystal 201189f data/libs/tables/doors.lua:4,9,265,273 supplies the identities.
Mintwallin's source Key 3667 gate needs locked 5106 alongside 5107/5108.
The Vampire Shield level70 placements are startup/tables/door_level.lua:152-158.
Crystal startup/tables/door_level.lua:116-129 binds the restored level-50 gates.
The classic quest captures add locked5106 and locked6251 from door_key.lua,
and Drefia's 9565/9566 level pair from door_level.lua:131-149.
Outlaw Camp key 3304 uses locked6251 from Crystal doors.lua:34;
closed6252 and open6253 are already supplied by the plain door provider.
Dwacatra and its key route retain locked5106, locked7711 and locked7714;
their closed and open counterparts are already supplied.
Alawar's lock uses 7033/7034/7035 (Crystal doors.lua:39 and door_key.lua:163-166).
The existing pinned importer supplies physics and classifications without overrides.
"""

import argparse
import json
from pathlib import Path

from build_tool_item_profiles import build as source_profiles

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = "tools/content-schema/native-gameplay/key-door-item-profiles.json"
IDS = {
    1628,
    1631,
    1646,
    1647,
    1648,
    1649,
    1678,
    1679,
    1680,
    1681,
    2967,
    2968,
    5097,
    5102,
    5103,
    5106,
    5291,
    5292,
    6248,
    6251,
    6262,
    6263,
    6264,
    6265,
    7711,
    7714,
    9565,
    9566,
    7033,
    7034,
    7035,
}


def build(directory):
    document = source_profiles(directory, IDS)
    document["schema"] = "OTERYN_NATIVE_KEY_DOOR_ITEM_PROFILES/v1"
    document["producer"] = (
        "tools/content-schema/native-gameplay/build_key_door_profiles.py"
    )
    return document


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source_directory", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (
        json.dumps(build(args.source_directory), indent=2, ensure_ascii=False) + "\n"
    ).encode()
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != raw:
            raise SystemExit("KEY_DOOR_ITEM_PROFILE_DRIFT")
    else:
        (ROOT / OUTPUT).write_bytes(raw)
