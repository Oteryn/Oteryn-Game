"""Source task values and all thirteen Spike NPC dialogue archives."""

import re

from lua_tables import Parser, _string, as_python, assignments, tokenize


def task_records():
    # Each tuple binds the reviewed source branch, not a keyword-nearness heuristic.
    tiers = [
        (
            "Upper",
            "gnomilly",
            25,
            49,
            1000,
            [
                ("Pacifier", "pacifiers", "pacifiers", 7, 19204, 147, 173, 88, 100),
                ("Mound", "release", "release", 1, 19203, 179, 205, 101, 112),
                ("Track", "track", "tracking", 3, 19205, 211, 244, 113, 124),
                ("Kill", "kill", "killing", 7, None, 248, 274, 125, 136),
            ],
        ),
        (
            "Middle",
            "gnombold",
            50,
            79,
            2000,
            [
                ("Charge", "charges", "charges", 3, 19207, 145, 176, 89, 100),
                (
                    "Mushroom",
                    "fertilise",
                    "fertilisation",
                    4,
                    19214,
                    181,
                    208,
                    101,
                    112,
                ),
                ("Nest", "nests", "nests", 5, None, 213, 239, 113, 124),
                ("Kill", "kill", "killing", 7, None, 244, 269, 125, 136),
            ],
        ),
        (
            "Lower",
            "gnomargery",
            80,
            None,
            3500,
            [
                ("Parcel", "deliver", "delivery", 4, 19219, 150, 177, 91, 103),
                ("Undercover", "undercover", "undercover", 3, None, 182, 207, 104, 115),
                ("Lava", "temperature", "temperature", 1, 19206, 213, 240, 116, 127),
                ("Kill", "kill", "kill", 7, None, 245, 271, 128, 139),
            ],
        ),
    ]
    records = []
    prefix = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/"
    for tier, npc, lo, hi, xp, tasks in tiers:
        for task, start, report, threshold, item, sl, sh, rl, rh in tasks:
            records.append(
                {
                    "tier": tier.lower(),
                    "task": task.lower(),
                    "npc": npc,
                    "start_keyword": start,
                    "report_keyword": report,
                    "min_level": lo,
                    "max_level": hi,
                    "threshold": threshold,
                    "experience": xp,
                    "cooldown_seconds": 72000,
                    "grant_item": item,
                    "grant_count": 4 if task == "Parcel" else (1 if item else 0),
                    "storage_prefix": "Storage.Quest.U10_20.SpikeTaskQuest.Spike_"
                    + tier
                    + "_"
                    + task,
                    "source": [
                        prefix + f"{npc}.lua:{sl}-{sh}",
                        prefix + f"{npc}.lua:{rl}-{rh}",
                    ],
                }
            )
    return records


