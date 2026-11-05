"""Capture all Targuna NPC text literals for a repository authoring input.
This is a bounded Lua lexer, not an evaluator. Unknown text expression shapes fail.
By default emits an authoring fixture. --runtime-targuna emits the scoped text
input read by the Targuna native NPC adapter; it does not enable other NPC logic.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

from lua_source_text import (
    arguments,
    lex,
    text_expression,
    verify_sources,
    write_or_check,
)

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
NAMES = ("lizzie", "camilla", "sterling", "morla", "emiliana", "adrian")
NPC_BLOBS = {
    "lizzie": "40e8bbc7e6e0ad8bfd71ebd6fbf6c3c6c0115988",
    "camilla": "14dd037141cc7ec6a27b1d45e02b5d3356774485",
    "sterling": "e3204853d188107b57462112fa014f41a61f86b3",
    "morla": "e2dc06a2333662902d12a612ed6d5024f71ebcce",
    "emiliana": "06a0197536d6e4fbd77e71cfb9dd55a1c801f1e8",
    "adrian": "075b3cffe4574c2a1e88a7fc2265586674d625bc",
}


def capture(path: Path) -> dict:
    raw = path.read_bytes()
    source = raw.decode()
    tokens = lex(source)
    constants = {
        match.group(1): int(match.group(2))
        for match in re.finditer(
            r"local\s+([A-Z][A-Z_0-9]*)\s*=\s*([0-9]+)\s*(?:\n|$)", source
        )
    }
    records = []
    for index in range(len(tokens) - 3):
        prefix = [token.value for token in tokens[index : index + 4]]
        if prefix in (
            ["npcHandler", ":", "say", "("],
            ["npcHandler", ":", "setMessage", "("],
            ["keywordHandler", ":", "addKeyword", "("],
        ):
            args, _ = arguments(tokens, index + 3)
            line = tokens[index].line
            record = {"key": f"{path.stem}:{line}", "source_line": line}
            if prefix[2] == "say":
                record.update(
                    kind="scripted_reply", **text_expression(args[0], constants)
                )
            elif prefix[2] == "setMessage":
                if len(args[0]) != 1:
                    raise ValueError("unknown message event")
                record.update(
                    kind="message",
                    event=args[0][0].value,
                    **text_expression(args[1], constants),
                )
            else:
                if [t.value for t in args[1]] != ["StdModule", ".", "say"]:
                    raise ValueError("unsupported keyword action")
                keywords = [t.value for t in args[0] if t.string]
                config = args[2]
                text_index = next(
                    i
                    for i, t in enumerate(config)
                    if t.value == "text" and not t.string
                )
                value = config[text_index + 2 :]
                # The bounded donor shape puts text as the final config field.
                if value[-1].value != "}":
                    raise ValueError("unclosed keyword config")
                value = value[:-1]
                if value and value[-1].value == ",":
                    value = value[:-1]
                record.update(
                    kind="keyword",
                    keywords=keywords,
                    **text_expression(value, constants),
                )
            record["text_sha256"] = hashlib.sha256(
                json.dumps(
                    record["parts"], ensure_ascii=False, separators=(",", ":")
                ).encode()
            ).hexdigest()
            records.append(record)
    if len({record["key"] for record in records}) != len(records):
        raise ValueError("text line key collision")
    return {
        "npc": path.stem,
        "source": {
            "repository": "zimbadev/crystalserver",
            "commit": PIN,
            "path": f"data-global/npc/{path.name}",
            "sha256": hashlib.sha256(raw).hexdigest(),
        },
        "text_records": records,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--donor", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--runtime-targuna", action="store_true")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compare exact output bytes without writing",
    )
    args = parser.parse_args()
    verify_sources(
        args.donor,
        {f"data-global/npc/{name}.lua": blob for name, blob in NPC_BLOBS.items()},
    )
    records = [
        capture(args.donor / "data-global/npc" / f"{name}.lua") for name in NAMES
    ]
    result = {
        "schema": "oteryn.quests1.targuna-dialogue-source.v1",
        "source_only": not args.runtime_targuna,
        "npc_count": len(records),
        "text_count": sum(len(record["text_records"]) for record in records),
        "records": records,
    }
    write_or_check(
        args.out, json.dumps(result, ensure_ascii=False, indent=2) + "\n", args.check
    )
    print(json.dumps({"npcs": result["npc_count"], "texts": result["text_count"]}))


if __name__ == "__main__":
    main()
