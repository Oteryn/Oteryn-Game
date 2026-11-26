"""Preserve published trade facts and source names when composing native Item admissions."""

import argparse
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

from authored_static_item_constructors import definitions
from grizzly_shop_profiles import accepted_first

ROOT = Path(__file__).resolve().parents[2]
PATH = "tools/content-schema/native-gameplay/item-consumer-facts.json"
MANIFEST = "content/spells.manifest.json"
SECTIONS = ("item_profiles", "starting_loadout_item_profiles", "item_profile_updates")
PREDECESSOR = "5d0b4f33c3f3a83bd312944ab220b20a00694073"
TRAIN = "1d9d66feb25e21754c308cd97afc4f2dd3e42342"
CRYSTAL = "201189f871d23597b82caf88fd611454a2ddefb8"
XML_SHA = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
DOORS_SHA = "6b22b5c4c213ef5a3f18fab43ea61acca7be227c06eb99bbc25d1a8d3e75112c"
CRYSTAL_MARKET = "docs/agents/evidence/OTV2-20261031-item-market-crystal-parity-v1.json"
PREFIX = "oteryn:item.tibia.i"
UNKNOWN = {"state": "UNKNOWN"}


def known(value):
    return {"state": "KNOWN", "value": value}


def fact(record, group, field):
    return record.get("semantics", {}).get(group, {}).get("value", {}).get(field)


def marketable(record):
    return fact(record, "trade_restrictions", "marketable") == known(True)


def display_name(record):
    return (fact(record, "presentation", "name") or {}).get("value", "").strip()


def published_items(read):
    manifest = json.loads(read(MANIFEST))
    records = {}
    for section in SECTIONS:
        pin = manifest[section]
        raw = read("content/" + pin["path"])
        if hashlib.sha256(raw).hexdigest() != pin["sha256"]:
            raise ValueError(f"Published Item profile digest differs: {section}")
        records.update(
            (row["production_definition"]["production_key"], row)
            for row in json.loads(raw)["records"]
        )
    return records


def crystal_names(xml, doors):
    if (
        hashlib.sha256(xml).hexdigest() != XML_SHA
        or hashlib.sha256(doors).hexdigest() != DOORS_SHA
    ):
        raise ValueError("Crystal Item name source digest differs")
    names = {}
    for node in ET.fromstring(xml):
        first = int(node.get("id", node.get("fromid", "0")))
        last = int(node.get("id", node.get("toid", "0")))
        if name := node.get("name"):
            names.update(dict.fromkeys(range(first, last + 1), name))
    # Crystal registers these door states in Lua even when XML omits a name.
    # Its ordinary open/closed names also match TibiaWiki BR's Portas catalogue.
    for locked, closed, opened in re.findall(
        rb"\{ lockedDoor = (\d+), closedDoor = (\d+), openDoor = (\d+) \}", doors
    ):
        for identifier, exemplar in ((locked, 1629), (closed, 1629), (opened, 1630)):
            names.setdefault(int(identifier), names[exemplar])
    return names


def capture(root, xml, doors):
    def at(revision):
        return published_items(
            lambda path: subprocess.check_output(
                ["git", "show", f"{revision}:{path}"], cwd=root
            )
        )

    prior, train = at(PREDECESSOR), at(TRAIN)
    lost_marketability = [
        key
        for key in sorted(prior.keys() & train.keys())
        if marketable(prior[key]) and not marketable(train[key])
    ]
    market = json.loads((root / CRYSTAL_MARKET).read_bytes())
    crystal_market_ids = set(market["crystal_market_ids"])
    if market["donor"]["revision"] != CRYSTAL or any(
        int(key.removeprefix(PREFIX)) not in crystal_market_ids
        for key in lost_marketability
    ):
        raise ValueError("Published eligibility lacks Crystal market parity")
    names = crystal_names(xml, doors)
    missing_names = {}
    for key, row in sorted(train.items()):
        previous = prior.get(key, {})
        newly_admitted = row["admission"]["materializable"] and not previous.get(
            "admission", {}
        ).get("materializable")
        if not display_name(row) and (newly_admitted or display_name(previous)):
            key_id = int(key.removeprefix(PREFIX))
            if int(row["production_binding"]["external_id"]) != key_id:
                raise ValueError(f"Crystal Item name identity differs: {key}")
            missing_names[key] = names[key_id]
    return {
        "previous_release": PREDECESSOR,
        "train": TRAIN,
        "crystal_revision": CRYSTAL,
        "marketable_items": lost_marketability,
        "display_names": missing_names,
    }


def apply(root, items, loot_admissions):
    packet = json.loads((root / PATH).read_bytes())
    market_items = packet["marketable_items"]
    if (
        packet["previous_release"] != PREDECESSOR
        or packet["train"] != TRAIN
        or packet["crystal_revision"] != CRYSTAL
        or market_items != sorted(set(market_items))
        or not set(market_items) <= loot_admissions
    ):
        raise ValueError("Published Item consumer fact scope differs")

    canonical = definitions(root)
    records = {
        row["production_definition"]["production_key"]: row for row in items["records"]
    }
    for key in market_items:
        row = records[key]
        definition = canonical[key]
        if definition["identity"] != row["production_binding"][
            "target"
        ] or not marketable(definition):
            raise ValueError(
                f"Published loot trade restriction lacks canonical proof: {key}"
            )
        semantics = row["semantics"]
        semantics["trade_restrictions"] = accepted_first(
            semantics.get("trade_restrictions", UNKNOWN),
            definition["semantics"]["trade_restrictions"],
        )
        if not marketable(row):
            raise ValueError(
                f"Published loot trade restriction conflicts with native facts: {key}"
            )
    for key, name in packet["display_names"].items():
        row = records[key]
        binding = row["production_binding"]
        if (
            binding["target"] != canonical[key]["identity"]
            or binding["external_id"] != key.removeprefix(PREFIX)
            or binding["disposition"] != "EXACT"
            or not name.strip()
        ):
            raise ValueError(f"Crystal Item name lacks exact binding: {key}")
        if not display_name(row):
            semantics = row["semantics"]
            presentation = semantics.get("presentation", {}).get("value", {})
            semantics["presentation"] = known(
                {
                    "description": UNKNOWN,
                    **presentation,
                    "name": known(name),
                }
            )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("items_xml", type=Path)
    parser.add_argument("doors_lua", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = capture(ROOT, args.items_xml.read_bytes(), args.doors_lua.read_bytes())
    raw = (json.dumps(packet, indent=2, ensure_ascii=False) + "\n").encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit("Published Item consumer fact capture differs")
    else:
        path.write_bytes(raw)
