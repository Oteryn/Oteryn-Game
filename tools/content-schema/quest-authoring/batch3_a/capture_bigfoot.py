"""Capture the reviewed Bigfoot NPC graphs and retain blocked dialogue evidence."""

import argparse
import copy
import hashlib
import json

from build_dialogues import PIN, REPO
from capture import Capture, inventory, topic
from lower_dialogue import Lowering
from lua_blocks import function_body
from lua_tables import _string, assignments, tokenize

ANSWER = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205"
BEER_QUESTION = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589103264"


def expand_rank(c):
    """Expand checkGnomeRank against the old rank, preserving its position in a branch."""
    path = "data/libs/functions/player.lua"
    c.files[path] = hashlib.sha256((c.donor / path).read_bytes()).hexdigest()
    ranks = [(30, 119, 25, 26, 126, ["Gnome Little Helper"]),
             (120, 479, 26, 27, 132, ["Gnome Little Helper", "Gnome Friend"]),
             (480, 1439, 27, 28, 139, ["Gnome Little Helper", "Gnome Friend", "Gnomelike"]),
             (1440, 2147483647, 29, 30, 147, ["Gnome Little Helper", "Gnome Friend", "Gnomelike", "Honorary Gnome"])]
    output = []
    for rule in c.rules:
        markers = [index for index, effect in enumerate(rule["effects"]) if effect["kind"] == "authoring_expand_bigfoot_rank"]
        if not markers:
            output.append(rule)
            continue
        if len(markers) != 1:
            raise ValueError("A rank helper must be a single reviewed source call")
        marker = markers[0]
        deltas = [effect["value"] for effect in rule["effects"][:marker]
                  if effect["kind"] == "add" and effect["track"] == c.track("Rank")]
        if len(deltas) != 1:
            raise ValueError("Rank helper must follow the exact source rank increment")
        for minimum, maximum, previous, after, line, achievements in ranks:
            branch = copy.deepcopy(rule)
            branch["key"] += f"-rank-{after}"
            branch["guards"] += [c.guard("Rank", "between", minimum - deltas[0], maximum - deltas[0]),
                                 c.guard("QuestLine", "le", previous)]
            branch["effects"][marker:marker + 1] = [c.effect("QuestLine", after),
                {"kind": "magic_effect", "effect": "CONST_ME_MAGIC_BLUE"},
                *[{"kind": "achievement", "name": name} for name in achievements]]
            branch["sources"].append(f"{PIN}{path}:{line}")
            output.append(branch)
        rule["effects"].pop(marker)
        output.append(rule)
    c.rules = output


