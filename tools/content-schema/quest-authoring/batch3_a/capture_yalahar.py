"""Exact quest-only conversation scopes for In Service of Yalahar."""

import argparse

from capture import Capture
from lower_dialogue import Lowering
from lua_blocks import function_body


def repair_palimuth_job_line(line):
    # donor bug fix: the exact negated equality in Palimuth's job branch.
    # The following elseif explicitly handles Questline==54 in the same file.
    # https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589698604
    # Coordinator selected B in ANSWER #6589900671; source palimuth.lua:65/68.
    # https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589900671
    old = "not player:getStorageValue(Storage.Quest.U8_4.InServiceOfYalahar.Questline) == 54"
    if line.count(old) != 1:
        raise ValueError("Palimuth's pinned job condition changed")
    return line.replace(old, old.removeprefix("not ").replace("== 54", "~= 54"))


def capture(donor):
    c = Capture(donor, "in_service_of_yalahar", "quest/u8_4/in_service_of_yalahar")
    storage = "Storage.Quest.U8_4.InServiceOfYalahar"
    palimuth = Lowering(c, "palimuth", storage)
    palimuth.lines[64] = repair_palimuth_job_line(palimuth.lines[64])
    palimuth.lower(61, 63, "initialize", continue_matching=True)
    palimuth.lower(65, 336, "conversation")
    for npc, first in [("yalahari", 61), ("tamerin", 75), ("mr._west", 73), ("maritima", 56),
                       ("tony", 61), ("hal", 61), ("oliver", 61), ("bruce", 61), ("barry", 61),
                       ("reed", 61), ("timothy", 61), ("karith", 80), ("siflind", 194)]:
        lower = Lowering(c, npc, storage)
        start = next(i for i, line in enumerate(lower.lines, 1) if line.startswith("local function creatureSayCallback"))
        lower.lower(first, function_body(lower.lines, start)[-1])
    # Existing keyword travel is evaluated before Peter's default callback.
    for line, keyword, destination in [(85, "factory", (32859, 31302, -7)), (86, "trade", (32854, 31302, -7))]:
        c.rule("peter", f"direct-{keyword}", line, [keyword], effects=[{"kind": "teleport", "position": dict(zip(("x", "y", "floor"), destination))}])
    Lowering(c, "peter", storage).lower(61, 81)
    wyrdin = Lowering(c, "wyrdin", storage)
    wyrdin.aliases["Storage.Quest.U7_6.ExplorerSociety.QuestLine"] = "oteryn:quest-progress/quest/u7_6/explorer_society/quest_line"
    wyrdin.aliases["Storage.Quest.U7_6.ExplorerSociety.JoiningTheExplorers"] = "oteryn:quest-progress/quest/u7_6/explorer_society/joining_the_explorers"
    wyrdin.lower(143, 169)
    for questline, line, reply in [(30, 58, "Have you the {animal cure}?"), (31, 60, "Have you killed {morik}?")]:
        c.rule("tamerin", f"greet-{questline}", line, guards=[c.guard("Questline", "eq", questline)], replies=[reply], trigger="greet")
    c.rule("tamerin", "greet-default", 63, replies=["Hello, what brings you here?"], trigger="greet")
    # Greeting setters are full donor literals; the counter write happens only
    # on the first greeting, not on every route inquiry.
    c.rule("karith", "greet-initialize-counter", 63, guards=[c.guard("SearoutesAroundYalahar.TownsCounter", "eq", -1)],
           effects=[c.effect("SearoutesAroundYalahar.TownsCounter", 0)], replies=["Hello! Tell me what's on your mind. Time is money."], trigger="greet")
    c.rule("karith", "greet", 67, replies=["Hello! Tell me what's on your mind. Time is money."], trigger="greet")
    # Mr. West's conditional greeting is separate from his mission callback.
    c.rule("mr._west", "greet-peaceful-entry", 57, guards=[c.guard("MrWestDoor", "eq", 1)],
           replies=c.strings("mr._west", 58, 58), trigger="greet")
    c.rule("mr._west", "greet-violent-entry", 59, guards=[c.guard("MrWestDoor", "eq", 2)],
           replies=c.strings("mr._west", 60, 60), trigger="greet")
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
