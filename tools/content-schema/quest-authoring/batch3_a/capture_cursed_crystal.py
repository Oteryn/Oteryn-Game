"""Capture the Cursed Crystal branch, preserving the existing Tibia Tales owner."""

import argparse

from capture import Capture, inventory, topic
from literal_evidence import capture_evidence


def capture(donor):
    c = Capture(donor, "the_cursed_crystal", "quest/u10_70/the_cursed_crystal")
    npc = "one_eyed_joe"
    for name, maximum in [("Oneeyedjoe", 4), ("Questline", 4), ("MedusaOil", -1),
                          ("SheetOfPaper", 1), ("SmallCrystalBell", 1), ("Time", 2147483647)]:
        key = c.track(name, maximum=maximum)
        if name == "Time":
            # Keep the qualified existing absence value. No migration of zeros.
            c.tracks[key]["initial"] = c.tracks[key]["min"] = 0

    # Source greeting rewards require one atomic random item reward transaction.
    # Hold the entire branch, including its successful reply and three writes.
    c.rule(npc, "greet-reward-held", 68,
           guards=[c.guard("Oneeyedjoe", "eq", 3), c.guard("Questline", "eq", 3)], trigger="greet")
    c.rule(npc, "greet", 83, replies=[c.strings(npc, 83, 83)[0]], trigger="greet")
    c.rule(npc, "expired-reset", 99, any_message=True, continue_matching=True,
           guards=[c.guard("Time", "gt", 0), {"kind": "storage_time", "track": c.track("Time"), "op": "le"}],
           effects=[c.effect(name, -1) for name in ("Oneeyedjoe", "Questline", "MedusaOil", "SheetOfPaper", "SmallCrystalBell")])
    c.rule(npc, "mission", 107, ["mission"], guards=[c.guard("Oneeyedjoe", "lt", 0)],
           topics=[0], effects=[topic(1)], say=108)
    c.rule(npc, "still-cursed-yes", 113, ["yes"], guards=[c.guard("Oneeyedjoe", "between", 1, 2)], effects=[topic(0)], say=114)
    c.rule(npc, "still-cursed-no", 116, ["no"], guards=[c.guard("Oneeyedjoe", "between", 1, 2)], effects=[topic(0)], say=117)
    c.rule(npc, "protect-ears", 119, ["protect ears"], effects=[topic(0)], say=120)
    c.rule(npc, "story", 122, ["yes"], topics=[1], effects=[topic(2)], say=123)
    # This branch also writes the shared Tibia Tales DefaultStart. Retain its
    # complete transaction in source evidence until that cross-branch write is
    # qualified; do not advance only its local subset or emit success.
    c.rule(npc, "mission-acceptance-held", 128, ["yes"], topics=[2])
    c.rule(npc, "crystals", 136, ["crystals"], effects=[topic(0)], say=137)
    c.rule(npc, "cursed", 142, ["cursed"], guards=[c.guard("Oneeyedjoe", "lt", 0)], topics=[0], effects=[topic(1)], say=143)
    c.rule(npc, "sounds", 148, ["sounds"], effects=[topic(0)], say=149)
    c.rule(npc, "job", 154, ["job"], say=155)
    c.rule(npc, "name", 156, ["name"], say=157)
    c.rule(npc, "bye", 158, ["bye"], effects=[topic(0)], say=159)
    c.rule(npc, "unfinished-reminder", 161, guards=[c.guard("Oneeyedjoe", "between", 0, 2)],
           any_message=True, effects=[topic(0)], say=162)
    capture_evidence(c, {npc: [(59, 164)]}, [
        {"key": "greet-reward", "trigger": "greet", "guards": [c.guard("Oneeyedjoe", "eq", 3), c.guard("Questline", "eq", 3)],
         "effects": [{"kind": "achievement", "name": "Wail of the Banshee"},
                     *[inventory("give_item", item) for item in (16119, 16120, 16121)],
                     {"kind": "uniform_choice", "minimum": 1, "maximum": 4,
                      "items_by_choice": [5926, 6098, 6097, 6126], "count": 1},
                     c.effect("Oneeyedjoe", 4), c.effect("Questline", 4), c.effect("Time", 72000, "set_now_plus")],
         "reply": [c.strings(npc, 81, 81)[0]], "sources": [c.ref(npc, line) for line in (68, 69, 70, 74, 75, 78, 80, 81)],
         "left_out": "New NPC atomic random reward and inventory placement owner is not authorised by ANSWER 5/8."},
        {"key": "mission-acceptance", "trigger": "say", "keywords": ["yes"], "topics": [2],
         "effects": [{"kind": "shared_source_storage_set", "storage": "Storage.Quest.U8_1.TibiaTales.DefaultStart", "value": 1},
                     {"kind": "conditional_set", "track": c.track("Questline"), "when": {"op": "lt", "value": 0}, "value": 0},
                     c.effect("Oneeyedjoe", 0), topic(0)],
         "reply": c.say(npc, 134), "sources": [c.ref(npc, line) for line in (128, 129, 130, 131, 133, 134)],
         "left_out": "Shared Tibia Tales entry flag is outside the assigned Cursed Crystal branch; no partial local acceptance."},
    ], ["All new generic NPC rules are source plans only; no generic durable NPC caller is approved.",
        "Canonical quest remains oteryn:quest.tibia_tales; no sibling Tibia Tales rule or flag is changed."])
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
