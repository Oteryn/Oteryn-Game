"""Reviewed Dark Trails bindings, with complete literal source messages."""

from lua_tables import _string, tokenize


def build(source):
    C = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:"
    P = "oteryn:quest-progress/quest/u10_50/dark_trails/"
    Q = "oteryn:quest.dark_trails_quest"
    rules = []
    texts = {}

    def field(s):
        import re

        return re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()

    def track(s):
        return P + field(s)

    def say(npc, lo, hi):
        chunk = "\n".join(
            source.read(f"data-global/npc/{npc}.lua").splitlines()[lo - 1 : hi]
        )
        return [_string(t) for t in tokenize(chunk) if t[0] in ("string", "lstring")]

    def add(key, npc, keyword, topic, next_topic, guards, sets, span, replyspan=None):
        lo, hi = span
        replyspan = replyspan or span
        source = C + f"data-global/npc/{npc}.lua:{lo}-{hi}"
        rules.append(
            {
                "key": key,
                "npc": "oteryn:npc." + npc,
                "keyword": keyword,
                "topic": topic,
                "next_topic": next_topic,
                "greeting_only": npc == "quandons_ghost",
                "guards": [
                    {"track": track(k), "minimum": a, "maximum": b}
                    for k, a, b in guards
                ],
                "sets": [{"track": track(k), "value": v} for k, v in sets],
                "reply": key,
                "sources": [source],
            }
        )
        texts[key] = {
            "lines": say(npc, *replyspan),
            "source": C + f"data-global/npc/{npc}.lua:{replyspan[0]}-{replyspan[1]}",
        }

    add(
        "ezebeth-invite",
        "ezebeth",
        "mission",
        None,
        1,
        [("Mission01", -1, -1)],
        [],
        (61, 64),
        (63, 63),
    )
    add(
        "ezebeth-already",
        "ezebeth",
        "mission",
        None,
        0,
        [("Mission01", 0, 2)],
        [],
        (65, 67),
        (66, 66),
    )
    add("ezebeth-assistance", "ezebeth", "assistance", 1, 2, [], [], (69, 72), (71, 71))
    add(
        "ezebeth-start",
        "ezebeth",
        "strange",
        2,
        0,
        [],
        [("Mission01", 1)],
        (74, 78),
        (76, 76),
    )
    add(
        "beggar-want",
        "a_beggar",
        "want",
        None,
        1,
        [("Mission01", 1, 1)],
        [],
        (60, 64),
        (64, 64),
    )
    add("beggar-want-other", "a_beggar", "want", None, None, [], [], (60, 64), (64, 64))
    add(
        "beggar-report",
        "a_beggar",
        "yes",
        1,
        0,
        [],
        [("Mission01", 2), ("Mission02", 1)],
        (65, 76),
        (68, 72),
    )
    for npc, first, report, line in [
        ("jacob", 59, 114, 118),
        ("elliott", 59, 111, 115),
    ]:
        add(
            npc + "-investigate",
            npc,
            "abandoned sewers",
            None,
            7,
            [("Mission05", 1, 1)],
            [],
            (57, 60),
            (first, first),
        )
        add(
            npc + "-report",
            npc,
            "report",
            7,
            0,
            [("Mission05", 1, 1)],
            [("Mission06", 1)],
            (report, report + 9),
            (line, line + 1),
        )
        add(
            npc + "-already",
            npc,
            "report",
            None,
            None,
            [("Mission05", 1, 1)],
            [],
            (report + 9, report + 11),
            (line + 6, line + 6),
        )
    add(
        "barazbaz-ritual",
        "barazbaz",
        "ritual",
        None,
        0,
        [("Mission06", 1, 1), ("Mission07", -1, 0)],
        [("Mission07", 1)],
        (57, 66),
        (59, 63),
    )
    add(
        "barazbaz-archives",
        "barazbaz",
        "abandoned sewers",
        None,
        0,
        [("Mission08", 1, 1), ("Mission09", -1, 0)],
        [("Mission09", 1)],
        (67, 77),
        (69, 74),
    )
    add(
        "roswitha-harsin",
        "roswitha",
        "harsin",
        None,
        0,
        [("Mission13", 1, 1), ("Mission14", -1, 0)],
        [("Mission14", 1), ("DoorQuandon", 1)],
        (57, 62),
        (59, 59),
    )
    add(
        "sholley-quandon",
        "sholley",
        "quandon",
        None,
        0,
        [("Mission14", 2, 2), ("Mission15", -1, 0)],
        [("Mission15", 1)],
        (66, 73),
        (69, 70),
    )
    add(
        "sholley-body-missing",
        "sholley",
        "quandon",
        None,
        0,
        [],
        [],
        (74, 76),
        (75, 75),
    )
    add(
        "terrence-manway",
        "terrence",
        "manway",
        None,
        1,
        [("Mission16", 1, 1), ("Mission17", -1, 0)],
        [],
        (57, 60),
        (59, 59),
    )
    add(
        "terrence-manway-denied",
        "terrence",
        "manway",
        None,
        0,
        [],
        [],
        (61, 63),
        (62, 62),
    )
    add(
        "terrence-effort",
        "terrence",
        "effort",
        1,
        0,
        [],
        [("Mission17", 1), ("DoorHideout", 1)],
        (65, 69),
        (66, 66),
    )
    # The native NPC placement must be source-qualified independently. The rule itself
    # retains CALLBACK_GREET semantics and is selected only after ordinary greeting.
    add(
        "ghost-witness",
        "quandons_ghost",
        "hi",
        None,
        0,
        [("Mission15", 3, 3), ("Mission16", -1, 0)],
        [("Mission16", 1)],
        (53, 63),
        (59, 61),
    )
    add(
        "ghost-witness-hello",
        "quandons_ghost",
        "hello",
        None,
        0,
        [("Mission15", 3, 3), ("Mission16", -1, 0)],
        [("Mission16", 1)],
        (53, 63),
        (59, 61),
    )
    add(
        "ghost-greet-other", "quandons_ghost", "hi", None, 0, [], [], (64, 65), (65, 65)
    )
    add(
        "ghost-hello-other",
        "quandons_ghost",
        "hello",
        None,
        0,
        [],
        [],
        (64, 65),
        (65, 65),
    )

    tracks = []
    for i in range(1, 19):
        maximum = {1: 2, 2: 2, 14: 2, 15: 3}.get(i, 1)
        tracks.append(
            {
                "key": track(f"Mission{i:02}"),
                "initial": -1,
                "minimum": -1,
                "maximum": maximum,
                "source": C + f"data-global/lib/core/storages.lua:{2152 + i}",
            }
        )
    for name, line in [
        ("Outfit", 2171),
        ("DoorQuandon", 2172),
        ("DoorHideout", 2173),
        ("OramondTaskProbing", 2177),
    ]:
        tracks.append(
            {
                "key": track(name),
                "initial": -1,
                "minimum": -1,
                "maximum": 1,
                "source": C + f"data-global/lib/core/storages.lua:{line}",
            }
        )

    branches = {
        "oteryn:npc.ezebeth": ["mission", "assistance", "strange", "outfit", "addon"],
        "oteryn:npc.a_beggar": ["want", "yes"],
        "oteryn:npc.jacob": ["abandoned sewers", "mission", "yes", "ok", "report"],
        "oteryn:npc.elliott": ["abandoned sewers", "mission", "yes", "ok", "report"],
        "oteryn:npc.sholley": ["friend", "quandon"],
        "oteryn:npc.terrence": ["manway", "effort"],
    }
    return {
        "content/quests/dark-trails.json": {
            "schema": "OTERYN_DARK_TRAILS_V1",
            "quest": Q,
            "tracks": tracks,
            "rules": rules,
            "branches": branches,
        },
        "content/dialogues/quests/dark-trails.json": {
            "schema": "OTERYN_DARK_TRAILS_DIALOGUE_V1",
            "replies": texts,
        },
    }
