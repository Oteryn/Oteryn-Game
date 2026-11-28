"""Native compositions of the lane's pinned, explicitly chosen quest recipes."""

import hashlib
import json
import re

GOLDEN_OWNED_CONFIRMATION = "already owned"
GOLDEN_OWNED_PROMPT = " If your account already owns it, say {already owned}."


def golden_owned_confirmation(paid):
    owned = {
        key: value
        for key, value in paid.items()
        if key not in {"fee", "forbids_outfits", "forbids_addons"}
    }
    owned.update(
        key=paid["key"] + "-owned",
        keywords=[GOLDEN_OWNED_CONFIRMATION],
        outfit=None,
        reply=["Your account already owns it. I have recorded this commission step."],
    )
    for kind in ("outfits", "addons"):
        if guard := paid.get("forbids_" + kind):
            owned["requires_" + kind] = guard
    return owned


def golden_recipe(root):
    path = "content/quests/definitions/quests-00100-00199.json"
    raw = (root / path).read_bytes()
    records = json.loads(raw)["records"]
    index, definition = next(
        (i, r["definition"])
        for i, r in enumerate(records)
        if r["definition"]["identity"]["key"] == "oteryn:quest.golden_outfits_quest"
    )
    recipe = definition["oteryn_recipe"]["payload"]["recipe"]
    prices = [s["count"] for s in recipe["stages"] if s["kind"] == "use"]
    if prices != [100000, 50000, 50000] or recipe["repeat"]["kind"] != "once":
        raise ValueError("GOLDEN_SELECTED_RECIPE_DRIFT")
    return path, index, raw, definition, prices


def append_golden(root, callbacks, offers, inputs, pin):
    path, index, raw, definition, prices = golden_recipe(root)
    source = "data-global/npc/king_tibianus.lua"
    sources = [
        f"canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}",
        f"crystal:{pin}:{source}:131-200:sha256:{hashlib.sha256(inputs[source]).hexdigest()}",
    ]
    track = "oteryn:quest-progress/questpc7/golden/commission"
    npc = "oteryn:npc.king_tibianus"

    def storage(before, after, preserve=False):
        return {
            "track": track,
            "minimum": before,
            "maximum": before,
            "set": after,
            **({"preserve": True} if preserve else {}),
        }

    def add(
        name,
        before,
        after,
        words,
        reply,
        *,
        topic=0,
        money=None,
        addon=None,
        completes=False,
    ):
        row = {
            "key": "oteryn:quest-reward/questpc7-golden-" + name,
            "quest": definition["identity"]["key"],
            "npc": npc,
            "keywords": words,
            "topic": topic,
            "next_topic": 0,
            "storage": [storage(before, after)],
            "take": [],
            "give": None,
            "outfit": None
            if addon is None
            else {"outfit": "oteryn:outfit.golden", "addons": addon},
            "achievement": None,
            "reply": [reply],
            "sources": sources,
        }
        if money is not None:
            row["fee"] = {"gold": money, "include_bank": True}
            if addon == 0:
                row["forbids_outfits"] = ["oteryn:outfit.golden"]
            else:
                row["forbids_addons"] = [
                    {"outfit": "oteryn:outfit.golden", "addons": addon}
                ]
        if completes:
            row["completes"] = True
        callbacks.append(row)

    add(
        "intro",
        -1,
        0,
        ["golden commission", "golden outfit"],
        "I can commission the Golden outfit and its two addons. Ask me for the base outfit.",
    )
    for addon, (name, words, label, topic) in enumerate(
        [
            ("base", ["base outfit"], "basic Golden outfit", 101),
            ("gloves", ["boots and gloves"], "Golden boots and gloves", 102),
            ("helmet", ["golden helmet"], "Golden helmet", 103),
        ]
    ):
        price = prices[addon]
        guards = [storage(addon, addon, True)]
        offers.append(
            {
                "npc": npc,
                "keywords": words,
                "topic": 0,
                "next_topic": topic,
                "storage": guards,
                "reply": [
                    f"The {label} costs {price} gold. Do you agree?"
                    + GOLDEN_OWNED_PROMPT
                ],
            }
        )
        add(
            name,
            addon,
            addon + 1,
            ["yes"],
            f"Your {label} is ready.",
            topic=topic,
            money=price,
            addon=addon,
        )
        callbacks.append(golden_owned_confirmation(callbacks[-1]))
    add(
        "complete",
        3,
        4,
        ["complete commission"],
        "Your Golden commission is complete.",
        completes=True,
    )


