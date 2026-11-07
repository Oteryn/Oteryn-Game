"""Generate the native Grizzly catalogue from Crystal 201189f, without executing Lua.

python3 tools/content-schema/quest-authoring/grizzly_source.py --source-root /path/to/crystal
Add --check to verify the committed fact-only catalogue. The existing Lua table reader
retains donor lines; exact blob pins reject a different source revision.
"""

import argparse
import hashlib
import json
import re
from pathlib import Path

import lua_tables

ROOT = Path(__file__).resolve().parents[3]
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
OUTPUT = ROOT / "content/quests/killing-in-the-name-of.json"
PREFIX = "oteryn:quest-progress/quest/u8_5/killing_in_the_name_of/"
STORAGE_PREFIX = "Storage.Quest.U8_5.KillingInTheNameOf."
LIBRARY = "data-global/lib/quests/killing_in_the_name_of.lua"
MOVEMENT = "data-global/scripts/quests/killing_in_the_name_of/movements_boss.lua"
STORAGES = "data-global/lib/core/storages.lua"
NPC = "data-global/npc/grizzly_adams.lua"
BLOBS = {
    LIBRARY: "1e051f206687ccc2904de52d6c0348a8f3bf2b1e",
    MOVEMENT: "4220280abdca9db1c6d5b12113dcdcf9756de764",
    STORAGES: "1a9a7e931eb984a75b285be01df6b1dba36476d8",
    NPC: "5dacc4024d2574dd0bfa5ea69e88a0a6cd8ccc58",
    "data-global/scripts/quests/killing_in_the_name_of/creaturescripts_monster_kill.lua":
        "c740ef1297187fe6ec06f3419eafc4ac9605389e",
    "data-global/scripts/quests/killing_in_the_name_of/creaturescripts_boss_kill.lua":
        "444e23c65e62056c2ffd80cc63403470341545c0",
    "data/libs/functions/functions.lua": "6aeb1fdb7891a45b3bb344b3728ebb0268c91e52",
    "data/libs/functions/party.lua": "1c177559978c8f7d4a1cc4bf71aba86e28e28d38",
}


def canonical_storage(name):
    def snake(part):
        return re.sub(r"(?<!^)(?=[A-Z])", "_", part).lower()
    return PREFIX + "/".join(snake(part) for part in name.split("."))


def lower_positions(text):
    return re.sub(
        r"Position\((\d+),\s*(\d+),\s*(\d+)\)",
        r"{ x = \1, y = \2, z = \3 }",
        text,
    )


def storage_number(value, storages):
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and value.get("expr") in storages:
        return storages[value["expr"]]
    raise ValueError(f"unknown storage: {value}")


def rewards(entries, storages):
    result = {"experience": None, "points": None, "item": None,
              "completion_storage": None, "achievement": None, "boss_storage": None}
    for entry in entries:
        kind, values = entry["type"], entry["value"]
        if kind == "exp":
            result["experience"] = values[0]
        elif kind == "points":
            result["points"] = values[0]
        elif kind == "item":
            result["item"] = {"source_id": values[0], "count": values[1]}
        elif kind == "achievement":
            result["achievement"] = values[0]
        elif kind == "storage" and len(values) == 2:
            result["completion_storage"] = {"storage": storage_number(values[0], storages),
                                             "value": values[1]}
        elif kind == "storage" and len(values) == 4:
            if values[2:] != [1, 0]:
                raise ValueError("unsupported boss admission values")
            result["boss_storage"] = [storage_number(v, storages) for v in values[:2]]
        else:
            raise ValueError(f"unsupported reward: {entry}")
    return result


def flattened(values, prefix=""):
    for key, value in values.items():
        name = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            yield from flattened(value, name)
        elif isinstance(value, int):
            yield name, value


def creatures():
    result = {}
    for path in sorted((ROOT / "content/creatures/definitions").glob("creatures-*.json")):
        for record in json.loads(path.read_text())["records"]:
            name = record["authoring"]["profile"]["details"]["display_name"].lower()
            key = record["definition"]["identity"]["key"]
            result.setdefault(name, set()).add(key)
    return result