def gnomerik(c):
    npc = "gnomerik"
    # donor bug fix: NeedsBeer is an explicit named source symbol, with value 1
    # here and reset 0 in actions_beer.lua. No numeric legacy storage is invented.
    # QUESTION A-NEEDS-BEER / coordinator ANSWER point 3:
    # https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589103264
    # https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205
    c.track("NeedsBeer", maximum=1, npc=npc, line=68)
    first = len(c.rules)
    c.rule(npc, "beer-required", 68, ["recruit", "test", "result"],
           guards=[c.guard("NeedsBeer", "eq", 1)], say=70)
    # The early source return consumes every message while NeedsBeer=1.
    c.rule(npc, "beer-required-other", 68, guards=[c.guard("NeedsBeer", "eq", 1)], any_message=True)
    c.rule(npc, "recruit-again", 76, ["recruit"], guards=[c.guard("QuestLine", "eq", 5)], say=77)
    c.rule(npc, "recruit-offer", 78, ["recruit"], guards=[c.guard("QuestLine", "eq", 3)], effects=[topic(1)], say=79)
    c.rule(npc, "recruit-block", 75, ["recruit"])
    questions = [(0, 88), (3, 94), (5, 100), (7, 106), (9, 112), (11, 118),
                 (13, 121), (15, 127), (17, 133), (19, 139), (21, 142),
                 (23, 148), (25, 151), (27, 154), (29, 157), (31, 160)]
    for index, (before, say) in enumerate(questions, 1):
        effects = [c.effect("Test", 0)] if before == 0 else []
        c.rule(npc, f"question-{index}", say, ["test"], guards=[c.guard("QuestLine", "eq", 5)],
               effects=effects + [topic(index * 2)], say=say, topics=[before])
    c.rule(npc, "test-block", 84, ["test"])
    correct = {2: ("a", 169), 4: ("c", 225), 6: ("b", 203), 8: ("d", 251),
               10: ("d", 255), 12: ("d", 259), 14: ("b", 207), 16: ("d", 263),
               18: ("a", 173), 20: ("a", 177), 22: ("c", 229), 24: ("c", 233),
               26: ("d", 267), 28: ("a", 181), 30: ("a", 185), 32: ("d", 271)}
    for before in range(0, 33, 2):
        for letter in "abcd":
            answer = correct.get(before)
            good = answer is not None and answer[0] == letter
            say = answer[1] if good else {"a": 190, "b": 212, "c": 238, "d": 276}[letter]
            replies = c.say(npc, say)
            if before == 32 and not good:
                replies += c.say(npc, {"a": 193, "b": 215, "c": 241, "d": 279}[letter])
            c.rule(npc, f"answer-{before}-{letter}", say, [letter],
                   guards=[c.guard("QuestLine", "eq", 5)],
                   effects=([c.effect("Test", 7, "add")] if good else []) + [topic(before + 1)],
                   replies=replies, topics=[before], keyword_mode="exact")
    # The only reachable scores are 0..112 in increments of 7: reset at line 87,
    # sixteen answers and +7 for each correct one. Retain all literal dialogue.
    for score in range(0, 113, 7):
        passed = score >= 100
        if passed:
            fragments = c.strings(npc, 295, 295)
            replies = [fragments[0] + str(score) + fragments[1]]
            effects = [c.effect("QuestLine", 6)]
            line = 295
        else:
            fragments = c.strings(npc, 291, 292)
            replies = [fragments[0] + str(score) + fragments[1], fragments[2]]
            effects = [c.effect("NeedsBeer", 1)]
            line = 288
        c.rule(npc, f"results-{score}", line, ["result"],
               guards=[c.guard("Test", "eq", score)], effects=effects, replies=replies, topics=[33])
    c.rule(npc, "results-block", 286, ["result"])
    c.rule(npc, "accept-recruitment", 300, ["yes"], effects=[c.effect("QuestLine", 5), topic(0)], say=301, topics=[1])
    for rule in c.rules[first:]:
        rule["sources"].append(c.ref(npc, 68))
    c.rule(npc, "greet-initialize", 53, guards=[c.guard("QuestLine", "eq", 1)],
           effects=[c.effect("QuestLine", 3)], replies=[c.strings(npc, 53, 53)[1]], trigger="greet")
    c.rule(npc, "greet", 309, replies=[c.strings(npc, 309, 309)[0]], trigger="greet")


def gnomelvis(c):
    npc = "gnomelvis"
    c.rule(npc, "looking", 62, ["looking"], say=63)
    for create in (True, False):
        effects = [c.effect("QuestLine", 21), c.effect("MelodyStatus", 1)]
        if create:
            for tone in range(1, 8):
                c.track(f"MelodyTone{tone}", maximum=4)
            effects.append({"kind": "bigfoot_soul_melody"})
        c.rule(npc, f"musical-start-{create}", 66, ["musical"],
               guards=[c.guard("QuestLine", "eq", 19), c.guard("MelodyTone1", "lt" if create else "ge", 1)],
               effects=effects, say=67, exclude=["looking"])
    c.rule(npc, "musical-reminder", 79, ["musical"], guards=[c.guard("QuestLine", "eq", 21)],
           replies=c.say(npc, 80) + c.say(npc, 81), exclude=["looking"])
    # The missing third Lua argument is tonumber(nil)=0 in the pinned C++ binding.
    rule = c.rule(npc, "musical-complete", 82, ["musical"], guards=[c.guard("QuestLine", "eq", 22)],
           effects=[c.effect("QuestLine", 25), c.effect("QuestLineComplete", 2), c.effect("Rank", 0),
                    {"kind": "achievement", "name": "Becoming a Bigfoot"}], say=83, exclude=["looking"])
    rule["sources"] += [f"{PIN}src/lua/functions/creatures/player/player_functions.cpp:2727",
                        f"{PIN}src/lua/functions/lua_functions_loader.hpp:74"]
    c.rule(npc, "musical-complete-reminder", 91, ["musical"], guards=[c.guard("QuestLine", "eq", 25)], say=92, exclude=["looking"])


