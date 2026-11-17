"""Capture Monk dialogue without replacing the qualified mainland quest owner."""

import argparse
import re

from capture import Capture, inventory, topic
from literal_evidence import capture_evidence

PREFIX = "oteryn:quest-progress/quest/u15_00/the_way_of_the_monk/"
QUESTION = "https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589102441"


def manop_confirmation_aliases(source, value):
    """Resolve the explicit questKV/legacy aliases, never a guessed numeric ID.

    donor bug fix: ambassador_manop.lua:167/168 and :209/210 use `kv`
    outside its earlier local scope. The same blocks write the exact matching
    legacy fields, and MONK_QUEST is literal at line 69. Coordinator ANSWER 3:
    https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205
    QUESTION: https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589102441
    """
    if not re.search(r'local\s+MONK_QUEST\s*=\s*"the_way_of_the_monk_quest"', source):
        raise ValueError("Missing explicit Manop quest scope")
    aliases = {}
    for field, legacy in [("questline", "Questline"), ("questlog", "Questlog")]:
        kv = rf'kv:set\("{field}",\s*{value}\)'
        storage = rf'player:setStorageValue\(Storage\.Quest\.U15_00\.TheWayOfTheMonk\.{legacy},\s*{value}\)'
        if not re.search(kv, source) or not re.search(storage, source):
            raise ValueError(f"Missing matching Manop questKV/legacy write: {field}={value}")
        aliases[field] = PREFIX + field
    return aliases


