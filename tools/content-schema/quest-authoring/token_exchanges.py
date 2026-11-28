"""Fixed token recipes from Crystal Server 201189f8 (OTS_HYPOTHESIS_ONLY)."""

import argparse
import hashlib
import json
import re
import tarfile
from pathlib import Path

PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SHARD = "content/quests/npc-callbacks/token-exchanges.json"
VAMPIRE_CALLBACKS = "content/quests/npc-callbacks/vampire-hunter.json"
VAMPIRE_OFFERS = "content/quests/npc-offers/vampire-hunter.json"
BIGFOOT_RULES = "content/dialogues/quest-rules/quest-batch3-a-bigfoot_burden.json"
GNOMALLY_SOURCE = "content/dialogues/quest-source/quest-batch3-a-bigfoot-burden.json"
SOURCES = {
    "cledwyn": "crystal:"
    + PIN
    + ":data-global/npc/cledwyn.lua:26,102-163:sha256:915bab4d2c1e44b43cc4855c7b896c64e31cb3abe6598af461155bcbf8143431",
    "gnomally": "crystal:"
    + PIN
    + ":data-global/npc/gnomally.lua:75-169:sha256:e598782edbd947acf0de2a13e42b4791536b5b8923d5132841891a77604133b1",
    "jorge": "crystal:"
    + PIN
    + ":data-global/npc/jorge.lua:60-136:sha256:127ff40ceb3fd0612663cbd952176840977e1ba9b9f22534ba52b2d059963db0",
    "ruprecht": "crystal:"
    + PIN
    + ":data-global/npc/ruprecht.lua:54-129:sha256:6788ae0c3290515135d30609ec56dba504713884822b82312e7dea84055cbb6f",
}

CLEDWYN = (
    ("pendulet", 29429, 2, 30344),
    ("sleep shawl", 29428, 2, 30342),
    ("blister ring", 31621, 2, 31557),
    ("theurgic amulet", 30401, 2, 30403),
    ("ring of souls", 32636, 2, 32621),
    ("spiritthorn ring", 39179, 5, 39177),
    ("alicorn ring", 39182, 5, 39180),
    ("arcanomancer sigil", 39185, 5, 39183),
    ("arboreal ring", 39188, 5, 39187),
    ("turtle amulet", 39235, 2, 39233),
    ("merudri brooch", 50156, 2, 50154),
    ("ethereal ring", 50149, 5, 50147),
    ("flamingo amulet of valor", 54510, 5, 53197),
    ("flamingo amulet of precision", 54511, 5, 53199),
    ("flamingo amulet of destruction", 54512, 5, 53201),
    ("flamingo amulet of nature", 54513, 5, 53203),
    ("swan amulet of balance", 54514, 5, 53205),
)

GNOMALLY = (
    ("muck", 16128, 8, 16101),
    ("mission", 16128, 10, 16242),
    ("lamp", 16128, 15, 16094),
    ("backpack", 16128, 15, 16099),
    ("gill gugel", 16129, 10, 16104),
    ("gill coat", 16129, 10, 16105),
    ("gill legs", 16129, 10, 16106),
    ("spellbook", 16129, 10, 16107),
    ("prismatic helmet", 16129, 10, 16109),
    ("prismatic armor", 16129, 10, 16110),
    ("prismatic legs", 16129, 10, 16111),
    ("prismatic boots", 16129, 10, 16112),
    ("prismatic shield", 16129, 10, 16116),
    ("iron loadstone", 16129, 20, 16153),
    ("glow wine", 16129, 20, 16154),
)

