"""Capture the complete Isle quest dialogue branches and their storage-only transitions.

The bounded text lexer is provided by lua_source_text.py. It never executes Lua.
Inventory, currency, appearance and teleport effects remain held.
"""

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCE_BLOBS = {
    "data-global/npc/rapanaio.lua": "b5a1ee36924fd594d9f40ae65e9da1ae029070cd",
    "data-global/npc/rapanaio_boat.lua": "a511b50dd92c201f669a70ad2a5e14cb897222df",
    "data-global/npc/rapanaio_isle_of_evil.lua": "7ce4803f69ba8ea32808059fb342137968478810",
    "data-global/npc/scutty.lua": "c683ba04d455fac5d29db0244bc57ee65dcb0f2c",
    "data-global/npc/mortimer.lua": "3644975c04a238af4f318e2f27e23f7c966058a6",
    "data-global/npc/king_tibianus.lua": "77e9a4f6890cfddb7ce6c775fe64a6662861cacf",
    "data/npclib/npc_system/npc_handler.lua": "5ac57f68c335e7aa5e2d03ab032ef5f42a5e266d",
    "data-global/scripts/quests/the_isle_of_evil_quest/actions_heated_worm_punisher.lua": "bc6eeb3531cc8c16a9fc6e5ac20882eda7f9372a",
    "data-global/lib/core/storages.lua": "1a9a7e931eb984a75b285be01df6b1dba36476d8",
}
QUEST = "oteryn:quest.the_isle_of_evil_quest"
PREFIX = "oteryn:quest-progress/crystalserver/quest/u8_5/the_isle_of_evil/"
KV_PREFIX = "oteryn:quest-progress/crystalserver/kv/the-isle-of-evil/"
NAMES = (
    "rapanaio",
    "rapanaio_boat",
    "rapanaio_isle_of_evil",
    "scutty",
    "mortimer",
    "king_tibianus",
)
RANGES = {
    "scutty": [(73, 133), (136, 136)],
    "mortimer": [(626, 641), (650, 650)],
    "king_tibianus": [(220, 233)],
}


