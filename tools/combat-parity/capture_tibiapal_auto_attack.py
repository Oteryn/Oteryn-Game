#!/usr/bin/env python3
"""Capture the TibiaTools (TibiaPal) auto-attack fixture grid.

Decision: ATTACK-PARITY-1 packet sections 1.1 and 1.2. The tool talks to the public,
read-only TibiaTools API. CI never runs it against the network: the offline test only
reproduces the request bodies from the grid definition stored in the fixture.

Usage:
  python3 tools/combat-parity/capture_tibiapal_auto_attack.py capture [--out PATH] [--workers N]
  python3 tools/combat-parity/capture_tibiapal_auto_attack.py verify [--fixture PATH]
"""

import argparse
import concurrent.futures
import glob
import json
import os
import sys
import time
import urllib.request
from datetime import datetime, timezone

API = "https://tibiatools.io/api/v1"
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FIXTURE = os.path.join(REPO, "content", "combat", "parity", "tibiapal_auto_attack_v1.json")
ITEM_DEFINITIONS = os.path.join(REPO, "content", "items", "definitions", "items-*.json")

SCHEMA = "OTERYN_TIBIAPAL_AUTO_ATTACK_FIXTURE/v1"
# Monk is excluded: the TibiaTools guide says its calculator hardcodes VoH (packet 1.1).
VOCATIONS = ["knight", "paladin", "sorcerer", "druid"]
LEVELS = [8, 50, 100, 300, 600, 1000]
SKILLS = [10, 50, 100, 120]
WEAPON_CLASSES = ["axe", "club", "sword"]
FISTS_ID = 1
OTERYN_FIGHT_MODE = "OFFENSIVE"
OTERYN_ATTACK_FACTOR = 1.0
WEAPON_TYPE_OF_CLASS = {"axe": "AXE", "club": "CLUB", "sword": "SWORD"}


def request_body(vocation, level, skill, weapon_id):
    """The exact POST /damage body. No stances, perks, charms, rotation, targets or ammunition."""
    return {
        "stats": {"vocation": vocation, "level": level, "skill": skill},
        "weapon": {"id": weapon_id},
    }


def weapon_slug(weapon):
    return "fists" if weapon is None else "w%d" % weapon["tibiatools_id"]


def row_key(vocation, level, skill, weapon):
    return "%s-L%d-S%d-%s" % (vocation, level, skill, weapon_slug(weapon))


def build_requests(grid):
    """Every (key, weapon, request body) of the grid definition, in a fixed order."""
    out = []
    weapons = [None] + list(grid["weapons"])
    for weapon in weapons:
        weapon_id = FISTS_ID if weapon is None else weapon["tibiatools_id"]
        for vocation in grid["vocations"]:
            for level in grid["levels"]:
                for skill in grid["skills"]:
                    out.append(
                        (
                            row_key(vocation, level, skill, weapon),
                            weapon,
                            request_body(vocation, level, skill, weapon_id),
                        )
                    )
    return out


def select_weapons(catalogue, oteryn_weapons):
    """One weapon per melee class and attack residue mod 5 where the catalogue has one.

    catalogue: TibiaTools /meta/weapons items. oteryn_weapons: {lower name: [(key, weapon_type,
    attack or None)]}. A candidate needs a unique catalogue name without a variant suffix and
    exactly one Oteryn item of the same weapon type. Among candidates the weapon whose Oteryn
    attack equals the catalogue attack wins, then one-handed, then the lowest id.
    Returns (weapons, missing) with missing as [(class, residue, reason)].
    """
    names = {}
    for item in catalogue:
        names.setdefault(item["name"].lower(), []).append(item)
    chosen, missing = [], []
    for cls in WEAPON_CLASSES:
        for residue in range(5):
            candidates = []
            reason = "no catalogue weapon with this attack residue"
            for item in catalogue:
                if item.get("skill") != cls or item["attack"] % 5 != residue:
                    continue
                reason = "no candidate with a unique catalogue name and a unique Oteryn item"
                if "(" in item["name"] or len(names[item["name"].lower()]) != 1:
                    continue
                mapped = [
                    m
                    for m in oteryn_weapons.get(item["name"].lower(), [])
                    if m[1] == WEAPON_TYPE_OF_CLASS[cls]
                ]
                if len(mapped) != 1:
                    continue
                key, _, oteryn_attack = mapped[0]
                rank = (oteryn_attack != item["attack"], item.get("hands") != "one", item["id"])
                candidates.append((rank, item, key))
            if not candidates:
                missing.append((cls, residue, reason))
                continue
            _, item, key = min(candidates, key=lambda c: c[0])
            chosen.append(
                {
                    "tibiatools_id": item["id"],
                    "tibiatools_name": item["name"],
                    "class": cls,
                    "hands": item.get("hands"),
                    "tibiatools_attack": item["attack"],
                    "attack_residue_mod_5": residue,
                    "oteryn_item_key": key,
                }
            )
    return chosen, missing


