#!/usr/bin/env python3
"""W05d capture and project the pinned Crystal Cake raid through the normal catalogue."""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import urllib.request
from compact_record_json import encode as encoded

ROOT = Path(__file__).resolve().parents[2]
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CAPTURE = "imports/crystalserver/seasonal-raids-201189f.json"
CONFIG = "data-global/scripts/lib/a_piece_of_cake_config.lua"
SLOTS = "data-global/scripts/quests/a_piece_of_cake/globalevents_cake_raids.lua"
BLOBS = {CONFIG: "b8291626b473b91916cdac2fe5727323c1367c6d", SLOTS: "e927d84e1eea91574a2198745da3c1474daf8353"}
SCHEMA = "OTERYN_CRYSTAL_SEASONAL_RAID_SOURCE_CAPTURE/v1"


def verify(document):
    if document.get("schema") != SCHEMA or document.get("repository") != "zimbadev/crystalserver" or document.get("revision") != REVISION:
        raise ValueError("SEASONAL_RAID_REVISION")
    result = {}
    for row in document["files"]:
        raw = row["text"].encode()
        blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        if row["path"] in result or BLOBS.get(row["path"]) != blob or row["git_blob"] != blob or row["sha256"] != hashlib.sha256(raw).hexdigest():
            raise ValueError("SEASONAL_RAID_SOURCE_HASH")
        result[row["path"]] = row
    if set(result) != set(BLOBS):
        raise ValueError("SEASONAL_RAID_SOURCE_SET")
    return result


def capture(root):
    files = []
    for path, blob in BLOBS.items():
        raw = urllib.request.urlopen(f"https://raw.githubusercontent.com/zimbadev/crystalserver/{REVISION}/{path}", timeout=30).read()
        files.append({"path": path, "text": raw.decode(), "git_blob": blob, "sha256": hashlib.sha256(raw).hexdigest()})
    document = {"schema": SCHEMA, "repository": "zimbadev/crystalserver", "revision": REVISION, "files": files}
    verify(document)
    target = root / CAPTURE
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(encoded(document))


def records(root, qualify, bind_area, citation, creatures, placements):
    sources = verify(json.loads((root / CAPTURE).read_bytes()))
    config = sources[CONFIG]["text"]
    def number(name):
        match = re.search(rf"\b{name}\s*=\s*(\d+)", config)
        if not match:
            raise ValueError("SEASONAL_RAID_NUMBER:" + name)
        return int(match[1])
    # Crystal config:105-107 and globalevents_cake_raids.lua:9-29: the
    # Berlin06:00 save produces absolute HH:04 slots, never elapsed retries.
    if (number("RAID_INTERVAL_MINUTES"), number("RAID_OFFSET_MINUTES"), number("GOLEMS_PER_RAID")) != (60, 4, 20):
        raise ValueError("SEASONAL_RAID_CLOCK_RULE")
    body = config.split("Cities = {", 1)[1].split("-- Bakers NPC", 1)[0]
    cities = re.findall(r'(Carlin|Thais|Edron|Darashia|\["Liberty Bay"\])\s*=\s*\{\s*\{ x = (\d+), y = (\d+), z = (\d+) \},\s*\{ x = (\d+), y = (\d+), z = (\d+) \}', body)
    if len(cities) != 5:
        raise ValueError("SEASONAL_RAID_CITY_POSITIONS")
    entries = []
    for city, *values in cities:
        choices = []
        for offset in (0, 3):
            x, y, z = map(int, values[offset:offset+3])
            area = {"from": {"x": x-3, "y": y-3, "floor": z}, "to": {"x": x+3, "y": y+3, "floor": z}}
            if not bind_area(area, placements):
                raise ValueError("SEASONAL_RAID_AREA_UNBOUND:" + city)
            choices.append(area)
        for ordinal in range(number("GOLEMS_PER_RAID")):
            entries.append({"creature": "oteryn:creature.cake_golem", "source_name": "Cake Golem", "amount": {"min": 1, "max": 1},
                "area_anchor": f"oteryn:raid-area.piece_of_cake.{re.sub('[^a-z]+', '_', city.lower()).strip('_')}.{ordinal}",
                "area": copy.deepcopy(choices[0]), "area_choices": copy.deepcopy(choices), "boss": False, "spawn_mode": "area",
                "source": citation(sources[CONFIG], 394)})
    # Crystal config:121-124 and isWithinEventWindow:547-553 retain the inclusive source window.
    record = {"key": "oteryn:raid.piece_of_cake", "name": "A Piece of Cake", "kind": "calendar", "wiki_area": "Carlin, Thais, Edron, Darashia, Liberty Bay", "boss_names": [],
        "wiki_records": [], "wiki_sources": [], "sources": [citation(sources[CONFIG], 205), citation(sources[SLOTS], 9)],
        "schedule": {"kind": "calendar", "window": {"start_month": number("EVENT_START_MONTH"), "start_day": number("EVENT_START_DAY"), "end_month": number("EVENT_END_MONTH"), "end_day": number("EVENT_END_DAY")}, "interval_ms": 3600000, "repeat": True,
            "daily_slot_minutes": list(range(4, 1440, 60)), "source": citation(sources[SLOTS], 9)},
        "variants": [{"key": "crystal:piece_of_cake", "announcements": [], "waves": [{"delay_ms": 0, "entries": entries}], "unresolved": []}],
        "variant_group": None, "end": {"kind": "until_reset", "despawn_after_ms": None}, "inactive_reasons": []}
    qualify(record, creatures, placements)
    return [record]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["capture"])
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    capture(args.root)
    print("seasonal raid source capture verified")

if __name__ == "__main__":
    main()
