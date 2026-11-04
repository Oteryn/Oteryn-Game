#!/usr/bin/env python3
"""Generate the bounded Flickering Soul Dialogue from Crystal201189f literals.

This authoring tool never executes Lua. The native Soul War caller owns keyword
precedence, topics, authority and durable effects; this file only captures text.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCE = "data-global/npc/flickering_soul.lua"
ROOT = Path(__file__).resolve().parents[3]
REPLIES = (
    ("living", 63, ("living",)),
    ("mortal", 65, ("mortal",)),
    ("goshnar", 67, ("goshnar",)),
    ("ambition", 72, ("ambition",)),
    ("milestone", 77, ("milestone",)),
    ("everything", 79, ("everything",)),
    ("accomplish", 81, ("accomplish",)),
    ("dead", 87, ("dead",)),
    ("confident", 89, ("confident",)),
    ("peace", 91, ("peace",)),
    ("soul", 98, ("soul",)),
    ("weariness", 103, ("weariness",)),
    ("knowledge", 108, ("knowledge",)),
    ("return", 110, ("return",)),
    ("fetters", 115, ("fetters",)),
    ("powerful", 121, ("powerful",)),
    ("task_complete", 126, ("task",)),
    ("task_offer", 133, ("task",)),
    ("accept", 137, ("yes",)),
    ("burden", 140, ("burden",)),
    ("shards", 145, ("shards",)),
    ("hate", 147, ("hate",)),
    ("fermuba", 152, ("fermuba",)),
    ("ferumbras", 154, ("ferumbras",)),
    ("grandson", 159, ("grandson",)),
    ("pale_worm", 161, ("pale worm",)),
    ("necromant_king", 163, ("necromant king",)),
    ("followers", 168, ("minions", "followers")),
    ("cleansed", 187, ("taints", "penalties")),
    ("untainted", 191, ("taints", "penalties")),
)
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')


def literal_speech(lines: list[str], line: int) -> tuple[list[str], int | None]:
    remaining = "\n".join(lines[line - 1 :])
    match = re.match(
        r'\s*npcHandler:say\(("(?:[^"\\]|\\.)*"|\{.*?\})'
        r', npc, player(?:, (\d+))?\)',
        remaining,
        flags=re.DOTALL,
    )
    if not match:
        raise ValueError(f"Pinned literal speech moved or became computed at {SOURCE}:{line}")
    expression = match[1]
    strings = STRING.findall(expression)
    residue = STRING.sub("", expression)
    if not strings or residue.strip("{}, \t\r\n"):
        raise ValueError(f"Nonliteral speech at {SOURCE}:{line}")
    return [json.loads(text) for text in strings], int(match[2]) if match[2] else None


def build(crystal: Path) -> dict:
    raw = subprocess.run(
        ["git", "-C", str(crystal), "show", f"{PIN}:{SOURCE}"],
        check=True, capture_output=True,
    ).stdout
    lines = raw.decode("utf-8").splitlines()
    keywords = []
    witnesses = []
    for key, line, triggers in REPLIES:
        reply, delay = literal_speech(lines, line)
        keywords.append({"key": key, "triggers": list(triggers), "reply": reply})
        witnesses.append({"key": key, "line": line, "source_interval_ms": delay})
    greet_line = lines[197]
    greet_match = re.fullmatch(
        r'npcHandler:setMessage\(MESSAGE_GREET, ("(?:[^"\\]|\\.)*")\)',
        greet_line.strip(),
    )
    if not greet_match:
        raise ValueError("Pinned greeting moved")
    evidence = {
        "repository": "zimbadev/crystalserver", "revision": PIN,
        "path": SOURCE, "sha256": hashlib.sha256(raw).hexdigest(),
        "speech": witnesses, "greeting_line": 198,
        "matcher_source": "data/npclib/npc.lua:13-20",
        "unreachable_shards_branch": {
            "line": 169, "shadowed_by_line": 144,
            "nonempty_expression": lines[178].strip(),
            "empty_expression": lines[180].strip(),
        },
    }
    return {
        "schema": "OTERYN_DIALOGUE_AUTHORING_SHARD/v1", "family": "Dialogue",
        "shard": {"index": 0, "start": 0, "end": 0, "count": 1},
        "source_legacy_role": SOURCE,
        "records": [{"declaration": {
            "kind": "Dialogue",
            "identity": {"key": "oteryn:dialogue.npc.flickering_soul", "revision": "definition-r1"},
            "greet": [json.loads(greet_match[1])],
            "keywords": keywords,
            "fields": [{
                "field_path": "oteryn:source.quest.soul_war.dialogue",
                "value": {"type": "Text", "value": json.dumps(evidence, ensure_ascii=False, sort_keys=True)},
            }],
        }}],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "content/dialogues/soul-war.json")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = json.dumps(build(args.crystal), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if args.output.read_text() != expected:
            raise SystemExit("Flickering Soul dialogue differs from pinned-source regeneration")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(expected)
    print("Flickering Soul: 30 source replies, 1 source greeting")


if __name__ == "__main__":
    main()