def append_friends(root, callbacks, offers, inputs, pin):
    """The explicitly selected finite programme, plus its two unlocked services."""
    path = "content/quests/definitions/quests-00100-00199.json"
    raw = (root / path).read_bytes()
    index, definition = next(
        (i, r["definition"])
        for i, r in enumerate(json.loads(raw)["records"])
        if r["definition"]["identity"]["key"] == "oteryn:quest.friends_and_traders"
    )
    recipe = definition["oteryn_recipe"]["payload"]["recipe"]
    priests = [
        "quentin",
        "tibra",
        "maealil",
        "yberius",
        "isimov",
        "amanda",
        "kasmir",
        "rahkem",
        "brewster",
        "tyrias",
    ]
    prayer_targets = [
        s["targets"][0].lower()
        for s in recipe["stages"]
        if s["kind"] == "talk" and s["key"] != "s1"
    ]
    if prayer_targets != priests or recipe["repeat"]["kind"] != "once":
        raise ValueError("FRIENDS_SELECTED_RECIPE_DRIFT")
    track = "oteryn:quest-progress/questpc7/friends/program"
    canonical = f"canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}"

    def add(
        name,
        npc,
        before,
        after,
        words,
        reply,
        *,
        take=(),
        give=None,
        present=(),
        fee=None,
        completes=False,
        counter=None,
        maximum=None,
        achievement=None,
        wiki=None,
        topic=0,
        next_topic=0,
    ):
        source = f"data-global/npc/{npc}.lua"
        source_bytes = inputs[source]
        state = {
            "track": track,
            "minimum": before,
            "maximum": before if maximum is None else maximum,
            "set": after,
            **({"preserve": True} if counter else {}),
        }
        row = {
            "key": "oteryn:quest-reward/questpc7-friends-" + name,
            "quest": definition["identity"]["key"],
            "npc": "oteryn:npc." + npc,
            "keywords": words,
            "topic": topic,
            "next_topic": next_topic,
            "storage": [state],
            "take": [{"item": f"oteryn:item.tibia.i{i}", "count": n} for i, n in take],
            "present": [
                {"item": f"oteryn:item.tibia.i{i}", "count": n} for i, n in present
            ],
            "give": None
            if give is None
            else {
                "item": f"oteryn:item.tibia.i{give[0]}",
                "count": give[1],
                "contents": [],
            },
            "outfit": None,
            "achievement": achievement,
            "reply": [reply],
            "sources": ([wiki] if wiki else [])
            + [
                canonical,
                f"crystal:{pin}:{source}:1-{len(source_bytes.splitlines())}:sha256:{hashlib.sha256(source_bytes).hexdigest()}",
            ],
        }
        if fee:
            row["fee"] = fee
        if completes:
            row["completes"] = True
        if counter:
            row["counter"] = {
                "track": "oteryn:quest-progress/questpc7/friends/" + counter,
                "maximum": 1000000000000,
            }
        callbacks.append(row)

    add(
        "intro",
        "a_sweaty_cyclops",
        -1,
        0,
        ["forging friendship"],
        "Bring three bast skirts for my partner.",
    )
    add(
        "skirts",
        "a_sweaty_cyclops",
        0,
        1,
        ["deliver skirts"],
        "Now I can forge royal steel for you.",
        take=[(3560, 3)],
    )
    # The transcript also permits naming the materials at the hand-in.
    offers.append(
        {
            "npc": "oteryn:npc.a_sweaty_cyclops",
            "keywords": ["bast skirt"],
            "topic": 0,
            "any_topic": True,
            "next_topic": 4,
            "storage": [
                {
                    "track": track,
                    "minimum": 0,
                    "maximum": 0,
                    "set": 0,
                    "preserve": True,
                }
            ],
            "reply": ["Did you bring three bast skirts?"],
        }
    )
    add(
        "honey",
        "marina",
        1,
        2,
        ["deliver honey"],
        "I can now spin ten spider silks into one spool of yarn.",
        take=[(5902, 50)],
    )
    # The selected recipe specifies acquisition from Gamon without replacing his source price.
    add(
        "stake",
        "gamon",
        2,
        3,
        ["buy wooden stake"],
        "Here is your wooden stake.",
        give=(5941, 1),
        fee={"gold": 5000, "include_bank": True},
    )
    for number, npc in enumerate(priests):
        add(
            "prayer-" + npc,
            npc,
            3 + number,
            4 + number,
            ["stake prayer"],
            "Your stake has received this line of the prayer.",
            present=[(5941, 1)],
        )
        # An out-of-order prayer is a normal reply, with no quest transition.
        # Eligible reward callbacks are selected before these ordinary offers.
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": ["stake prayer"],
                "topic": 0,
                "next_topic": 0,
                "storage": [
                    {
                        "track": track,
                        "minimum": -1,
                        "maximum": 15,
                        "set": -1,
                        "preserve": True,
                    }
                ],
                "reply": [
                    "Follow the selected prayer order and keep your wooden stake."
                ],
            }
        )
    add(
        "blessing",
        "quentin",
        13,
        14,
        ["finish prayer"],
        "The ten-part prayer has blessed your wooden stake.",
        take=[(5941, 1)],
        give=(5942, 1),
        achievement="oteryn:achievement/blessed",
        # TibiaWiki Blessed!: finishing The Blessed Stake unlocks the achievement.
        wiki="https://tibia.fandom.com/index.php?title=Blessed!&oldid=844049",
    )
    offers.append(
        {
            "npc": "oteryn:npc.quentin",
            "keywords": ["finish prayer"],
            "topic": 0,
            "next_topic": 0,
            "storage": [
                {
                    "track": track,
                    "minimum": -1,
                    "maximum": 15,
                    "set": -1,
                    "preserve": True,
                }
            ],
            "reply": ["The prayer may bless your wooden stake only once."],
        }
    )
    add(
        "complete",
        "quentin",
        14,
        15,
        ["complete friendships"],
        "Your Friends and Traders programme is complete.",
        completes=True,
    )
    add(
        "royal-steel",
        "a_sweaty_cyclops",
        1,
        1,
        ["forge royal steel"],
        "Cling clang!",
        take=[(3381, 1)],
        give=(5887, 1),
        counter="steel-uses",
        maximum=15,
    )
    add(
        "yarn",
        "marina",
        2,
        2,
        ["spin yarn"],
        "Here is your spool of yarn.",
        take=[(5879, 10)],
        give=(5886, 1),
        counter="yarn-uses",
        maximum=15,
    )


