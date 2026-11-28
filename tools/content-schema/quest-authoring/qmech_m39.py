"""M39 source NPC offers, carried guards and knowledge hand-ins; reuse L2 be6626b4."""

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
NPCS = ("cael", "curos", "zalamon", "muriel")
ITEMS = {131, 4852}
SHARD = "content/quests/npc-callbacks/m39-stories.json"
WORLD_SHARD = "content/quests/world-callbacks/m39-stories.json"
PHANTOM_SOURCE_ONLY = frozenset(
    {
        "oteryn:quest-reward/l2-zalamon-phantom-accept",
        "oteryn:quest-reward/l2-zalamon-finish",
    }
)


def append(add, callbacks, offers, declarations, inputs, pin):
    frontier = "oteryn:quest-progress/quest/u8_54/the_new_frontier/"

    def state(track, before, after, maximum=None, preserve=False):
        return {
            "track": track,
            "minimum": before,
            "maximum": before if maximum is None else maximum,
            "set": after,
            **({"preserve": True} if preserve else {}),
        }

    def witness(path, lines):
        return f"crystal:{pin}:{path}:{lines}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}"

    alliance = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/"
    curos_source = (
        "https://github.com/zimbadev/crystalserver/blob/"
        "201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/curos.lua"
    )
    add(
        "l2-alliance-start",
        "an_uneasy_alliance_quest",
        "curos",
        ["mission"],
        [state(alliance + "questline", 0, 1), state(alliance + "quest_door", 0, 0)],
        requires=[{"track": frontier + "mission06", "minimum": 5, "maximum": 5}],
    )
    callbacks[-1].update(
        sources=[curos_source + "#L70-L82"],
        reply=[
            "A rebel commander is hiding in the mountains northwest of here. Kill him and return to me."
        ],
    )
    add(
        "l2-alliance-report-rebel",
        "an_uneasy_alliance_quest",
        "curos",
        ["mission"],
        [state(alliance + "questline", 1, 2)],
        requires=[{"track": alliance + "quest_door", "minimum": 1, "maximum": 1}],
    )
    callbacks[-1].update(
        sources=[curos_source + "#L83-L85"],
        reply=["The rebels are returning to camp now that their commander is dead."],
    )
    add(
        "l2-alliance-tower",
        "an_uneasy_alliance_quest",
        "curos",
        ["mission"],
        [state(alliance + "questline", 2, 3)],
    )
    callbacks[-1].update(
        sources=[curos_source + "#L86-L94"],
        reply=[
            (
                "The lizards spy on us from a tower in the Dragonblaze Mountains. Enter through their "
                "underground fortress and destroy the scrying device."
            )
        ],
    )
    add(
        "l2-alliance-finish",
        "an_uneasy_alliance_quest",
        "curos",
        ["mission"],
        [state(alliance + "questline", 4, 5)],
        give=(10217, 1),
    )
    callbacks[-1].update(
        completes=True,
        sources=[witness("data-global/npc/curos.lua", "95-101")],
        reply=[
            "Here is a reward. It's a strange tome that we've found in the lizard ruins. Maybe it is of some value for you or your allies."
        ],
    )
    # Cael owns New Frontier's existing counter, rather than a parallel quest flag.
    cael = "data-global/npc/cael.lua"
    counter = frontier + "tomeof_knowledge"
    delivered_at = frontier + "last_tome_delivery"
    declarations.append(
        {
            "key": delivered_at,
            "quest": "oteryn:quest.the_new_frontier_quest",
            "initial": 0,
            "min": 0,
            "max": 9223372036854775807,
            "writable": True,
            "sources": ["https://www.tibiawiki.com.br/wiki/Tomes_of_Knowledge_Quest"],
        }
    )
    for number in range(1, 14):
        before = min(number - 1, 12)
        states = [
            state(
                counter,
                -1 if number == 1 else before,
                min(number, 12),
                maximum=0 if number == 1 else None,
            )
        ]
        unlocks = {
            7: ("snake_head_teleport", -1),
            8: ("corruption_hole", -1),
            10: ("zao_palace_doors", 0),
        }
        if number in unlocks:
            name, _initial = unlocks[number]
            states.append(state(frontier + name, -1, 1, 1))
        requires = [{"track": frontier + "mission08", "minimum": 2, "maximum": 2}]
        if 7 <= number <= 12:
            requires.append(
                {"track": frontier + "mission10_1", "minimum": 2, "maximum": 2}
            )
        add(
            "l2-cael-tome-" + str(number),
            "the_new_frontier_quest",
            "cael",
            ["yes"],
            states,
            take=[(10217, 1)],
            topic=number,
            next_topic=number + 20,
            requires=requires,
        )
        callbacks[-1]["state_ops"] = [
            {
                "track": delivered_at,
                "from": {"kind": "elapsed_at_least", "seconds": 72000},
                "effect": {"kind": "set_now"},
            }
        ]
        callbacks[-1]["sources"] = [witness(cael, "84-128")]
        callbacks[-1]["reply"] = [
            "Thank you! I look forward to reading this interesting discovery of yours and learn a few things about Zao."
        ]
        callbacks[-1].update(
            inventory=[
                {
                    "item": "oteryn:item.tibia.i10217",
                    "minimum": 1,
                    "maximum": 4294967295,
                }
            ],
            failure_reply=["You dont have one!"],
        )
        if number == 13:
            callbacks[-1].update(repeatable=True, experience=5000)
            callbacks[-1]["storage"][0]["preserve"] = True
        offers.append(
            {
                "npc": "oteryn:npc.cael",
                "keywords": ["tome", "knowledge", "tomes of knowledge"],
                "topic": 0,
                "any_topic": True,
                "next_topic": number,
                "storage": [
                    state(
                        counter,
                        -1 if number == 1 else before,
                        before,
                        maximum=0 if number == 1 else None,
                        preserve=True,
                    )
                ],
                "requires": requires,
                "failure_reply": [
                    "I'm sorry I'm busy. Speak with Ongulf to get some missions!"
                ],
                "reply": ["Have you found a Tome of Knowledge for me to read?"],
            }
        )
    declarations.append(
        {
            "writable": True,
            "key": frontier + "corruption_hole",
            "quest": "oteryn:quest.the_new_frontier_quest",
            "initial": -1,
            "min": -1,
            "max": 1,
            "sources": [witness(cael, "118-122")],
        }
    )
    child = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/"
    add(
        "l2-zalamon-tome",
        "children_of_the_revolution_quest",
        "zalamon",
        ["yes"],
        [state(child + "questline", 17, 18), state(child + "mission04", 5, 6)],
        give=(10217, 1),
        topic=12,
    )
    callbacks[-1]["sources"] = [witness("data-global/npc/zalamon.lua", "325-335")]
    offers.append(
        {
            "npc": "oteryn:npc.zalamon",
            "keywords": ["mission"],
            "topic": 0,
            "any_topic": True,
            "next_topic": 12,
            "storage": [state(child + "questline", 17, 17, preserve=True)],
            "reply": ["Have you found the way which will lead us to the great gate?"],
        }
    )

    quest = "children_of_the_revolution_quest"
    npc = "zalamon"
    source = (
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/zalamon.lua"
    )

    def progress(
        name, before, after, lines, reply, *, topic=0, words=("mission",), **options
    ):
        records = [state(child + "questline", before, after)]
        records.extend(
            state(child + track, low, value, high)
            for track, low, high, value in options.pop("mission", ())
        )
        add(
            "l2-zalamon-" + name,
            quest,
            npc,
            list(words),
            records,
            topic=topic,
            take=options.pop("take", ()),
            give=options.pop("give", None),
            requires=options.pop("requires", ()),
        )
        callbacks[-1].update(sources=[f"{source}:{lines}"], reply=[reply], **options)

    def prompt(words, topic, next_topic, step, reply, *, any_topic=False, guards=()):
        offers.append(
            {
                "npc": "oteryn:npc.zalamon",
                "keywords": list(words),
                "topic": topic,
                "any_topic": any_topic,
                "next_topic": next_topic,
                "storage": [
                    state(child + "questline", step, step, preserve=True),
                    *guards,
                ],
                "reply": [reply],
            }
        )

    prompt(("mission",), 0, 1, 0, "Will you work for the resistance?", any_topic=True)
    prompt(
        ("yes",),
        1,
        2,
        0,
        "Find the enemy plans in the outpost beyond the mountains. Will you go?",
    )
    progress(
        "accept",
        0,
        1,
        "244-262",
        "Watch the mountain passes.",
        topic=2,
        words=("yes",),
        mission=[("mission00", 0, 0, 1)],
    )
    prompt(("mission",), 0, 3, 2, "Did you find the enemy plans?", any_topic=True)
    prompt(("yes",), 3, 4, 2, "Will you give me the map?")
    progress(
        "map",
        2,
        3,
        "262-275",
        "These troop movements will help us plan a defence.",
        topic=4,
        words=("yes",),
        mission=[("mission00", 1, 1, 2)],
        take=[(637, 1)],
    )

    prompt(
        ("mission",),
        0,
        5,
        3,
        "Will you investigate the Temple of Equilibrium?",
        any_topic=True,
    )
    progress(
        "temple-accept",
        3,
        4,
        "275-281",
        "The temple lies northwest of the outpost.",
        topic=5,
        words=("yes",),
        mission=[("mission01", 0, 0, 1)],
    )
    progress(
        "temple-report",
        5,
        6,
        "80-90",
        "The corruption has reached the temple.",
        mission=[("mission01", 2, 2, 3)],
    )
    prompt(
        ("mission",),
        0,
        6,
        6,
        "Will you scout the enemy camp for food, weapons, and soldiers?",
        any_topic=True,
    )
    progress(
        "spy-accept",
        6,
        7,
        "281-293",
        "Look inside all three buildings.",
        topic=6,
        words=("yes",),
        mission=[("mission02", 0, 0, 1)],
    )
    progress(
        "spy-report",
        7,
        8,
        "98-107",
        "A direct attack on an army that size would fail.",
        mission=[("mission02", 4, 4, 5)],
        requires=[
            {"track": child + f"spy_building_0{number}", "minimum": 1, "maximum": 1}
            for number in range(1, 4)
        ],
    )
    prompt(("mission",), 0, 7, 8, "Can you disrupt their food supply?", any_topic=True)
    prompt(
        ("yes",),
        7,
        8,
        8,
        "Take a flask from the storeroom. Will you go to the rice fields?",
    )
    progress(
        "poison-accept",
        8,
        9,
        "293-310",
        "Mix it into the water on the uppermost terrace.",
        topic=8,
        words=("yes",),
        mission=[("mission03", 0, 0, 1)],
    )
    prompt(
        ("poison", "poizzon"),
        0,
        0,
        9,
        "The emperor spreads corruption through those bound to him.",
        any_topic=True,
    )
    progress(
        "poison-report",
        11,
        12,
        "115-123",
        "You have weakened the emperor’s forces.",
        mission=[("mission03", 2, 2, 3)],
    )
    prompt(
        ("mission",),
        0,
        10,
        12,
        "Can you find a route north to the great gate?",
        any_topic=True,
    )
    progress(
        "waystones-accept",
        12,
        13,
        "310-322",
        "Search the temple for a passage.",
        topic=10,
        words=("yes",),
    )
    # The symbol room may be visited before the Waystones mission is accepted.
    callbacks[-1]["storage"].append(
        {**state(child + "mission04", 0, 1, 6), "at_least": True}
    )
    for step in (12, 13):
        prompt(
            ("symbols", "strange symbols"),
            0,
            11,
            step,
            "The symbols form a riddle. Will you try the levers?",
            any_topic=True,
            guards=[state(child + "strange_symbols", 1, 1, preserve=True)],
        )
    add(
        "l2-zalamon-symbols",
        quest,
        npc,
        ["yes"],
        [state(child + "strange_symbols", 1, 2), state(child + "mission04", 2, 3)],
        topic=11,
    )
    callbacks[-1].update(
        sources=[f"{source}:322-330"],
        reply=[
            "Fetch grease from the shelf in the storeroom, then arrange the symbols with the levers."
        ],
    )
    # Retain the source callbacks for catalogue inspection. The runtime keeps
    # them held until battlefield settlement, with one report for saved returns.
    progress(
        "phantom-accept",
        18,
        19,
        "337-345",
        "Go through the portal and approach the gate.",
        topic=13,
        words=("yes",),
        mission=[("teleport_access", 0, 0, 1), ("mission05", -1, 0, 1)],
    )
    progress(
        "finish",
        20,
        21,
        "150-162",
        "Take this serpent crest for your service to the resistance.",
        mission=[("mission05", 2, 2, 3)],
        give=(10199, 1),
        experience=10_000,
        completes=True,
    )

    bone = "oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit"
    warning = "Indeed there is something to be done, but I need someone more experienced. Come back later if you want to."
    muriel = "data-global/npc/muriel.lua"
    if warning not in inputs[muriel].decode():
        raise ValueError("M39 Muriel source refusal drift")
    add(
        "l2-bone-start",
        "tibia_tales",
        "muriel",
        ["yes"],
        [state(bone, -1, 1, 0)],
        give=(4852, 1),
        topic=1,
    )
    callbacks[-1].update(
        minimum_level=35,
        failure_reply=[warning],
        sources=[witness(muriel, "61-96")],
        reply=[
            "Great! Here is the container for the bone. Once, I used it to collect ectoplasma of ghosts, but it will work here as well. ..."
        ],
    )
    offers.append(
        {
            "npc": "oteryn:npc.muriel",
            "keywords": ["mission"],
            "topic": 0,
            "next_topic": 1,
            "storage": [state(bone, -1, -1, 0, preserve=True)],
            "minimum_level": 35,
            "failure_reply": [warning],
            "reply": [
                "Find that pit, dig for a well-preserved human skeleton and conserve a sample in a special container which you receive from me. Are you going to help me?"
            ],
        }
    )
    for name, minimum, maximum, take, give, experience, reply in [
        (
            "l2-bone-return",
            1,
            4294967295,
            [(131, 1)],
            (6299, 1),
            1000,
            "Excellent! Now I can try to put my theoretical thoughts into practice and find a cure for the symptoms of undead. Here, take this for your efforts.",
        ),
        (
            "bone-pit-lost-sample",
            0,
            0,
            [],
            None,
            None,
            "I am so glad you are still alive. Benjamin found the container with the bone sample inside. Fortunately, I inscribe everything with my name, so he knew it was mine. ...",
        ),
    ]:
        add(
            name,
            "tibia_tales",
            "muriel",
            ["mission"],
            [state(bone, 2, 3)],
            take=take,
            give=give,
        )
        callbacks[-1].update(
            minimum_level=35,
            failure_reply=[warning],
            inventory=[
                {
                    "item": "oteryn:item.tibia.i131",
                    "minimum": minimum,
                    "maximum": maximum,
                }
            ],
            sources=[witness(muriel, "61-96")],
            reply=[reply],
        )
        if experience is not None:
            callbacks[-1]["experience"] = experience