def capture(donor):
    c = Capture(donor, "the_way_of_the_monk", "quest/u15_00/the_way_of_the_monk")
    for name, maximum in [("Questline", 4), ("Questlog", 4), ("ShrineCounter", 4),
                          *[("Shrines." + shrine, 1) for shrine in ("FirstShrine", "SecondShrine", "ThirdShrine", "FourthShrine")],
                          *[("MainShrines." + shrine, 1) for shrine in ("Guidance", "Tranquility", "Respect", "Legacy", "Empathy", "Harmony", "Power", "Knowledge", "Serenity", "Eternity")]]:
        key = c.track(name, maximum=maximum)
        # questKV `or 0` and absent booleans lower to zero. Existing qualified
        # mainland tracks already use zero; do not migrate or mirror them.
        c.tracks[key]["initial"] = c.tracks[key]["min"] = 0
    dawnport = [c.guard("Shrines." + shrine, "eq", 1)
                for shrine in ("FirstShrine", "SecondShrine", "ThirdShrine", "FourthShrine")]
    mainland = [c.guard("MainShrines." + shrine, "eq", 1)
                for shrine in ("Guidance", "Tranquility", "Respect", "Legacy", "Empathy", "Harmony", "Power", "Knowledge", "Serenity", "Eternity")]
    npc = "ambassador_manop"
    source = c.read(npc)
    invitation_aliases = manop_confirmation_aliases(source, 1)
    vocation_aliases = manop_confirmation_aliases(source, 2)
    c.rule(npc, "farewell", 85, ["bye", "farewell"], say=86, keyword_mode="exact")
    # hasAllShrines wins for every questline. The first source branch requires
    # NOT all shrines, so these ordered cases preserve its complete truth table.
    c.rule(npc, "greeting-four-shrines", 96, ["hi", "hello"], guards=dawnport,
           effects=[topic(3)], say=97, keyword_mode="exact")
    c.rule(npc, "greeting-invitation", 93, ["hi", "hello"], guards=[c.guard("Questline", "lt", 1)],
           effects=[topic(1)], say=94, keyword_mode="exact")
    c.rule(npc, "greeting-seeker", 99, ["hi", "hello"], effects=[topic(0)], say=100, keyword_mode="exact")
    for keyword, line in [("job", 107), ("legacy", 115), ("blue valley", 120), ("guide", 125),
                          ("monk", 130), ("serenity", 135), ("dawnport", 140), ("shrine", 145),
                          ("enpa", 150), ("pilgrimage", 155)]:
        c.rule(npc, "information-" + keyword.replace(" ", "-"), line - 1, [keyword], say=line, keyword_mode="exact")
    # Lua `or` has lower precedence than `and`: invitation works at any topic.
    c.rule(npc, "invitation", 159, ["invitation"], effects=[topic(2)], say=160, keyword_mode="exact")
    c.rule(npc, "merudri", 159, ["merudri"], topics=[1], effects=[topic(2)], say=160, keyword_mode="exact")
    invitation = c.rule(npc, "accept-invitation", 165, ["yes"], topics=[2],
                       effects=[{"kind": "set", "track": value, "value": 1} for value in invitation_aliases.values()] + [topic(0)],
                       say=166, keyword_mode="exact")
    invitation["sources"] += [c.ref(npc, line) for line in (69, 167, 168, 169, 170)]
    c.rule(npc, "seeker-ready", 178, ["seeker"], guards=dawnport, effects=[topic(4)], say=179, keyword_mode="exact")
    for count, line in enumerate((186, 188, 190, 192)):
        c.rule(npc, f"seeker-{count}", line - 1, ["seeker"], guards=[c.guard("ShrineCounter", "eq", count)],
               say=line, keyword_mode="exact")
    c.rule(npc, "seeker-other-count", 175, ["seeker"], keyword_mode="exact")
    c.rule(npc, "vocation-low-level", 198, ["yes"], topics=[4],
           guards=[{"kind": "level", "op": "lt", "value": 8}], say=199, keyword_mode="exact")
    c.rule(npc, "vocation-change-held", 197, ["yes"], topics=[4], keyword_mode="exact")

    npc = "enpa_deia_pema"
    c.rule(npc, "pilgrimage-start", 72, ["pilgrimage"], guards=[c.guard("Questline", "lt", 3)],
           effects=[c.effect("Questline", 3), c.effect("Questlog", 3)], say=73)
    # The existing native owner already owns completion, title and Wheel points.
    # Keep its final-report flag and threshold 3, not a parallel boolPointsWheel.
    c.rule(npc, "pilgrimage-completion-held", 79, ["pilgrimage"],
           guards=[c.guard("Questline", "eq", 3), *mainland])
    c.rule(npc, "pilgrimage-completed-held", 91, ["pilgrimage"], guards=mainland)
    c.rule(npc, "pilgrimage-incomplete", 97, ["pilgrimage"], say=98)
    c.rule(npc, "greet", 143, trigger="greet", replies=[c.strings(npc, 143, 143)[0]])

    npc = "ambassador_manop"
    capture_evidence(c, {npc: [(85, 215)], "enpa_deia_pema": [(70, 100), (143, 143), (145, 145)]}, [
        {"key": "manop-vocation-change", "npc": "oteryn:npc.ambassador_manop", "trigger": "say",
         "keywords": ["yes"], "keyword_mode": "exact", "topics": [4],
         "guards": [{"kind": "level", "op": "ge", "value": 8}],
         "effects": [{"kind": "set_town", "source_id": 32}, {"kind": "set_vocation", "source_id": 9},
                     {"kind": "teleport", "position": {"x": 33614, "y": 31494, "floor": -7}},
                     {"kind": "magic_effect", "effect": "CONST_ME_TELEPORT"},
                     *[inventory("give_item", item) for item in (50257, 50195, 50171)],
                     *[{"kind": "set", "track": value, "value": 2} for value in vocation_aliases.values()], topic(0)],
         "reply": c.say(npc, 213), "sources": [c.ref(npc, line) for line in (197, 198, 202, 203, 204, 206, 209, 211, 213)],
         "donor_bug_fix_question": QUESTION,
         "left_out": "Vocation/town/teleport/items must commit through an approved existing composite owner; no partial state or reward and no new owner in this batch."},
        {"key": "pema-completion", "npc": "oteryn:npc.enpa_deia_pema", "trigger": "say", "keywords": ["pilgrimage"],
         "guards": [c.guard("Questline", "eq", 3), *mainland],
         "effects": [c.effect("Questline", 4), c.effect("Questlog", 4),
                     {"kind": "quest_kv_set", "scope": "the_way_of_the_monk_quest", "key": "boolPointsWheel", "value": True},
                     {"kind": "conditional_title", "name": "Pensive Wanderer", "when": "title exists and id != 0"},
                     {"kind": "achievement", "name": "Hope of the Merudri"}],
         "reply": c.say("enpa_deia_pema", 80), "sources": [c.ref("enpa_deia_pema", line) for line in (61, 79, 81, 83, 86, 90)],
         "left_out": "Existing qualified native final-report/Wheel/title owner is retained; this capture is not a second completion handler."},
        {"key": "pema-already-completed", "npc": "oteryn:npc.enpa_deia_pema", "trigger": "say", "keywords": ["pilgrimage"],
         "guards": [c.guard("Questline", "ge", 4), *mainland],
         "effects": [{"kind": "quest_kv_set_if_absent", "scope": "the_way_of_the_monk_quest", "key": "boolPointsWheel", "value": True}],
         "reply": c.say("enpa_deia_pema", 95), "sources": [c.ref("enpa_deia_pema", line) for line in (91, 92, 93, 95)],
         "left_out": "Do not overwrite or mirror the already qualified final-report/Wheel grant."},
    ], ["The complete scoped source dialogue is retained, including held branches; healing and shop features have separate existing owners.",
        "Mainland shrine admission stays at the existing qualified threshold 3; this batch does not change shrine rewards or stored zero defaults.",
        "Manop confirmation questKV scope repair is an explicit donor bug fix; two matching KV/legacy aliases produce two logical track writes.",
        "New generic NPC source plans have no approved durable caller. Existing native Monk invitation/start/finish remains the active owner."])
    c.save()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("donor")
    capture(parser.parse_args().donor)