def append_eligibility_starts(root, callbacks, offers, inputs, pin):
    """Own quest starts; subsequent native routes follow the shared eligibility owner."""
    wanted = [
        ("oteryn:quest.hunter_outfits_quest", "hunter", "elane", "hunter commission"),
        (
            "oteryn:quest.authored.illuminated_warrior_outfits_quest",
            "illuminated",
            "wayland_smythers",
            "illuminated commission",
        ),
    ]
    definitions = {}
    for path in sorted((root / "content/quests/definitions").glob("quests-*.json")):
        raw = path.read_bytes()
        for index, row in enumerate(json.loads(raw)["records"]):
            definition = row["definition"]
            if definition["identity"]["key"] in {r[0] for r in wanted}:
                definitions[definition["identity"]["key"]] = (
                    path.relative_to(root).as_posix(),
                    index,
                    raw,
                    definition,
                )
    wayland_menu = [
        "If you have collected enough radiant attire scraps, I can arrange the creation of an Illuminated Warrior for you.",
        (
            "The full outfit requires 2500 scraps. Each addon requires 1250 more. "
            "Are you interested in the {basic} outfit, the {insignia}, or the {halo} helmet?"
        ),
    ]
    for quest, slug, npc, words in wanted:
        path, index, raw, definition = definitions[quest]
        recipe = (
            definition.get("recipe") or definition["oteryn_recipe"]["payload"]["recipe"]
        )
        requirements = recipe["requirements"]
        source_path = f"data-global/npc/{npc}.lua"
        source_pin = (
            "crystal-summer:00ce02a57ca5a12e48f32a3476e37471167e4c3f"
            if npc == "wayland_smythers"
            else f"crystal:{pin}"
        )
        # Crystal 201189f8, wayland_smythers.lua:112-115 offers the outfit
        # independently of Wayland's Make Believe dialogue.
        keywords = [words, "outfit"] if slug == "illuminated" else [words]
        callbacks.append(
            {
                "key": f"oteryn:quest-reward/questpc7-{slug}-intro",
                "quest": quest,
                "npc": "oteryn:npc." + npc,
                "keywords": keywords,
                "topic": 0,
                "next_topic": 0,
                "storage": [
                    {
                        "track": f"oteryn:quest-progress/questpc7/{slug}/programme",
                        "minimum": -1,
                        "maximum": -1,
                        "set": 0,
                    }
                ],
                "requirements": {
                    "min_level": requirements["min_level"],
                    "premium": requirements["premium"],
                    "completed_quests": [],
                },
                "take": [],
                "give": None,
                "outfit": None,
                "achievement": None,
                "reply": (
                    ["Bring back my crossbow from the cultist cache."]
                    if slug == "hunter"
                    else wayland_menu
                ),
                "sources": [
                    f"canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}",
                    f"{source_pin}:{source_path}:1-{len(inputs[source_path].splitlines())}:sha256:{hashlib.sha256(inputs[source_path]).hexdigest()}",
                ],
            }
        )

        if slug == "illuminated":
            intro = callbacks[-1]
            outfit = "oteryn:outfit.illuminated_warrior"
            track = intro["storage"][0]["track"]
            offers.append(
                {
                    "npc": intro["npc"],
                    "keywords": ["outfit"],
                    "topic": 0,
                    "next_topic": 0,
                    "storage": [
                        {
                            "track": track,
                            "minimum": 0,
                            "maximum": 3,
                            "set": 0,
                            "preserve": True,
                        }
                    ],
                    "reply": wayland_menu,
                }
            )
            for keyword, addon, count in [
                ("basic", 0, 2500),
                ("insignia", 1, 1250),
                ("halo", 2, 1250),
            ]:
                reward = {"outfit": outfit, "addons": addon}
                topic = 101 + addon
                stage = {
                    "track": track,
                    # Ownership guards on confirmation order the account-wide rewards.
                    # Another character on the account may be at the start of this commission.
                    "minimum": 0,
                    "maximum": addon,
                    "set": addon + 1,
                }
                offers.extend(
                    [
                        {
                            "npc": intro["npc"],
                            "keywords": [keyword],
                            "topic": 0,
                            "next_topic": topic,
                            "storage": [{**stage, "set": addon, "preserve": True}],
                            "reply": [
                                f"The {keyword} exchange costs {count} radiant attire scraps. Do you agree?"
                            ],
                        },
                        {
                            "npc": intro["npc"],
                            "keywords": ["no"],
                            "topic": topic,
                            "next_topic": 0,
                            "storage": [{**stage, "set": addon, "preserve": True}],
                            "reply": ["Come back when you are ready."],
                        },
                    ]
                )
                exchange = {
                    **intro,
                    "key": f"oteryn:quest-reward/questpc7-illuminated-{keyword}",
                    "keywords": ["yes"],
                    "topic": topic,
                    "storage": [stage],
                    "take": [{"item": "oteryn:item.tibia.i54640", "count": count}],
                    "outfit": reward,
                    "reply": ["Your Illuminated Warrior commission is ready."],
                    "sources": intro["sources"] + ["wiki:111141:1196354"],
                }
                if addon:
                    exchange.update(
                        requires_outfits=[outfit],
                        forbids_addons=[reward],
                    )
                else:
                    exchange.update(forbids_outfits=[outfit])
                if addon == 2:
                    exchange.update(
                        requires_addons=[{"outfit": outfit, "addons": 1}],
                        completes=True,
                    )
                callbacks.append(exchange)


