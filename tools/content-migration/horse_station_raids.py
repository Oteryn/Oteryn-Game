"""WCONTRIB native horse projections from the retained, hash-verified donor.

The owner authorizes the three-hour/zero-horse phase rule in WORLDCHG-Q.md.
Source geometry and population are retained independently of that authored rule.
"""
from __future__ import annotations

import copy
import hashlib
import json
import re
import tarfile

REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SPAWN = "data-global/scripts/globalevents/spawn/horse_station_world_change.lua"
NPC = "data-global/npc/palomino.lua"
SOURCES = {
    SPAWN: "631b28ccc0120d2f8d9d74c1f9540b40989065b6785d7d4331f495389c92e07b",
    NPC: "595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da",
}


def retained(root):
    archive = root / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    with tarfile.open(archive, "r:gz") as tar:
        manifest = json.load(tar.extractfile("corpus-manifest.json"))
        result = {}
        for path, digest in SOURCES.items():
            rows = [row for row in manifest["files"]
                    if row["source"] == "crystalserver" and row["revision"] == REVISION
                    and row["path"] == path]
            if len(rows) != 1:
                raise ValueError("HORSE_SOURCE_IDENTITY " + path)
            raw = tar.extractfile(rows[0]["cache_path"]).read()
            if hashlib.sha256(raw).hexdigest() != digest or rows[0]["sha256"] != digest:
                raise ValueError("HORSE_SOURCE_SHA256 " + path)
            result[path] = raw.decode()
        return result


def citation(path, line):
    return {"repository": "zimbadev/crystalserver", "revision": REVISION,
            "path": path, "line": line, "sha256": SOURCES[path], "evidence": "OTS_HYPOTHESIS_ONLY"}


def records(root, creatures, placements, qualify):
    sources = retained(root)
    stable_text, npc_text = sources[SPAWN], sources[NPC]
    cohorts = [(match[1], int(match[2]), stable_text.count("\n", 0, match.start()) + 1)
               for match in re.finditer(r'name = "([^"]+)", amount = (\d+)', stable_text)]
    stable_positions = [tuple(map(int, values)) for values in
                        re.findall(r"Position\((\d+), (\d+), (\d+)\)", stable_text)]
    area_text = re.search(r"local raidAreas = \{(.*?)\n\}", npc_text, re.S)
    names_text = re.search(r"local raidHorseNames = \{(.*?)\n\t\}", npc_text, re.S)
    if not area_text or not names_text:
        raise ValueError("HORSE_SOURCE_AREAS")
    wild_positions = [tuple(map(int, values)) for values in
                      re.findall(r"Position\((\d+), (\d+), (\d+)\)", area_text[1])]
    names = re.findall(r'"([^"]+)"', names_text[1])
    repeat = re.search(r"for _ = 1, (\d+) do", npc_text)
    if (len(cohorts) != 6 or [n for _, n, _ in cohorts] != [3, 1, 2, 1, 1, 1]
            or len(stable_positions) != 4 or len(wild_positions) != 4 or len(names) != 4
            or not repeat or repeat[1] != "3"):
        raise ValueError("HORSE_SOURCE_POPULATION")

    def entry(name, count, low, high, source):
        area = {"from": dict(zip(("x", "y", "floor"), low)),
                "to": dict(zip(("x", "y", "floor"), high))}
        return {"creature": "oteryn:creature." + name.lower().replace(" ", "_"),
                "source_name": name, "amount": {"min": count, "max": count},
                "area_anchor": "raid-anchor:" + "-".join("_".join(map(str, p)) for p in (low, high)),
                "area": area, "boss": False, "boss_source": None,
                "spawn_mode": "area", "source": source}

    def record(key, entries, refs, end, assumptions):
        record = {"key": key, "name": key.removeprefix("raid:"), "wiki_area": "Horse stations",
                  "boss_names": [], "wiki_records": [], "wiki_sources": [], "global_observations": [],
                  "sources": refs, "schedule": {"kind": "world_change"}, "variant_group": None,
                  "variants": [{"key": key + "/source", "announcements": [], "waves": [
                      {"delay_ms": 1000, "raw_delay_ms": 0, "entries": entries}],
                      "sources": refs, "unresolved": []}],
                  "end": end, "inactive_reasons": [], "assumptions": assumptions}
        qualify(record, creatures, placements)
        return record

    stable = [entry(name, count, *stable_positions[0 if index < 3 else 2:2 if index < 3 else 4],
                    citation(SPAWN, line)) for index, (name, count, line) in enumerate(cohorts)]
    result = []
    for count in range(1, 10):
        remaining, entries = count, []
        for original in stable:
            retained_count = min(remaining, original["amount"]["max"])
            if retained_count:
                current = copy.deepcopy(original)
                current["amount"] = {"min": retained_count, "max": retained_count}
                entries.append(current)
                remaining -= retained_count
        result.append(record("raid:horsestation.stable." + str(count), entries,
                             [citation(SPAWN, 1), citation(SPAWN, 18), citation(SPAWN, 39)],
                             {"kind": "until_reset"},
                             ["Owner-authorized escape reports retain the first N sourced stable horses; no new creature or position is invented."]))
    wild = [entry(name, int(repeat[1]), *wild_positions[index:index + 2], citation(NPC, 138))
            for index in (0, 2) for name in names]
    result.append(record("raid:horsestation.wild", wild,
                         [citation(NPC, 57), citation(NPC, 126), citation(NPC, 138)],
                         {"kind": "despawn_after", "despawn_after_ms": 10_800_000},
                         ["Owner-authorized zero horses or three hours from first escape activates a three-hour raid; replaces donor rent30%/24h policy."]))
    return result