JORGE = (
    ("abacus", 19083, 20, 19151),
    ("assassin doll", 19083, 20, 28897),
    ("bag of oriental spices", 19083, 20, 23682),
    ("bookworm doll", 19083, 20, 28895),
    ("citizen doll", 19083, 20, 43511),
    ("crimson doll", 19083, 20, 25981),
    ("doll of durin the almighty", 19083, 20, 23679),
    ("dragon eye", 19083, 20, 22027),
    ("dragon goblet", 19083, 20, 31265),
    ("draken doll", 19083, 20, 25979),
    ("emblem", 19083, 20, 25980),
    ("encyclopedia", 19083, 20, 23678),
    ("friendship amulet", 19083, 20, 19153),
    ("frozen heart", 19083, 20, 19156),
    ("golden falcon", 19083, 20, 28896),
    ("golden newspaper", 19083, 20, 23681),
    ("goromaphone", 19083, 20, 34210),
    ("hand puppets", 19083, 20, 23676),
    ("imortus", 19083, 20, 23683),
    ("jade amulet", 19083, 20, 31268),
    ("key of numerous locks", 19083, 20, 19152),
    ("little adventurer doll", 19083, 20, 37058),
    ("loremaster doll", 19083, 20, 31267),
    ("lucky clover amulet", 19083, 20, 37059),
    ("mathmaster shield", 19083, 20, 25982),
    ("medusa skull", 19083, 20, 23680),
    ("music box", 19083, 20, 23677),
    ("noble sword", 19083, 20, 22028),
    ("norseman doll", 19083, 20, 19150),
    ("old radio", 19083, 20, 28894),
    ("orc's jaw shredder", 19083, 20, 19155),
    ("phoenix statue", 19083, 20, 22026),
    ("pigeon trophy", 19083, 20, 31266),
    ("shield of destiny", 19083, 20, 43517),
    ("shield of endless search", 19083, 20, 37060),
    ("the mexcalibur", 19083, 20, 19154),
    ("tibiora's box", 19083, 20, 43510),
    ("love elixir", 19083, 20, 49272),
    ("chaos critical dice", 19083, 20, 49273),
)

RUPRECHT = (
    ("gingerbreadman", 6526, 1, 6500),
    ("christmas cookie tray", 6526, 1, 20310),
    ("gingerbread recipe", 6526, 10, 174),
    ("jewel case", 6526, 25, 7527),
    ("santa hat", 6526, 50, 6531),
    ("santa backpack", 6526, 75, 10346),
    ("snow flake tapestry", 6526, 75, 20315),
    ("santa doll", 6526, 100, 6511),
    ("snowman doll", 6526, 150, 10339),
    ("snow globe", 6526, 150, 20311),
    ("frazzlemaw santa", 6526, 250, 20308),
    ("leaf golem santa", 6526, 250, 20309),
    ("santa music box", 6526, 250, 20313),
    ("santa teddy", 6526, 500, 10338),
    ("maxxen santa", 6526, 250, 21952),
    ("present bag", 6496, 1, 6526),
    ("ferumbras' teddy santa", 6526, 250, 22879),
    ("nightmare beast santa", 6526, 250, 29947),
    ("orclops santa", 6526, 250, 24394),
    ("raccoon santa", 6526, 250, 35692),
    ("santa fox", 6526, 250, 27591),
    ("santa leech", 6526, 250, 32746),
    ("santa ahau", 6526, 250, 42175),
    ("draccoon santa", 6526, 250, 44755),
    ("arbaziloth santa", 6526, 250, 50124),
    ("ancient bonelord santa", 6526, 250, 52725),
)


def donor_replies():
    """Extract the new dialogue branches from the committed, digest-checked corpus."""
    archive = Path(__file__).parent / "samples/donor-source/corpus.tar.gz"
    with tarfile.open(archive) as corpus:
        manifest = json.load(corpus.extractfile("corpus-manifest.json"))
        sources = {}
        for npc in ("cledwyn", "gnomally"):
            path = f"data-global/npc/{npc}.lua"
            expected = SOURCES[npc].rsplit(":", 1)[1]
            entry = next(
                row
                for row in manifest["files"]
                if row["source"] == "crystalserver"
                and row["path"] == path
                and row["sha256"] == expected
            )
            data = corpus.extractfile(
                entry.get("blob_path", entry.get("cache_path"))
            ).read()
            if hashlib.sha256(data).hexdigest() != entry["sha256"]:
                raise ValueError(f"token exchange source drift: {path}")
            sources[npc] = data.decode()

    def reply(npc, keyword):
        pattern = (
            rf'MsgContains\(message, "{keyword}"\)[^\n]* then\s+npcHandler:say\('
            r'("[^"\n]+"|\{[^\n]+\})'
        )
        replies = re.findall(pattern, sources[npc])
        if len(replies) != 1:
            raise ValueError(f"missing unique exchange dialogue: {npc}/{keyword}")
        return re.findall(r'"([^"\n]+)"', replies[0])

    return reply