def creature_key(name, keys):
    # Names such as Giant Spider also have explicit raid/nostalgia variants. Use
    # the exact ordinary identity when present, never the first duplicate record.
    expected = "oteryn:creature." + re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")
    if expected not in keys.get(name, set()):
        raise ValueError(f"unknown ordinary native creature: {name}")
    return expected


def read_sources(source_root):
    sources, evidence = {}, []
    for path, expected in BLOBS.items():
        content = (source_root / path).read_bytes()
        blob = hashlib.sha1(b"blob " + str(len(content)).encode() + b"\0" + content).hexdigest()
        if blob != expected:
            raise ValueError(f"{path}: source blob differs from pinned Crystal {REVISION}")
        sources[path] = content.decode("utf-8")
        evidence.append({"path": path, "git_blob": blob,
                         "sha256": hashlib.sha256(content).hexdigest()})
    return sources, evidence


def shop_offers(text):
    """Source prices/rank thresholds; canonical Item identity repairs the eye typo.

    Crystal npc/grizzly_adams.lua:55-56 repeats 11491 for bone and eye. The accepted
    repository Item definitions identify eye as 11488 and bone as 11491. Keep each
    source name's price, as directed by PR2770's repository-over-donor ruling.
    """
    table = lua_tables.assignments(text, {"shop"})["shop"]
    offers = []
    for field in table["fields"]:
        row = lua_tables.as_python(field["value"])
        if row.get("storageKey") != {"expr": "POINTSSTORAGE"}:
            raise ValueError("unknown Grizzly shop gate")
        if row.get("storageValue") not in (10, 20, 40, 70):
            raise ValueError("unknown Grizzly shop rank")
        if ("buy" in row) == ("sell" in row):
            raise ValueError("ambiguous Grizzly shop direction")
        item_id = 11488 if row["itemName"] == "quara eye" and row["clientId"] == 11491 else row["clientId"]
        offers.append({
            "source_id": row["clientId"], "source_name": row["itemName"],
            "item": {"family": "Item", "key": f"oteryn:item.tibia.i{item_id}", "revision": "definition-r1"},
            "direction": "SellToPlayer" if "buy" in row else "BuyFromPlayer",
            "unit_price": row.get("buy", row.get("sell")),
            "minimum_points": row["storageValue"], "donor_line": field["line"],
        })
    keys = [(offer["item"]["key"], offer["direction"]) for offer in offers]
    if len(keys) != len(set(keys)):
        raise ValueError("duplicate Grizzly shop Item identity")
    return offers