def gnominus(c):
    npc = "gnominus"
    lower = Lowering(c, npc, "Storage.Quest.U9_60.BigfootsBurden")
    lower.lower(62, 78)
    c.rule(npc, "beer-payment-blocked", 81, ["yes"], guards=[{"kind": "money", "amount": 10, "include_bank": True}],
           effects=[{"kind": "take_money", "amount": 10, "include_bank": True}, inventory("give_item", 15794), topic(0)],
           say=82, topics=[2], exclude=["recruitment", "tavern", "beer"])
    c.rule(npc, "beer-poor", 89, ["yes"], effects=[topic(0)], say=89, topics=[2], exclude=["recruitment", "tavern", "beer"])
    c.rule(npc, "beer-decline", 92, effects=[topic(0)], say=92, topics=[2], any_message=True, exclude=["recruitment", "tavern", "beer"])


def dialogue_evidence(c, npcs):
    """Full literal pieces, including dynamic templates, never an executable fallback."""
    entries = []
    for npc in npcs:
        source = c.read(npc)
        tokens = tokenize(source)
        fragments = []
        for index, token in enumerate(tokens):
            if token[1] not in {"say", "setMessage"} or index < 1 or tokens[index - 1][1] != ":":
                continue
            cursor = index + 1
            if tokens[cursor][1] != "(":
                continue
            depth = 1
            parts = []
            cursor += 1
            while cursor < len(tokens) and depth:
                value = tokens[cursor]
                if value[0] in {"string", "lstring"}:
                    parts.append(_string(value))
                elif value[1] == "(":
                    depth += 1
                elif value[1] == ")":
                    depth -= 1
                cursor += 1
            fragments.append({"line": token[2], "literal_parts": parts, "source": c.ref(npc, token[2])})
        entries.append({"npc": "oteryn:npc." + npc, "fragments": fragments,
                        "source_sha256": hashlib.sha256(source.encode()).hexdigest()})
    document = {"schema": 1, "quest": c.key, "runtime_enabled": False,
                "purpose": "Complete source dialogue evidence for blocked native and exchange branches; not a second handler.",
                "decision": ANSWER, "npcs": entries}
    # Gnomally's dynamic offers remain data with their exact tokens and reward IDs.
    document["gnomally_offers_literal_table"] = assignments(c.read("gnomally"), {"config"})["config"]
    path = REPO / "content/dialogues/quest-source/quest-batch3-a-bigfoot-burden.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")


def capture(donor):
    c = Capture(donor, "bigfoot_burden", "quest/u9_60/bigfoots_burden")
    for npc, first in [("xelvar", 60), ("gnomespector", 65), ("gnomaticus", 61),
                       ("gnomewart", 61), ("gnomission", 61), ("commander_stone", 61), ("gnomeral", 65)]:
        lower = Lowering(c, npc, "Storage.Quest.U9_60.BigfootsBurden")
        if npc == "gnomeral":
            lower.statement_overrides[85] = [{"kind": "bigfoot_matchmaker_target"}]
            c.track("MatchmakerIdNeeded")
        start = next(index for index, line in enumerate(lower.lines, 1) if line.startswith("local function creatureSayCallback"))
        lower.lower(first, function_body(lower.lines, start)[-1])
    expand_rank(c)
    gnomerik(c)
    gnomelvis(c)
    gnominus(c)
    for keyword, line in [("equipment", 107), ("major", 112), ("minor", 117)]:
        c.rule("gnomally", f"information-{keyword}", line, [keyword], say=line,
               exclude=[name for name, previous in [("equipment", 107), ("major", 112), ("minor", 117)] if previous < line])
    dialogue_evidence(c, ["gnomally", "doctor_gnomedix", "gnominus", "gnomerik"])
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