def append(add, callbacks, offers):
    def offer(npc, word, topic, next_topic, reply, *, msg_contains=False, **predicates):
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": [word],
                "topic": topic,
                "next_topic": next_topic,
                "storage": [],
                "reply": [reply] if isinstance(reply, str) else reply,
                **predicates,
            }
        )
        if msg_contains:
            offers[-1]["msg_contains"] = True

    def recipe(npc, quest, output, topic, take, reply, failure, keyword="yes"):
        add(
            f"{npc}-exchange-{output}",
            quest,
            npc,
            [keyword],
            [],
            take=take,
            give=(output, 1),
            topic=topic,
            source=SOURCES[npc],
        )
        callbacks[-1].update(
            reply=[reply],
            failure_reply=[failure],
            msg_contains=True,
            clear_topic_on_refusal=True,
        )

    def choices(rows):
        return ", ".join("{" + name + "}" for name, *_ in rows)

    donor_reply = donor_replies()
    for word in ("token", "information", "talk"):
        offer(
            "cledwyn",
            word,
            0,
            0,
            donor_reply("cledwyn", word),
            msg_contains=True,
            any_topic=True,
            preserve_topic=True,
            reward_order="before",
        )
        if word == "token":
            offers[-1]["keywords"].append("tokens")
    cledwyn_no = donor_reply("cledwyn", "no")
    gnomally_no = donor_reply("gnomally", "no")
    offer(
        "cledwyn",
        "enchant",
        0,
        1,
        "I can enchant " + choices(CLEDWYN) + ". Which item?",
        msg_contains=True,
        any_topic=True,
        reward_order="before",
    )
    for topic, (name, discharged, cost, charged) in enumerate(CLEDWYN, 101):
        offer(
            "cledwyn",
            name,
            1,
            topic,
            f"Should I enchant the {name} for {cost} silver tokens?",
        )
        offer("cledwyn", "no", topic, 0, cledwyn_no, msg_contains=True)
        recipe(
            "cledwyn",
            "rift_warrior_outfits_quest",
            charged,
            topic,
            [(discharged, 1), (22516, cost)],
            f"Ah, excellent. Here is your {name}.",
            f"Bring an unenchanted {name} and {cost} silver tokens.",
        )

    offer(
        "gnomally",
        "equipment",
        0,
        0,
        "Ask me about {minor} or {major} crystal token goods.",
        msg_contains=True,
        any_topic=True,
        preserve_topic=True,
        reward_order="before",
    )
    for token, currency in ((16129, "major"), (16128, "minor")):
        rows = [row for row in GNOMALLY if row[1] == token]
        existing = (
            "{supply}, {addition to the soil guardian outfit}, {addition to the crystal warlord armor outfit}"
            if token == 16128
            else "{basic soil guardian outfit}, {basic crystal warlord outfit}"
        )
        offer(
            "gnomally",
            currency,
            0,
            0,
            f"For {currency} crystal tokens I offer {choices(rows)}, {existing}.",
            msg_contains=True,
            any_topic=True,
            preserve_topic=True,
            reward_order="before",
        )
    root = Path(__file__).resolve().parents[3]
    source = json.loads((root / GNOMALLY_SOURCE).read_text())
    gnomally = next(
        row for row in source["npcs"] if row["npc"] == "oteryn:npc.gnomally"
    )
    replies = {row["line"]: row["literal_parts"] for row in gnomally["fragments"]}
    quest_line = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line"
    tracks = json.loads((root / BIGFOOT_RULES).read_text())["tracks"]
    declaration = next(row for row in tracks if row["key"] == quest_line)
    offer(
        "gnomally",
        "relations",
        0,
        0,
        replies[133],
        msg_contains=True,
        any_topic=True,
        preserve_topic=True,
        reward_order="before",
        requires=[{"track": quest_line, "minimum": declaration["min"], "maximum": 24}],
    )
    offer(
        "gnomally",
        "relations",
        0,
        0,
        [],
        msg_contains=True,
        any_topic=True,
        preserve_topic=False,
        reward_order="before",
        source_dialogue=True,
        requires=[{"track": quest_line, "minimum": 25, "maximum": declaration["max"]}],
    )
    offer(
        "gnomally",
        "items",
        0,
        5,
        replies[143],
        msg_contains=True,
        any_topic=True,
        reward_order="before",
    )

    for word, lines in (("yes", replies[186]), ("no", gnomally_no)):
        offer(
            "gnomally",
            word,
            5,
            0,
            lines,
            msg_contains=True,
            reward_order="before",
            **({"open_shop": True} if word == "yes" else {}),
        )

    # The retained vouchers and supply package share the same quote lifecycle.
    for row in callbacks:
        if row["npc"] == "oteryn:npc.gnomally" and 101 <= row["topic"] <= 105:
            row.update(msg_contains=True, clear_topic_on_refusal=True)
    retained = [
        row
        for row in offers
        if row["npc"] == "oteryn:npc.gnomally" and 101 <= row["next_topic"] <= 105
    ]
    for row in retained:
        row["any_topic"] = True

    for topic, (name, token, cost, output) in enumerate(GNOMALLY, 106):
        currency = "minor" if token == 16128 else "major"
        offer(
            "gnomally",
            name,
            0,
            topic,
            f"Trade {cost} {currency} crystal tokens for {name}?",
            any_topic=True,
        )
        recipe(
            "gnomally",
            "bigfoot_s_burden_quest",
            output,
            topic,
            [(token, cost)],
            "Thank you for your help.",
            f"You don't have enough {currency} crystal tokens.",
        )

    for topic in range(101, 106 + len(GNOMALLY)):
        offer("gnomally", "no", topic, 0, gnomally_no, msg_contains=True)

    # The donor charges immediately on the named selection after "souvenir".
    # The existing Abacus commission owns a distinct conversation and state.
    catalogue = [JORGE[start : start + 9] for start in range(0, len(JORGE), 9)]
    for row in offers:
        if row["npc"] == "oteryn:npc.jorge" and row["keywords"] == [
            "replica rewards",
            "souvenir",
        ]:
            row["keywords"] = ["replica rewards"]
    offer(
        "jorge",
        "souvenir",
        0,
        101,
        "Souvenirs cost 20 silver raid tokens each. Which one do you want?",
        msg_contains=True,
        any_topic=True,
        reward_order="before",
    )
    offers[-1]["reply"].extend(choices(rows) for rows in catalogue)
    for name, token, cost, output in JORGE:
        recipe(
            "jorge",
            "hunting_for_tokens",
            output,
            101,
            [(token, cost)],
            f"You just swapped 20 silver raid tokens for 1 {name}.",
            "I need at least 20 silver raid tokens for that.",
            keyword=name,
        )

    catalogue = [RUPRECHT[start : start + 8] for start in range(0, len(RUPRECHT), 8)]
    offer(
        "ruprecht",
        "offers",
        0,
        0,
        "I have these offers:",
        msg_contains=True,
        any_topic=True,
        preserve_topic=True,
        reward_order="alongside",
    )
    offers[-1]["reply"].extend(choices(rows) for rows in catalogue)
    offer(
        "ruprecht",
        "token",
        0,
        0,
        "Ask me for {offers} to spend Christmas tokens or bring me a {present bag}.",
        msg_contains=True,
    )
    for topic, (name, payment, cost, output) in enumerate(RUPRECHT, 101):
        if payment == 6496:
            prompt = "Would you exchange a present bag for one Christmas token?"
            reply = "Thank you, here is your Christmas token."
            failure = "You don't have a present bag."
        else:
            currency = "Christmas token" if cost == 1 else "Christmas tokens"
            prompt = f"Would you exchange {cost} {currency} for {name}?"
            reply = f"Thank you, here is your {name}."
            failure = "You don't have enough Christmas tokens."
        offer("ruprecht", name, 0, topic, prompt)
        recipe(
            "ruprecht",
            "present_quest",
            output,
            topic,
            [(payment, cost)],
            reply,
            failure,
        )