def append_tokens(root, callbacks, offers, inputs):
    quest = "oteryn:quest.hunting_for_tokens"
    npc = "oteryn:npc.jorge"
    track = "oteryn:quest-progress/questpc7/tokens/programme"
    path = "content/quests/definitions/quests-00100-00199.json"
    raw = (root / path).read_bytes()
    index, definition = next(
        (i, row["definition"])
        for i, row in enumerate(json.loads(raw)["records"])
        if row["definition"]["identity"]["key"] == quest
    )
    recipe = (
        definition.get("recipe") or definition["oteryn_recipe"]["payload"]["recipe"]
    )
    if (
        [(row["kind"], row["count"]) for row in recipe["stages"]]
        != [("talk", 1), ("collect", 20), ("use", 20), ("complete", 1)]
        or recipe["repeat"]["kind"] != "once"
        or recipe["requirements"]["prerequisites"]
    ):
        raise ValueError("QUESTPC7_TOKEN_SELECTED_RECIPE_DRIFT")

    source_path = "data-global/npc/jorge.lua"
    source = inputs[source_path]
    digest = hashlib.sha256(source).hexdigest()
    # The retained Jorge blob matches Crystal 201189f byte for byte.
    if digest != "127ff40ceb3fd0612663cbd952176840977e1ba9b9f22534ba52b2d059963db0":
        raise ValueError("QUESTPC7_JORGE_EXACT_TOKEN_SOURCE_DRIFT")
    replicas = [
        (name.decode(), int(item))
        for name, item in re.findall(rb'name = "([^"]+)", id = (\d+)', source)
    ]
    sources = [
        f"canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}",
        f"crystal:201189f871d23597b82caf88fd611454a2ddefb8:{source_path}:60-132:sha256:{digest}",
    ]

    def add(key, keywords, storage, reply, *, topic=0, next_topic=0, **reward):
        callbacks.append(
            {
                "key": "oteryn:quest-reward/questpc7-tokens-" + key,
                "quest": quest,
                "npc": npc,
                "keywords": keywords,
                "topic": topic,
                "next_topic": next_topic,
                "storage": [storage],
                "requirements": {
                    "min_level": 0,
                    "premium": False,
                    "completed_quests": [],
                },
                "take": [],
                "give": None,
                "outfit": None,
                "achievement": None,
                "completes": False,
                "reply": reply,
                "sources": sources,
                **reward,
            }
        )

    menu_words = ["replica rewards", "souvenir"]
    menu = ["Each replica costs 20 Silver Raid Tokens. Which would you like?"]
    for start in range(0, len(replicas), 8):
        menu.append(
            ", ".join("{" + name + "}" for name, _ in replicas[start : start + 8])
        )
    add(
        "replica-rewards",
        menu_words,
        {"track": track, "minimum": -1, "maximum": -1, "set": 0},
        menu,
        next_topic=1,
    )
    offers.append(
        {
            "npc": npc,
            "keywords": menu_words,
            "topic": 0,
            "any_topic": True,
            "next_topic": 1,
            "storage": [
                {"track": track, "minimum": 0, "maximum": 2, "set": 0, "preserve": True}
            ],
            "reply": menu,
        }
    )

    token = "oteryn:item.tibia.i19083"
    for name, item in replicas:
        word = name.lower()
        alias = word + " replica"
        slug = re.sub(r"[^a-z0-9]+", "-", word) + "-replica"
        payment = {
            "take": [{"item": token, "count": 20}],
            "give": {"item": f"oteryn:item.tibia.i{item}", "count": 1, "contents": []},
            "failure_reply": ["You need 20 Silver Raid Tokens for this exchange."],
        }
        add(
            slug,
            [alias, word],
            {"track": track, "minimum": 0, "maximum": 0, "set": 1},
            [f"Here is your {name} replica. You can now {{finish the commission}}."],
            topic=1,
            **payment,
        )
        add(
            slug + "-repeat",
            [alias, word],
            {"track": track, "minimum": 1, "maximum": 2, "set": 1, "preserve": True},
            [f"Here is your {name} replica."],
            topic=1,
            repeatable=True,
            **payment,
        )
    add(
        "finish-the-commission",
        ["finish the commission"],
        {"track": track, "minimum": 1, "maximum": 1, "set": 2},
        ["Your replica commission is complete."],
        completes=True,
    )
