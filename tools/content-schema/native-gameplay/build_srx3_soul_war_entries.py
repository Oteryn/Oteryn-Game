#!/usr/bin/env python3
"""Capture the six pinned literal Soul War lever recipes without executing Lua."""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
SOURCE = "data-global/lib/quests/soul_war.lua"
SHA256 = "89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75"
OUTPUT = Path(__file__).with_name("srx3-soul-war-entry-recipes.json")
NAMES = ("Spite", "Malice", "Greed", "Hatred", "Cruelty", "Megalomania")


def build(source_root):
    raw = (source_root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError("pinned Soul War library changed")
    text = raw.decode()
    records = []
    for name in NAMES:
        start = text.index("\t\tgoshnars" + name + " = {")
        tail = text[start:]
        block = tail[:tail.index("\n\t\t},") + len("\n\t\t},")]
        slug = name.lower() + ("_purple" if name == "Megalomania" else "")
        boss = re.search(r"position = Position\((\d+), (\d+), (\d+)\)", block)
        slots = re.findall(r"pos = Position\((\d+), (\d+), (\d+)\), teleport = Position\((\d+), (\d+), (\d+)\)", block)
        exit_cell = re.search(r"exit = Position\((\d+), (\d+), (\d+)\)", block)
        cooldowns = re.findall(r"timeToFightAgain = (\d+) \* 60 \* 60", block)
        if boss is None or exit_cell is None or len(slots) != 5 or cooldowns != ["72" if name == "Megalomania" else "20"]:
            raise ValueError("source literal recipe changed: " + name)
        records.append(dict(
            slug=slug, room="Goshnars" + name, source_path=SOURCE,
            source_sha256=SHA256, line=text[:start].count("\n") + 1,
            boss_key="oteryn:boss.goshnar_s_" + name.lower(),
            creature_key="oteryn:creature.goshnar_s_" + slug,
            boss=list(map(int, boss.groups())),
            levers=[list(map(int, cell[:3])) for cell in slots],
            entries=[list(map(int, cell[3:])) for cell in slots],
            exit=list(map(int, exit_cell.groups())), minimum_players=1,
            minimum_level=250, cooldown_seconds=int(cooldowns[0]) * 3600,
            time_limit_seconds=1200, after_kill_seconds=60,
        ))
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(args.source_root), indent=2) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("stale Soul War entry recipe capture")
    else:
        OUTPUT.write_bytes(raw)


if __name__ == "__main__":
    main()
