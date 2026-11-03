"""Preserve Bozo's literal tables and every own NPC source reply; no runtime model."""

from lua_tables import _string, as_python, assignments, tokenize


def build(source):
    path = "data-global/npc/bozo.lua"
    text = source.read(path)
    tables = assignments(text, ["config", "jesterOutfit"])
    config = as_python(tables["config"])
    fields = tables["config"]["fields"]
    outfits = as_python(tables["jesterOutfit"])
    prefix = (
        "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:" + path + ":"
    )
    replies = {}
    for index, stage in enumerate(config, 1):
        start = fields[index - 1]["line"]
        end = fields[index]["line"] - 1 if index < 36 else 561
        parts = (
            stage["text"].items()
            if isinstance(stage["text"], dict)
            else enumerate(stage["text"], 1)
        )
        for key, part in parts:
            replies[f"stage-{index}-{key}"] = {
                "lines": [part] if isinstance(part, str) else part,
                "source": prefix + f"{start}-{end}",
            }
    for current, stage in outfits.items():
        for key, part in enumerate(stage["text"], 1):
            replies[f"outfit-{current}-{key}"] = {
                "lines": [part] if isinstance(part, str) else part,
                "source": prefix + "563-601",
            }
    raw = text.splitlines()
    for key, lo, hi in [
        ("join", 630, 630),
        ("join-again", 626, 626),
        ("join-confirm", 672, 674),
        ("joined", 681, 685),
        ("outfit-failure", 761, 761),
        ("cookie-failure", 716, 716),
        ("pie-failure", 724, 724),
        ("decline-join", 781, 781),
        ("lost-beard", 787, 788),
        ("decline-cookies", 791, 791),
        ("lost-disguise", 795, 795),
        ("decline", 797, 797),
        ("outfit-already", 667, 667),
    ]:
        replies[key] = {
            "lines": [
                _string(token)
                for token in tokenize("\n".join(raw[lo - 1 : hi]))
                if token[0] in ("string", "lstring")
            ],
            "source": prefix + f"{lo}-{hi}",
        }
    return {
        "content/dialogues/quests/what-a-foolish.json": {
            "schema": "OTERYN_FOOLISH_DIALOGUE_V1",
            "replies": replies,
        }
    }
