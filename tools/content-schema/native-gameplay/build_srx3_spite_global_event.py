#!/usr/bin/env python3
"""Capture one pinned registered Lua GlobalEvent, independently of Creature bodies."""
import argparse
import hashlib
import json
import re
from pathlib import Path

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCES = {
    "data-global/scripts/quests/soul_war/soul_war_mechanics.lua": "9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916",
    "data-global/lib/quests/soul_war.lua": "89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75",
}
OUTPUT = Path(__file__).with_name("srx3-spite-global-event.json")


def build(root):
    sources = []
    texts = {}
    for path, digest in SOURCES.items():
        raw = (Path(root) / path).read_bytes()
        if hashlib.sha256(raw).hexdigest() != digest:
            raise ValueError("pinned Crystal GlobalEvent source changed: " + path)
        texts[path] = raw.decode("utf-8")
        sources.append(dict(path=path, sha256=digest,
                            git_blob_sha1=hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()))
    script = texts[next(iter(SOURCES))]
    library = texts["data-global/lib/quests/soul_war.lua"]
    lines = script.splitlines(keepends=True)

    def line(marker):
        found = [index + 1 for index, value in enumerate(lines) if value.strip() == marker]
        if len(found) != 1:
            raise ValueError("GlobalEvent source marker is not unique: " + marker)
        return found[0]

    declaration = line('local goshnarSpiteFire = GlobalEvent("CreateGoshnarSpiteFire")')
    start = line("function goshnarSpiteFire.onThink(interval)")
    interval = line("goshnarSpiteFire:interval(SoulWarQuest.timeToCreateSearingFire * 1000)")
    register = line("goshnarSpiteFire:register()")
    end = interval - 2
    create = "".join(lines[start - 1:end])
    expire_start = line("local function removeSearingFire(position)")
    expire_end = declaration - 2
    expire = "".join(lines[expire_start - 1:expire_end])
    required = [
        "math.random(#SoulWarQuest.goshnarsSpiteFirePositions)",
        "Game.createItem(SoulWarQuest.searingFireId, 1, firePosition)",
        "addEvent(removeSearingFire, SoulWarQuest.timeToRemoveSearingFire * 1000, firePosition)",
    ]
    if not all(value in create for value in required) or "Creature(" in create or "onUse" in create:
        raise ValueError("registered GlobalEvent unexpectedly requires a Creature or lever")
    if ('tile:getItemById(SoulWarQuest.searingFireId)' not in expire
            or 'Creature("Goshnar\'s Spite")' not in expire
            or "monster:addDefense(SoulWarQuest.goshnarsSpiteIncreaseDefense)" not in expire
            or "fire:remove()" not in expire):
        raise ValueError("Spite fire expiry source changed")

    def integer(name):
        values = re.findall(r"\b" + name + r"\s*=\s*(\d+)", library)
        if len(values) != 1:
            raise ValueError("source literal missing or repeated: " + name)
        return int(values[0])

    block = re.search(r"goshnarsSpiteFirePositions\s*=\s*{(.*?)\n\t},", library, re.S)
    if block is None:
        raise ValueError("Spite GlobalEvent positions missing")
    positions = [[int(value) for value in row] for row in re.findall(
        r"x\s*=\s*(\d+),\s*y\s*=\s*(\d+),\s*z\s*=\s*(\d+)", block[1])]
    if len(positions) != 4 or len({tuple(row) for row in positions}) != 4:
        raise ValueError("Spite GlobalEvent needs exactly four distinct literal positions")
    event = dict(
        name="CreateGoshnarSpiteFire", owner="RegisteredGlobalEvent",
        encounter=dict(family="Encounter", key="oteryn:encounter.soul_war_taint_zones", revision="definition-r1"),
        item=dict(family="Item", key=f"oteryn:item.tibia.i{integer('searingFireId')}", revision="definition-r1"),
        positions=positions, interval_ms=integer("timeToCreateSearingFire") * 1000,
        removal_delay_ms=integer("timeToRemoveSearingFire") * 1000,
        expiry_defense_add=integer("goshnarsSpiteIncreaseDefense"),
        requires_parent_creature=False, requires_committed_lever=False,
        declaration_line=declaration, create_lines=[start, end], expiry_lines=[expire_start, expire_end],
        registration_lines=[interval, register], create_body=create, expiry_body=expire,
        create_body_sha256=hashlib.sha256(create.encode()).hexdigest(),
        expiry_body_sha256=hashlib.sha256(expire.encode()).hexdigest(),
    )
    return dict(schema="OTERYN_SOURCE_GLOBAL_EVENT/v1", donor="zimbadev/crystalserver", revision=PIN,
                sources=sources, events=[event])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source_root", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = (json.dumps(build(args.source_root), indent=2) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != encoded:
            raise SystemExit("Spite GlobalEvent capture drift")
    else:
        OUTPUT.write_bytes(encoded)


if __name__ == "__main__":
    main()