def source(path, line):
    return f"zimbadev/crystalserver@{PIN}:{path}:{line}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--text-parser", type=Path, required=True)
    parser.add_argument(
        "--out",
        type=Path,
        help="Explicit staging directory (alternative to the two output paths)",
    )
    parser.add_argument("--quest-out", type=Path)
    parser.add_argument("--dialogue-out", type=Path)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Compare exact output bytes without writing",
    )
    args = parser.parse_args()
    if args.out is not None:
        if args.quest_out is not None or args.dialogue_out is not None:
            parser.error("--out cannot be combined with --quest-out or --dialogue-out")
        quest_out = args.out / "isle-of-evil.json"
        dialogue_out = args.out / "isle-of-evil-dialogue.json"
    elif args.quest_out is not None and args.dialogue_out is not None:
        quest_out, dialogue_out = args.quest_out, args.dialogue_out
    else:
        parser.error(
            "provide --quest-out and --dialogue-out, or an explicit --out staging directory"
        )
    spec = importlib.util.spec_from_file_location("isle_text_lexer", args.text_parser)
    lexer = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = lexer
    spec.loader.exec_module(lexer)
    lexer.verify_sources(args.donor, SOURCE_BLOBS)
    replies = {}
    sources = []
    events = {}
    static_keywords = []
    for name in NAMES:
        path = f"data-global/npc/{name}.lua"
        raw = (args.donor / path).read_bytes()
        sources.append({"path": path, "sha256": hashlib.sha256(raw).hexdigest()})
        tokens = lexer.lex(raw.decode())
        events[name] = {}
        for index in range(len(tokens) - 3):
            prefix = [token.value for token in tokens[index : index + 4]]
            if prefix not in (
                ["npcHandler", ":", "say", "("],
                ["npcHandler", ":", "setMessage", "("],
                ["keywordHandler", ":", "addKeyword", "("],
            ):
                continue
            line = tokens[index].line
            if name in RANGES and not any(
                first <= line <= last for first, last in RANGES[name]
            ):
                continue
            argv, _ = lexer.arguments(tokens, index + 3)
            key = f"{name}:{line}"
            if prefix[2] == "say":
                text = lexer.text_expression(argv[0], {})
            elif prefix[2] == "setMessage":
                event = argv[0][0].value
                text = lexer.text_expression(argv[1], {})
                events[name][event] = key
            else:
                # Only the three dedicated Rapanaio NPCs have quest-owned static lore.
                if name not in NAMES[:3] or [token.value for token in argv[1]] != [
                    "StdModule",
                    ".",
                    "say",
                ]:
                    continue
                words = [token.value for token in argv[0] if token.string]
                if len(words) != 1 or words[0] not in ("name", "job"):
                    continue
                config = argv[2]
                field = next(
                    i
                    for i, token in enumerate(config)
                    if not token.string and token.value == "text"
                )
                value = config[field + 2 : -1]
                if value and value[-1].value == ",":
                    value = value[:-1]
                text = lexer.text_expression(value, {})
                static_keywords.append({"npc": name, "keyword": words[0], "reply": key})
            if set(text) != {"parts"}:
                raise ValueError(f"ISLE_UNQUALIFIED_TEXT_FORMAT:{key}")
            replies[key] = {"lines": text["parts"], "source": source(path, line)}
    helper_path = "data/npclib/npc_system/npc_handler.lua"
    raw = (args.donor / helper_path).read_bytes()
    sources.append({"path": helper_path, "sha256": hashlib.sha256(raw).hexdigest()})
    for event in ("MESSAGE_GREET", "MESSAGE_FAREWELL", "MESSAGE_WALKAWAY"):
        match = re.search(r"\[" + event + r'\]\s*=\s*("(?:[^"\\]|\\.)*")', raw.decode())
        if match is None:
            raise ValueError("ISLE_DEFAULT_MESSAGE_MISSING")
        line = raw.decode()[: match.start()].count("\n") + 1
        key = f"npc_handler:{line}"
        replies[key] = {
            "lines": [json.loads(match.group(1))],
            "source": source(helper_path, line),
        }
        for name in NAMES[:-1]:
            events[name].setdefault(event, key)
    npcs = [
        {
            "key": "oteryn:npc." + name,
            "greet": events[name]["MESSAGE_GREET"],
            "farewell": events[name]["MESSAGE_FAREWELL"],
            "walkaway": events[name]["MESSAGE_WALKAWAY"],
        }
        for name in NAMES[:-1]
    ]
    document = {
        "schema": "OTERYN_ISLE_OF_EVIL_DIALOGUE_V1",
        "source_revision": PIN,
        "sources": sources,
        "npcs": npcs,
        "replies": replies,
        "static_keywords": static_keywords,
    }
    snapshot = json.loads(args.state.read_bytes())
    quests = [quest for quest in snapshot["quests"] if quest["quest"] == QUEST]
    if len(quests) != 1:
        raise ValueError("ISLE_EXISTING_QUEST_IDENTITY")
    tracks = [
        {
            "key": track["key"],
            "initial": track["initial"],
            "min": track["min"],
            "max": track["max"],
            "source": "Oteryn/Oteryn-Game@798e5b1ce27f441d5cbeece333dc7ac45e19d348:content/quests/missions/quest-state.json:1",
        }
        for track in quests[0]["tracks"]
    ]
    if len(tracks) != 11 or any(
        not track["key"].startswith(PREFIX) for track in tracks
    ):
        raise ValueError("ISLE_EXISTING_TRACK_SCOPE")
    for key, file, line in [
        ("rapanaio-travel", "data-global/npc/rapanaio_isle_of_evil.lua", 88),
        (
            "mission04-mustpay",
            "data-global/scripts/quests/the_isle_of_evil_quest/actions_heated_worm_punisher.lua",
            56,
        ),
    ]:
        tracks.append(
            {
                "key": KV_PREFIX + key,
                "initial": 0,
                "min": 0,
                "max": 1,
                "source": source(file, line),
            }
        )

    def track(suffix):
        return KV_PREFIX + suffix[3:] if suffix.startswith("kv:") else PREFIX + suffix

    def effect(suffix, old, value):
        return {"track": track(suffix), "from": old, "set": value}

    transitions = []

    def add(key, npc, line, effects):
        transitions.append(
            {
                "key": key,
                "source": source("data-global/npc/" + npc + ".lua", line),
                "effects": effects,
            }
        )

    add("accept-fight", "rapanaio", 187, [effect("questline", None, 1)])
    for q, mission, line in [
        (3, "mission02", 86),
        (5, "mission03", 94),
        (7, "mission04", 105),
        (9, "mission05", 123),
    ]:
        add(
            "mission-" + str(q),
            "rapanaio",
            line,
            [effect("questline", q, q + 1), effect(mission, None, 1)],
        )
    add(
        "prepare-departure",
        "rapanaio",
        134,
        [
            effect("questline", 11, 12),
            effect("mission06", None, 1),
            effect("door_06", None, 1),
        ],
    )
    add(
        "mortimer-map",
        "mortimer",
        637,
        [effect("mission05", 1, 2), effect("door_05", None, 1)],
    )
    add(
        "begin-final-mission",
        "rapanaio_isle_of_evil",
        86,
        [
            effect("questline", 13, 14),
            effect("mission07", None, 1),
            effect("kv:rapanaio-travel", None, 1),
        ],
    )
    add(
        "report-evil-master",
        "rapanaio_isle_of_evil",
        98,
        [effect("questline", 14, 15), effect("mission07", 2, 3)],
    )
    mortimer = (args.donor / "data-global/npc/mortimer.lua").read_text().splitlines()
    blocks = sorted(
        set(
            re.findall(
                r'MsgContains\(message,\s*"([^"]+)"\)', "\n".join(mortimer[:625])
            )
        )
    )
    # Static KeywordHandler roots run before CALLBACK_MESSAGE_DEFAULT.
    static_blocks = re.findall(
        r'keywordHandler:addKeyword\(\{\s*"([^"]+)"', "\n".join(mortimer[650:])
    )
    data = {
        "schema": "OTERYN_ISLE_OF_EVIL_V1",
        "quest": QUEST,
        "tracks": tracks,
        "transitions": transitions,
        "mortimer_prior_keywords": blocks,
        "mortimer_static_keywords": static_blocks,
        "kv_lowering": "Source scoped boolean keys: missing/false -> 0, true -> 1; no legacy storage ID or alias is allocated.",
        "scutty_bug_fix": {
            "question": "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589174504",
            "answer": "https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728",
            "reason": "Mission04MustPay/Mission04WormCount are absent from the exact source Storage table; use only the existing scoped KV meaning, never invented legacy IDs.",
        },
    }
    lexer.write_or_check(
        quest_out, json.dumps(data, ensure_ascii=False, indent=2) + "\n", args.check
    )
    lexer.write_or_check(
        dialogue_out,
        json.dumps(document, ensure_ascii=False, indent=2) + "\n",
        args.check,
    )
    print(
        json.dumps(
            {
                "transitions": len(transitions),
                "existing_tracks": 11,
                "source_kv_tracks": 2,
                "source_texts": len(replies),
            }
        )
    )


if __name__ == "__main__":
    main()