def vampire_offers(root):
    """Generate Storkus's quotes from the published rank reward authority."""
    ranks = json.loads((root / VAMPIRE_CALLBACKS).read_text())["callbacks"]
    common = {
        "npc": ranks[0]["npc"],
        "topic": 0,
        "msg_contains": True,
        "any_topic": True,
        "reward_order": "before",
    }
    # The earlier mission branch preserves a pending quote after the parent quest.
    offers = [
        {
            **common,
            "keywords": ["mission"],
            "next_topic": 0,
            "storage": [],
            "requires": ranks[0]["requires"],
            "reply": [],
            "preserve_topic": True,
        }
    ]
    for callback in ranks:
        rank = next(
            step for step in callback["storage"] if step["track"].endswith("/rank")
        )
        cost = callback["take"][0]["count"]
        currency = "Vampire Lord Token" if cost == 1 else "Vampire Lord Tokens"
        offers.append(
            {
                **common,
                "keywords": ["vampire lord token"],
                "next_topic": callback["topic"],
                "storage": [{**rank, "set": rank["minimum"], "preserve": True}],
                "requires": callback["requires"],
                "reply": [
                    f"Would ye' give me {cost} {currency} for rank {rank['set']}?"
                ],
            }
        )
    return {"schema": 1, "offers": offers}


