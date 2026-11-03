"""Reproduce pinned A Piece of Cake source evidence without activating gameplay.

Only git show/ls-tree of Crystal 201189f is read. The worktree/HEAD is not trusted;
all bytes are checked against the pinned Git blob. --check never writes output.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
PREFIX = f"zimbadev/crystalserver@{PIN}:"
CONFIG = "data-global/scripts/lib/a_piece_of_cake_config.lua"
QUEST = "data-global/scripts/quests/a_piece_of_cake/"
NPC_NAMES = ["benny", "biff", "brian", "balduin", "billy", "brutus"]
NPCS = [f"data-global/npc/{name}_the_baker.lua" for name in NPC_NAMES]
MONSTER = "data-global/monster/quests/a_piece_of_cake/cake_golem.lua"


def git_blob(raw: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()


def snapshot(crystal_root: Path):
    listing = subprocess.check_output(
        [
            "git",
            "-C",
            str(crystal_root),
            "ls-tree",
            "-r",
            "-z",
            PIN,
            "--",
            CONFIG,
            QUEST,
            *NPCS,
            MONSTER,
        ]
    )
    tree = {}
    for entry in listing.split(b"\0"):
        if not entry:
            continue
        meta, name = entry.split(b"\t", 1)
        _, kind, sha = meta.decode().split()
        if kind == "blob":
            tree[name.decode()] = {"sha": sha}
    paths = [
        CONFIG,
        *sorted(
            path for path in tree if path.startswith(QUEST) and path.endswith(".lua")
        ),
        *NPCS,
        MONSTER,
    ]
    raw = {}
    for path in paths:
        value = subprocess.check_output(
            [
                "git",
                "-C",
                str(crystal_root),
                "show",
                f"{PIN}:{path}",
            ]
        )
        if git_blob(value) != tree[path]["sha"]:
            raise ValueError(f"Pinned source blob mismatch: {path}")
        raw[path] = value
    return paths, tree, raw


def extract(paths, tree, raw):
    # Keep the original report's ordering and source line accounting byte-stable.
    PATHS, TREE, RAW = paths, tree, raw
    TEXT = {path: value.decode() for path, value in raw.items()}

    def source(path, offset):
        return PREFIX + path + ":" + str(TEXT[path].count("\n", 0, offset) + 1)

    def number_table(marker):
        text = TEXT[CONFIG]
        start = text.index(marker)
        end = text.index("\n}", start)
        result = []
        for match in re.finditer(
            r"^\s*([A-Za-z_][A-Za-z_0-9]*)\s*=\s*([0-9]+)\s*[,;]",
            text[start:end],
            re.MULTILINE,
        ):
            result.append(
                {
                    "name": match[1],
                    "value": int(match[2]),
                    "source": source(CONFIG, start + match.start()),
                }
            )
        return result

    def unquote(value):
        # These source utterances use quoted strings and ordinary escaped punctuation.
        return re.sub(
            r"\\([\\\"'nrt])",
            lambda m: {"n": "\n", "r": "\r", "t": "\t"}.get(m[1], m[1]),
            value,
        )

    def utterances(path):
        text = TEXT[path]
        rows = []
        # Keep format placeholders and duplicate source branches verbatim. No Lua runs.
        pattern = re.compile(
            r'(npcHandler:say|npcHandler:setMessage)\(\s*(?:MESSAGE_([A-Z_]+)\s*,\s*)?\(?\s*"((?:\\.|[^"\\])*)"',
            re.DOTALL,
        )
        for match in pattern.finditer(text):
            rows.append(
                {
                    "kind": match[2].lower() if match[2] else "reply",
                    "text": unquote(match[3]),
                    "source": source(path, match.start()),
                }
            )
        expected = len(re.findall(r"npcHandler:(?:say|setMessage)\(", text))
        if len(rows) != expected:
            raise ValueError(
                f"Uncaptured NPC utterance: {path}: {len(rows)}/{expected}"
            )
        return rows

    dialogs = []
    for path in NPCS:
        name = re.search(r'local internalNpcName = "([^"]+)"', TEXT[path])[1]
        city = re.search(r'local CITY_NAME = "([^"]+)"', TEXT[path])
        dialogs.append(
            {
                "source_npc_name": name,
                "city": city[1] if city else None,
                "utterances": utterances(path),
            }
        )

    config = TEXT[CONFIG]
    keys_start = config.index("CakeQuest.Keys = {")
    keys_end = config.index("\n}", keys_start)
    keys = [
        {"name": m[1], "key": m[2], "source": source(CONFIG, keys_start + m.start())}
        for m in re.finditer(
            r'^\s*([A-Za-z][A-Za-z0-9]+)\s*=\s*"([^"]+)"',
            config[keys_start:keys_end],
            re.MULTILINE,
        )
    ]
    item_start = config.index("CakeQuest.Items = {")
    item_end = config.index("\n}", item_start)
    items = [
        {
            "name": m[1],
            "ids": [int(n) for n in re.findall(r"\d+", m[2])],
            "source": source(CONFIG, item_start + m.start()),
        }
        for m in re.finditer(
            r"^\s*([A-Za-z][A-Za-z0-9]+)\s*=\s*(\{[^}]*\}|\d+)",
            config[item_start:item_end],
            re.MULTILINE,
        )
    ]
    packet = {
        "schema": "quest_batch_1_source_evidence",
        "quest": "a_piece_of_cake",
        "purpose": "Pinned source inventory and complete NPC utterances. Not an activated runtime or new world owner.",
        "runtime_scope_decision": "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728",
        "source_pin": PIN,
        "configuration": number_table("CakeQuest.Config = {"),
        "world_keys": keys,
        "player_storages": number_table("CakeQuest.PlayerStorage = {"),
        "items": items,
        "dialogues": dialogs,
        "source_manifest": [
            {
                "path": path,
                "git_blob": TREE[path]["sha"],
                "sha256": hashlib.sha256(RAW[path]).hexdigest(),
                "lines": len(TEXT[path].splitlines()),
            }
            for path in PATHS
        ],
    }
    return packet


def encoded(packet):
    return (json.dumps(packet, ensure_ascii=False, indent=2) + "\n").encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    payload = encoded(extract(*snapshot(args.crystal_root)))
    if args.check:
        if not args.output.is_file() or args.output.read_bytes() != payload:
            raise SystemExit("CAKE_SOURCE_EVIDENCE_DRIFT")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(payload)


if __name__ == "__main__":
    main()
