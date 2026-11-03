"""Capture all Hidden Threats NPC text from the pinned Crystal Git tree.

The output is source-only evidence. Existing exact NPC bindings identify the
records; this tool grants neither placement nor conversation authority.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

from lua_tables import Parser, _string, as_python, tokenize

ROOT = Path(__file__).resolve().parents[3]
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
PREFIX = f"zimbadev/crystalserver@{PIN}:data-global/npc/"
STEMS = (
    "corym_butler",
    "corym_footman",
    "corym_ratter",
    "corym_servant",
    "corym_slave",
    "corym_worker_01",
    "corym_worker_02",
    "corym_worker_03",
    "corym_worker_04",
    "corym_worker_05",
)
REASON = (
    "Full pinned source text only; no Hidden Threats NPC has a qualified placement "
    "in the current canonical World inputs (QUESTION 6590445954, ANSWER 6589524728 "
    "section 7). Canonical NPC keys are documentary joins to current Oteryn NPC "
    "declarations, never placement authority. This archive activates no greeting, "
    "quest write or reward."
)


def canonical_npcs(root):
    found = {}
    for path in sorted((root / "content/npcs/definitions").glob("npcs-*.json")):
        for record in json.loads(path.read_bytes())["records"]:
            identity = record["declaration"]["identity"]
            for binding in record.get("source_bindings", []):
                stem = binding.get("external_id")
                if (
                    binding.get("source_key") != "oteryn:source.crystalserver"
                    or binding.get("identity_namespace") != "crystalserver/npc-file"
                    or stem not in STEMS
                ):
                    continue
                target = binding.get("target", {})
                if (
                    binding.get("disposition") != "EXACT"
                    or target != {"family": "NPC", **identity}
                    or stem in found
                ):
                    raise ValueError("HIDDEN_SOURCE_NPC_BINDING")
                found[stem] = identity["key"]
    if set(found) != set(STEMS) or len(set(found.values())) != len(STEMS):
        raise ValueError("HIDDEN_SOURCE_NPC_BINDING")
    return found


def messages(raw, stem):
    tokens = [
        token
        for token in tokenize(raw.decode("utf-8"))
        if token[0] not in ("comment", "lcomment")
    ]
    result = []
    for index in range(len(tokens) - 4):
        head = [token[1] for token in tokens[index : index + 4]]
        if head not in (
            ["npcHandler", ":", "say", "("],
            ["npcHandler", ":", "setMessage", "("],
        ):
            continue
        method = tokens[index + 2][1]
        depth, arguments, argument = 0, [], []
        end = index + 4
        while end < len(tokens):
            token = tokens[end]
            if token[1] == ")" and depth == 0:
                arguments.append(argument)
                break
            if token[1] == "," and depth == 0:
                arguments.append(argument)
                argument = []
            else:
                argument.append(token)
                if token[1] in ("(", "{", "["):
                    depth += 1
                elif token[1] in (")", "}", "]"):
                    depth -= 1
            end += 1
        if end == len(tokens):
            raise ValueError("HIDDEN_SOURCE_UNTERMINATED_MESSAGE")
        text_argument = arguments[0 if method == "say" else 1]
        if len(text_argument) == 1:
            parts = [_string(text_argument[0])]
        else:
            parts = as_python(
                Parser("".join(token[1] for token in text_argument)).value()
            )
        if not isinstance(parts, list) or not all(
            isinstance(part, str) for part in parts
        ):
            raise ValueError("HIDDEN_SOURCE_NONLITERAL_MESSAGE")
        result.append(
            {
                "kind": method,
                "event": "".join(token[1] for token in arguments[0])
                if method == "setMessage"
                else None,
                "parts": parts,
                "source": f"{PREFIX}{stem}.lua:{tokens[index][2]}-{tokens[end][2]}",
            }
        )
    return result


def extract(blobs, keys):
    if set(blobs) != set(STEMS) or set(keys) != set(STEMS):
        raise ValueError("HIDDEN_SOURCE_NPC_SET")
    records = [
        {
            "npc": keys[stem],
            "sha256": hashlib.sha256(blobs[stem]).hexdigest(),
            "messages": messages(blobs[stem], stem),
            "source_npc_file": stem,
        }
        for stem in STEMS
    ]
    if sum(len(record["messages"]) for record in records) != 44:
        raise ValueError("HIDDEN_SOURCE_MESSAGE_SET")
    return {
        "schema": "OTERYN_HIDDEN_THREATS_SOURCE_DIALOGUE_V1",
        "source_only": True,
        "reason": REASON,
        "records": records,
    }


def capture(crystal_root, repo_root=ROOT):
    blobs = {
        stem: subprocess.check_output(
            [
                "git",
                "-C",
                str(crystal_root),
                "show",
                f"{PIN}:data-global/npc/{stem}.lua",
            ]
        )
        for stem in STEMS
    }
    return extract(blobs, canonical_npcs(repo_root))


def encoded(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = encoded(capture(args.crystal_root, args.repo_root))
    if args.check:
        if not args.output.exists() or args.output.read_bytes() != data:
            raise SystemExit("HIDDEN_SOURCE_EVIDENCE_DRIFT")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(data)


if __name__ == "__main__":
    main()