def shops(inputs, pin):
    path = "data-global/npc/cael.lua"
    track = "oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge"
    return {
        "schema": 1,
        "shops": [
            {
                "npc": "oteryn:npc.cael",
                "service": "oteryn:service.quest.cael.tomes",
                "sources": [
                    f"crystal:{pin}:{path}:30-35:sha256:{hashlib.sha256(inputs[path]).hexdigest()}"
                ],
                "offers": [
                    {
                        "item": {
                            "family": "Item",
                            "key": f"oteryn:item.tibia.i{item}",
                            "revision": "definition-r1",
                        },
                        "direction": "SellToPlayer",
                        "unit_price": price,
                        "requirements": [
                            {
                                "quest": "oteryn:quest.the_new_frontier_quest",
                                "track": track,
                                "initial": 0,
                                "minimum": count,
                                "maximum": 12,
                            }
                        ],
                    }
                    for item, price, count in [
                        (2965, 5000, 6),
                        (2966, 1000, 6),
                        (10216, 2500, 11),
                        (10288, 5000, 12),
                    ]
                ],
            }
        ],
    }


WORLD_BINDINGS = (
    ROOT / "tools/content-schema/quest-authoring/samples/qmech-m39/world-bindings.json"
)


def source_paths():
    callbacks = json.loads(WORLD_BINDINGS.read_bytes())["callbacks"]
    return sorted(
        {
            source.split(":", 2)[2].rsplit(":sha256:", 1)[0].rsplit(":", 1)[0]
            for callback in callbacks
            for source in callback["sources"]
        }
    )


def worlds(inputs):
    document = json.loads(WORLD_BINDINGS.read_bytes())
    for callback in document["callbacks"]:
        for source in callback["sources"]:
            descriptor, digest = source.rsplit(":sha256:", 1)
            path = descriptor.split(":", 2)[2].rsplit(":", 1)[0]
            if hashlib.sha256(inputs[path]).hexdigest() != digest:
                raise ValueError("M39 donor world binding drift: " + path)
    return document