RENOWN_RULES = "content/dialogues/quest-rules-supplemental/gnomally-renown.json"


def renown_rules(root):
    original = json.loads((root / BIGFOOT_RULES).read_text())
    source = SOURCES["gnomally"].split(":sha256:")[0].rsplit(":", 1)[0]
    prefix = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/"
    npc = "oteryn:npc.gnomally"
    rules = []
    for name, keywords, topics, line in [
        ("relations", ["relations"], None, 126),
        ("ask", ["yes"], [121], 169),
        ("quote", [], [122], 135),
        ("confirm", ["yes"], None, 174),
        ("cancel", ["no"], None, 186),
    ]:
        rules.append(
            {
                "key": "oteryn:quest-npc/gnomally-renown/" + name,
                "quest": original["quest"],
                "npc": npc,
                "trigger": "say",
                "keyword_mode": "msg_contains",
                "keywords": keywords,
                "any_message": not keywords,
                "topics": topics,
                "guards": [
                    {
                        "kind": "storage",
                        "track": prefix + "quest_line",
                        "op": "ge",
                        "value": 25,
                    }
                ],
                "effects": [{"kind": "gnomally_renown", "action": name}],
                "sources": [f"{source}:{line}"],
            }
        )
    return {
        "schema": 1,
        "program_key": "oteryn:quest-npc-program.gnomally-renown",
        "quest": original["quest"],
        "tracks": [
            r
            for r in original["tracks"]
            if r["key"] in (prefix + "rank", prefix + "quest_line")
        ],
        "rules": rules,
        "source_files": [
            r
            for r in original["source_files"]
            if r["path"].endswith(("gnomally.lua", "storages.lua"))
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Capture Gnomally's renown programme")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    path = root / RENOWN_RULES
    capture = json.dumps(renown_rules(root), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if path.read_text() != capture:
            raise SystemExit("Gnomally renown rules are stale")
    else:
        path.write_text(capture)