def load_oteryn_weapons():
    out = {}
    for path in sorted(glob.glob(ITEM_DEFINITIONS)):
        with open(path, encoding="utf-8") as handle:
            records = json.load(handle)["records"]
        for record in records:
            definition = record.get("definition", {})
            semantics = definition.get("semantics")
            if not semantics:
                continue
            weapon = semantics.get("weapon", {})
            if weapon.get("state") != "KNOWN":
                continue
            name = semantics["presentation"].get("value", {}).get("name", {}).get("value")
            if not name:
                continue
            value = weapon["value"]
            weapon_type = value.get("weapon_type", {}).get("value")
            attack = value.get("attack", {}).get("value")
            out.setdefault(name.lower(), []).append(
                (definition["identity"]["key"], weapon_type, attack)
            )
    return out


def http(method, path, body=None, retries=4):
    data = None if body is None else json.dumps(body).encode()
    headers = {"content-type": "application/json", "user-agent": "oteryn-combat-parity/1"}
    for attempt in range(retries):
        try:
            req = urllib.request.Request(API + path, data=data, method=method, headers=headers)
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.load(resp)
        except Exception:
            if attempt == retries - 1:
                raise
            time.sleep(2 ** (attempt + 1))


def now():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def capture(out_path, workers):
    started = now()
    meta = http("GET", "/meta/weapons")
    catalogue = meta["items"]
    fists = next(i for i in catalogue if i["id"] == FISTS_ID)
    weapons, missing = select_weapons(catalogue, load_oteryn_weapons())
    grid = {
        "vocations": VOCATIONS,
        "levels": LEVELS,
        "skills": SKILLS,
        "fists_weapon_id": FISTS_ID,
        "fists_attack": fists["attack"],
        "weapons": weapons,
    }
    requests = build_requests(grid)

    def one(item):
        key, weapon, body = item
        resp = http("POST", "/damage", body)
        spell = next(s for s in resp["spells"] if s["name"] == "Auto-attack")
        raw = spell["raw"]
        row = {
            "key": key,
            "request": body,
            "raw": {"min": raw["min"], "avg": raw["avg"], "max": raw["max"]},
            "captured_at": now(),
            "api_description": meta["description"],
            "oteryn_fight_mode": OTERYN_FIGHT_MODE,
            "oteryn_attack_factor": OTERYN_ATTACK_FACTOR,
        }
        if weapon is not None:
            row["tibiatools_attack"] = weapon["tibiatools_attack"]
            row["oteryn_item_key"] = weapon["oteryn_item_key"]
        return row

    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
        rows = list(pool.map(one, requests))
    fixture = {
        "schema": SCHEMA,
        "decision": "ATTACK-PARITY-1 packet sections 1.1, 1.2",
        "source": API + "/damage (Auto-attack row, raw)",
        "catalogue_source": API + "/meta/weapons",
        "catalogue_count": meta["count"],
        "capture_started_at": started,
        "capture_finished_at": now(),
        "grid": grid,
        "missing_residues": [
            {"class": c, "residue_mod_5": r, "reason": why} for c, r, why in missing
        ],
        "rows": rows,
    }
    write_fixture(out_path, fixture)
    print("rows=%d weapons=%d missing=%d" % (len(rows), len(weapons), len(missing)))


def write_fixture(path, fixture):
    """Stable layout: header keys indented, one row per line."""
    head = {k: v for k, v in fixture.items() if k != "rows"}
    text = json.dumps(head, indent=1, sort_keys=False)
    rows = ",\n".join("  " + json.dumps(r, separators=(",", ":")) for r in fixture["rows"])
    text = text[:-2] + ',\n "rows": [\n' + rows + "\n ]\n}\n"
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def verify(path):
    with open(path, encoding="utf-8") as handle:
        fixture = json.load(handle)
    built = build_requests(fixture["grid"])
    rows = fixture["rows"]
    if [(k, b) for k, _, b in built] != [(r["key"], r["request"]) for r in rows]:
        print("request bodies differ from the grid definition")
        return 1
    print("ok rows=%d" % len(rows))
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)
    cap = sub.add_parser("capture", help="live capture against TibiaTools")
    cap.add_argument("--out", default=FIXTURE)
    cap.add_argument("--workers", type=int, default=4)
    ver = sub.add_parser("verify", help="offline: rebuild the request bodies")
    ver.add_argument("--fixture", default=FIXTURE)
    args = parser.parse_args(argv)
    if args.cmd == "capture":
        capture(args.out, args.workers)
        return 0
    return verify(args.fixture)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
