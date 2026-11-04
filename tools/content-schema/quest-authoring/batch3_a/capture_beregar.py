"""Reviewed Beregar conversation graph, captured from Crystal 201189f.

Only the quest's branches are included. Local keyword-tree nodes use conversation
topics 7101..7103; these replace the donor's ephemeral keyword node, not storage.
"""

import argparse

from capture import Capture, inventory, topic


def capture(donor):
    c = Capture(donor, "the_hidden_city_of_beregar", "quest/u8_4/the_hidden_city_of_beregar")
    g, e, r = c.guard, c.effect, c.rule
    r("nokmir", "mission-offer", 60, ["mission"], [g("JusticeForAll", "lt", 1)], [topic(1)], say=62)
    r("nokmir", "mission-cleared", 64, ["mission"], [g("JusticeForAll", "eq", 5)], [e("JusticeForAll", 6), e("DoorNorthMine", 1)], say=67)
    r("nokmir", "ring", 69, ["ring"], effects=[topic(2)], say=71, topics=[1], exclude=["mission"])
    r("nokmir", "grombur", 77, ["grombur"], effects=[topic(3)], say=79, topics=[2], exclude=["mission", "ring"])
    r("nokmir", "rerun", 82, ["rerun"], effects=[topic(4)], say=84, topics=[3], exclude=["mission", "ring", "grombur"])
    r("nokmir", "rehal", 87, ["rehal"], effects=[e("DefaultStart", 1), e("JusticeForAll", 1), topic(0)], say=91, topics=[4], exclude=["mission", "ring", "grombur", "rerun"])

    r("emperor_rehal", "hail-emperor", 63, ["hail emperor"], say=68, trigger="greet")
    r("emperor_rehal", "reject-other-greeting", 63, effects=[{"kind": "end_conversation", "delay_ms": 1000}], trigger="greet", exclude=["hail emperor"])
    r("emperor_rehal", "nokmir", 78, ["nokmir"], [g("JusticeForAll", "eq", 1)], [topic(1)], say=80)
    r("emperor_rehal", "ring-evidence", 82, ["nokmir"], [g("JusticeForAll", "eq", 4), inventory("item", 8777)], [inventory("take_item", 8777), e("JusticeForAll", 5)], say=[83, 84])
    r("emperor_rehal", "grombur", 87, ["grombur"], effects=[e("JusticeForAll", 2), topic(0)], say=89, topics=[1], exclude=["nokmir"])
    r("emperor_rehal", "royal-rescue-offer", 93, ["mission"], [g("RoyalRescue", "lt", 1), g("JusticeForAll", "gt", 4)], [topic(2)], say=95, exclude=["nokmir", "grombur"])
    r("emperor_rehal", "royal-rescue-reward-offer", 97, ["mission"], [g("RoyalRescue", "eq", 7)], [topic(3)], say=98, exclude=["nokmir", "grombur"])
    r("emperor_rehal", "royal-rescue-accept", 101, ["yes"], effects=[e("RoyalRescue", 1), topic(0)], say=103, topics=[2], exclude=["nokmir", "grombur", "mission"])
    r("emperor_rehal", "royal-rescue-reward", 109, ["yes"], effects=[inventory("give_item", 3398), e("RoyalRescue", 8), topic(0)], say=110, topics=[3], exclude=["nokmir", "grombur", "mission"])
    r("emperor_rehal", "decline", 115, ["no"], effects=[topic(0)], say=117, topics=[1, 2], exclude=["nokmir", "grombur", "mission", "yes"])

    r("grombur", "nokmir", 60, ["nokmir"], [g("JusticeForAll", "eq", 2)], [topic(1)], say=62)
    r("grombur", "rerun", 65, ["rerun"], effects=[e("JusticeForAll", 3), topic(0)], say=68, topics=[1], exclude=["nokmir"])
    r("grombur", "ale-offer", 71, ["mission"], [g("TheGoodGuard", "lt", 1)], [topic(2)], say=73, exclude=["nokmir", "rerun"])
    r("grombur", "ale-deliver", 75, ["mission"], [g("TheGoodGuard", "eq", 1), inventory("item", 8774)], [inventory("take_item", 8774), e("TheGoodGuard", 2), e("DoorSouthMine", 1)], say=78, exclude=["nokmir", "rerun"])
    r("grombur", "ale-accept", 80, ["yes"], effects=[e("TheGoodGuard", 1), e("DefaultStart", 1), topic(0)], say=84, topics=[2], exclude=["nokmir", "rerun", "mission"])

    r("frafnar", "cake-offer", 60, ["mission"], [g("SweetAsChocolateCake", "lt", 1)], [topic(1)], say=62)
    r("frafnar", "cake-delivered", 64, ["mission"], [g("SweetAsChocolateCake", "eq", 2)], [topic(2)], say=65)
    r("frafnar", "cake-accept", 68, ["yes"], effects=[e("SweetAsChocolateCake", 1), e("DefaultStart", 1), topic(0)], say=72, topics=[1], exclude=["mission"])
    r("frafnar", "cake-report", 74, ["yes"], effects=[topic(3), e("SweetAsChocolateCake", 3), e("DoorWestMine", 1)], say=75, topics=[2], exclude=["mission"])
    r("frafnar", "cake-sent-no", 83, ["no"], effects=[topic(0)], say=84, topics=[3], exclude=["mission", "yes"])

    r("bolfona", "cake-offer", 60, ["chocolate cake"], [g("SweetAsChocolateCake", "eq", 1), inventory("item", 8019)], [topic(1)], say=62)
    r("bolfona", "cake-already-delivered", 64, ["chocolate cake"], [g("SweetAsChocolateCake", "eq", 2)], [topic(2)], say=65)
    r("bolfona", "cake-delivery", 70, ["yes"], [inventory("item", 8019)], [inventory("take_item", 8019), topic(2), e("SweetAsChocolateCake", 2)], say=71, topics=[1], exclude=["chocolate cake"])
    r("bolfona", "cake-missing", 74, ["yes"], effects=[topic(0)], say=75, topics=[1], exclude=["chocolate cake"])
    r("bolfona", "sender-frafnar", 80, ["Frafnar"], effects=[topic(0)], say=81, topics=[2], exclude=["chocolate cake", "yes"])
    r("bolfona", "sender-other", 83, effects=[topic(0)], say=84, topics=[2], exclude=["chocolate cake", "yes", "Frafnar"], any_message=True)

    r("xorlosh", "elevator-offer", 60, ["mission"], [g("GoingDown", "lt", 1)], [topic(1)], say=62)
    r("xorlosh", "elevator-deliver", 64, ["mission"], [g("GoingDown", "eq", 1), inventory("item", 8775, 3)], [inventory("take_item", 8775, 3), e("GoingDown", 2)], say=66)
    r("xorlosh", "elevator-accept", 68, ["yes"], effects=[e("GoingDown", 1), e("DefaultStart", 1), topic(0)], say=72, topics=[1], exclude=["mission"])
    r("xorlosh", "tunnel", 75, ["tunnel"], [g("RoyalRescue", "eq", 1)], say=77, exclude=["mission", "yes"])
    r("xorlosh", "book", 82, ["book"], [g("RoyalRescue", "eq", 1)], say=84, exclude=["mission", "yes", "tunnel"])
    r("tehlim", "rescue-offer", 61, ["mission"], [g("RoyalRescue", "eq", 5)], [topic(1)], say=63)
    r("tehlim", "rescue-accept", 66, ["yes"], effects=[e("RoyalRescue", 6), topic(0)], say=69, topics=[1], exclude=["mission"])
    r("rehon", "cell-key", 56, ["mission"], [g("RoyalRescue", "eq", 6), inventory("item", 9172)], [inventory("take_item", 9172), e("RoyalRescue", 7), topic(0)], say=60)

    # The keyword callback always speaks, and only the first call writes storage.
    mistrock = c.strings("maris_mistrock", 73, 74)
    r("maris_mistrock", "hidden-path", 67, ["mistrock"], [g("WayToBeregar", "ne", 1)], [e("WayToBeregar", 1)], replies=mistrock)
    r("maris_mistrock", "hidden-path-repeat", 67, ["mistrock"], replies=mistrock)

    # Keyword-handler child context is encoded in session-only topics.
    def keyword_text(line):
        return c.strings("pythius_the_rotten", line, line)[-1]
    r("pythius_the_rotten", "treasure", 48, ["treasure"], effects=[topic(7101)], replies=[keyword_text(48)])
    r("pythius_the_rotten", "treasure-yes", 49, ["yes"], effects=[topic(0)], topics=[7101], replies=[keyword_text(49)])
    r("pythius_the_rotten", "treasure-no", 50, ["no"], topics=[7101], replies=[keyword_text(50)])
    r("pythius_the_rotten", "dungeon-offer", 52, ["offer"], [{"kind": "level", "op": "gt", "value": 99}], [topic(7102)], replies=[keyword_text(52)])
    r("pythius_the_rotten", "dungeon-underlevel", 65, ["offer"], replies=[keyword_text(65)])
    r("pythius_the_rotten", "dungeon-yes", 55, ["yes"], topics=[7102], effects=[topic(7103)], replies=[keyword_text(55)])
    c.track("PythiusTheRotten", maximum=9223372036854775807)
    r("pythius_the_rotten", "golden-mug", 56, ["golden mug"], [inventory("item", 2903)], [inventory("take_item", 2903), e("PythiusTheRotten", 180, "set_now_plus"), topic(0)], topics=[7103], replies=[keyword_text(56)])
    r("pythius_the_rotten", "golden-mug-missing", 62, ["golden mug"], topics=[7103], effects=[topic(0)], replies=[keyword_text(62)])
    r("pythius_the_rotten", "unwanted-treasure", 63, topics=[7103], effects=[topic(0)], replies=[keyword_text(63)], any_message=True)
    r("pythius_the_rotten", "offer-declined", 64, topics=[7102], effects=[topic(0)], replies=[keyword_text(64)], any_message=True)
    for line, keyword in [(68, "awaited"), (69, "exchange"), (70, "mission"), (71, "undead")]:
        r("pythius_the_rotten", f"lore-{keyword}", line, [keyword], replies=[keyword_text(line)])
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
