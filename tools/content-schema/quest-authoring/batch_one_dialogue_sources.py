"""Capture the held Curos/Eroth dialogues with the existing source literal reader.

The output is full source text, never an executable NPC route or a grant. In
particular, Curos' foreign New Frontier guards and Eroth's atomic item rewards
remain closed under the coordinator's answer linked in the output.
"""

import argparse
import hashlib
import json
from pathlib import Path

from lua_tables import _string, tokenize

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
NPCS = {
    "an_uneasy_alliance": ("curos", "4fdffd8e90b8de6d2f17ce5c38c45020397c48c9"),
    "to_blind_the_enemy_quest": ("eroth", "adc5789d7f85d0310e802a63f8462084d19ec2c1"),
}
ANSWER = "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728"


def arguments(tokens, start):
    depth, part, result = 0, [], []
    for token in tokens[start + 1 :]:
        kind, value, _ = token
        if kind == "symbol" and value in "({[":
            depth += 1
        elif kind == "symbol" and value in ")}]":
            if value == ")" and depth == 0:
                return result + [part]
            depth -= 1
        if kind == "symbol" and value == "," and depth == 0:
            result.append(part)
            part = []
        else:
            part.append(token)
    raise ValueError("unclosed source call")


def literal_parts(tokens):
    if any(
        kind not in ("string", "lstring") and value not in ("{", "}", ",")
        for kind, value, _ in tokens
    ):
        raise ValueError("nonliteral source speech needs an explicit adapter")
    parts = [_string(token) for token in tokens if token[0] in ("string", "lstring")]
    if not parts:
        raise ValueError("empty source speech")
    return parts


def capture(donor, quest):
    npc, expected = NPCS[quest]
    path = f"data-global/npc/{npc}.lua"
    raw = (donor / path).read_bytes()
    blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if blob != expected:
        raise ValueError(f"pinned donor blob mismatch: {path}")
    tokens = [
        token
        for token in tokenize(raw.decode())
        if token[0] not in ("comment", "lcomment")
    ]
    records = []
    for index, token in enumerate(tokens):
        prefix = [part[1] for part in tokens[index : index + 4]]
        if prefix in (
            ["npcHandler", ":", "say", "("],
            ["npcHandler", ":", "setMessage", "("],
        ):
            args = arguments(tokens, index + 3)
            message = prefix[2] == "setMessage"
            record = {
                "line": token[2],
                "kind": "message" if message else "reply",
                "parts": literal_parts(args[1 if message else 0]),
            }
            if message:
                record["event"] = args[0][0][1]
            records.append(record)
        elif (
            token[1] == "text"
            and index + 2 < len(tokens)
            and tokens[index + 1][1] == "="
        ):
            value = tokens[index + 2]
            records.append(
                {
                    "line": token[2],
                    "kind": "keyword_text",
                    "parts": literal_parts([value]),
                }
            )
    return {
        "schema": "OTERYN_BATCH_ONE_NPC_SOURCE/v1",
        "quest": quest,
        "npc": npc,
        "source_only": True,
        "held_by": ANSWER,
        "source": {
            "repository": "zimbadev/crystalserver",
            "commit": PIN,
            "path": path,
            "git_blob": blob,
            "sha256": hashlib.sha256(raw).hexdigest(),
        },
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--quest", choices=sorted(NPCS), required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = capture(args.donor, args.quest)
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if args.out.read_text() != encoded:
            raise SystemExit("source capture drift")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(encoded)
    print(
        json.dumps(
            {
                "quest": args.quest,
                "records": len(result["records"]),
                "paragraphs": sum(len(row["parts"]) for row in result["records"]),
            }
        )
    )


if __name__ == "__main__":
    main()