def build(source):
    SOURCE = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"
    tasks = task_records()
    replies = {}
    calls = {}
    set_messages = {}
    texts = {}
    names = source.cohort("spike_tasks")
    for filename in names:
        name = filename[:-4]
        text = source.read("data-global/npc/" + filename)
        texts[name] = text
        tokens = [t for t in tokenize(text) if t[0] not in ("comment", "lcomment")]
        calls[name] = []
        set_messages[name] = []
        for i in range(len(tokens) - 4):
            if [t[1] for t in tokens[i : i + 4]] not in [
                ["npcHandler", ":", "say", "("],
                ["npcHandler", ":", "setMessage", "("],
            ]:
                continue
            method = tokens[i + 2][1]
            j = i + 4
            depth = 0
            args = []
            arg = []
            while j < len(tokens):
                t = tokens[j]
                if t[1] == ")" and depth == 0:
                    args.append(arg)
                    break
                if t[1] == "," and depth == 0:
                    args.append(arg)
                    arg = []
                else:
                    arg.append(t)
                    if t[1] in ("(", "{", "["):
                        depth += 1
                    elif t[1] in (")", "}", "]"):
                        depth -= 1
                j += 1
            arg = args[0 if method == "say" else 1]
            line = tokens[i][2]
            end = tokens[j][2]
            key = f"{name}:{line}"
            citation = SOURCE + f"data-global/npc/{filename}:{line}-{end}"
            expression = "".join(t[1] for t in arg)
            if len(arg) == 1 and arg[0][0] in ("string", "lstring"):
                reply = {
                    "kind": "static",
                    "lines": [_string(arg[0])],
                    "source": citation,
                }
            elif arg[0][1] == "{":
                value = as_python(Parser(expression).value())
                assert isinstance(value, list) and all(
                    isinstance(x, str) for x in value
                ), (name, line, value)
                reply = {"kind": "static", "lines": value, "source": citation}
            elif "string.diff(" in expression:
                strings = [_string(t) for t in arg if t[0] in ("string", "lstring")]
                assert len(strings) == 2
                reply = {
                    "kind": "wait_time",
                    "lines": [strings[0] + "{remaining}" + strings[1]],
                    "source": citation,
                }
            elif "levels[1]" in expression:
                strings = [_string(t) for t in arg if t[0] in ("string", "lstring")]
                assert len(strings) == 3
                reply = {
                    "kind": "level_range",
                    "lines": [
                        strings[0] + "{minimum}" + strings[1] + "{maximum}" + strings[2]
                    ],
                    "source": citation,
                }
            elif "..level.." in expression:
                strings = [_string(t) for t in arg if t[0] in ("string", "lstring")]
                assert len(strings) == 2
                reply = {
                    "kind": "minimum_level",
                    "lines": [strings[0] + "{minimum}" + strings[1]],
                    "source": citation,
                }
            elif expression == "response[parcelStatus]":
                calls[name].append(
                    {"key": key, "line": line, "end": end, "dynamic": "parcel_response"}
                )
                continue
            else:
                raise RuntimeError((name, line, expression))
            replies[key] = reply
            row = {"key": key, "line": line, "end": end}
            (calls if method == "say" else set_messages)[name].append(row)
    # The parcel response table is keyed by progress0..3, including literal key0.
    response = as_python(assignments(texts["gnome_trooper"], ["response"])["response"])
    parcel_replies = []
    for state, text in response.items():
        key = f"gnome_trooper:parcel-{state}"
        replies[key] = {
            "kind": "static",
            "lines": [text],
            "source": SOURCE + "data-global/npc/gnome_trooper.lua:53-58",
        }
        parcel_replies.append({"state": int(state), "reply": key})
    officers = []
    for name in ["gnomilly", "gnombold", "gnomargery"]:
        lines = texts[name].splitlines()
        own = [t for t in tasks if t["npc"] == name]
        first_start = min(
            int(t["source"][0].rsplit(":", 1)[1].split("-")[0]) for t in own
        )
        mission_line = next(
            i
            for i, s in enumerate(lines, 1)
            if 'if MsgContains(message, "mission")' in s
        )
        report_line = next(
            i
            for i, s in enumerate(lines, 1)
            if 'if MsgContains(message, "report")' in s
        )
        info = []
        for i, s in enumerate(lines[: mission_line - 1], 1):
            m = re.search(r'if MsgContains\(message, "([^"]+)"\)', s)
            if m:
                candidate = next(c for c in calls[name] if i < c["line"] < mission_line)
                info.append({"keyword": m[1], "reply": candidate["key"]})
        mission = [
            c["key"] for c in calls[name] if mission_line < c["line"] < report_line
        ]
        assert len(mission) == 2
        report = next(c["key"] for c in calls[name] if report_line < c["line"])
        invalid = [c["key"] for c in calls[name] if c["line"] < first_start][-1]
        staged = []
        for task in own:
            lo, hi = map(int, task["source"][0].rsplit(":", 1)[1].split("-"))
            rlo, rhi = map(int, task["source"][1].rsplit(":", 1)[1].split("-"))
            start = [c["key"] for c in calls[name] if lo <= c["line"] <= hi]
            report_replies = [c["key"] for c in calls[name] if rlo <= c["line"] <= rhi]
            assert len(start) == 6, (name, task["task"], start)
            assert len(report_replies) == 3, (name, task["task"], report_replies)
            topic = re.search(
                r'talkState\[playerId\] = "([^"]+)"', "\n".join(lines[lo - 1 : hi])
            )[1]
            staged.append(
                {
                    "task": task["task"],
                    "topic": topic,
                    "cooldown": start[0],
                    "level": start[1],
                    "offer": start[2],
                    "started": start[3],
                    "accept": start[4],
                    "decline": start[5],
                    "report_absent": report_replies[0],
                    "report_success": report_replies[1],
                    "report_incomplete": report_replies[2],
                }
            )
        officers.append(
            {
                "npc": "oteryn:npc." + name,
                "tier": own[0]["tier"],
                "info": info,
                "mission_refusal": mission[0],
                "mission_offer": mission[1],
                "report_prompt": report,
                "invalid_report": invalid,
                "greeting": set_messages[name][-1]["key"],
                "tasks": staged,
            }
        )
    contacts = [
        {"npc": "oteryn:npc." + name, "reply": set_messages[name][0]["key"]}
        for name in sorted(calls)
        if name.startswith("a_")
    ]
    document = {
        "schema": "OTERYN_SPIKE_DIALOGUE_V1",
        "replies": replies,
        "officers": officers,
        "contacts": contacts,
        "parcel_replies": parcel_replies,
    }
    return {
        "content/quests/spike-tasks.json": tasks,
        "content/dialogues/quests/spike-tasks.json": document,
    }