def generate(source_root):
    sources, evidence = read_sources(source_root)
    storage_table = lua_tables.as_python(lua_tables.assignments(sources[STORAGES], {"Storage"})["Storage"])
    own_storages = storage_table["Quest"]["U8_5"]["KillingInTheNameOf"]
    storage_lookup = {STORAGE_PREFIX + name: value for name, value in flattened(own_storages)}
    names = {value: canonical_storage(name) for name, value in flattened(own_storages)}
    all_tasks = lua_tables.assignments(sources[LIBRARY], {"tasks"})["tasks"]
    raw_tasks = next(field["value"] for field in all_tasks["fields"] if field["key"] == "GrizzlyAdams")
    creature_keys = creatures()
    tasks, bosses_by_id = [], {}
    ids = {1500, 2500, 41300, 42167, 42168, 42169, 42170, 42171, 42179, 100157}
    alt_first_names = {
        "Apes": "AltKillCount.KongraCount",
        "Quara Scouts": "AltKillCount.QuaraConstrictorScoutCount",
        "Underwater Quara": "AltKillCount.QuaraConstrictorCount",
        "Nightmares": "AltKillCount.NightmareCount",
        "High Class Lizards": "AltKillCount.LizardChosenCount",
        "Sea Serpents": "AltKillCount.SeaSerpentCount",
        "Drakens": "AltKillCount.DrakenAbominationCount",
    }
    for field in raw_tasks["fields"]:
        number = field["key"]
        task = lua_tables.as_python(field["value"])
        reward = rewards(task["rewards"], storage_lookup)
        alt = alt_first_names.get(task["raceName"])
        alt_start = storage_lookup[STORAGE_PREFIX + alt] if alt else None
        members = []
        for index, name in enumerate(task["creatures"]):
            key = creature_key(name, creature_keys)
            alt_storage = alt_start + index if alt_start is not None else None
            if alt_storage is not None:
                ids.add(alt_storage)
            members.append({"name": name, "key": key, "alt_storage": alt_storage})
        ids.update([1500 + number, 65000 + number, 48950 + number, 42179 + number])
        if reward["boss_storage"]:
            ids.update(reward["boss_storage"])
            bosses_by_id[task["bossId"]] = {
                "task_id": number, "name": task["bossName"],
                "achievement": reward["achievement"], "kill_storage": reward["boss_storage"][1],
            }
        if task.get("rank") not in (None, {"expr": "RANK_ELITEHUNTER"}):
            raise ValueError("unsupported task rank")
        tasks.append({
            "id": number, "name": task["raceName"], "required": task["killsRequired"],
            "minimum_level": task["level"][0], "maximum_level": task["level"][1],
            "premium": task.get("premium", False), "minimum_rank": 6 if task.get("rank") else 0,
            "repeat_limit": 1 if task.get("norepeatable") else None if task.get("repeatable") else 3,
            "boss_id": task.get("bossId"), "creatures": members,
            "experience": reward["experience"], "points": reward["points"],
            "item": reward["item"], "completion_storage": reward["completion_storage"],
            "donor_line": field["line"],
        })
    movement = lua_tables.assignments(lower_positions(sources[MOVEMENT]), {"bosses"})["bosses"]
    bosses = []
    for number, field in enumerate(movement["fields"][:30], 1):
        entry = lua_tables.as_python(field["value"])
        storage = storage_number(entry["storage"], storage_lookup)
        task_fields = bosses_by_id.get(number, {
            "task_id": None, "name": entry["bossName"], "achievement": None,
            "kill_storage": storage_lookup[STORAGE_PREFIX + (
                "BossKillCount.TiquandasCount" if number == 29 else "BossKillCount.DemodrasCount")],
        })
        name = entry["bossName"]
        key = creature_key(name, creature_keys)
        ids.update([storage, task_fields["kill_storage"]])
        bosses.append({
            "id": number, **task_fields, "creature_name": name, "creature_key": key,
            "storage": storage, "portal": entry["teleportPosition"],
            "player_position": entry["playerPosition"], "boss_position": entry["bossPosition"],
            "center": entry["centerPosition"], "range_x": entry["rangeX"], "range_y": entry["rangeY"],
            "donor_line": field["line"],
        })
    if [task["id"] for task in tasks] != list(range(1, 43)) or len(bosses) != 30:
        raise ValueError("pinned Grizzly task/boss coverage differs")
    # Derived facts only; source narrative and Lua callbacks are not redistributed.
    return {
        "schema": "OTERYN_GRIZZLY_ADAMS/v1", "classification": "OTS_HYPOTHESIS_ONLY",
        "source_repository": "zimbadev/crystalserver", "source_revision": REVISION,
        "sources": evidence, "tasks": tasks, "bosses": bosses, "shop": shop_offers(sources[NPC]),
        "storages": [{"id": number, "key": names.get(number, f"oteryn:quest-progress/storage/{number}")}
                     for number in sorted(ids)],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(generate(args.source_root), indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != rendered:
            raise SystemExit("Grizzly catalogue differs; run the generator without --check")
        print("GRIZZLY_CATALOGUE_CHECK_OK")
    else:
        OUTPUT.write_text(rendered)
        print(f"generated {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
