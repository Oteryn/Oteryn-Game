"""Reproduce Mazarius's ordered dialogue from the exact Crystal Git pin.

This tool only produces the dialogue model. It does not enable the held atomic
item exchange, author map placements or regenerate world callback extensions.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path

from lua_tables import Parser, as_python

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
PREFIX = f"crystal:{PIN}:"
NPC_PATH = "data-global/npc/mazarius.lua"
STORAGE_PATH = "data-global/lib/core/storages.lua"
KEYWORD_PATH = "data/npclib/npc.lua"
BOUNDS = (
    (62, 64),
    (65, 67),
    (68, 85),
    (86, 91),
    (92, 93),
    (94, 96),
    (97, 109),
    (110, 120),
    (121, 123),
    (124, 129),
    (130, 138),
    (139, 147),
    (148, 150),
    (151, 153),
    (154, 156),
    (157, 159),
    (160, 165),
    (166, 171),
    (172, 174),
    (175, 180),
    (181, 183),
    (184, 186),
)


def quoted(text):
    return [
        json.loads(match.group()) for match in re.finditer(r'"(?:\\.|[^"\\])*"', text)
    ]


def extract(npc_raw, storage_raw, keyword_raw):
    text = npc_raw.decode("utf-8")
    lines = text.splitlines()
    storage = storage_raw.decode("utf-8")
    declaration = storage.index("FerumbrasAscension = {")
    start = storage.index("{", declaration)
    constants = as_python(Parser(storage[start:]).value())
    if "function MsgContains" not in keyword_raw.decode("utf-8"):
        raise ValueError("FERUMBRAS_KEYWORD_SOURCE")
    branches = []
    for low, high in BOUNDS:
        block = "\n".join(lines[low - 1 : high])
        first = lines[low - 1]
        keywords = re.findall(r'MsgContains\(message, "([^"]+)"\)', first)
        topic = re.search(r"npcHandler:getTopic\(playerId\) == (\d+)", first)
        limit = re.search(r"FerumbrasAscension.Access\) < (\d+)", first)
        outputs = [
            quoted(match.group(1))
            for match in re.finditer(
                r'npcHandler:say\((\{.*?\}|"(?:\\.|[^"\\])*")\s*,\s*npc\s*,\s*creature\s*\)',
                block,
                re.DOTALL,
            )
        ]
        topics = re.findall(r"npcHandler:setTopic\(playerId, (\d+)\)", block)
        if not outputs or not outputs[0]:
            raise ValueError("FERUMBRAS_SOURCE_REPLY")
        row = {
            "keywords": keywords,
            "topic": int(topic.group(1)) if topic else None,
            "access_below": int(limit.group(1)) if limit else None,
            "next_topic": int(topics[-1]) if topics else None,
            "reply": outputs[0],
            "exchange": None,
            "source": f"{PREFIX}{NPC_PATH}:{low}",
        }
        if low == 97:
            remove = re.findall(r"player:removeItem\((\d+), (\d+)\)", block)
            give = re.findall(r"player:addItem\((\d+), (\d+)\)", block)
            writes = re.findall(
                r"player:setStorageValue\(Storage.Quest.U10_90.FerumbrasAscension.(\w+), (\d+)\)",
                block,
            )
            if (
                len(remove) != 1
                or len(give) != 1
                or len(writes) != 4
                or len(outputs) != 2
            ):
                raise ValueError("FERUMBRAS_EXCHANGE_SOURCE")
            row["exchange"] = {
                "remove": {"source_id": int(remove[0][0]), "count": int(remove[0][1])},
                "give": {"source_id": int(give[0][0]), "count": int(give[0][1])},
                "writes": [
                    {"storage": constants[name], "value": int(value)}
                    for name, value in writes
                ],
                "refusal": outputs[1],
            }
        branches.append(row)
    if len(branches) != 22:
        raise ValueError("FERUMBRAS_SOURCE_BRANCH_SET")
    return {
        "schema": 1,
        "npc": "oteryn:npc.mazarius",
        "quest": "oteryn:quest.ferumbras_ascension_quest",
        "source": f"{PREFIX}{NPC_PATH}:54",
        "keyword_source": f"{PREFIX}{KEYWORD_PATH}:13",
        "greet": quoted(lines[188])[0],
        "farewell": quoted(lines[189])[0],
        "walkaway": quoted(lines[190])[0],
        "branches": branches,
    }


def capture(crystal_root):
    raw = [
        subprocess.check_output(
            [
                "git",
                "-C",
                str(crystal_root),
                "show",
                f"{PIN}:{path}",
            ]
        )
        for path in (NPC_PATH, STORAGE_PATH, KEYWORD_PATH)
    ]
    return extract(*raw)


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = encoded(capture(args.crystal_root))
    if args.check:
        if not args.output.exists() or args.output.read_bytes() != data:
            raise SystemExit("FERUMBRAS_MAZARIUS_DRIFT")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(data)


if __name__ == "__main__":
    main()
