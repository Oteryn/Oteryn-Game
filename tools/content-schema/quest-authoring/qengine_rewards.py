"""QENGINE-2 offline donor catalogue and bounded NPC reward compositions.

Only committed corpus blobs and the existing appearance source are read. This
producer promotes the requested quest catalogue; it never contacts a donor host.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
import tarfile
import xml.etree.ElementTree as ET
from copy import deepcopy
from pathlib import Path

import earned_outfit_catalogue
import engoutfit_m36
import lua_tables
import qb_barbarian
import qb_citizen_warrior
import qengine_mounts
import qmech_m39
import questl2_ready
import questpc7_smith_programme
import taming_cosmetics
import wizard_cosmetics
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[3]
PIN = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
GOLDEN_NPC_SOURCE = "data-global/npc/king_tibianus.lua"
PROVENANCE = "crystal-9f5a72c-quest-rewards-v1"
DECISION = "owner:QENGINE-2:canonical-donor-outfit-and-atomic-quest-grants:2026-11-06"

# These independently registered documents retain their existing greeting owner.
# Zalamon's chosen opening is explicitly authored; M10, M13 and L6 (Lubo) retain source text.
REWARD_GREETING_OWNERS = {
    "oteryn:npc.zalamon": "content/dialogues/quests/chosen-reward-greetings.json",
    "oteryn:npc.henricus": "content/dialogues/quests/qnpcfx-m10-greetings.json",
    "oteryn:npc.lubo": "content/dialogues/quests/l6-story-greetings.json",
    "oteryn:npc.rafzan": "content/dialogues/quests/m13-goblin-greetings.json",
}


# Sven opens through the native Dialogue fallback, which also admits its
# conditional greeting handling. Do not assign him a shared reward greeting.
NATIVE_DIALOGUE_GREETING_NPCS = frozenset({"oteryn:npc.sven"})


def reward_greetings(callbacks, inputs, *, root=None):
    root = ROOT if root is None else root
    owners = {}
    for path in sorted(set(REWARD_GREETING_OWNERS.values())):
        try:
            document = json.loads((root / path).read_bytes())
        except (OSError, ValueError) as error:
            raise ValueError("reward greeting owner document: " + path) from error
        if (
            not isinstance(document, dict)
            or document.get("schema") != 1
            or not isinstance(document.get("npcs"), list)
        ):
            raise ValueError("reward greeting owner schema: " + path)
        for row in document["npcs"]:
            match row:
                case {"npc": str() as npc}:
                    pass
                case _:
                    raise ValueError("reward greeting owner row: " + path)
            if npc not in REWARD_GREETING_OWNERS:
                continue
            if REWARD_GREETING_OWNERS[npc] != path or npc in owners:
                raise ValueError("reward greeting owner conflict: " + npc)
            if any(
                not isinstance(row.get(field), str) or not row[field]
                for field in ("greet", "farewell", "walkaway")
            ):
                raise ValueError("reward greeting owner payload: " + npc)
            owners[npc] = path
    if set(owners) != set(REWARD_GREETING_OWNERS):
        raise ValueError(
            "reward greeting owner missing: "
            + ", ".join(sorted(set(REWARD_GREETING_OWNERS) - set(owners)))
        )

    greetings = []
    npcs = {c["npc"] for c in callbacks} | {
        "oteryn:npc." + npc for npc in ("alesar", "yaman", "haroun", "nah_bob", "arito")
    }
    for npc in sorted(npcs - owners.keys() - NATIVE_DIALOGUE_GREETING_NPCS):
        path = "data-global/npc/" + npc.removeprefix("oteryn:npc.") + ".lua"
        messages = {}
        citations = []
        for field, constant in [
            ("greet", "GREET"),
            ("farewell", "FAREWELL"),
            ("walkaway", "WALKAWAY"),
        ]:
            matches = list(
                re.finditer(
                    r"npcHandler:setMessage\(MESSAGE_"
                    + constant
                    + r',\s*"([^"\n]*)"\)',
                    inputs.get(
                        path, inputs["data/npclib/npc_system/npc_handler.lua"]
                    ).decode(),
                )
            )
            origin = path
            # Conditional NPC messages belong to their functional selectors.
            # Use the source handler's neutral default when no unique static message exists.
            if len(matches) != 1:
                origin = "data/npclib/npc_system/npc_handler.lua"
                matches = list(
                    re.finditer(
                        r"\[MESSAGE_" + constant + r'\]\s*=\s*"([^"\n]*)"',
                        inputs[origin].decode(),
                    )
                )
            if len(matches) != 1:
                raise ValueError("ambiguous NPC greeting " + npc + field)
            messages[field] = matches[0].group(1)
            line = inputs[origin][: matches[0].start()].count(b"\n") + 1
            citations.append(
                f"crystal:{PIN}:{origin}:{line}:sha256:{sha(inputs[origin])}"
            )
        greetings.append({"npc": npc, **messages, "sources": citations})
    return greetings


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def donor_inputs():
    paths = [
        "data/XML/outfits.xml",
        "data/XML/mounts.xml",
        "data-crystal/scripts/special/addon_doll.lua",
        "data-global/scripts/actions/mounts/mounts.lua",
        "data/npclib/npc_system/npc_handler.lua",
    ] + [
        f"data-global/npc/{n}.lua"
        for n in (
            "morgan",
            "navigator",
            "black_bert",
            "rock_steady",
            *qb_barbarian.DONOR_NPCS,
            *qb_citizen_warrior.DONOR_NPCS,
            *wizard_cosmetics.DONOR_NPCS,
            *qmech_m39.NPCS,
            "costello",
            "budrik",
            "sam",
            "cael",
            "cranky_lizard_crone",
            "palimuth",
            "yalahari",
            "dedoras",
            "sven",
            "muriel",
            "the_first_dragon",
            "broken_servant_sentry",
            "avar_tar",
            "ezebeth",
            "irmana",
            "percybald",
            "raymond_striker",
            "duncan",
            "ariella",
            "ishina",
            "habdel",
            "miraia",
            "razan",
            "hjaern",
            "tom",
            "amber",
            "norma",
            "hugo",
            "simon_the_beggar",
            "gregor",
            "ustan",
            "ceiron",
            "a_majestic_warwolf",
            "vescu",
            "erayo",
            "atrad",
            "elane",
            "legola",
            "zoltan",
            "lynda",
            "sandra",
            "myra",
            "angelina",
            "eruaran",
            "gnomally",
            "ottokar",
            "king_tibianus",
            "the_bone_master",
            "the_dream_master",
            "undal",
            "vanys",
            "eshaya",
            "gnomadness",
            "a_sweaty_cyclops",
            "gnommander",
            "gnomilly",
            "gnombold",
            "gnomargery",
            "flickering_soul",
            "bloodshade_rotten",
            "cledwyn",
            *questl2_ready.NPCS,
        )
    ]
    paths += [
        "data/scripts/actions/items/usable_outfit_items.lua",
        "data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua",
        "data-global/scripts/actions/valuables/random_items.lua",
        "data-global/scripts/quests/the_new_frontier/action_outfit.lua",
        "data-global/scripts/quests/wrath_of_the_emperor/actions_mission12_just_rewards.lua",
        "data-global/startup/tables/item.lua",
        "data-global/scripts/actions/other/others/quest_system2.lua",
        "data-global/scripts/quests/the_djinn_war_quest/action_water_fountain.lua",
    ]
    paths += [
        f"data-global/npc/{name}.lua"
        for name in (
            "arito",
            "dermot",
            "marina",
            "gamon",
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
            "rafzan",
            "jorge",
            "the_forgemaster",
            "henricus",
            "yana",
            "ivalisse",
            "a_strange_chalice",
        )
    ]
    paths += [
        "data/XML/imbuements.xml",
        "data-global/scripts/quests/25_years_of _tibia_quest/lord_retro_death.lua",
    ]
    paths += qmech_m39.source_paths()
    paths = sorted(set(paths))
    result = {}
    with tarfile.open(
        ROOT / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    ) as corpus:
        manifest = json.load(corpus.extractfile("corpus-manifest.json"))
        summer_path = "data-global/npc/wayland_smythers.lua"
        summer_row = next(
            r
            for r in manifest["files"]
            if r["source"] == "crystal-summer"
            and r["revision"] == "00ce02a57ca5a12e48f32a3476e37471167e4c3f"
            and r["path"] == summer_path
        )
        summer_raw = corpus.extractfile(
            summer_row.get("blob_path", summer_row.get("cache_path"))
        ).read()
        if sha(summer_raw) != summer_row["sha256"]:
            raise ValueError("Wayland retained source changed")
        result[summer_path] = summer_raw
        for path in paths:
            row = next(
                r
                for r in manifest["files"]
                if r["source"] == "crystalserver"
                and r["revision"] == PIN
                and r["path"] == path
            )
            raw = corpus.extractfile(row.get("blob_path", row.get("cache_path"))).read()
            if sha(raw) != row["sha256"]:
                raise ValueError("donor blob changed: " + path)
            result[path] = raw
        result[qengine_mounts.CATALOGUE_INPUT] = qengine_mounts.read_catalogue_xml(
            corpus, manifest
        )
    return result


def golden_source_prices(source):
    # Crystal charges for armour, helmet, then boots.
    armour, helmet, boots = map(
        int, re.findall(rb"player:removeMoneyBank\(([0-9]+)\)", source)
    )
    return [armour, boots, helmet]


def elane_gloves(inputs):
    path = "data-global/npc/elane.lua"
    raw = inputs[path]
    source = raw.decode()
    branches = []
    for start, end in (
        ("Free Account", "Premium account with addon"),
        ("Premium account with addon", "If you don't have the addon"),
    ):
        block = source.split(f"-- {start}\n", 1)[1].split(f"-- {end}\n", 1)[0]
        lines = re.findall(r'^\s*"([^"\n]+)",?$', block, re.MULTILINE)
        item, count = map(
            int, re.search(r"player:removeItem\((\d+), (\d+)\)", block).groups()
        )
        gold = int(re.search(r"player:addMoney\((\d+)\)", block)[1])
        branches.append((lines, item, count, gold))
    (free, item, count, gold), (owned, owned_item, owned_count, owned_gold) = branches
    if (item, count, gold) != (owned_item, owned_count, owned_gold) or gold % 100:
        raise ValueError("Elane's glove sales need one exact platinum payment")

    offer_path = "content/quests/chosen-reward-offers.json"
    callback_path = "content/quests/chosen-reward-callbacks.json"
    offers = json.loads((ROOT / offer_path).read_bytes())
    callbacks = json.loads((ROOT / callback_path).read_bytes())
    key = "oteryn:quest-reward/chosen/hunter-outfits/elane-gloves"
    addon = next(row for row in callbacks["callbacks"] if row["key"] == key)
    addon["addon_only"] = True
    sale_topic = 9
    sale = {
        "key": key + "-sale",
        "quest": addon["quest"],
        "npc": addon["npc"],
        "keywords": addon["keywords"],
        "topic": sale_topic,
        "next_topic": addon["next_topic"],
        "storage": [],
        "take": [{"item": f"oteryn:item.tibia.i{item}", "count": count}],
        "give": {
            "item": "oteryn:item.tibia.i3035",
            "count": gold // 100,
            "contents": [],
        },
        "outfit": None,
        "achievement": None,
        "repeatable": True,
        "reply": [free[2]],
        "failure_reply": [free[1]],
        "sources": [f"crystal:{PIN}:{path}:172-194:sha256:{sha(raw)}"],
    }
    callbacks["callbacks"] = [
        row for row in callbacks["callbacks"] if row["key"] != sale["key"]
    ]
    callbacks["callbacks"].insert(callbacks["callbacks"].index(addon) + 1, sale)
    validate_callbacks({"schema": 1, "callbacks": [sale]})

    def matches(row):
        return row["npc"] == addon["npc"] and row["keywords"] == ["sniper gloves"]

    offer = next(
        row
        for row in offers["offers"]
        if matches(row) and row["next_topic"] == addon["topic"]
    )
    offers["offers"] = [row for row in offers["offers"] if not matches(row)]
    offers["offers"].extend(
        [
            {
                **offer,
                "requires_premium": False,
                "next_topic": sale_topic,
                "storage": [],
                "reply": [free[0]],
            },
            {
                **offer,
                "requires_premium": True,
                "next_topic": sale_topic,
                "reply": [owned[0]],
                "storage": [
                    {**offer["storage"][0], "minimum": 1, "maximum": 1, "set": 1}
                ],
            },
            {**offer, "requires_premium": True},
        ]
    )
    return {offer_path: encoded(offers), callback_path: encoded(callbacks)}


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def lost_brother_shop():
    # Crystal 201189f8 tarun.lua:109-114 gates the exact menu at lines 124-149.
    source_prices = {
        "BuyFromPlayer": {
            11514: 110,
            283: 5,
            284: 5,
            285: 5,
            21974: 270,
            10304: 475,
            10321: 100,
            7786: 700,
            21975: 350,
            10311: 400,
            5879: 100,
            3044: 100,
            2874: 5,
        },
        "SellToPlayer": {
            239: 225,
            238: 158,
            7642: 254,
            266: 50,
            268: 56,
            236: 115,
            237: 108,
            23375: 650,
            7643: 379,
            23373: 488,
            23374: 488,
        },
    }
    expected = {
        (f"oteryn:item.tibia.i{item}", direction, price)
        for direction, items in source_prices.items()
        for item, price in items.items()
    }
    service = "oteryn:service.trade.tarun"
    directory = ROOT / "content/services/trade"
    index = json.loads((directory / "index.json").read_bytes())
    declarations = [
        row["declaration"]
        for name in index["shards"]
        for row in json.loads((ROOT / name).read_bytes())["records"]
    ]
    (shop,) = [row for row in declarations if row["identity"]["key"] == service]

    def identity(offer):
        return (offer["item"]["key"], offer["direction"], offer["unit_price"])

    offers = [offer for offer in shop["offers"] if identity(offer) in expected]
    if (
        len(offers) != len(expected)
        or {identity(offer) for offer in offers} != expected
    ):
        raise ValueError("Tarun catalogue differs from Crystal 201189f8 shop")
    requirement = {
        "quest": "oteryn:quest.adventurers_guild",
        "track": "oteryn:quest-progress/quest/u10_80/the_lost_brother_quest",
        "initial": 0,
        "minimum": 3,
        "maximum": 3,
    }
    return {
        "npc": "oteryn:npc.tarun",
        "service": service,
        "sources": [
            (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/tarun.lua:109-149:sha256:"
                "8e032f7015dd3ee2135ba9552a588a3f26083327e23fb7a1619423486c482fde"
            )
        ],
        "offers": [
            offer
            | {"requirements": [requirement]}
            # Crystal's unqualified vial is the empty fluid variant.
            | (
                {"sub_type": 0}
                if offer["item"]["key"] == "oteryn:item.tibia.i2874"
                else {}
            )
            for offer in offers
        ],
    }


def cover_legacy_addon_slots(outfits, legacy_outfits, mappings):
    by_key = {outfit["key"]: outfit for outfit in outfits}
    addon_mappings = {}
    covered = set()
    for mapping in mappings:
        target = mapping["target"]
        if target["kind"] != "Addon":
            continue
        key, addon = target["outfit_key"], target["addon"]
        addon_mappings.setdefault((key, addon), []).append(mapping)
        covered.add((key, addon))
        covered.update((alias, addon) for alias in mapping.get("outfit_aliases", []))

    for alias in legacy_outfits:
        key = alias["key"]
        prefix, suffix = "oteryn:content.outfit.", "_outfits"
        if not key.startswith(prefix) or not key.endswith(suffix):
            continue
        canonical_key = "oteryn:outfit." + key.removeprefix(prefix).removesuffix(suffix)
        canonical = by_key.get(canonical_key)
        if canonical is None or any(
            alias[sex] != canonical[sex] for sex in ("female", "male")
        ):
            continue
        for addon in range(1, alias["addon_count"] + 1):
            if (key, addon) in covered:
                continue
            # Reuse the earned receipt and its existing sex and provenance checks.
            for mapping in addon_mappings.get((canonical_key, addon), []):
                mapping["outfit_aliases"] = sorted(
                    [*mapping.get("outfit_aliases", []), key]
                )


def build():
    rottin = ROOT / "content/quests/npc-callbacks/rottin-wood.json"
    validate_callbacks(json.loads(rottin.read_bytes()))
    inputs = donor_inputs()
    gameplay = json.loads((ROOT / "content/spells.manifest.json").read_bytes())
    source = {
        "reference": "crystal:data/XML/outfits.xml",
        "revision": PIN,
        "sha256": sha(inputs["data/XML/outfits.xml"]),
    }
    xml = ET.fromstring(inputs["data/XML/outfits.xml"])
    # Explicit identities for the two sex-specific donor display names.
    aliases = {
        "Noblewoman": "Nobleman",
        "Norsewoman": "Norseman",
        "Retro Noblewoman": "Retro Nobleman",
    }
    groups = {}
    for row in xml:
        name = aliases.get(row.get("name"), row.get("name"))
        # S06 admits source definitions; ownership still requires an acquisition receipt.
        if row.get("enabled") == "yes":
            groups.setdefault(name, {})[row.get("type")] = row.attrib
    appearance = ROOT / "imports/cipsoft-appearances/outfits"
    manifest_raw = (appearance / "manifest.json").read_bytes()
    manifest = json.loads(manifest_raw)
    looks = {}
    addon_patterns = {}
    for shard in manifest["files"]:
        raw = (appearance / shard["path"]).read_bytes()
        if sha(raw) != shard["sha256"]:
            raise ValueError("appearance source drift")
        looks.update(
            {
                r["source_id"]: r["source_record_sha256"]
                for r in json.loads(raw)["records"]
            }
        )
        for record in json.loads(raw)["records"]:
            addon_patterns[record["source_id"]] = (
                min(
                    next(
                        (
                            f.get("value", 1)
                            for f in group["sprite_info_fields"]
                            if f["field"] == 2
                        ),
                        1,
                    )
                    for group in record["frame_groups"]
                )
                - 1
            )
    ram_look_hash = looks[1934]
    outfits, mappings = [], []
    for name, sexes in sorted(groups.items()):
        if set(sexes) != {"0", "1"}:
            raise ValueError("unpaired donor outfit: " + name)
        female, male = sexes["0"], sexes["1"]
        if any(int(s["looktype"]) not in looks for s in (female, male)):
            continue  # Newer looks need a later admitted appearance corpus; never fabricate one.
        key = "oteryn:outfit." + name.lower().replace(" ", "_").replace("-", "_")
        # Monk is explicitly addon-unavailable; all other admitted quest looks use bits 1/2.
        addons = (
            0
            if name == "Monk"
            else min(
                2,
                addon_patterns[int(female["looktype"])],
                addon_patterns[int(male["looktype"])],
            )
        )
        outfits.append(
            {
                "key": key,
                "revision": "crystal-9f5a72c-r1",
                "female": {
                    "id": int(female["looktype"]),
                    "source_record_sha256": looks[int(female["looktype"])],
                },
                "male": {
                    "id": int(male["looktype"]),
                    "source_record_sha256": looks[int(male["looktype"])],
                },
                "addon_count": addons,
                "starter_female": female["unlocked"] == "yes",
                "starter_male": male["unlocked"] == "yes",
                "starter_addons_female": 0,
                "starter_addons_male": 0,
                "premium": female["premium"] == "yes",
                "unlock_sources": [source],
                "source": source,
            }
        )
    earned_outfit_catalogue.append_wiki_outfits(ROOT, outfits, looks, addon_patterns)
    item_capture_raw = (
        ROOT / "tools/content-schema/native-gameplay/item-cosmetic-actions.json"
    ).read_bytes()
    item_admission = load_module(
        "qengine_item_admission",
        "tools/content-schema/quest-authoring/qengine_item_acquisitions.py",
    )
    table_parser = load_module(
        "qengine_cosmetic_source_tables",
        "tools/content-migration/event_item_actions.py",
    )
    acquisition_path = "data-crystal/scripts/special/addon_doll.lua"
    acquisition_source = {
        "reference": "crystal:" + acquisition_path,
        "revision": PIN,
        "sha256": sha(inputs[acquisition_path]),
    }
    source_rows, _ = table_parser.table(inputs[acquisition_path].decode(), "outfits")
    item_admission.extend_catalogue(
        outfits, source_rows, looks, addon_patterns, acquisition_source
    )
    item_packet = item_admission.build(
        json.loads(item_capture_raw), outfits, source, sha(item_capture_raw)
    )
    construction = load_module(
        "qengine_item_construction",
        "tools/content-schema/quest-authoring/qengine_item_construction.py",
    )
    construction_path = "data-global/scripts/actions/mounts/mounts.lua"
    construction.append_glider(
        item_packet,
        inputs[construction_path].decode(),
        inputs["data/XML/mounts.xml"],
        table_parser,
        json.loads(
            (ROOT / "content/presentations/bindings/client-ids.json").read_bytes()
        ),
        {
            "reference": "crystal:" + construction_path,
            "revision": PIN,
            "sha256": sha(inputs[construction_path]),
        },
    )
    qengine_mounts.append_pegasus_feather(item_packet)
    earned_outfit_catalogue.append_consumptive_outfit_grants(item_packet, outfits)
    # Retain every source row, but bind construction costs to the existing atomic
    # pieces owner. The ordinary one-item taming path cannot execute these rows.
    native_taming = json.loads((ROOT / "content/cosmetics/taming.json").read_bytes())
    recipes = {
        f"oteryn:item.tibia.i{r['item_id']}": r
        for r in item_packet["rules"]
        if r["mode"] == "pieces"
    }
    for rule in native_taming:
        if recipe := recipes.get(rule["item_key"]):
            rule["required_items"] = [
                f"oteryn:item.tibia.i{item}" for item in recipe["required_items"]
            ]
    for outfit in outfits:
        key = outfit["key"]
        for addon in range(outfit["addon_count"] + 1):
            target = (
                {"kind": "Outfit", "outfit_key": key}
                if addon == 0
                else {"kind": "Addon", "outfit_key": key, "addon": addon}
            )
            mappings.append(
                {
                    "grant_key": f"{key}/quest-grant/{addon}",
                    "target": target,
                    "compatible_provenance": [PROVENANCE],
                }
            )
    earned_outfit_catalogue.append_task_board_outfit_mappings(outfits, mappings)
    # Every native source grant must match the complete admitted sex pair.
    for outfit in outfits:
        outfit_looks = sorted([outfit["female"]["id"], outfit["male"]["id"]])
        for grant in item_packet["grants"]:
            if (
                grant["kind"] not in ("outfit", "addon")
                or grant.get("look_types") != outfit_looks
            ):
                continue
            target = (
                {"kind": "Outfit", "outfit_key": outfit["key"]}
                if grant["kind"] == "outfit"
                else {
                    "kind": "Addon",
                    "outfit_key": outfit["key"],
                    "addon": grant["addon"],
                }
            )
            mappings.append(
                {
                    "grant_key": grant["key"],
                    "target": target,
                    "compatible_provenance": [item_packet["provenance"]],
                }
            )
    mappings.append(
        {
            "grant_key": "oteryn:appearance-grant/quest/soul-war/revenant-base",
            "target": {"kind": "Outfit", "outfit_key": "oteryn:outfit.revenant"},
            "compatible_provenance": ["crystal-201189f-soul-war-revenant-v1"],
        }
    )
    # Keep the accepted Rotten Blood base receipt usable after catalogue promotion.
    # This owner deliberately has no admitted addon grant.
    mappings.append(
        {
            "grant_key": "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base",
            "target": {
                "kind": "Outfit",
                "outfit_key": "oteryn:outfit.decaying_defender",
            },
            "compatible_provenance": ["owner-pr2784-decaying-defender-base-v1"],
        }
    )
    for row in outfits:
        # Source NPCs may grant either sex independently; each mapping is authored.
        for sex in ("female", "male"):
            for addon in range(row["addon_count"] + 1):
                mappings.append(
                    {
                        "grant_key": row["key"] + f"/quest-grant/{sex}/{addon}",
                        "target": {"kind": "Outfit", "outfit_key": row["key"]}
                        if addon == 0
                        else {
                            "kind": "Addon",
                            "outfit_key": row["key"],
                            "addon": addon,
                        },
                        "sex": sex,
                        "compatible_provenance": [PROVENANCE],
                    }
                )
    mount_source = {
        "reference": "crystal:data/XML/mounts.xml",
        "revision": PIN,
        "sha256": sha(inputs["data/XML/mounts.xml"]),
    }
    mounts, mount_mappings = qengine_mounts.populate(
        ROOT, inputs, looks, mount_source, item_packet
    )
    mappings.extend(mount_mappings)
    lane_programmes = load_module(
        "questpc7_costume_sources",
        "tools/content-schema/quest-authoring/questpc7_outfit_programmes.py",
    )
    lane_programmes.consumptive_mappings(ROOT, outfits, mappings)
    atlas = load_module(
        "qengine_atlas_palette", "tools/game-atlas-appearances/export.py"
    )
    palette_path = "tools/game-atlas-appearances/export.py"
    palette_source = {
        "reference": palette_path + "#outfit_color",
        "revision": "rounded-hsi-133-v1",
        "sha256": sha((ROOT / palette_path).read_bytes()),
    }
    doc = {
        "schema": "OTERYN_CHARACTER_COSMETICS/v1",
        "content_revision": gameplay["build_training"]["content_revision"],
        "source_manifest_sha256": sha(manifest_raw),
        "source_appearances_sha256": manifest["inputs"]["appearances"]["sha256"],
        "palette": {
            "key": "oteryn:palette.outfit",
            "revision": "rounded-hsi-133-v1",
            "colours": [list(atlas.outfit_color(i)) for i in range(133)],
            "source": palette_source,
        },
        "outfits": outfits,
        "mounts": mounts,
        "grant_mappings": mappings,
        "item_acquisitions": [
            {
                "item_key": f"oteryn:item.tibia.i{r['item_id']}",
                "donor": r["donor"],
                "catalogue_sha256": sha(encoded(item_packet)),
                "decision": "owner:S06:source-bound-native-item-acquisitions:2026-11-07",
                "source": acquisition_source,
            }
            for r in item_packet["rules"]
            if r["mode"] == "random_outfit"
            and r["donor"].startswith(acquisition_path + ":")
        ],
        "fallbacks": {
            sex: {"outfit_key": "oteryn:outfit.citizen", "colours": [0, 0, 0, 0]}
            for sex in ("female", "male")
        },
    }
    cosmetics = load_module(
        "qengine_cosmetics_authoring",
        "tools/content-schema/cosmetics-authoring/cosmetics_authoring.py",
    )
    author = {
        "schema": "OTERYN_CHARACTER_COSMETICS_AUTHORING/v1",
        "profile": {
            "kind": "reference",
            "revision": "quest-cosmetics-r1",
            "mount_speed_bonus": 10,
        },
        "client_version": manifest["client_version"],
        "document": doc,
        "promotions": [],
    }
    # NX-5's promoted item grants and mounts remain admitted alongside quest rewards.
    nx5 = json.loads(
        (
            ROOT / "tools/content-schema/cosmetics-authoring/nx5-authoring.json"
        ).read_bytes()
    )
    cosmetics.validate(nx5)
    nx5_doc = nx5["document"]
    doc["outfits"].extend(nx5_doc["outfits"])
    # Retain main's existing mount identities and source promotions.
    nx5_mounts = {row["key"]: row for row in nx5_doc["mounts"]}
    doc["mounts"] = [nx5_mounts.get(row["key"], row) for row in mounts]
    # Reliable Ram uses the named look, not a mutable donor mount ordinal.
    # Crystal 201189f actions_reliable_ram.lua still says 250, but its mounts.xml
    # assigns Reliable Ram/look1934 to 252 (250 is Jaracal/look1962). The retained
    # 9f5a72c corpus agrees on the semantic Ram identity/look and source speed.
    ram = next(
        row
        for row in ET.fromstring(inputs["data/XML/mounts.xml"])
        if row.get("name") == "Reliable Ram"
    )
    if (
        ram.get("clientid") != "1934"
        or ram.get("speed") != "10"
        or ram.get("premium") != "no"
    ):
        raise ValueError("Reliable Ram source identity drift")
    ram_key = "oteryn:content.mount.reliable_ram"
    admitted_ram = [row for row in doc["mounts"] if row["key"] == ram_key]
    if (
        len(admitted_ram) != 1
        or admitted_ram[0]["look"]
        != {"id": 1934, "source_record_sha256": ram_look_hash}
        or admitted_ram[0]["speed_bonus"] != 10
        or admitted_ram[0]["premium"] is not False
    ):
        raise ValueError("Reliable Ram native catalogue identity drift")
    mappings.append(
        {
            "grant_key": ram_key + "/quest-grant",
            "target": {"kind": "Mount", "mount_key": ram_key},
            "compatible_provenance": [PROVENANCE],
        }
    )
    grant_keys = {row["grant_key"] for row in mappings}
    for row in nx5_doc["grant_mappings"]:
        if row["grant_key"] not in grant_keys:
            mappings.append(row)
        else:
            existing = next(m for m in mappings if m["grant_key"] == row["grant_key"])
            if existing["target"] != row["target"]:
                existing["outfit_aliases"] = [row["target"]["outfit_key"]]
    cover_legacy_addon_slots(outfits, nx5_doc["outfits"], mappings)
    # Taming uses the same production cosmetics owner as quest/item grants.
    taming_cosmetics.append(ROOT, doc, looks)
    nx5_promotions = {row["path"]: row for row in nx5["promotions"]}
    quest_outfits = len(outfits) - len(nx5_doc["outfits"])
    for path in sorted(cosmetics.promotion_paths(doc)):
        parts = path.split("/")
        nx5_path = None
        if parts[1] == "mounts":
            mount_key = doc["mounts"][int(parts[2])]["key"]
            for index, row in enumerate(nx5_doc["mounts"]):
                if row["key"] == mount_key:
                    nx5_path = "/mounts/" + str(index) + "/" + parts[3]
                    break
        if parts[1] == "outfits" and int(parts[2]) >= quest_outfits:
            nx5_path = "/outfits/" + str(int(parts[2]) - quest_outfits) + "/" + parts[3]
        if nx5_path is not None:
            author["promotions"].append({**nx5_promotions[nx5_path], "path": path})
            continue
        citation = (
            palette_source
            if path == "/palette"
            else doc["mounts"][int(parts[2])]["source"]
            if path.startswith("/mounts/")
            else source
        )
        source_kind = (
            "OWNER_DECISION"
            if path.startswith("/fallbacks/") or path == "/grant_mappings"
            else "STRUCTURED_REFERENCE_DATA"
        )
        if path == "/item_acquisitions":
            citation = acquisition_source
            source_kind = "OWNER_DECISION"
        if path.startswith("/outfits/"):
            row = outfits[int(path.split("/")[2])]
            citation = row["source"]
            if citation == acquisition_source and (
                path.rsplit("/", 1)[-1] == "premium"
                or path.rsplit("/", 1)[-1].startswith("starter_")
            ):
                source_kind = "OWNER_DECISION"
        if path in ("/source_manifest_sha256", "/source_appearances_sha256"):
            citation = {
                "reference": "imports/cipsoft-appearances/outfits/manifest.json",
                "revision": manifest["client_version"],
                "sha256": sha(manifest_raw),
            }
        author["promotions"].append(
            {
                "path": path,
                "value": cosmetics.pointer(doc, path),
                "state": "PROMOTED",
                "decision": DECISION,
                "source_kind": source_kind,
                "source": citation,
            }
        )
    callbacks = []

    def storage(track, before, after, maximum=None, preserve=False):
        return {
            "track": track,
            "minimum": before,
            "maximum": before if maximum is None else maximum,
            "set": after,
            **({"preserve": True} if preserve else {}),
        }

    def add(
        name,
        quest,
        npc,
        keywords,
        states,
        take=(),
        give=None,
        outfit=None,
        topic=0,
        next_topic=0,
        requires=(),
        present=(),
        achievement=None,
        fee=None,
        source=None,
    ):
        path = f"data-global/npc/{npc}.lua"
        callbacks.append(
            {
                "key": "oteryn:quest-reward/" + name,
                "quest": "oteryn:quest." + quest,
                "npc": "oteryn:npc." + npc,
                "keywords": keywords,
                "topic": topic,
                "next_topic": next_topic,
                "storage": states,
                "take": [
                    {"item": f"oteryn:item.tibia.i{i}", "count": n} for i, n in take
                ],
                "give": None
                if give is None
                else {
                    "item": f"oteryn:item.tibia.i{give[0]}",
                    "count": give[1],
                    "contents": [],
                },
                "outfit": None
                if outfit is None
                else {"outfit": "oteryn:outfit." + outfit[0], "addons": outfit[1]},
                "achievement": achievement
                or (
                    "oteryn:achievement/backpack_tourist"
                    if name == "sam-backpack"
                    else None
                ),
                "reply": ["Thank you for your help."],
                "sources": [
                    source or f"crystal:{PIN}:{path}:sha256:{sha(inputs[path])}"
                ],
            }
        )
        if fee is not None:
            callbacks[-1]["fee"] = fee
        if requires:
            callbacks[-1]["requires"] = list(requires)
        if present:
            callbacks[-1]["present"] = [
                {"item": f"oteryn:item.tibia.i{i}", "count": n} for i, n in present
            ]

    prefix = "oteryn:quest-progress/quest/"
    add(
        "pirate-sabre",
        "outfit_and_addon_quests",
        "morgan",
        ["firebird"],
        [storage(prefix + "u7_8/pirate_outfits/pirate_sabre_addon", 4, 5)],
        outfit=("pirate", 1),
    )
    add(
        "knight-sword",
        "outfit_and_addon_quests",
        "morgan",
        ["knight's sword"],
        [storage(prefix + "u7_8/knight_outfits/addon_sword", 1, 2)],
        take=[(5892, 1), (5880, 100)],
        outfit=("knight", 1),
        topic=1,
    )
    add(
        "deepling-helmet",
        "outfit_and_addon_quests",
        "navigator",
        ["yes"],
        [storage("oteryn:quest-progress/navigator", -1, 4, 0)],
        outfit=("deepling", 2),
        topic=8,
    )
    add(
        "hallowed-holy-water",
        "tibia_tales",
        "costello",
        ["amanda"],
        [storage(prefix + "u8_1/rest_in_hallowed_ground/questline", 1, 2)],
        give=(133, 1),
        topic=3,
    )
    add(
        "hallowed-water-offer",
        "tibia_tales",
        "costello",
        ["holy water"],
        [
            storage(
                prefix + "u8_1/rest_in_hallowed_ground/questline", 1, 1, preserve=True
            )
        ],
        next_topic=3,
    )
    # The offer is conversation-only and deliberately excluded from the durable callback list.
    offer = callbacks.pop()
    add(
        "outfox-helmet",
        "tibia_tales",
        "budrik",
        ["mission", "quest"],
        [
            storage(prefix + "u8_1/to_outfox_afox_quest", 1, 2),
            storage(prefix + "u8_1/to_outfox_afox_quest/questline", -1, 2, 1),
        ],
        take=[(139, 1)],
        give=(875, 1),
    )
    add(
        "sam-backpack",
        "sam_s_old_backpack_quest",
        "sam",
        ["yes"],
        [storage(prefix + "u7_5/sams_old_backpack/sams_old_backpack_npc", -1, 1, 0)],
        take=[(3244, 1)],
        topic=1,
    )
    # Offers bind confirmation to this NPC and the common conversation topic.
    offers = [
        {
            "npc": "oteryn:npc.morgan",
            "keywords": ["forge"],
            "topic": 0,
            "next_topic": 1,
            "reply": ["What would you like me to forge?"],
            "storage": [
                storage(prefix + "u7_8/knight_outfits/addon_sword", 1, 1, preserve=True)
            ],
        },
        {
            "npc": offer["npc"],
            "keywords": offer["keywords"],
            "topic": 0,
            "next_topic": 3,
            "reply": ["Who sent you?"],
            "storage": offer["storage"],
        },
        {
            "npc": "oteryn:npc.sam",
            "keywords": ["old backpack", "backpack"],
            "topic": 0,
            "next_topic": 1,
            "reply": ["Did you find my old backpack?"],
            "storage": callbacks[-1]["storage"],
        },
    ]
    for word, topic, next_topic in [
        ("flou", 0, 1),
        ("explain", 1, 2),
        ("yes", 2, 3),
        ("no", 3, 4),
        ("yes", 4, 5),
        ("yes", 5, 6),
        ("yes", 6, 7),
        ("helmet", 7, 8),
    ]:
        offers.append(
            {
                "npc": "oteryn:npc.navigator",
                "keywords": [word],
                "topic": topic,
                "next_topic": next_topic,
                "reply": ["Listen carefully."],
                "storage": [storage("oteryn:quest-progress/navigator", -1, 4, 0)],
            }
        )
    # L1: keep parent QuestState ownership for shared prerequisite tracks.
    frontier = prefix + "u8_54/the_new_frontier/"
    yalahar = prefix + "u8_4/in_service_of_yalahar/"

    def finish_outfit(other_track, other_addon, achievement, page, revision):
        trade = callbacks[-1]
        trade["forbids_addons"] = [trade["outfit"]]
        # Preserve the character-local completion callback for existing routes.
        trade["requires"] = [{"track": other_track, "minimum": -1, "maximum": 0}]
        callbacks.append(
            {
                **trade,
                "key": trade["key"] + "-complete",
                "requires": [{"track": other_track, "minimum": 1, "maximum": 1}],
                "requires_addons": [
                    {"outfit": trade["outfit"]["outfit"], "addons": other_addon}
                ],
                "achievement": "oteryn:achievement/" + achievement,
                "sources": [
                    *trade["sources"],
                    f"https://tibia.fandom.com/index.php?curid={page}&oldid={revision}",
                ],
            }
        )
        trade["achievement"] = "oteryn:achievement/" + achievement
        trade["achievement_requires_full_outfit"] = True

    for name, bit, item, word in [
        ("crest", 1, 10199, "crest"),
        ("tribal", 2, 10198, "tribal crest"),
    ]:
        add(
            "l1-warmaster-" + name,
            "the_new_frontier_quest",
            "cael",
            ["yes"],
            [
                storage(frontier + "questline", 27, 27, preserve=True),
                storage(frontier + "warmaster_addon_" + str(bit), -1, 1, 0),
            ],
            take=[(item, 1)],
            outfit=("warmaster", bit),
            topic=60 + bit - 1,
        )
        callbacks[-1].update(
            addon_only=True,
            requires_outfits=["oteryn:outfit.warmaster"],
            forbids_addons=[callbacks[-1]["outfit"]],
        )
        offers.append(
            {
                "npc": "oteryn:npc.cael",
                "keywords": [word],
                "topic": 0,
                "next_topic": 60 + bit - 1,
                "storage": callbacks[-1]["storage"],
                "reply": [
                    "Can I have that "
                    + ("serpent" if bit == 1 else "tribal")
                    + " crest?"
                ],
            }
        )
        finish_outfit(
            frontier + "warmaster_addon_" + str(3 - bit),
            3 - bit,
            "master_of_war",
            42269,
            795619,
        )
    for name, bit, item, words in [
        ("cape", 1, 11701, ["cape", "first addon"]),
        ("hat", 2, 11700, ["hat", "second addon"]),
    ]:
        track = "oteryn:quest-progress/wayfarer_outfit" + ("" if bit == 1 else "_1")
        add(
            "l1-wayfarer-" + name,
            "wayfarer_outfits_quest",
            "cranky_lizard_crone",
            ["yes"],
            [storage(track, -1, 1, 0)],
            take=[(item, 1)],
            outfit=("wayfarer", bit),
            topic=bit,
        )
        callbacks[-1]["requires_completed_quests"] = [
            "oteryn:quest.the_spirit_will_get_you_quest"
        ]
        callbacks[-1]["requires_premium"] = True
        callbacks[-1]["addon_only"] = True
        offers.append(
            {
                "npc": "oteryn:npc.cranky_lizard_crone",
                "keywords": words,
                "topic": 0,
                "next_topic": bit,
                "storage": callbacks[-1]["storage"],
                "requires": [
                    {
                        "track": "oteryn:quest-progress/l6/spirit/stage",
                        "minimum": 2,
                        "maximum": 2,
                    }
                ],
                "reply": [
                    "Do you have the "
                    + ("old cape" if bit == 1 else "sedge hat")
                    + " with you?"
                ],
            }
        )
        finish_outfit(
            "oteryn:quest-progress/wayfarer_outfit" + ("_1" if bit == 1 else ""),
            3 - bit,
            "peazzekeeper",
            46398,
            844411,
        )
    for npc, side in [("palimuth", 1), ("yalahari", 2)]:
        states = [
            storage(yalahar + "questline", 52, 53),
            storage(yalahar + "door_to_reward", -1, 1, 1),
            storage(yalahar + "mission10", -1, 4, 4),
        ]
        # Palimuth's completion checks questline only; Yalahari also requires
        # his side. The remaining tracks are assignments, not prerequisites.
        if npc == "yalahari":
            states.insert(
                1, storage(yalahar + "side_decision", side, side, preserve=True)
            )
        add(
            "l1-yalaharian-base-" + npc,
            "in_service_of_yalahar_quest",
            npc,
            ["mission"],
            states,
            outfit=("yalaharian", 0),
        )
        path = "data-global/npc/" + npc + ".lua"
        lines = "244-259" if npc == "palimuth" else "246-252"
        callbacks[-1]["sources"] = [
            f"crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}"
        ]
    for name, side, bit in [("sage", 1, 1), ("villain", 2, 2)]:
        add(
            "l1-yalaharian-" + name,
            "in_service_of_yalahar_quest",
            "palimuth",
            ["yes"],
            [
                storage(yalahar + "questline", 54, 55),
                storage(yalahar + "side_decision", side, side, preserve=True),
            ],
            outfit=("yalaharian", bit),
            topic=6,
            achievement="oteryn:achievement/yalahari_of_"
            + ("wisdom" if side == 1 else "power"),
        )
        page, revision = (62819, 753357) if side == 1 else (62818, 753121)
        callbacks[-1]["sources"].append(
            f"https://tibia.fandom.com/index.php?curid={page}&oldid={revision}"
        )
        # The captured NPC checks possession and retains the original crest.
        callbacks[-1]["present"] = [{"item": "oteryn:item.tibia.i9041", "count": 1}]
        offers.append(
            {
                "npc": "oteryn:npc.palimuth",
                "keywords": ["job", "addon"],
                "topic": 0,
                "next_topic": 6,
                "storage": callbacks[-1]["storage"],
                "reply": ["Did you bring me the vampiric crest?"],
            }
        )

    # Parent prerequisites stay read-only and retain their original quest owner.
    def requirement(track, value, maximum=None):
        return {
            "track": track,
            "minimum": value,
            "maximum": value if maximum is None else maximum,
        }

    library = prefix + "u11_80/the_secret_library/"
    permission = library + "library_permission"

    # The retained corpus and Crystal201189f use identical Sven and Dedoras blobs.
    def barbarian_and_library_source(npc, lines):
        path = f"data-global/npc/{npc}.lua"
        revision = "201189f871d23597b82caf88fd611454a2ddefb8"
        return f"crystal:{revision}:{path}:{lines}:sha256:{sha(inputs[path])}"

    declarations = [
        {
            "key": permission,
            "quest": "oteryn:quest.the_secret_library_quest",
            "initial": -1,
            "min": -1,
            "max": 7,
            "writable": True,
            "sources": [barbarian_and_library_source("dedoras", "90-193")],
        }
    ]
    reports = [
        ("isle", "small_islands", 4, "153-165"),
        ("fishmen", "liquid_death", 8, "119-131"),
        ("asuri", "asuras", 7, "142-153"),
        ("order", "falcon_bastion", 3, "131-142"),
        ("desert", "darashia", 9, "114-124"),
        ("museum", "mo_ta", 8, "104-115"),
    ]
    for keyword, name, value, lines in reports:
        add(
            "l2-battle-mage-report-" + keyword,
            "the_secret_library_quest",
            "dedoras",
            [keyword],
            [
                storage(library + name + "/questline", value - 1, value),
                storage(library + "questlog", -1, 1, 1),
            ],
        )
        callbacks[-1]["state_ops"] = [
            {
                "track": permission,
                "from": {"kind": "between", "minimum": -1, "maximum": 5},
                "effect": {"kind": "add_at_least", "value": 1, "minimum": 1},
            }
        ]
        callbacks[-1]["reply"] = ["I will add your findings to the library research."]
        callbacks[-1]["sources"] = [barbarian_and_library_source("dedoras", lines)]
    library_ready = [
        storage(library + "library_permission", 6, 7),
        storage(library + "questlog", -1, 1, 1),
    ]
    for _, name, value, _ in reports:
        library_ready.append(
            storage(library + name + "/questline", value, value, preserve=True)
        )
    add(
        "l2-battle-mage-base",
        "the_secret_library_quest",
        "dedoras",
        ["check"],
        library_ready,
        outfit=("battle_mage", 0),
        achievement="oteryn:achievement/battle_mage",
    )
    battle_mage_completions = []
    library_done = [requirement(library + "library_permission", 7)]
    for name, bit, item, count, words in [
        ("book", 1, 28792, 5, ["book", "books", "sturdy books"]),
        ("epaulettes", 2, 28793, 20, ["epaulettes"]),
    ]:
        addon = prefix + "u11_80/battle_mage_outfits/addon"
        other = addon + str(3 - bit)
        for complete in (False, True):
            add(
                "l2-battle-mage-" + name + ("-complete" if complete else ""),
                "battle_mage_outfits_quest",
                "dedoras",
                words,
                [storage(addon + str(bit), -1, 1, 0)],
                take=[(item, count)],
                outfit=("battle_mage", bit),
                topic=3,
                requires=library_done
                + [requirement(other, 1) if complete else requirement(other, -1, 0)],
                achievement="oteryn:achievement/exalted_battle_mage"
                if complete
                else None,
            )
            callbacks[-1].update(
                inventory=[
                    {"item": f"oteryn:item.tibia.i{item}", "minimum": count + 1, "maximum": 4294967295}
                ],
                failure_reply=["Bring more of the required materials before we trade."],
                sources=[barbarian_and_library_source("dedoras", "198-216")],
            )
            if complete:
                callbacks[-1]["completes"] = True
                completion = callbacks.pop()
                completion["sources"].append(
                    "https://tibia.fandom.com/?curid=86073&oldid=847545"
                )
                battle_mage_completions.append(completion)
    for word, topic, next_topic in [("addon", 0, 1), ("yes", 1, 2), ("yes", 2, 3)]:
        offers.append(
            {
                "npc": "oteryn:npc.dedoras",
                "keywords": [word],
                "topic": topic,
                "next_topic": next_topic,
                "storage": [],
                "requires": library_done,
                "reply": [
                    "I need five sturdy books for the first addon or twenty epaulettes for the second."
                ],
            }
        )
    barbarian = prefix + "u8_0/barbarian_test/"
    ice = "the_ice_islands_quest"
    stage = barbarian + "questline"
    for words, topic, next_topic, before, reply in [
        (["barbarian"], 0, 1, -1, "Ask me about the test to become a barbarian."),
        (["test"], 0, 2, -1, "Are you willing to take the barbarian test?"),
        (["test"], 1, 2, -1, "Are you willing to take the barbarian test?"),
        (["yes"], 2, 3, -1, "Can you take ten sips of our mead without passing out?"),
        (
            ["mead", "barbarian mead"],
            0,
            4,
            1,
            "Did you bring a honeycomb for the mead?",
        ),
    ]:
        offers.append(
            {
                "npc": "oteryn:npc.sven",
                "keywords": words,
                "topic": topic,
                "next_topic": next_topic,
                "storage": [storage(stage, before, before, preserve=True)],
                "reply": [reply],
            }
        )
    add(
        "sven-start",
        ice,
        "sven",
        ["yes"],
        [
            storage(stage, -1, 1),
            storage(barbarian + "mission01", -1, 1),
            storage(prefix + "u8_0/the_ice_islands/questline", 0, 1),
        ],
        topic=3,
    )
    callbacks[-1]["reply"] = ["Bring me a honeycomb and ask about mead."]
    add(
        "sven-honeycomb",
        ice,
        "sven",
        ["yes"],
        [
            storage(stage, 1, 2),
            storage(barbarian + "mission01", 1, 2),
            storage(barbarian + "mead_total_sips", 0, 0, 2147483647),
        ],
        take=[(5902, 1)],
        topic=4,
    )
    callbacks[-1]["reply"] = [
        "That buys twenty attempts at the mead barrel. Report back once you pass."
    ]
    add(
        "sven-mead-report",
        ice,
        "sven",
        ["mead", "barbarian mead"],
        [storage(stage, 3, 4), storage(barbarian + "mission02", -1, 1, 0)],
        give=(7140, 1),
    )
    callbacks[-1]["reply"] = [
        "Take this horn, fill it at the barrel, and use it to calm the bear before hugging it."
    ]
    add(
        "sven-bear-report",
        ice,
        "sven",
        ["hug", "bear hugging"],
        [storage(stage, 5, 6), storage(barbarian + "mission03", -1, 1, 0)],
    )
    callbacks[-1]["reply"] = [
        "Your last test is to push the mammoth northwest of town. Come back when you succeed."
    ]
    add(
        "sven-mammoth-report",
        "barbarian_test",
        "sven",
        ["mammoth", "mammoth pushing"],
        [storage("oteryn:quest-progress/native/barbarian_test/completed", -1, 1)],
        achievement="oteryn:achievement/honorary_barbarian",
    )
    callbacks[-1].update(
        completes=True,
        participants=[
            {"quest": "oteryn:quest." + ice, "storage": [storage(stage, 7, 8)]}
        ],
        reply=["You passed all three tests. Welcome as an honorary barbarian."],
    )
    for name, lines in [
        ("start", "129-143"),
        ("honeycomb", "144-152"),
        ("mead-report", "84-94"),
        ("bear-report", "96-105"),
        ("mammoth-report", "107-115"),
    ]:
        next(c for c in callbacks if c["key"] == "oteryn:quest-reward/sven-" + name)[
            "sources"
        ] = [barbarian_and_library_source("sven", lines)]
    feathers = prefix + "u11_02/the_first_dragon/feathers"
    first_dragon_victory = (
        "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5"
    )
    add(
        "l2-festive-base",
        "the_first_dragon_quest",
        "the_first_dragon",
        ["reward"],
        [storage(feathers, 1, 2)],
        outfit=("festive", 0),
    )
    # The native fight on main records its selected victory on s5. Feather
    # credit from the original route continues to qualify via the row above.
    add(
        "l2-festive-base-earned",
        "the_first_dragon_quest",
        "the_first_dragon",
        ["reward"],
        [storage(feathers, 0, 2)],
        outfit=("festive", 0),
        requires=[requirement(first_dragon_victory, 1)],
    )
    festive_ready = [requirement(feathers, 2)]
    # Muriel's captured offers require the stage that only those offers initialize.
    # Bootstrap with parent completion and retained matching evidence; only the
    # subsequent exact hand-in consumes items and earns the addon.
    offers.append(
        {
            "npc": "oteryn:npc.muriel",
            "keywords": ["addons"],
            "topic": 0,
            "next_topic": 1,
            "storage": [],
            "requires": festive_ready,
            "reply": ["Choose the first or second addon."],
        }
    )
    for name, bit, item, count, choice, word in [
        ("mask", 1, 25088, 3, "first", "mask"),
        ("feather", 2, 25089, 50, "second", "feather"),
    ]:
        track = prefix + "u11_02/festive_outfits/addon" + str(bit)
        add(
            "l2-festive-" + name + "-start",
            "festive_outfits_quest",
            "muriel",
            [choice],
            [storage(track, -1, 1, 0)],
            topic=1,
            requires=festive_ready,
            present=[(item, count)],
        )
        add(
            "l2-festive-" + name,
            "festive_outfits_quest",
            "muriel",
            [word, word + "s"],
            [storage(track, 1, 2)],
            take=[(item, count)],
            outfit=("festive", bit),
            requires=festive_ready,
        )
        callbacks[-1].update(
            requires_outfits=["oteryn:outfit.festive"],
            achievement_progress=[
                {"key": "oteryn:achievement/reason_to_celebrate", "target": 2}
            ],
        )
    elementalist = prefix + "u9_1/elementalist_outfits/"
    add(
        "l2-elementalist-start",
        "elementalist_outfits_quest",
        "broken_servant_sentry",
        ["yes"],
        [
            storage(elementalist + "outfit", -1, 1, 0),
            storage(elementalist + "questline", -1, 1, 1),
        ],
        topic=2,
    )
    add(
        "l2-elementalist-base",
        "elementalist_outfits_quest",
        "broken_servant_sentry",
        ["yes"],
        [storage(elementalist + "outfit", 1, 2)],
        take=[(12601, 20)],
        outfit=("elementalist", 0),
        topic=4,
    )
    for name, bit, item, words in [
        ("cap", 1, 12599, ["cap", "mage"]),
        ("staff", 2, 12803, ["staff", "spike"]),
    ]:
        add(
            "l2-elementalist-" + name,
            "elementalist_outfits_quest",
            "broken_servant_sentry",
            words,
            [
                storage(elementalist + "outfit", 2, 2, preserve=True),
                storage(elementalist + "addon" + str(bit), -1, 1, 0),
            ],
            take=[(item, 1)],
            outfit=("elementalist", bit),
        )
    for before, maximum, first, second in [(-1, 0, 1, 2), (1, 1, 3, 4)]:
        guard = [
            storage(elementalist + "outfit", before, before, maximum, preserve=True)
        ]
        offers.append(
            {
                "npc": "oteryn:npc.broken_servant_sentry",
                "keywords": ["slime", "mould", "fungus", "sample"],
                "topic": 0,
                "next_topic": first,
                "storage": guard,
                "reply": ["Did you bring samples for me to analyse?"],
            }
        )
        offers.append(
            {
                "npc": "oteryn:npc.broken_servant_sentry",
                "keywords": ["yes"],
                "topic": first,
                "next_topic": second,
                "storage": guard,
                "reply": ["Let me analyse the samples."],
            }
        )
    annihilator = prefix + "u7_24/the_annihilator/reward"
    add(
        "l3-demon-base",
        "the_annihilator_quest",
        "avar_tar",
        ["base"],
        [storage(annihilator, 1, 2)],
        outfit=("demon", 0),
        topic=2,
    )
    demon_helmet = prefix + "u6_4/demon_helmet/rewards/demon_helmet"
    demon_oak = prefix + "u8_2/the_demon_oak/done"
    for name, quest, track, before, after, bit, other, other_done in [
        ("shield", "demon_helmet_quest", demon_helmet, 1, 2, 1, demon_oak, 4),
        ("helmet", "demon_oak", demon_oak, 3, 4, 2, demon_helmet, 2),
    ]:
        for complete in (False, True):
            guard = (
                requirement(other, other_done)
                if complete
                else requirement(other, -1, other_done - 1)
            )
            add(
                "l3-demon-" + name + ("-complete" if complete else ""),
                quest,
                "avar_tar",
                [name],
                [storage(track, before, after)],
                outfit=("demon", bit),
                topic=2,
                requires=[requirement(annihilator, 2), guard],
                achievement="oteryn:achievement/true_dedication" if complete else None,
            )
            if complete:
                callbacks[-1]["requires_addons"] = [
                    {"outfit": "oteryn:outfit.demon", "addons": 3 - bit}
                ]
                callbacks[-1]["sources"].append(
                    "wiki:True Dedication:achievementid=324:full Demon Outfits"
                )
    # Avar Tar's donor selector matches the word outfit in the Demon Outfit phrase.
    for words, topic, next_topic in [
        (["outfit", "demon outfit"], 0, 1),
        (["yes"], 1, 2),
    ]:
        offers.append(
            {
                "npc": "oteryn:npc.avar_tar",
                "keywords": words,
                "topic": topic,
                "next_topic": next_topic,
                "storage": [],
                "reply": ["Choose the base, shield or helmet."],
            }
        )
    dark = prefix + "u10_50/dark_trails/"
    votes = prefix + "u10_50/oramond_quest/voting_points"
    glooth = prefix + "u10_50/glooth_engineer_outfits/"
    add(
        "l3-glooth-engineer-base",
        "dark_trails_quest",
        "ezebeth",
        ["outfit"],
        [storage(dark + "outfit", -1, 1, 0)],
        outfit=("glooth_engineer", 0),
        requires=[requirement(dark + "mission18", 1)],
    )
    for suffix, bit, minimum, maximum, finished_second in [
        ("second", 2, 6, 2147483647, False),
        ("second-after-first", 2, 6, 2147483647, True),
        ("first-low", 1, 3, 5, False),
        ("first-high", 1, 6, 2147483647, True),
    ]:
        required = [
            requirement(dark + "outfit", 1),
            {"track": votes, "minimum": minimum, "maximum": maximum},
        ]
        if bit == 2:
            required.append(
                requirement(glooth + "addon1", 1)
                if finished_second
                else requirement(glooth + "addon1", -1, 0)
            )
        elif finished_second:
            required.append(requirement(glooth + "addon2", 1))
        add(
            "l3-glooth-engineer-" + suffix,
            "glooth_engineer_outfits_quest",
            "ezebeth",
            ["addon"],
            [storage(glooth + "addon" + str(bit), -1, 1, 0)],
            outfit=("glooth_engineer", bit),
            requires=required,
            achievement="oteryn:achievement/glooth_engineer"
            if finished_second
            else None,
        )
        callbacks[-1]["requires_outfits"] = ["oteryn:outfit.glooth_engineer"]
        if finished_second:
            callbacks[-1]["requires_addons"] = [
                {"outfit": "oteryn:outfit.glooth_engineer", "addons": 3 - bit}
            ]
            callbacks[-1]["storage"].append(storage(glooth + "completed", -1, 1))
            callbacks[-1]["completes"] = True
    add(
        "l3-glooth-engineer-complete-owned",
        "glooth_engineer_outfits_quest",
        "ezebeth",
        ["addon"],
        [storage(glooth + "completed", -1, 1)],
        requires=[requirement(glooth + "addon1", 1), requirement(glooth + "addon2", 1)],
        achievement="oteryn:achievement/glooth_engineer",
    )
    callbacks[-1]["requires_addons"] = [
        {"outfit": "oteryn:outfit.glooth_engineer", "addons": 3}
    ]
    callbacks[-1]["completes"] = True
    # Reward guards load before the native kill adapter installs this exact track.
    declarations.append(
        {
            "key": first_dragon_victory,
            "quest": "oteryn:quest.the_first_dragon_quest",
            "initial": 0,
            "min": 0,
            "max": 1,
            "sources": [
                "content/encounters/bosses/selected-first-dragon.json#stage-s5"
            ],
        }
    )
    noble = prefix + "u7_8/nobleman_outfits/"
    first = noble + "nobleman_first_addon"
    second = noble + "nobleman_second_addon"
    for name, track, topic, pay_topic, bit, words in [
        ("first", first, 23, 24, 1, ["dress", "coat"]),
        ("second", second, 29, 30, 2, ["hat"]),
    ]:
        parent = [] if bit == 1 else [storage(first, 2, 2, preserve=True)]
        add(
            "l4-nobleman-" + name + "-start",
            "nobleman_outfits_quest",
            "irmana",
            ["150000"],
            parent + [storage(track, -1, 1, 0)],
            topic=topic,
        )
        add(
            "l4-nobleman-" + name,
            "nobleman_outfits_quest",
            "irmana",
            ["yes"],
            parent + [storage(track, 1, 2)],
            outfit=("nobleman", bit),
            topic=pay_topic,
            fee={"gold": 150000, "include_bank": False},
            achievement="oteryn:achievement/aristocrat" if bit == 2 else None,
        )
        start_topic = 19 if bit == 1 else 25
        guard = parent + [storage(track, -1, 0, 0, preserve=True)]
        for keywords, before, after in [
            (["addon"], 0, start_topic),
            (["yes"], start_topic, start_topic + 1),
            (words, start_topic + 1, start_topic + 2),
            (["yes"], start_topic + 2, start_topic + 3),
            (["yes"], start_topic + 3, topic),
        ]:
            offers.append(
                {
                    "npc": "oteryn:npc.irmana",
                    "keywords": keywords,
                    "topic": before,
                    "next_topic": after,
                    "storage": guard,
                    "reply": ["The accessory costs 150000 gold carried with you."],
                }
            )
        offers.append(
            {
                "npc": "oteryn:npc.irmana",
                "keywords": ["addon"],
                "topic": 0,
                "next_topic": pay_topic,
                "storage": parent + [storage(track, 1, 1, preserve=True)],
                "reply": ["Are you here to collect your accessory for 150000 gold?"],
            }
        )
    royal = "oteryn:quest-progress/outfit_quest/royal_costume_outfit"
    for name, before, after, silver, gold, bit, topic, word in [
        ("base", -1, 1, 15000, 12500, 0, 16, "armor"),
        ("shield", 1, 2, 7500, 6250, 1, 17, "shield"),
        ("crown", 2, 3, 7500, 6250, 2, 18, "crown"),
    ]:
        guard = [storage(royal, before, after, 0 if bit == 0 else before)]
        add(
            "l4-royal-costume-" + name,
            "royal_costume_outfits_quest",
            "percybald",
            ["yes"],
            guard,
            take=[(22516, silver), (22721, gold)],
            outfit=("royal_costume", bit),
            topic=topic,
            next_topic=15,
        )
        if name == "crown":
            callbacks[-1]["completes"] = True
        offers.append(
            {
                "npc": "oteryn:npc.percybald",
                "keywords": [word],
                "topic": 15,
                "next_topic": topic,
                "storage": guard,
                "reply": [
                    f"The donation is {silver} silver tokens and {gold} gold tokens."
                ],
            }
        )
    for words, before, after in [
        (["outfit", "addon", "royal"], 0, 14),
        (["yes"], 14, 15),
    ]:
        offers.append(
            {
                "npc": "oteryn:npc.percybald",
                "keywords": words,
                "topic": before,
                "next_topic": after,
                "storage": [],
                "reply": ["Choose armor, shield or crown for a token donation."],
            }
        )
    pirate = prefix + "u7_8/pirate_outfits/"
    shattered = prefix + "u7_8/the_shattered_isles/"
    pirate_owner = (
        "outfit_and_addon_quests"  # retain the admitted shared source track owner
    )
    add(
        "l4-pirate-base",
        pirate_owner,
        "raymond_striker",
        ["pirate outfit"],
        [storage(pirate + "pirate_base_outfit", -1, 1, 0)],
        outfit=("pirate", 0),
        requires=[
            requirement(shattered + "access_to_laguna_island", 1),
            requirement(shattered + "rays_mission4", 5),
        ],
    )
    sabre = pirate + "pirate_sabre_addon"
    start = [
        storage(sabre, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", -1, 1, 1),
    ]
    add("l4-pirate-sabre-start", pirate_owner, "duncan", ["yes"], start, topic=2)
    for words, before, after in [(["task"], 0, 1), (["yes"], 1, 2)]:
        offers.append(
            {
                "npc": "oteryn:npc.duncan",
                "keywords": words,
                "topic": before,
                "next_topic": after,
                "storage": [storage(sabre, -1, 0, 0, preserve=True)],
                "reply": [
                    "Bring 100 eye patches, 100 peg legs and 100 hooks in that order."
                ],
            }
        )
    for name, before, after, item, topic, words in [
        ("eye-patches", 1, 2, 6098, 3, ["eye patches"]),
        ("peg-legs", 2, 3, 6126, 4, ["peg legs"]),
        ("hooks", 3, 4, 6097, 5, ["hooks", "pirate hooks"]),
    ]:
        states = [storage(sabre, before, after)]
        add(
            "l4-pirate-" + name,
            pirate_owner,
            "duncan",
            ["yes"],
            states,
            take=[(item, 100)],
            topic=topic,
        )
        offers.append(
            {
                "npc": "oteryn:npc.duncan",
                "keywords": words,
                "topic": 0,
                "next_topic": topic,
                "storage": states,
                "reply": ["Have you gathered 100 " + words[0] + "?"],
            }
        )
    # Morgan's existing firebird callback owns the only sabre grant; do not duplicate it.
    hat = pirate + "pirate_hat_addon"
    hat_start = [
        storage(pirate + "pirate_base_outfit", 1, 1, preserve=True),
        storage(hat, -1, 1, 0),
    ]
    add("l4-pirate-hat-start", pirate_owner, "ariella", ["yes"], hat_start, topic=4)
    for words, before, after in [(["addon"], 0, 2), (["task"], 2, 3), (["yes"], 3, 4)]:
        offers.append(
            {
                "npc": "oteryn:npc.ariella",
                "keywords": words,
                "topic": before,
                "next_topic": after,
                "storage": hat_start,
                "reply": ["Bring proof from the four pirate leaders."],
            }
        )
    add(
        "l4-pirate-hat",
        pirate_owner,
        "ariella",
        ["yes"],
        [storage(hat, 1, 2)],
        take=[(6101, 1), (6102, 1), (6100, 1), (6099, 1)],
        outfit=("pirate", 2),
        topic=5,
    )
    offers.append(
        {
            "npc": "oteryn:npc.ariella",
            "keywords": ["task"],
            "topic": 0,
            "next_topic": 5,
            "storage": [storage(hat, 1, 2)],
            "reply": ["Did you bring the shirt, sabre, hat and eye patch?"],
        }
    )
    oriental = prefix + "u7_8/oriental_outfits/"
    orc_powder = prefix + "u7_6/explorer_society/the_orc_powder"
    # The door belongs to Oriental; the source addon/global-start tracks retain their shared owner.
    for sex, first_npc, second_npc in [
        ("female", "ishina", "miraia"),
        ("male", "habdel", "razan"),
    ]:
        first = oriental + "first_oriental_addon"
        second = oriental + "second_oriental_addon"
        common = "oteryn:quest-progress/outfit_quest/default_start"
        states = [storage(first, -1, 1, 0), storage(common, -1, 1, 1)]
        add(
            "l5-oriental-" + sex + "-first-start",
            "outfit_and_addon_quests",
            first_npc,
            ["yes"],
            states,
            topic=2,
            requires=[requirement(orc_powder, 35, 2147483647)],
        )
        callbacks[-1]["creation_sex"] = sex
        callbacks[-1]["participants"] = [
            {
                "quest": "oteryn:quest.oriental_outfits_quest",
                "storage": [storage(oriental + "oriental_door", 0, 1, 1)],
            }
        ]
        for words, before, after in [(["outfit"], 0, 1), (["yes"], 1, 2)]:
            offers.append(
                {
                    "npc": "oteryn:npc." + first_npc,
                    "keywords": words,
                    "topic": before,
                    "next_topic": after,
                    "storage": states,
                    "requires": [requirement(orc_powder, 35, 2147483647)],
                    "reply": ["Could you bring a mermaid comb?"],
                }
            )
        add(
            "l5-oriental-" + sex + "-comb",
            "outfit_and_addon_quests",
            first_npc,
            ["yes"],
            [storage(first, 1, 2)],
            take=[(5945, 1)],
            outfit=("oriental", 1),
            topic=3,
        )
        callbacks[-1]["creation_sex"] = sex
        offers.append(
            {
                "npc": "oteryn:npc." + first_npc,
                "keywords": ["comb", "mermaid comb", "mermaid's comb"],
                "topic": 0,
                "next_topic": 3,
                "storage": [storage(first, 1, 2)],
                "reply": ["Have you brought the mermaid comb?"],
            }
        )
        states = [storage(second, -1, 1, 0), storage(common, -1, 1, 1)]
        add(
            "l5-oriental-" + sex + "-second-start",
            "outfit_and_addon_quests",
            second_npc,
            ["yes"],
            states,
            topic=2,
        )
        callbacks[-1]["creation_sex"] = sex
        for words, before, after in [(["task"], 0, 1), (["yes"], 1, 2)]:
            offers.append(
                {
                    "npc": "oteryn:npc." + second_npc,
                    "keywords": words,
                    "topic": before,
                    "next_topic": after,
                    "storage": states,
                    "reply": [
                        "Bring ape fur, fish fins, enchanted chicken wings and blue cloth in order."
                    ],
                }
            )
        for name, word, item, count, before in [
            ("ape-fur", "ape fur", 5883, 100, 1),
            ("fish-fins", "fish fins", 5895, 100, 2),
            ("wings", "enchanted chicken wings", 5891, 2, 3),
            ("blue-cloth", "blue cloth", 5912, 100, 4),
        ]:
            states = [storage(second, before, before + 1)]
            add(
                "l5-oriental-" + sex + "-" + name,
                "outfit_and_addon_quests",
                second_npc,
                ["yes"],
                states,
                take=[(item, count)],
                outfit=("oriental", 2) if before == 4 else None,
                topic=3,
            )
            callbacks[-1]["creation_sex"] = sex
            offers.append(
                {
                    "npc": "oteryn:npc." + second_npc,
                    "keywords": [word],
                    "topic": 0,
                    "next_topic": 3,
                    "storage": states,
                    "reply": ["Did you bring the " + word + "?"],
                }
            )
    ice = prefix + "u8_0/the_ice_islands/"
    norseman = ice + "norseman_outfit"
    shared_start = "oteryn:quest-progress/outfit_quest/default_start"
    add(
        "l5-norseman-base",
        "the_ice_islands_quest",
        "hjaern",
        ["mission"],
        [
            storage(ice + "questline", 39, 40),
            storage(ice + "mission12", 0, 6, 2147483647),
            storage(ice + "yakchal_door", 0, 1, 1),
        ],
        take=[(7289, 1)],
        outfit=("norseman", 0),
        requires=[requirement(ice + "obelisk0" + str(n), 5) for n in range(1, 5)],
    )
    callbacks[-1]["participants"] = [
        {
            "quest": "oteryn:quest.outfit_and_addon_quests",
            "storage": [storage(norseman, 0, 1, 3), storage(shared_start, -1, 1, 1)],
        }
    ]
    for name, count, before, topic in [("first", 5, 40, 3), ("second", 10, 42, 4)]:
        states = [storage(ice + "questline", before, before + 1)]
        add(
            "l5-norseman-" + name + "-shards",
            "the_ice_islands_quest",
            "hjaern",
            ["yes"],
            states,
            take=[(7290, count)],
            topic=topic,
        )
        # The donor's shard keyword is a root branch, including after reward topic3/4.
        for current_topic in (0, 3, 4):
            offers.append(
                {
                    "npc": "oteryn:npc.hjaern",
                    "keywords": ["shard"],
                    "topic": current_topic,
                    "next_topic": topic,
                    "storage": states,
                    "reply": ["Have you brought " + str(count) + " frostheart shards?"],
                }
            )
        bit = 1 if name == "first" else 2
        add(
            "l5-norseman-" + name + "-reward",
            "the_ice_islands_quest",
            "hjaern",
            ["reward"],
            [storage(ice + "questline", before + 1, before + 2)],
            outfit=("norseman", bit),
            next_topic=topic,
            achievement="oteryn:achievement/out_in_the_snowstorm" if bit == 2 else None,
        )
        callbacks[-1]["participants"] = [
            {
                "quest": "oteryn:quest.outfit_and_addon_quests",
                "storage": [storage(norseman, 0, bit + 1, 3)],
            }
        ]
        if bit == 2:
            callbacks[-1]["sources"].append(
                "https://tibia.fandom.com/index.php?curid=62872&oldid=752988"
            )
    rook_body = prefix + "u7_8/druid_outfits_rook/druid_body_addon_rook"
    add(
        "l6-druid-rook-start",
        "druid_outfits_quest_on_rook",
        "tom",
        ["addon"],
        [storage(rook_body, -1, 1, 0)],
    )
    add(
        "l6-druid-rook-body",
        "druid_outfits_quest_on_rook",
        "tom",
        ["yes"],
        [storage(rook_body, 1, 2)],
        take=[(5896, 50), (5897, 50)],
        outfit=("druid", 1),
        topic=1,
    )
    offers.append(
        {
            "npc": "oteryn:npc.tom",
            "keywords": ["paws", "bear paws"],
            "topic": 0,
            "next_topic": 1,
            "storage": [storage(rook_body, 1, 2)],
            "reply": ["Have you brought 50 bear paws and 50 wolf paws?"],
        }
    )
    citizen_rook = prefix + "u7_8/citizen_outfits_rook/"
    rook_ref = "oteryn:quest-progress/outfit_quest/ref"
    ref_source = f"crystal:{PIN}:data-global/npc/amber.lua:103-106:sha256:{sha(inputs['data-global/npc/amber.lua'])}"
    declarations.append(
        {
            "key": rook_ref,
            "quest": "oteryn:quest.outfit_and_addon_quests",
            "initial": 0,
            "min": 0,
            "max": 2147483647,
            "writable": True,
            "sources": [ref_source],
        }
    )
    backpack = citizen_rook + "addon_backpack_rook"
    mission = citizen_rook + "mission_backpack_rook"
    timer = citizen_rook + "addon_backpack_rook_timer"
    states = [storage(backpack, -1, 1, 0), storage(mission, 0, 1, 4)]
    add(
        "l6-citizen-rook-backpack-start",
        "citizen_outfits_quest_on_rook",
        "amber",
        ["yes"],
        states,
        topic=2,
    )
    callbacks[-1]["participants"] = [
        {
            "quest": "oteryn:quest.outfit_and_addon_quests",
            "storage": [],
            "state_ops": [
                {
                    "track": rook_ref,
                    "from": {"kind": "between", "minimum": 0, "maximum": 2147483646},
                    "effect": {"kind": "add", "value": 1},
                }
            ],
        }
    ]
    for words, before, after in [
        (["addon", "outfit"], 0, 1),
        (["backpack", "minotaur", "leather"], 1, 2),
    ]:
        offers.append(
            {
                "npc": "oteryn:npc.amber",
                "keywords": words,
                "topic": before,
                "next_topic": after,
                "storage": states,
                "reply": [
                    "Would you bring 100 minotaur leathers for a handmade backpack?"
                ],
            }
        )
    states = [storage(backpack, 1, 2), storage(mission, 0, 2, 4)]
    add(
        "l6-citizen-rook-backpack-materials",
        "citizen_outfits_quest_on_rook",
        "amber",
        ["yes"],
        states,
        take=[(5878, 100)],
        topic=3,
    )
    callbacks[-1]["forbids_addons"] = [{"outfit": "oteryn:outfit.citizen", "addons": 1}]
    callbacks[-1]["state_ops"] = [
        {"track": timer, "from": {"kind": "any"}, "effect": {"kind": "set_now"}}
    ]
    offers.append(
        {
            "npc": "oteryn:npc.amber",
            "keywords": ["addon", "outfit", "backpack", "leather"],
            "topic": 0,
            "next_topic": 3,
            "storage": states,
            "reply": [
                "Did you bring 100 minotaur leathers? Your backpack takes two hours to make."
            ],
        }
    )
    add(
        "l6-citizen-rook-backpack",
        "citizen_outfits_quest_on_rook",
        "amber",
        ["addon", "outfit", "backpack"],
        [storage(backpack, 2, 3), storage(mission, 2, 4)],
        outfit=("citizen", 1),
    )
    callbacks[-1]["state_ops"] = [
        {
            "track": timer,
            "from": {"kind": "elapsed_at_least", "seconds": 7201},
            "effect": {"kind": "add", "value": 0},
        }
    ]
    callbacks[-1]["participants"] = [
        {
            "quest": "oteryn:quest.outfit_and_addon_quests",
            "storage": [storage(rook_ref, 0, 0, 2147483647)],
        }
    ]
    offers.append(
        {
            "npc": "oteryn:npc.amber",
            "keywords": ["addon", "outfit"],
            "topic": 0,
            "next_topic": 0,
            "storage": [storage(backpack, 3, 3)],
            "reply": [
                "Sorry, but I can only make one backpack per person, else I'd have to close my shop and open a leather manufactory."
            ],
        }
    )
    hat = citizen_rook + "addon_hat_rook"
    hat_mission = citizen_rook + "mission_hat_rook"
    # The offer sort is stable. Crystal checks active hand-in before the mission-zero recipe.
    states = [storage(hat, 1, 2), storage(hat_mission, 0, 0, 1)]
    add(
        "l6-citizen-rook-hat",
        "citizen_outfits_quest_on_rook",
        "norma",
        ["yes"],
        states,
        take=[(3374, 1), (5902, 50), (5890, 100)],
        outfit=("citizen", 2),
        topic=2,
    )
    callbacks[-1]["forbids_addons"] = [{"outfit": "oteryn:outfit.citizen", "addons": 2}]
    offers.append(
        {
            "npc": "oteryn:npc.norma",
            "keywords": ["addon", "outfit", "hat"],
            "topic": 0,
            "next_topic": 2,
            "storage": states,
            "reply": [
                "Have you brought a legion helmet, 50 honeycombs and 100 chicken feathers?"
            ],
        }
    )
    # Crystal reoffers the recipe when the mission returns to zero after hand-in.
    states = [storage(hat, -1, 1, 2), storage(hat_mission, 0, 1)]
    add(
        "l6-citizen-rook-hat-start",
        "citizen_outfits_quest_on_rook",
        "norma",
        ["yes"],
        states,
        topic=1,
    )
    offers.append(
        {
            "npc": "oteryn:npc.norma",
            "keywords": ["addon", "outfit", "hat"],
            "topic": 0,
            "next_topic": 1,
            "storage": states,
            "reply": ["Shall I teach you how to make a feather hat?"],
        }
    )
    qb_citizen_warrior.append(add, storage, offers, callbacks, prefix)
    qb_barbarian.append(add, storage, offers, callbacks, prefix)
    wizard_cosmetics.append(add, storage, offers, prefix)
    beggar = prefix + "u7_8/beggar_outfits/"
    beggar_stage = beggar + "beggar_outfit"
    beggar_timer = beggar + "beggar_outfit_timer"
    beggar_addon_timer = beggar + "beggar_outfit_timer_addon"
    beggar_door = beggar + "beggar_first_addon_door"
    general = "outfit_and_addon_quests"

    def beggar_source(npc, lines):
        path = "data-global/npc/" + npc + ".lua"
        return f"crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}"

    declarations.append(
        {
            "key": beggar_door,
            "quest": "oteryn:quest." + general,
            "initial": 0,
            "min": -1,
            "max": 1,
            "writable": True,
            "sources": [beggar_source("simon_the_beggar", "158-166")],
        }
    )

    def beggar_offer(npc, words, topic, next_topic, states, reply):
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": words,
                "topic": topic,
                "next_topic": next_topic,
                "storage": states,
                "reply": [reply],
            }
        )

    states = [
        storage(beggar_stage, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add("l6-beggar-start", general, "hugo", ["yes"], states, topic=3)
    callbacks[-1]["requires_premium"] = True
    for word, topic, next_topic in [("outfit", 0, 2), ("yes", 2, 3)]:
        beggar_offer(
            "hugo",
            [word],
            topic,
            next_topic,
            states,
            "Will you help gather the materials for my creation?",
        )
    for suffix, word, item, count, stage in [
        ("cloth", "20 brown pieces of cloth", 5913, 20, 1),
        ("leather", "50 minotaur leathers", 5878, 50, 2),
        ("wings", "10 bat wings", 5894, 10, 3),
        ("blossoms", "30 heaven blossoms", 5921, 30, 4),
    ]:
        states = [storage(beggar_stage, stage, stage + 1)]
        add(
            "l6-beggar-" + suffix,
            general,
            "hugo",
            ["yes"],
            states,
            take=[(item, count)],
            topic=4,
        )
        if suffix == "blossoms":
            callbacks[-1]["state_ops"] = [
                {
                    "track": beggar_timer,
                    "from": {"kind": "any"},
                    "effect": {"kind": "set_now"},
                }
            ]
        beggar_offer(
            "hugo",
            [word],
            0,
            4,
            states,
            f"Have you brought {count} of the requested materials?",
        )
    states = [storage(beggar_stage, 5, 6)]
    add(
        "l6-beggar-base",
        general,
        "hugo",
        ["yes"],
        states,
        outfit=("beggar", 0),
        topic=5,
    )
    callbacks[-1]["requires_premium"] = True
    callbacks[-1]["state_ops"] = [
        {
            "track": beggar_timer,
            "from": {"kind": "elapsed_at_least", "seconds": 86401},
            "effect": {"kind": "add", "value": 0},
        }
    ]
    beggar_offer(
        "hugo", ["outfit"], 0, 5, states, "Have you come to collect your new outfit?"
    )
    states = [storage(beggar_stage, 6, 7)]
    add("l6-beggar-first-start", general, "simon_the_beggar", ["yes"], states, topic=2)
    callbacks[-1]["requires_outfits"] = ["oteryn:outfit.beggar"]
    for word, topic, next_topic in [("outfit", 0, 1), ("yes", 1, 2)]:
        beggar_offer(
            "simon_the_beggar",
            [word],
            topic,
            next_topic,
            states,
            "Will you bring 100 ape fur and pay 20000 gold for our deal?",
        )
    states = [
        storage(beggar_stage, 7, 8),
        storage(beggar_door, -1, 0, 0, preserve=True),
    ]
    add(
        "l6-beggar-first-materials",
        general,
        "simon_the_beggar",
        ["yes"],
        states,
        take=[(5883, 100)],
        fee={"gold": 20000, "include_bank": True},
        topic=3,
    )
    callbacks[-1]["requires_premium"] = True
    callbacks[-1]["state_ops"] = [
        {
            "track": beggar_addon_timer,
            "from": {"kind": "any"},
            "effect": {"kind": "set_now"},
        }
    ]
    beggar_offer(
        "simon_the_beggar",
        ["100 ape fur"],
        0,
        3,
        states,
        "Did you bring the fur and the 20000 gold fee?",
    )
    add(
        "l6-beggar-dress",
        general,
        "simon_the_beggar",
        ["gypsy dress"],
        [storage(beggar_stage, 8, 10), storage(beggar_door, -1, 1, 0)],
        outfit=("beggar", 1),
    )
    callbacks[-1]["creation_sex"] = "female"
    add(
        "l6-beggar-beard-start",
        general,
        "simon_the_beggar",
        ["beard"],
        [storage(beggar_stage, 8, 9)],
    )
    callbacks[-1]["creation_sex"] = "male"
    callbacks[-1]["state_ops"] = [
        {
            "track": beggar_addon_timer,
            "from": {"kind": "any"},
            "effect": {"kind": "set_now"},
        }
    ]
    add(
        "l6-beggar-beard",
        general,
        "simon_the_beggar",
        ["beard", "addon"],
        [storage(beggar_stage, 9, 10), storage(beggar_door, -1, 1, 0)],
        outfit=("beggar", 1),
    )
    callbacks[-1]["creation_sex"] = "male"
    callbacks[-1]["state_ops"] = [
        {
            "track": beggar_addon_timer,
            "from": {"kind": "elapsed_at_least", "seconds": 432000},
            "effect": {"kind": "add", "value": 0},
        }
    ]
    states = [storage(beggar_stage, 10, 11), storage(beggar_door, 1, 1)]
    add("l6-beggar-staff-start", general, "simon_the_beggar", ["yes"], states, topic=6)
    for word, topic, next_topic in [("addon", 0, 4), ("staff", 4, 5), ("yes", 5, 6)]:
        beggar_offer(
            "simon_the_beggar",
            [word],
            topic,
            next_topic,
            states,
            "Will you retrieve my favourite staff?",
        )
    states = [
        storage(beggar_stage, 11, 11, preserve=True),
        storage(beggar + "beggar_second_addon", 0, 2, 1),
    ]
    add(
        "l6-beggar-staff",
        general,
        "simon_the_beggar",
        ["yes"],
        states,
        take=[(6107, 1)],
        outfit=("beggar", 2),
        topic=7,
        achievement="oteryn:achievement/life_on_the_streets",
    )
    callbacks[-1]["requires_premium"] = True
    beggar_offer(
        "simon_the_beggar", ["staff"], 0, 7, states, "Did you bring my favourite staff?"
    )
    for callback in callbacks:
        if callback["key"].startswith("oteryn:quest-reward/l6-beggar-"):
            npc = callback["npc"].split(".")[-1]
            callback["sources"] = [
                beggar_source(npc, "55-199" if npc == "hugo" else "84-222")
            ]
            if callback["achievement"]:
                callback["sources"].append(
                    "https://tibia.fandom.com/index.php?curid=62859&oldid=753124"
                )

    def source_offer(npc, words, topic, next_topic, states):
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": words,
                "topic": topic,
                "next_topic": next_topic,
                "storage": states,
                "reply": ["Will you help me with this task?"],
            }
        )

    knight = prefix + "u7_8/knight_outfits/"
    ape = prefix + "u7_6/the_ape_city/questline"
    knight_helmet = knight + "addon_helmet"
    knight_mission = knight + "mission_helmet"
    for key, maximum, npc, lines in [
        (knight_helmet, 7, "gregor", "63-211"),
        (knight + "ramsays_helmet_door", 1, "gregor", "149-160"),
        (prefix + "u7_8/druid_outfits/druid_amulet_door", 1, "ceiron", "174-180"),
        ("oteryn:quest-progress/atrad", 2, "atrad", "75-104"),
    ]:
        path = "data-global/npc/" + npc + ".lua"
        declarations.append(
            {
                "key": key,
                "quest": "oteryn:quest." + general,
                "initial": 0,
                "min": -1,
                "max": maximum,
                "writable": True,
                "sources": [f"crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}"],
            }
        )

    def stamp(track):
        return {"track": track, "from": {"kind": "any"}, "effect": {"kind": "set_now"}}

    def elapsed(track, seconds):
        return {
            "track": track,
            "from": {"kind": "elapsed_at_least", "seconds": seconds},
            "effect": {"kind": "add", "value": 0},
        }

    add(
        "l7-knight-sword-start",
        general,
        "morgan",
        ["knight's sword"],
        [storage(knight + "addon_sword", -1, 1, 0)],
    )
    callbacks[-1]["forbids_addons"] = [{"outfit": "oteryn:outfit.knight", "addons": 1}]
    next(c for c in callbacks if c["key"] == "oteryn:quest-reward/knight-sword")[
        "forbids_addons"
    ] = [{"outfit": "oteryn:outfit.knight", "addons": 1}]
    states = [storage(knight_helmet, -1, 1, 0), storage(knight_mission, 0, 1, 6)]
    add(
        "l7-knight-helmet-start",
        general,
        "gregor",
        ["yes"],
        states,
        topic=2,
        requires=[{"track": ape, "minimum": 16, "maximum": 18}],
    )
    callbacks[-1]["requires_premium"] = True
    callbacks[-1]["state_ops"] = [
        {
            "track": rook_ref,
            "from": {"kind": "between", "minimum": 0, "maximum": 2147483646},
            "effect": {"kind": "add", "value": 1},
        }
    ]
    source_offer("gregor", ["task"], 0, 1, states)
    source_offer("gregor", ["yes"], 1, 2, states)
    for name, word, item, count, before, topic in [
        ("fangs", "behemoth fang", 5893, 100, 1, 3),
        ("helmet", "ramsay the reckless helmet", 5924, 1, 2, 4),
        ("sweat", "sweat", 5885, 1, 3, 5),
        ("steel", "royal steel", 5887, 1, 4, 6),
    ]:
        states = [
            storage(knight_helmet, before, before + 1),
            storage(knight_mission, -1, before + 1, 6),
        ]
        if name == "fangs":
            states.append(storage(knight + "ramsays_helmet_door", -1, 1, 1))
        add(
            "l7-knight-" + name,
            general,
            "gregor",
            ["yes"],
            states,
            take=[(item, count)],
            topic=topic,
            requires=[{"track": ape, "minimum": 16, "maximum": 18}],
        )
        source_offer("gregor", [word], 0, topic, states)
    add(
        "l7-knight-helmet-work",
        general,
        "sam",
        ["adorn", "outfit", "addon"],
        [storage(knight_helmet, 5, 6), storage(knight_mission, -1, 6, 6)],
    )
    callbacks[-1]["state_ops"] = [stamp(knight + "addon_helmet_timer")]
    add(
        "l7-knight-helmet-reward",
        general,
        "sam",
        ["adorn", "outfit", "addon"],
        [
            storage(knight_helmet, 6, 7),
            storage(knight_mission, -1, 0, 6),
            storage(rook_ref, 0, 0, 2147483647),
        ],
        outfit=("knight", 2),
    )
    callbacks[-1]["state_ops"] = [elapsed(knight + "addon_helmet_timer", 7201)]
    druid = prefix + "u7_8/druid_outfits/"
    body = druid + "druid_body_addon"
    hat = druid + "druid_hat_addon"
    add("l7-druid-body-start", general, "ustan", ["addon"], [storage(body, -1, 1, 0)])
    states = [storage(body, 1, 2)]
    add(
        "l7-druid-body",
        general,
        "ustan",
        ["yes"],
        states,
        take=[(5896, 50), (5897, 50)],
        outfit=("druid", 1),
        topic=1,
    )
    source_offer("ustan", ["paws", "bear paws"], 0, 1, states)
    states = [storage(hat, -1, 1, 0), storage(druid + "default_start", 0, 1, 1)]
    add(
        "l7-druid-hat-start",
        general,
        "ceiron",
        ["yes"],
        states,
        give=(4867, 1),
        topic=4,
    )
    for words, topic, next_topic in [
        (["addon", "outfit"], 0, 2),
        (["faolan"], 2, 3),
        (["yes"], 3, 4),
    ]:
        source_offer("ceiron", words, topic, next_topic, states)
    states = [storage(hat, 1, 2)]
    add(
        "l7-druid-griffinclaw",
        general,
        "ceiron",
        ["yes"],
        states,
        take=[(5937, 1)],
        topic=5,
    )
    source_offer("ceiron", ["griffinclaw", "container"], 0, 5, states)
    for name, before, topic, give in [
        ("water-start", 2, 6, (5938, 1)),
        ("dust-start", 4, 8, None),
        ("chain-start", 6, 10, None),
    ]:
        states = [storage(hat, before, before + 1)]
        if name == "chain-start":
            states.append(storage(druid + "druid_amulet_door", -1, 1, 1))
        add(
            "l7-druid-" + name,
            general,
            "ceiron",
            ["yes"],
            states,
            give=give,
            topic=topic,
        )
        source_offer("ceiron", ["task"], 0, topic, states)
    for name, words, item, count, before, topic in [
        ("water", ["waterskin", "water skin"], 5939, 1, 3, 7),
        ("dust", ["dust", "demon dust"], 5906, 100, 5, 9),
        ("chain", ["chain", "wolf tooth chain"], 5940, 1, 7, 11),
    ]:
        states = [storage(hat, before, before + 1)]
        add(
            "l7-druid-" + name,
            general,
            "ceiron",
            ["yes"],
            states,
            take=[(item, count)],
            topic=topic,
        )
        source_offer("ceiron", words, 0, topic, states)
    add("l7-druid-language", general, "ceiron", ["faolan"], [storage(hat, 8, 9)])
    states = [storage(hat, 9, 10)]
    add(
        "l7-druid-hat",
        general,
        "a_majestic_warwolf",
        ["yes"],
        states,
        outfit=("druid", 2),
        topic=1,
    )
    source_offer("a_majestic_warwolf", ["addon", "outfit"], 0, 1, states)
    assassin = prefix + "u7_8/assassin_outfits/"
    base = assassin + "assassin_base_outfit"
    first = assassin + "assassin_first_addon"
    second = assassin + "assassin_second_addon"
    states = [
        storage(base, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add("l7-assassin-base-start", general, "vescu", ["yes"], states, topic=3)
    callbacks[-1]["requires_actor_conditions"] = ["drunk"]
    for words, topic, next_topic in [
        (["sober"], 0, 1),
        (["potion"], 1, 2),
        (["yes"], 2, 3),
    ]:
        source_offer("vescu", words, topic, next_topic, states)
    for name, word, item, count, before in [
        ("eyes", "30 bonelord eyes", 5898, 30, 1),
        ("scales", "10 red dragon scales", 5882, 10, 2),
        ("lizard", "30 lizard scales", 5881, 30, 3),
        ("fins", "20 fish fins", 5895, 20, 4),
        ("vampire", "20 vampire dust", 5905, 20, 5),
        ("demon", "10 demon dust", 5906, 10, 6),
        ("sweat", "warrior's sweat", 5885, 1, 7),
    ]:
        states = [storage(base, before, before + 1)]
        add(
            "l7-assassin-" + name,
            general,
            "vescu",
            ["yes"],
            states,
            take=[(item, count)],
            topic=4,
        )
        source_offer("vescu", [word], 0, 4, states)
    add(
        "l7-assassin-base",
        general,
        "vescu",
        ["secret"],
        [storage(base, 8, 9)],
        outfit=("assassin", 0),
    )
    states = [
        storage(first, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add(
        "l7-assassin-first-start",
        general,
        "erayo",
        ["yes"],
        states,
        topic=2,
        requires=[
            {
                "track": prefix + "u7_8/the_shattered_isles/shipwrecked",
                "minimum": 2,
                "maximum": 2,
            }
        ],
    )
    callbacks[-1]["requires_outfits"] = ["oteryn:outfit.assassin"]
    source_offer("erayo", ["addon"], 0, 1, states)
    source_offer("erayo", ["yes"], 1, 2, states)
    for index, (name, item) in enumerate(
        [
            ("blue", 5912),
            ("green", 5910),
            ("red", 5911),
            ("brown", 5913),
            ("yellow", 5914),
            ("white", 5909),
            ("yarn", 5886),
        ],
        1,
    ):
        count = 10 if name == "yarn" else 50
        word = "10 spools of yarn" if name == "yarn" else f"50 {name} cloth"
        states = [storage(first, index, index + 1)]
        add(
            "l7-assassin-" + name,
            general,
            "erayo",
            ["yes"],
            states,
            take=[(item, count)],
            outfit=("assassin", 1) if name == "yarn" else None,
            topic=3,
        )
        source_offer("erayo", [word], 0, 3, states)
    atrad = "oteryn:quest-progress/atrad"
    states = [storage(atrad, -1, 1, 0), storage(second, 0, 1)]
    add("l7-assassin-second-start", general, "atrad", ["yes"], states, topic=3)
    callbacks[-1]["requires_actor_conditions"] = ["fire"]
    source_offer("atrad", ["addon", "outfit"], 0, 2, states)
    source_offer("atrad", ["yes"], 2, 3, states)
    add(
        "l7-assassin-second",
        general,
        "atrad",
        ["nose ring", "ring"],
        [storage(atrad, 1, 2), storage(second, 1, 2)],
        take=[(5804, 1), (5930, 1)],
        outfit=("assassin", 2),
    )
    callbacks[-1]["requires_actor_conditions"] = ["fire"]
    for callback in callbacks:
        if callback["key"].startswith("oteryn:quest-reward/l7-"):
            npc = callback["npc"].split(".")[-1]
            path = "data-global/npc/" + npc + ".lua"
            lines = {
                "morgan": "115-148",
                "gregor": "63-211",
                "sam": "67-88",
                "ustan": "63-85",
                "ceiron": "60-187",
                "a_majestic_warwolf": "64-75",
                "vescu": "56-189",
                "erayo": "57-114",
                "atrad": "60-105",
            }[npc]
            callback["sources"] = [
                f"crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}"
            ]
    # L8: source looktypes are explicit sex-qualified grants, including cross-family rewards.
    for key, maximum, npc in [
        (prefix + "u7_8/hunter_outfits/hunter/addon_glove", 1, "legola"),
        (prefix + "u7_8/mage_and_summoner_outfits/addon_hat_cloak", 11, "myra"),
    ]:
        path = "data-global/npc/" + npc + ".lua"
        declarations.append(
            {
                "key": key,
                "quest": "oteryn:quest." + general,
                "initial": 0,
                "min": -1,
                "max": maximum,
                "writable": True,
                "sources": [f"crystal:{PIN}:{path}:sha256:{sha(inputs[path])}"],
            }
        )

    def targets(*values):
        callbacks[-1]["outfits"] = [
            {"outfit": "oteryn:outfit." + family, "addons": bit, "sex": sex}
            for family, bit, sex in values
        ]

    hunter = prefix + "u7_8/hunter_outfits/"
    hat = hunter + "hunter_hat_addon"
    states = [
        storage(hat, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add("l8-hunter-start", general, "elane", ["yes"], states, topic=3)
    for words, a, b in [(["addon", "outfit"], 0, 1), (["task"], 1, 2), (["yes"], 2, 3)]:
        source_offer("elane", words, a, b, states)
    for name, words, before, topic, items in [
        ("crossbow", ["crossbow"], 1, 4, [(5947, 1)]),
        ("leather", ["leather"], 2, 5, [(5876, 100), (5948, 100)]),
        ("wings", ["chicken wing"], 3, 6, [(5891, 5)]),
        ("hat", ["steel"], 4, 7, [(5887, 1), (5888, 1), (5889, 1)]),
    ]:
        states = [storage(hat, before, before + 1)]
        add(
            "l8-hunter-" + name,
            general,
            "elane",
            ["yes"],
            states,
            take=items,
            topic=topic,
        )
        source_offer("elane", words, 0, topic, states)
        if name == "hat":
            targets(("hunter", 1, "male"), ("hunter", 2, "female"))
    glove = hunter + "hunter/addon_glove"
    free_glove_offers = []
    glove_item = "oteryn:item.tibia.i5875"
    for npc, name, topic in [("legola", "gloves", 1), ("elane", "elane-gloves", 8)]:
        source = inputs[f"data-global/npc/{npc}.lua"].decode()
        (information,) = re.findall(
            r'addKeyword\(\{ "sniper gloves" \}, StdModule.say, '
            r'\{ npcHandler = npcHandler, text = "([^"]+)" \}, function\(player\)',
            source,
        )
        free, resale, addon = [
            re.findall(r'"([^"\n]+)"', block)
            for block in re.findall(
                r"addGloveKeyword\(\{(.*?)\}, function", source, re.DOTALL
            )
        ]
        source_offer(npc, ["sniper gloves"], 0, 0, [])
        offers[-1].update(
            reply=[information],
            inventory=[{"item": glove_item, "minimum": 0, "maximum": 0}],
        )
        unclaimed = [storage(glove, -1, 1, 0)]
        claimed = [storage(glove, 1, 1, preserve=True)]
        for next_topic, states, lines, premium in (
            (9, unclaimed, free, False),
            (topic, unclaimed, addon, True),
            (topic, claimed, resale, None),
        ):
            source_offer(npc, ["sniper gloves"], 0, next_topic, states)
            offers[-1].update(
                reply=lines[:1],
                inventory=[{"item": glove_item, "minimum": 1, "maximum": 2**32 - 1}],
            )
            if premium is not None:
                offers[-1]["premium"] = premium
            source_offer(npc, ["no"], next_topic, 0, states)
            offers[-1]["reply"] = lines[1:2]
            if next_topic == 9:
                free_glove_offers.extend(offers[-2:])
                del offers[-2:]

        add(
            "l8-hunter-" + name,
            general,
            npc,
            ["yes"],
            unclaimed,
            take=[(5875, 1)],
            topic=topic,
        )
        callbacks[-1]["requires_premium"] = True
        targets(("hunter", 2, "male"), ("hunter", 1, "female"))
        add(
            "l8-hunter-" + name + "-sale",
            general,
            npc,
            ["yes"],
            claimed,
            take=[(5875, 1)],
            give=(3035, 20),
            topic=topic,
        )
        callbacks[-1]["repeatable"] = True
    mage = prefix + "u7_8/mage_and_summoner_outfits/"
    wand = mage + "addon_wand"
    states = [
        storage(wand, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add("l8-mage-male-start", general, "angelina", ["yes"], states)
    states = [storage(wand, 1, 2)]
    add("l8-mage-male-wand-start", general, "lynda", ["yes"], states, topic=2)
    source_offer("lynda", ["angelina"], 0, 1, states)
    source_offer("lynda", ["yes"], 1, 2, states)
    for name, words, before, topic, items in [
        (
            "wands",
            ["wand", "rod"],
            2,
            3,
            [
                (i, 1)
                for i in (3065, 3066, 3067, 3069, 3070, 3071, 3072, 3073, 3074, 3075)
            ],
        ),
        ("sulphur", ["sulphur"], 3, 4, [(5904, 10)]),
        ("soul", ["soul stone"], 4, 5, [(5809, 1)]),
        ("ankhs", ["ankh"], 5, 6, [(3077, 20)]),
    ]:
        states = [storage(wand, before, before + 1)]
        add(
            "l8-mage-male-" + name,
            general,
            "lynda",
            ["yes"],
            states,
            take=items,
            topic=topic,
        )
        source_offer("lynda", words, 0, topic, states)
        if name == "ankhs":
            callbacks[-1]["state_ops"] = [stamp(mage + "addon_wand_timer")]
    add("l8-mage-male-ritual", general, "lynda", ["ritual"], [storage(wand, 6, 7)])
    callbacks[-1]["state_ops"] = [elapsed(mage + "addon_wand_timer", 10801)]
    targets(("mage", 1, "male"), ("summoner", 1, "female"))
    add("l8-mage-male-hat", general, "zoltan", ["yes"], [], take=[(5903, 1)], topic=1)
    callbacks[-1].update(
        creation_sex="male",
        exact_take=True,
        forbids_addons=[{"outfit": "oteryn:outfit.mage", "addons": 2, "sex": "male"}],
    )
    targets(("mage", 2, "male"), ("summoner", 2, "female"))
    source_offer("zoltan", ["proof"], 0, 1, [])
    belt = mage + "addon_belt"
    states = [
        storage(belt, -1, 1, 0),
        storage("oteryn:quest-progress/outfit_quest/default_start", 0, 1, 1),
    ]
    add("l8-mage-female-belt-start", general, "sandra", ["yes"], states, topic=2)
    source_offer("sandra", ["vial", "ticket", "bonus", "deposit"], 0, 1, states)
    source_offer("sandra", ["yes"], 1, 2, states)
    states = [storage(belt, 1, 2)]
    add(
        "l8-mage-female-belt",
        general,
        "sandra",
        ["yes"],
        states,
        take=[(5958, 1)],
        topic=4,
    )
    targets(("mage", 1, "female"), ("summoner", 1, "male"))
    source_offer("sandra", ["prize"], 0, 4, states)
    hat = mage + "addon_hat_cloak"
    mission = mage + "mission_hat_cloak"
    states = [storage(hat, -1, 1, 0), storage(mission, 0, 1, 11)]
    add("l8-mage-female-hat-start", general, "myra", ["yes"], states, topic=2)
    for words, a, b in [(["outfit"], 0, 100), (["tiara"], 100, 1), (["yes"], 1, 2)]:
        source_offer("myra", words, a, b, states)
    for before, (name, item, count, word) in enumerate(
        [
            ("wings", 5894, 70, "bat wings"),
            ("cloth", 5911, 20, "red cloth"),
            ("fur", 5883, 40, "ape fur"),
            ("orchids", 5922, 35, "holy orchids"),
            ("yarn", 5886, 10, "yarn"),
            ("lizard", 5881, 60, "lizard scales"),
            ("scales", 5882, 40, "red dragon scales"),
            ("sulphur", 5904, 15, "sulphur"),
            ("vampire", 5905, 30, "vampire dust"),
        ],
        1,
    ):
        states = [storage(hat, before, before + 1), storage(mission, 0, before + 1, 11)]
        add(
            "l8-mage-female-" + name,
            general,
            "myra",
            ["yes"],
            states,
            take=[(item, count)],
            topic=before + 2,
        )
        source_offer("myra", [word], 0, before + 2, states)
    add(
        "l8-mage-female-hat",
        general,
        "zoltan",
        ["myra"],
        [storage(hat, 10, 11), storage(mission, 0, 0, 11)],
    )
    callbacks[-1]["creation_sex"] = "female"
    targets(("mage", 2, "female"), ("summoner", 2, "female"))
    for name, bit, item, track, words in [
        ("mask", 1, 20276, "dream_outfit_1", ["dream warden mask"]),
        ("claw", 2, 20275, "dream_outfit", ["dream warden claw"]),
    ]:
        key = "oteryn:quest-progress/" + track
        path = "data-global/npc/eruaran.lua"
        declarations.append(
            {
                "key": key,
                "quest": "oteryn:quest." + general,
                "initial": 0,
                "min": -1,
                "max": 1,
                "writable": True,
                "sources": [f"crystal:{PIN}:{path}:153-199:sha256:{sha(inputs[path])}"],
            }
        )
        states = [storage(key, -1, 1, 0)]
        add(
            "l8-dream-warden-" + name,
            general,
            "eruaran",
            ["yes"],
            states,
            take=[(item, 1)],
            outfit=("dream_warden", bit),
            topic=bit,
        )
        callbacks[-1]["requires_premium"] = True
        callbacks[-1]["achievement_progress"] = [
            {"key": "oteryn:achievement/dream_warden", "target": 2}
        ]
        source_offer("eruaran", words, 0, bit, states)
    # Gnomally's private config selector is lowered to distinct conversation
    # topics. Source trades are repeatable, with no quest storage side effect.
    for family, base, first, start, base_word, first_word in [
        (
            "soil-guardian",
            16252,
            16253,
            101,
            "basic soil guardian outfit",
            "addition to the soil guardian outfit",
        ),
        (
            "crystal-warlord",
            16255,
            16256,
            103,
            "basic crystal warlord outfit",
            "addition to the crystal warlord armor outfit",
        ),
    ]:
        quest = family.replace("-", "_") + "_outfits_quest"
        for suffix, token, count, gift, topic, word in [
            ("base", 16129, 20, base, start, base_word),
            ("first", 16128, 70, first, start + 1, first_word),
        ]:
            add(
                "l9-" + family + "-" + suffix,
                quest,
                "gnomally",
                ["yes"],
                [],
                take=[(token, count)],
                give=(gift, 1),
                topic=topic,
            )
            source_offer("gnomally", [word], 0, topic, [])
    add(
        "l9-gnomish-supply",
        "bigfoot_s_burden_quest",
        "gnomally",
        ["yes"],
        [],
        take=[(16128, 2)],
        give=(15698, 1),
        topic=105,
    )
    source_offer("gnomally", ["supply"], 0, 105, [])
    add(
        "l9-afflicted-pouch",
        "afflicted_outfits_quest",
        "ottokar",
        ["yes"],
        [],
        take=[(12517, 1)],
        give=(12413, 1),
        topic=1,
    )
    callbacks[-1]["achievement_progress"] = [
        {"key": "oteryn:achievement/doctor_doctor", "target": 100}
    ]
    source_offer(
        "ottokar", ["medicine", "medicine pouch", "belongings of deceasead"], 0, 1, []
    )
    for callback in callbacks:
        if callback["key"].startswith("oteryn:quest-reward/l9-"):
            npc = callback["npc"].split(".")[-1]
            path = "data-global/npc/" + npc + ".lua"
            lines = "75-169" if npc == "gnomally" else "61-78"
            callback["sources"] = [
                f"crystal:{PIN}:{path}:{lines}:sha256:{sha(inputs[path])}"
            ]
    from questpc7_selected_rewards import (
        GOLDEN_OWNED_PROMPT,
        golden_owned_confirmation,
        golden_recipe,
    )

    # The L10 source dialogue charges Crystal's fees in recipe milestone order.
    path, index, raw, _, _ = golden_recipe(ROOT)
    prices = golden_source_prices(inputs[GOLDEN_NPC_SOURCE])
    golden = "oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit"
    base = "oteryn:outfit.golden"
    golden_sources = [
        f"crystal:{PIN}:{GOLDEN_NPC_SOURCE}:131-200:sha256:{sha(inputs[GOLDEN_NPC_SOURCE])}",
        f"canonical:{path}:1#/records/{index}/definition:sha256:{sha(raw)}",
    ]
    for bit, (suffix, topic, word) in enumerate(
        [
            ("base", 3, "armor"),
            ("boots", 5, "boots"),
            ("helmet", 4, "helmet"),
        ]
    ):
        after = 4 if bit == 2 else bit + 1
        gold = prices[bit]
        add(
            "l10-golden-" + suffix,
            "golden_outfits_quest",
            "king_tibianus",
            ["yes"],
            [storage(golden, bit, after)],
            outfit=("golden", bit),
            topic=topic,
            next_topic=2,
            fee={"gold": gold, "include_bank": True},
        )
        callbacks[-1]["forbids_outfits" if bit == 0 else "forbids_addons"] = (
            [base] if bit == 0 else [{"outfit": base, "addons": bit}]
        )
        if bit == 2:
            callbacks[-1]["completes"] = True
        callbacks[-1]["sources"] = golden_sources
        callbacks.append(golden_owned_confirmation(callbacks[-1]))
        offers.append(
            {
                "npc": "oteryn:npc.king_tibianus",
                "keywords": [word],
                "topic": 2,
                "next_topic": topic,
                "storage": [storage(golden, bit, bit, preserve=True)],
                "reply": [
                    f"The chosen commission costs {gold} gold. Do you agree?"
                    + GOLDEN_OWNED_PROMPT
                ],
            }
        )
    add(
        "l10-golden-complete-owned",
        "golden_outfits_quest",
        "king_tibianus",
        ["yes"],
        [storage(golden, 3, 4)],
        topic=1,
        next_topic=2,
    )
    callbacks[-1].update(
        completes=True,
        requires_addons=[{"outfit": base, "addons": 3}],
        sources=golden_sources,
    )
    for words, before, after in [(["outfit", "addon"], 0, 1), (["yes"], 1, 2)]:
        offers.append(
            {
                "npc": "oteryn:npc.king_tibianus",
                "keywords": words,
                "topic": before,
                "next_topic": after,
                "storage": [],
                "reply": ["Choose armor, boots or helmet for your golden commission."],
            }
        )
    # The pinned Crystal Phillip contains no book quest dialogue.
    between = "between_the_lines_quest"
    book_stage = prefix + "between_the_lines/phase"
    wiki = "tibia_fandom:108521:1206800:Pen and Sword/Inkwing"
    add(
        "between-the-lines-phillip-accept",
        between,
        "phillip",
        ["yes"],
        [storage(book_stage, -1, 1, 0)],
        topic=1,
        source=wiki,
    )
    callbacks[-1].update(
        minimum_level=200,
        requires_premium=True,
        reply=["Follow the ink traces outside Carlin to find the creature."],
    )
    source_offer(
        "phillip", ["book"], 0, 1, [storage(book_stage, -1, 0, 0, preserve=True)]
    )
    offers[-1]["reply"] = ["A creature escaped from my book. Will you track it down?"]

    # Distinct order ranks choose the shared three documents' actual USE branch.
    parent_quest = "oteryn:quest.dreamer_s_challenge_quest"
    parent = "oteryn:quest-progress/chosen-source/dreamer_s_challenge_quest/s4"
    declarations.append(
        {
            "key": parent,
            "quest": parent_quest,
            "initial": 0,
            "min": 0,
            "max": 1,
            "writable": False,
            "sources": ["owner:chosen-Dreamers-Challenge-terminal-puzzle-s4:read-only"],
        }
    )
    bindings = json.loads(
        (
            ROOT
            / "tools/content-schema/world-authoring/samples/quest-door-bindings.json"
        ).read_bytes()
    )
    (tower_track,) = {
        binding["track"]
        for binding in bindings["bindings"]
        if binding["action"] == 41014
    }
    # Both orders write the same door permit under their shared prerequisite quest.
    tower_permit = {
        "key": tower_track,
        "quest": parent_quest,
        "initial": -1,
        "min": -1,
        "max": 1,
        "writable": True,
        "sources": [],
    }
    declarations.append(tower_permit)
    for family, npc, opposite, join_achievement, final_achievement in [
        (
            "brotherhood",
            "the_bone_master",
            "nightmare",
            "bone_brother",
            "skull_and_bones",
        ),
        (
            "nightmare",
            "the_dream_master",
            "brotherhood",
            "nightmare_knight",
            "nightmare_walker",
        ),
    ]:
        quest = family + "_outfits_quest"
        order = prefix + "u7_9/" + family + "_outfits/"
        rank = order + "outfits"
        other = prefix + "u7_9/" + opposite + "_outfits/outfits"
        requirements = [requirement(parent, 1), requirement(other, 0)]
        path = f"data-global/npc/{npc}.lua"
        donor = inputs[path].decode()
        for stage, before, after, item, count, topic, achievement in [
            ("join", 0, 1, 768, 0, 2, join_achievement),
            ("officer", 1, 2, 769, 500, 3, None),
            ("leader", 2, 3, 770, 1000, 4, final_achievement),
        ]:
            guard = [storage(rank, before, after)]
            add(
                "l10-" + family + "-" + stage,
                quest,
                npc,
                ["yes"],
                guard,
                take=[] if count == 0 else [(6499, count)],
                give=(item, 1),
                topic=topic,
                requires=requirements,
                achievement="oteryn:achievement/" + achievement
                if achievement
                else None,
            )
            callbacks[-1]["requires_premium"] = True
            if stage in ("join", "leader"):
                branch = re.search(
                    rf"getTopic\(playerId\) == {topic} then(.*?)elseif npcHandler:getTopic",
                    donor,
                    re.DOTALL,
                )
                if branch is None:
                    raise ValueError(f"missing {npc} {stage} confirmation")
                if stage == "join":
                    permit = re.search(
                        r"NightmareOutfits\.KnightwatchTowerDoor, (1)\)", branch[1]
                    )
                    if permit is None:
                        raise ValueError(f"missing {npc} Knightwatch permit")
                    callbacks[-1]["participants"] = [
                        {
                            "quest": parent_quest,
                            "storage": [
                                storage(tower_track, -1, int(permit[1]), maximum=1)
                            ],
                        }
                    ]
                    tower_permit["sources"].extend(callbacks[-1]["sources"])
                else:
                    document, *additional = re.findall(
                        r"player:addItem\((\d+), (\d+)\)", branch[1]
                    )
                    if tuple(map(int, document)) != (item, 1) or len(additional) != 1:
                        raise ValueError(f"unexpected {npc} leader gifts")
                    callbacks[-1]["give"]["additional"] = [
                        {"item": f"oteryn:item.tibia.i{gift}", "count": int(quantity)}
                        for gift, quantity in additional
                    ]
            source_offer(
                npc,
                ["join"] if stage == "join" else ["advancement", "demonic", "essence"],
                0,
                1 if stage == "join" else topic,
                [storage(rank, before, before, preserve=True)],
            )
            offers[-1]["requires"] = requirements
        if family == "brotherhood":
            source = (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:"
                "data-global/npc/the_bone_master.lua:78-80,123-128:sha256:"
                "65a6ff562c38b78383ad4e9a4ba13ede2e875cd07686955f7c7741308d8fe3b1"
            )
            door = prefix + "u7_9/brotherhood_outfits/door"
            declarations.append(
                {
                    "key": door,
                    "quest": "oteryn:quest." + quest,
                    "initial": 0,
                    "min": 0,
                    "max": 1,
                    "writable": True,
                    "sources": [source],
                }
            )
            add(
                "l10-brotherhood-dread-lord",
                quest,
                npc,
                ["yes"],
                [storage(rank, 3, 4), storage(door, 0, 1)],
                take=[(6499, 1500)],
                topic=5,
                requires=requirements,
                achievement="oteryn:achievement/dread_lord",
            )
            callbacks[-1].update(
                requires_premium=True,
                sources=[source],
                reply=["You advanced to Dread Lord rank."],
            )
            source_offer(
                npc,
                ["advancement", "demonic", "essence"],
                0,
                5,
                [storage(rank, 3, 3, preserve=True)],
            )
            offers[-1].update(
                requires=requirements,
                reply=["Did you bring 1500 demonic essences to advance to Dread Lord?"],
            )
        else:
            offer = re.search(
                r'NightmareOutfits.Outfits\) == 3 then\s+npcHandler:say\("([^"]+)"[^\n]+\s+'
                r"npcHandler:setTopic\(playerId, (\d+)\)",
                donor,
            )
            if offer is None:
                raise ValueError("missing Dream Master rank-four offer")
            topic = int(offer[2])
            exchange = re.search(
                rf"getTopic\(playerId\) == {topic} then\s+if player:removeItem\((\d+), (\d+)\) then"
                r".*?NightmareOutfits.Outfits, 4\).*?NightmareOutfits.Door, 1\)"
                r'\s+player:addAchievement\("([^"]+)"\)\s+npcHandler:say\("([^"]+)"',
                donor,
                re.DOTALL,
            )
            if exchange is None:
                raise ValueError("missing Dream Master rank-four exchange")
            source = f"crystal:{PIN}:{path}:sha256:{sha(inputs[path])}"
            door = prefix + "u7_9/nightmare_outfits/door"
            achievement = exchange[3].lower().replace(" ", "_")
            declarations.append(
                {
                    "key": door,
                    "quest": "oteryn:quest." + quest,
                    "initial": 0,
                    "min": 0,
                    "max": 1,
                    "writable": True,
                    "sources": [source],
                }
            )
            add(
                "l10-nightmare-" + achievement.replace("_", "-"),
                quest,
                npc,
                ["yes"],
                [storage(rank, 3, 4), storage(door, 0, 1)],
                take=[(int(exchange[1]), int(exchange[2]))],
                topic=topic,
                requires=requirements,
                achievement="oteryn:achievement/" + achievement,
                source=source,
            )
            callbacks[-1].update(requires_premium=True, completes=True, reply=[exchange[4]])
            source_offer(
                npc,
                ["advancement", "demonic", "essence"],
                0,
                topic,
                [storage(rank, 3, 3, preserve=True)],
            )
            offers[-1].update(requires=requirements, reply=[offer[1]])
        source_offer(npc, ["yes"], 1, 2, [storage(rank, 0, 0, preserve=True)])
        offers[-1]["requires"] = requirements
        for stage, item, threshold, bit in [
            ("follower", 768, 1, 0),
            ("officer", 769, 2, 1),
            ("leader", 770, 3, 2),
        ]:
            claim = prefix + "u7_9/" + family + "_outfits/document_claim_" + str(bit)
            declarations.append(
                {
                    "key": claim,
                    "quest": "oteryn:quest." + quest,
                    "initial": 0,
                    "min": 0,
                    "max": 1,
                    "writable": True,
                    "sources": [
                        f"crystal:{PIN}:data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua:27-59:durable-one-shot-owned-guard"
                    ],
                }
            )
            add(
                "l10-" + family + "-use-" + stage,
                quest,
                npc,
                ["document " + stage],
                [
                    storage(rank, threshold, threshold, 4, preserve=True),
                    storage(claim, 0, 1),
                ],
                take=[(item, 1)],
                outfit=(family, bit),
                requires=requirements,
            )
            callback = callbacks[-1]
            callback["carried_item"] = f"oteryn:item.tibia.i{item}"
            callback["requires_premium"] = True
            if bit == 0:
                callback["forbids_outfits"] = ["oteryn:outfit." + family]
            else:
                callback["requires_outfits"] = ["oteryn:outfit." + family]
                callback["forbids_addons"] = [
                    {"outfit": "oteryn:outfit." + family, "addons": bit}
                ]
            document_path = "data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua"
            callback["sources"].append(
                f"crystal:{PIN}:{document_path}:1-69:sha256:{sha(inputs[document_path])}"
            )
    source_offer(
        "chavis",
        ["food"],
        0,
        1,
        [
            storage(
                "oteryn:quest-progress/quest/u10_50/oramond_quest/to_take_roots/count",
                5,
                5,
                preserve=True,
            )
        ],
    )
    offers[-1]["reply"] = [
        "Do you have five roots ready to hand in for a Rathleton vote?"
    ]
    # L11: source task rewards plus explicitly selected Oteryn qualification desks.
    dream_quest = "dream_warrior_outfits_quest"
    dream_outfit = "oteryn:outfit.dream_warrior"
    for npc, court, suffix in (("vanys", "summer", ""), ("undal", "winter", "-undal")):
        label = "l11-dream-warrior" + suffix
        ready = [
            requirement(prefix + "u12_00/the_dream_courts/ward_stones/questline", 3),
            requirement(prefix + f"u12_00/the_dream_courts/main/the_{court}_court", 1),
        ]
        if court == "summer":
            # Keep the original Vanys reward keyword as an alternate entry.
            add(
                label + "-base",
                dream_quest,
                npc,
                ["dream warrior outfit"],
                [],
                outfit=("dream_warrior", 0),
                requires=ready,
            )
            callbacks[-1].update(
                minimum_level=250, requires_premium=True, forbids_outfits=[dream_outfit]
            )
        source_offer(npc, ["talk"], 0, 2, [])
        offers[-1]["reply"] = ["Ask me about your task for the Court."]
        base = "-task-base" if court == "summer" else "-base"
        add(
            label + base,
            dream_quest,
            npc,
            ["task"],
            [],
            outfit=("dream_warrior", 0),
            topic=2,
            requires=ready,
        )
        callbacks[-1].update(
            minimum_level=250,
            requires_premium=True,
            forbids_outfits=[dream_outfit],
            reply=[
                "You defeated the Nightmare Beast. Accept this Dream Warrior outfit from the Court."
            ],
        )
        # Vanys and Undal allow either exchange as soon as the outfit is earned.
        for bit, part, word, item, count in (
            (1, "first", "pomegranates", 30169, 5),
            (2, "second", "ice shield", 30168, 1),
        ):
            add(
                label + "-" + part,
                dream_quest,
                npc,
                [word],
                [],
                take=[(item, count)],
                outfit=("dream_warrior", bit),
                topic=8,
            )
            callbacks[-1].update(
                minimum_level=250,
                requires_premium=True,
                requires_outfits=[dream_outfit],
                forbids_addons=[{"outfit": dream_outfit, "addons": bit}],
                achievement_progress=[
                    {"key": "oteryn:achievement/dream_warrior", "target": 2}
                ],
            )
        for word, before, after, reply in (
            (
                "addon",
                0,
                6,
                "Would you like to earn an addon for your Dream Warrior outfit?",
            ),
            (
                "yes",
                6,
                7,
                "Bring five pomegranates for the first addon or an ice shield for the second.",
            ),
            ("yes", 7, 8, "Which are you turning in, pomegranates or an ice shield?"),
        ):
            source_offer(npc, [word], before, after, [])
            offers[-1]["reply"] = [reply]
    wanted = "oteryn:quest-progress/chosen-source/kilmaresh_quest/s21"
    declarations.append(
        {
            "key": wanted,
            "quest": "oteryn:quest.kilmaresh_quest",
            "initial": 0,
            "min": 0,
            "max": 1,
            "sources": [
                "owner:chosen-Kilmaresh-s21:report-three-Wanted-outlaws:read-only"
            ],
        }
    )
    desks = [
        (
            "issavi",
            "authored.citizen_of_issavi_outfits_quest",
            "eshaya",
            "citizen_of_issavi",
            250,
            [requirement(wanted, 1)],
            "issavi attire",
        ),
    ]
    for label, quest, npc, family, level, ready, word in desks:
        add(
            "l11-" + label + "-base",
            quest,
            npc,
            [word],
            [],
            outfit=(family, 0),
            requires=ready,
        )
        callbacks[-1].update(
            minimum_level=level,
            requires_premium=True,
            forbids_outfits=["oteryn:outfit." + family],
        )
        if label == "issavi":
            callbacks[-1]["sources"].append(
                "owner:chosen-Issavi-attire-keeper-role:Eshaya:Wanted-report-s21"
            )
    for label, quest, npc, family, level, ready, tokens in [
        (
            "issavi",
            "authored.citizen_of_issavi_outfits_quest",
            "eshaya",
            "citizen_of_issavi",
            250,
            [requirement(wanted, 1)],
            [(37003, 1), (37002, 1)],
        ),
        (
            "makeshift",
            "makeshift_warrior_outfits_quest",
            "gnomadness",
            "makeshift_warrior",
            100,
            [],
            [(27657, 1), (27656, 1)],
        ),
    ]:
        for bit, (item, count) in enumerate(tokens, 1):
            add(
                "l11-" + label + "-" + ("first" if bit == 1 else "second"),
                quest,
                npc,
                ["document"],
                [],
                take=[(item, count)],
                outfit=(family, bit),
                requires=ready,
            )
            callback = callbacks[-1]
            callback.update(
                minimum_level=level,
                requires_premium=True,
                requires_outfits=["oteryn:outfit." + family],
                forbids_addons=[{"outfit": "oteryn:outfit." + family, "addons": bit}],
            )
            if bit == 2:
                callback["requires_addons"] = [
                    {"outfit": "oteryn:outfit." + family, "addons": 1}
                ]
                wiki = {
                    "dream-warrior": "https://tibia.fandom.com/?curid=86660&oldid=848561",
                    "issavi": "https://tibia.fandom.com/?curid=94721&oldid=869754",
                }.get(label)
                if wiki:
                    callback["achievement"] = "oteryn:achievement/" + family
                    callback["sources"].append(wiki)
            callback["carried_item"] = f"oteryn:item.tibia.i{item}"
            if label == "issavi":
                callback["sources"].append(
                    "owner:chosen-Issavi-veil-addon1-scabbard-addon2:one-token-each"
                )
            if label == "issavi" and bit == 2:
                callback.update(
                    achievement="oteryn:achievement/citizen_of_issavi", completes=True
                )
                callback["sources"].append(
                    "https://tibia.fandom.com/index.php?curid=94999&oldid=875550"
                )
    qualification = prefix + "makeshift_warrior/qualification"
    declarations.append(
        {
            "key": qualification,
            "quest": "oteryn:quest.makeshift_warrior_outfits_quest",
            "initial": 0,
            "min": 0,
            "max": 2,
            "writable": True,
            "sources": [
                "owner:chosen-Gnome-Deep-Hub-salvage-apprenticeship:s1-s2:gnomadness-role"
            ],
        }
    )
    add(
        "l11-makeshift-start",
        "makeshift_warrior_outfits_quest",
        "gnomadness",
        ["salvage"],
        [storage(qualification, 0, 1)],
        next_topic=11,
    )
    callbacks[-1].update(minimum_level=100, requires_premium=True)
    callbacks[-1]["sources"].append(
        "owner:chosen-Gnome-Deep-Hub-apprenticeship:meeting-admitted-Gnomadness-witnesses-exploration"
    )
    add(
        "l11-makeshift-base",
        "makeshift_warrior_outfits_quest",
        "gnomadness",
        ["yes"],
        [storage(qualification, 1, 2)],
        outfit=("makeshift_warrior", 0),
        topic=11,
    )
    callbacks[-1].update(
        minimum_level=100,
        requires_premium=True,
        forbids_outfits=["oteryn:outfit.makeshift_warrior"],
    )
    callbacks[-1]["sources"].append(
        "owner:chosen-Gnome-Deep-Hub-salvage-apprenticeship:base-qualification"
    )
    source_offer(
        "gnomadness", ["salvage"], 0, 11, [storage(qualification, 1, 1, preserve=True)]
    )
    # L12: owner-selected fixed hunting trials; native deaths are the only counter writer.
    for label, family, quest, npc, level, counts, materials in [
        (
            "dragon-slayer",
            "dragon_slayer",
            "authored.dragon_slayer_outfits_quest",
            "a_sweaty_cyclops",
            0,
            [("dragons", 20), ("dragon_lords", 20)],
            [(5920, 10), (5882, 10)],
        ),
        (
            "fire-fighter",
            "fire_fighter",
            "fire_fighter_outfits_quest",
            "gnomadness",
            250,
            [("kills", 10)],
            [],
        ),
    ]:
        root = "oteryn:quest-progress/chosen-trial/" + family + "/"
        provenance = (
            "owner:chosen-trial:"
            + family
            + ":fixed-native-creature-deaths:questpc3-engine-5e72a3f0b"
        )
        for track, maximum in [("phase", 2)] + counts:
            declarations.append(
                {
                    "key": root + track,
                    "quest": "oteryn:quest." + quest,
                    "initial": 0,
                    "min": 0,
                    "max": maximum,
                    "writable": True,
                    "sources": [provenance],
                }
            )
        add(
            "l12-" + label + "-start",
            quest,
            npc,
            [family.replace("_", " ") + " trial"],
            [storage(root + "phase", 0, 1)]
            + [storage(root + track, 0, 0, preserve=True) for track, _ in counts],
        )
        callbacks[-1].update(requires_premium=True)
        if level:
            callbacks[-1]["minimum_level"] = level
        callbacks[-1]["sources"].append(provenance)
        add(
            "l12-" + label + "-base",
            quest,
            npc,
            [family.replace("_", " ") + " report"],
            [storage(root + "phase", 1, 2)]
            + [
                storage(root + track, maximum, maximum, preserve=True)
                for track, maximum in counts
            ],
            take=materials,
            outfit=(family, 0),
        )
        callbacks[-1].update(
            requires_premium=True, forbids_outfits=["oteryn:outfit." + family]
        )
        if level:
            callbacks[-1]["minimum_level"] = level
        callbacks[-1]["sources"].append(provenance)
    # The Spike task reports share the same character-owned fame balance.
    spike = "oteryn:quest-progress/spike/"
    cave = "cave_explorer_outfits_quest"
    fame = spike + "fame"
    fame_max = 2_147_483_647
    gnommander = "oteryn:npc.gnommander"
    outfit = "oteryn:outfit.cave_explorer"
    pin = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/"

    # Keep settlement for published trials already assigned before the Spike offers.
    legacy_trial = "oteryn:quest-progress/chosen-trial/cave_explorer/"
    legacy_phase = legacy_trial + "phase"
    legacy_kills = legacy_trial + "kills"
    legacy_source = "owner:chosen-trial:cave_explorer:fixed-native-creature-deaths:questpc3-engine-5e72a3f0b"
    for key, maximum in [(legacy_phase, 2), (legacy_kills, 20)]:
        declarations.append(
            {
                "key": key,
                "quest": "oteryn:quest." + cave,
                "initial": 0,
                "min": 0,
                "max": maximum,
                "writable": True,
                "sources": [legacy_source],
            }
        )
    add(
        "l12-cave-explorer-legacy-report",
        cave,
        "gnommander",
        ["cave explorer report"],
        [storage(legacy_phase, 1, 2), storage(legacy_kills, 20, 20, preserve=True)],
        outfit=("cave_explorer", 0),
        fee={"gold": 1000, "include_bank": False},
    )
    callbacks[-1].update(
        minimum_level=80,
        requires_premium=True,
        forbids_outfits=[outfit],
        sources=[legacy_source],
    )
    for bit, gold in [(1, 2000), (2, 3000)]:
        add(
            f"l12-cave-explorer-legacy-addon-{bit}",
            cave,
            "gnommander",
            [f"cave explorer addon {bit}"],
            [storage(legacy_phase, 2, 2, preserve=True)],
            outfit=("cave_explorer", bit),
            fee={"gold": gold, "include_bank": False},
        )
        row = callbacks[-1]
        row.update(
            minimum_level=80,
            requires_premium=True,
            requires_outfits=[outfit],
            forbids_addons=[{"outfit": outfit, "addons": bit}],
            achievement_progress=[
                {"key": "oteryn:achievement/cave_completionist", "target": 2}
            ],
            sources=[legacy_source, "tibiawiki:Cave Completionist:846114"],
        )
        if bit == 2:
            row["requires_addons"] = [{"outfit": outfit, "addons": 1}]

    def spike_track(key, maximum, source):
        declarations.append(
            {
                "key": key,
                "quest": "oteryn:quest.spike_task",
                "initial": 0,
                "min": 0,
                "max": maximum,
                "writable": True,
                "sources": [source],
            }
        )

    def fame_change(minimum, maximum, delta):
        return {
            "track": fame,
            "from": {"kind": "between", "minimum": minimum, "maximum": maximum},
            "effect": {"kind": "add", "value": delta},
        }

    source = pin + "gnommander.lua:75-169"
    spike_track(fame, fame_max, source)
    offers.extend(
        [
            {
                "npc": gnommander,
                "keywords": ["reward"],
                "topic": 0,
                "any_topic": True,
                "next_topic": 0,
                "storage": [],
                "reply": [
                    "Your Spike fame makes you eligible for outfit rewards. Ask if you are worthy."
                ],
            },
            {
                "npc": gnommander,
                "keywords": ["worthy"],
                "topic": 0,
                "any_topic": True,
                "next_topic": 20,
                "storage": [],
                "requires": [requirement(fame, 100, fame_max)],
                "reply": [
                    "The basic outfit costs 1,000 gold. The first addon costs 2,000 and the second costs 3,000."
                ],
                "failure_reply": ["You need at least 100 Spike fame."],
            },
        ]
    )
    for label, level, addon, gold in [
        ("basic", 25, 0, 1000),
        ("first", 50, 1, 2000),
        ("second", 80, 2, 3000),
    ]:
        topic = 21 + addon
        offers.append(
            {
                "npc": gnommander,
                "keywords": [label],
                "topic": 20,
                "next_topic": topic,
                "storage": [],
                "minimum_level": level,
                "requires": [requirement(fame, 100, fame_max)],
                "reply": [
                    f"Buy the {label} outfit reward for {gold:,} gold and 100 Spike fame?"
                ],
                "failure_reply": ["You need more experience or Spike fame."],
            }
        )
        name = "base" if addon == 0 else label
        add(
            "l12-cave-explorer-" + name,
            cave,
            "gnommander",
            ["yes"],
            [],
            topic=topic,
            outfit=("cave_explorer", addon),
            fee={"gold": gold, "include_bank": False},
        )
        row = callbacks[-1]
        row.update(
            minimum_level=level,
            requires_premium=True,
            participants=[
                {
                    "quest": "oteryn:quest.spike_task",
                    "storage": [],
                    "state_ops": [fame_change(100, fame_max, -100)],
                }
            ],
            sources=[source],
            reply=["Here is your reward."],
        )
        if addon:
            row.update(
                requires_outfits=[outfit],
                forbids_addons=[{"outfit": outfit, "addons": addon}],
                achievement_progress=[
                    {"key": "oteryn:achievement/cave_completionist", "target": 2}
                ],
            )
            row["sources"].append("tibiawiki:Cave Completionist:846114")
        else:
            row["forbids_outfits"] = [outfit]
    for tier, npc, level, ceiling, xp, report in [
        ("upper", "gnomilly", 25, 49, 1000, "killing"),
        ("middle", "gnombold", 50, 79, 2000, "killing"),
        ("lower", "gnomargery", 80, None, 3500, "kill"),
    ]:
        task = spike + tier + "/"
        phase, kills, timer = (
            task + name for name in ("phase", "kills", "reported_at")
        )
        source = pin + npc + ".lua"
        spike_track(timer, 9_223_372_036_854_775_807, source)
        offers.extend(
            [
                {
                    "npc": "oteryn:npc." + npc,
                    "keywords": ["kill"],
                    "topic": 0,
                    "next_topic": 24,
                    "minimum_level": level,
                    "storage": [storage(phase, 0, 0, preserve=True)],
                    **(
                        {
                            # Crystal checks the "kill" request's level even when
                            # a prior offer is awaiting confirmation.
                            "any_topic": True,
                            "maximum_level": ceiling,
                            "failure_reply": [
                                f"This hunt is for levels {level} through {ceiling}."
                            ],
                        }
                        if ceiling is not None
                        else {}
                    ),
                    "reply": ["Will you take on the local hunting task?"],
                },
                {
                    "npc": "oteryn:npc." + npc,
                    "keywords": ["report"],
                    "topic": 0,
                    "any_topic": True,
                    "next_topic": 25,
                    "storage": [],
                    "reply": ["Which task are you reporting?"],
                },
            ]
        )
        add(
            "l12-spike-" + tier + "-accept",
            "spike_task",
            npc,
            ["yes"],
            [storage(phase, 0, 1)],
            topic=24,
        )
        callbacks[-1].update(
            minimum_level=level,
            state_ops=[elapsed(timer, 72_001)],
            sources=[source],
            reply=["Hunt seven of the local targets and then report back."],
        )
        if ceiling is not None:
            callbacks[-1]["maximum_level"] = ceiling
        add(
            "l12-spike-" + tier + "-report",
            "spike_task",
            npc,
            [report],
            [storage(phase, 1, 0), storage(kills, 7, 0)],
            topic=25,
        )
        callbacks[-1].update(
            experience=xp,
            state_ops=[stamp(timer), fame_change(0, fame_max - 1, 1)],
            sources=[source],
            reply=["Your report earns one point of Spike fame."],
        )
    engoutfit_m36.append(
        add,
        storage,
        callbacks,
        offers,
        declarations,
        f"crystal:{PIN}:data-global/npc/rock_steady.lua:56-98:sha256:{sha(inputs['data-global/npc/rock_steady.lua'])}",
    )
    qmech_m39.append(add, callbacks, offers, declarations, inputs, PIN)
    m39_keys = (
        "l2-cael-tome-",
        "l2-zalamon-",
        "l2-alliance-",
        "l2-bone-",
        "bone-pit-lost-sample",
    )
    m39_callbacks = [
        callback
        for callback in callbacks
        if callback["key"].split("/")[-1].startswith(m39_keys)
    ]
    callbacks = [callback for callback in callbacks if callback not in m39_callbacks]
    m39_tracks = [
        track for track in declarations if track["key"].endswith("/corruption_hole")
    ]
    declarations = [track for track in declarations if track not in m39_tracks]
    validate_callbacks({"schema": 1, "tracks": m39_tracks, "callbacks": m39_callbacks})
    # L13: original native parent bases remain with their existing reward owners.
    for family, npc, items, bits, achievement in [
        ("revenant", "flickering_soul", [34075, 34076], [2, 1], "unleash_the_beast"),
        (
            "decaying_defender",
            "bloodshade_rotten",
            [43899, 43900],
            [1, 2],
            "fully_decayed",
        ),
    ]:
        for index, (item, bit) in enumerate(zip(items, bits)):
            add(
                "l13-" + family + "-" + ("first" if index == 0 else "second"),
                family + "_outfits_quest",
                npc,
                ["accessory"],
                [],
                take=[(item, 1)],
                outfit=(family, bit),
                achievement="oteryn:achievement/" + achievement if index == 1 else None,
            )
            callbacks[-1].update(
                carried_item=f"oteryn:item.tibia.i{item}",
                minimum_level=250,
                requires_premium=True,
                requires_outfits=["oteryn:outfit." + family],
                forbids_addons=[{"outfit": "oteryn:outfit." + family, "addons": bit}],
            )
            if index == 1:
                callbacks[-1]["requires_addons"] = [
                    {"outfit": "oteryn:outfit." + family, "addons": bits[0]}
                ]
            path = "data/scripts/actions/items/usable_outfit_items.lua"
            callbacks[-1]["sources"].append(
                f"crystal:{PIN}:{path}:26-31:sha256:{sha(inputs[path])}"
            )
    add(
        "l13-rift-base",
        "rift_warrior_outfits_quest",
        "cledwyn",
        ["rift warrior outfit"],
        [],
        outfit=("rift_warrior", 0),
    )
    callbacks[-1].update(
        requires_premium=True,
        forbids_outfits=["oteryn:outfit.rift_warrior"],
        requires_completed_quests=["oteryn:quest.ferumbras_ascension_quest"],
    )
    callbacks[-1]["sources"].append(
        "owner:chosen-Cledwyn-completed-Ferumbras-base-attestation:read-only-parent"
    )
    for bit, stage in [(1, "first"), (2, "second")]:
        add(
            "l13-rift-" + stage,
            "rift_warrior_outfits_quest",
            "cledwyn",
            [stage],
            [],
            take=[(22516, 100)],
            outfit=("rift_warrior", bit),
            topic=3,
            achievement="oteryn:achievement/rift_warrior" if bit == 2 else None,
        )
        callbacks[-1].update(
            requires_premium=True,
            requires_outfits=["oteryn:outfit.rift_warrior"],
            forbids_addons=[{"outfit": "oteryn:outfit.rift_warrior", "addons": bit}],
        )
        if bit == 2:
            callbacks[-1]["requires_addons"] = [
                {"outfit": "oteryn:outfit.rift_warrior", "addons": 1}
            ]
    source_offer("cledwyn", ["addon"], 0, 3, [])
    questl2_ready.append(add, storage, callbacks, offers, inputs, PIN)
    ready_callbacks = [
        c
        for c in callbacks
        if c["key"]
        .split("/")[-1]
        .startswith(("l2-white-raven-", "l2-efreet-", "l2-marid-"))
    ]
    callbacks = [c for c in callbacks if c not in ready_callbacks]
    ready_callbacks.sort(key=lambda c: c["key"])
    lane = load_module(
        "questpc7_integration",
        "tools/content-schema/quest-authoring/questpc7_integration.py",
    )
    lane_data = lane.extend(ROOT, callbacks, offers, inputs, PIN, load_module)
    fibula_callbacks, fibula_tracks = [], []
    fibula = load_module(
        "fibula_reward_authoring",
        "tools/content-schema/quest-authoring/fibula_rewards.py",
    )
    fibula.append(fibula_callbacks, offers, inputs)
    fibula.append_simon(fibula_callbacks, offers, fibula_tracks, inputs)
    # Keep every registered document within the runtime's 256-record bound.
    l13_callbacks = [c for c in callbacks if c["key"].split("/")[-1].startswith("l13-")]
    callbacks = [c for c in callbacks if c not in l13_callbacks]
    programme_callbacks = [
        c for c in callbacks if c["key"].split("/")[-1].startswith("questpc7-")
    ]
    programme_tracks = lane_data[3]
    callbacks = [c for c in callbacks if c not in programme_callbacks]
    l13_callbacks.sort(key=lambda c: c["key"])
    programme_callbacks.sort(key=lambda c: c["key"])
    # Amber and Lubo teach the same account addon. Character storage retains
    # the donor's separate routes and timer, while payment reserves one grant.
    backpack = "oteryn:quest-reservation/citizen-backpack"
    for callback in callbacks:
        name = callback["key"].split("/")[-1]
        if name in (
            "l6-citizen-rook-backpack-materials",
            "qb-citizen-backpack-materials",
        ):
            pending = callback["storage"][0]
            callback["account_reservation"] = {
                "action": "start",
                "key": backpack,
                "track": pending["track"],
                "pending_value": pending["set"],
            }
        elif name in ("l6-citizen-rook-backpack", "qb-citizen-backpack"):
            callback["account_reservation"] = {"action": "finish", "key": backpack}
    # The donor exposes these additive rewards in either order. Use the other
    # hand-in's existing terminal track so characters with one older addon qualify.
    by_name = {callback["key"].rsplit("/", 1)[-1]: callback for callback in callbacks}
    warrior_completions = []
    for achievement, page, revision, rewards in [
        (
            "hunting_with_style",
            62855,
            753202,
            [
                ("l8-hunter-hat", "hunter_hat_addon"),
                ("l8-hunter-gloves", "addon_glove"),
            ],
        ),
        (
            "in_shining_armor",
            62856,
            753037,
            [
                ("knight-sword", "addon_sword"),
                ("l7-knight-helmet-reward", "addon_helmet"),
            ],
        ),
        (
            "master_of_war",
            62865,
            753026,
            [
                ("l1-warmaster-crest", "warmaster_addon_1"),
                ("l1-warmaster-tribal", "warmaster_addon_2"),
            ],
        ),
        (
            "mystic_fabric_magic",
            62867,
            844815,
            [("l2-elementalist-cap", "addon1"), ("l2-elementalist-staff", "addon2")],
        ),
        (
            "one_thousand_and_one",
            62871,
            753214,
            [
                ("l5-oriental-female-comb", "first_oriental_addon"),
                ("l5-oriental-female-blue-cloth", "second_oriental_addon"),
            ],
        ),
        (
            "one_thousand_and_one",
            62871,
            753214,
            [
                ("l5-oriental-male-comb", "first_oriental_addon"),
                ("l5-oriental-male-blue-cloth", "second_oriental_addon"),
            ],
        ),
        (
            "peazzekeeper",
            62873,
            753184,
            [
                ("l1-wayfarer-cape", "wayfarer_outfit"),
                ("l1-wayfarer-hat", "wayfarer_outfit_1"),
            ],
        ),
        (
            "swift_death",
            62886,
            753279,
            [
                ("l7-assassin-yarn", "assassin_first_addon"),
                ("l7-assassin-second", "assassin_second_addon"),
            ],
        ),
        (
            "swashbuckler",
            62885,
            753261,
            [
                ("pirate-sabre", "pirate_sabre_addon"),
                ("l4-pirate-hat", "pirate_hat_addon"),
            ],
        ),
        (
            "wild_warrior",
            None,
            None,
            [
                ("qb-warrior-shoulder", "warrior_shoulder_addon"),
                ("qb-warrior-sword", "warrior_sword_addon"),
            ],
        ),
    ]:
        for (name, _), (other, completion) in (rewards, rewards[::-1]):
            if name + "-complete" in by_name:
                continue
            original = by_name[name]
            earned = next(
                state
                for state in by_name[other]["storage"]
                if state["track"].endswith("/" + completion)
            )
            finishing = deepcopy(original)
            original.setdefault("requires", []).append(
                requirement(earned["track"], -1, earned["set"] - 1)
            )
            finishing.setdefault("requires", []).append(
                requirement(earned["track"], earned["set"])
            )
            finishing["key"] += "-full-outfit"
            finishing["achievement"] = "oteryn:achievement/" + achievement
            if page is not None:
                finishing["sources"].append(
                    f"https://tibia.fandom.com/index.php?curid={page}&oldid={revision}"
                )
            if achievement == "wild_warrior":
                warrior_completions.append(finishing)
            else:
                callbacks.append(finishing)
    callbacks.sort(key=lambda c: c["key"])
    free_glove_offers.sort(key=lambda offer: (offer["npc"], offer["topic"]))
    offers.sort(key=lambda c: (c["npc"], c["topic"], c["keywords"]))
    item_disguises = load_module(
        "quest_item_disguises",
        "tools/content-schema/quest-authoring/quest_item_disguises.py",
    ).build(ROOT)
    outfit_completions = [
        row
        for row in callbacks
        if row["key"].endswith(("-full-outfit", "-complete"))
        and row.get("outfit") is not None
    ]
    classic_routes = ("sven-", "l12-cave-explorer-", "l12-spike-", "l2-battle-mage-")
    classic_callbacks = [
        row for row in callbacks if row["key"].rsplit("/", 1)[-1].startswith(classic_routes)
    ]
    classic_track_keys = {
        state["track"]
        for callback in classic_callbacks
        for effects in [callback, *callback.get("participants", [])]
        for state in [*effects.get("storage", []), *effects.get("state_ops", [])]
    }
    classic_tracks = [track for track in declarations if track["key"] in classic_track_keys]
    shared_tracks = [track for track in declarations if track["key"] not in classic_track_keys]
    classic_document = {"schema": 1, "tracks": classic_tracks, "callbacks": classic_callbacks}
    validate_callbacks(classic_document)
    shared_callbacks = [
        row for row in callbacks if row not in outfit_completions and row not in classic_callbacks
    ]
    outfit_document = {"schema": 1, "callbacks": outfit_completions}
    validate_callbacks(outfit_document)
    validate_callbacks(
        {
            "schema": 1,
            "tracks": shared_tracks,
            "callbacks": shared_callbacks,
            "item_disguises": item_disguises,
        }
    )
    battle_mage_document = {
        "schema": 1,
        "callbacks": sorted(battle_mage_completions, key=lambda c: c["key"]),
    }
    validate_callbacks(battle_mage_document)
    warrior_document = {
        "schema": 1,
        "callbacks": sorted(warrior_completions, key=lambda c: c["key"]),
    }
    validate_callbacks(warrior_document)
    validate_callbacks(
        {"schema": 1, "tracks": fibula_tracks, "callbacks": fibula_callbacks}
    )
    validate_callbacks({"schema": 1, "callbacks": l13_callbacks})
    validate_callbacks(
        {"schema": 1, "tracks": programme_tracks, "callbacks": programme_callbacks}
    )
    native_cosmetics = cosmetics.provisioning_input(
        ROOT / cosmetics.AUTHORING,
        appearance / "manifest.json",
        ROOT / cosmetics.OUTPUT,
        authoring_bytes=encoded(author),
    )
    gameplay["schema"] = "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6"
    gameplay["cosmetics"] = {
        "path": "cosmetics/outfits/native-cosmetics.json",
        "sha256": sha(native_cosmetics),
    }
    generated = {
        "content/cosmetics/taming-native.json": encoded(native_taming),
        "tools/content-schema/native-gameplay/item-cosmetic-actions-native.json": encoded(
            item_packet
        ),
        "content/cosmetics/outfits/authoring.json": encoded(author),
        "content/cosmetics/outfits/native-cosmetics.json": native_cosmetics,
        "content/spells.manifest.json": (
            json.dumps(gameplay, indent=2, sort_keys=True) + "\n"
        ).encode(),
        "content/quests/npc-callbacks/outfit-completions.json": encoded(
            outfit_document
        ),
        "content/quests/shared-reward-callbacks.json": encoded(
            {
                "schema": 1,
                "tracks": shared_tracks,
                "callbacks": shared_callbacks,
                "item_disguises": item_disguises,
            }
        ),
        "content/quests/npc-callbacks/classic-quest-rewards.json": encoded(classic_document),
        "content/quests/npc-callbacks/battle-mage-completions.json": encoded(
            battle_mage_document
        ),
        "content/quests/npc-callbacks/warrior-completions.json": encoded(
            warrior_document
        ),
        "content/quests/npc-callbacks/fibula.json": encoded(
            {"schema": 1, "tracks": fibula_tracks, "callbacks": fibula_callbacks}
        ),
        "content/quests/npc-callbacks/l13-outfits.json": encoded(
            {"schema": 1, "callbacks": l13_callbacks}
        ),
        "content/quests/npc-callbacks/questpc7-programmes.json": encoded(
            {"schema": 1, "tracks": programme_tracks, "callbacks": programme_callbacks}
        ),
        "content/quests/npc-offers/hunter-glove-sales.json": encoded(
            {"schema": 1, "offers": free_glove_offers}
        ),
        "content/quests/shared-reward-offers.json": encoded(
            {"schema": 1, "offers": offers}
        ),
    }
    generated["content/quests/npc-callbacks/l2-ready-stories.json"] = encoded(
        {"schema": 1, "tracks": [], "callbacks": ready_callbacks}
    )
    generated[qmech_m39.SHARD] = encoded(
        {
            "schema": 1,
            "tracks": m39_tracks,
            "callbacks": sorted(m39_callbacks, key=lambda callback: callback["key"]),
        }
    )
    generated[qmech_m39.WORLD_SHARD] = encoded(qmech_m39.worlds(inputs))
    from hot_cuisine_books import build as cookbook_sources

    generated["content/quests/world-callbacks/hot-cuisine-books.json"] = encoded(
        {
            "schema": 1,
            "callbacks": cookbook_sources(ROOT),
        }
    )
    # Preserve the source's complete contiguous 1..10001 supply-package table.
    random_path = "data-global/scripts/actions/valuables/random_items.lua"
    parser = load_module(
        "qengine_literal_item_tables", "tools/content-migration/event_item_actions.py"
    )
    config, _ = parser.table(inputs[random_path].decode(), "config")
    chances = config[15698]["chances"]
    rewards = []
    last = 0
    if list(chances) != list(range(1, 26)):
        raise ValueError("GNOMISH_SOURCE_OUTCOME_COUNT")
    for row in chances.values():
        if row["from"] != last + 1 or row["to"] < row["from"]:
            raise ValueError("GNOMISH_SOURCE_RANGE_GAP")
        count = row.get("count", 1)
        rewards.append(
            {
                "item": f"oteryn:item.tibia.i{row['itemId']}",
                "name": "",
                "weight": row["to"] - row["from"] + 1,
                "minimum": count,
                "maximum": count,
            }
        )
        last = row["to"]
    if last != 10001:
        raise ValueError("GNOMISH_SOURCE_ROLL_BOUND")
    generated[
        "tools/content-schema/native-gameplay/qengine-reward-bag-profiles.json"
    ] = encoded(
        {
            "schema": "OTERYN_QENGINE_REWARD_BAG_PROFILES/v1",
            "sources": [
                {
                    "path": random_path,
                    "revision": PIN,
                    "sha256": sha(inputs[random_path]),
                }
            ],
            "bags": [
                {
                    "constant": "QENGINE_GNOMISH_SUPPLY",
                    "bag": "oteryn:item.tibia.i15698",
                    "playable": True,
                    "selection": "weighted",
                    "picks": 1,
                    "roll_max": 10001,
                    "donor": f"crystal:{PIN}:{random_path}:49-80:sha256:{sha(inputs[random_path])}",
                    "rewards": rewards,
                }
            ],
        }
    )
    # The sealed cave has no route to Muhad. Suppress the referral and its prompt.
    for kind, identity, disabled in (
        ("callbacks", "key", "oteryn:quest-reward/nomads-land-arito-referral"),
        ("offers", "npc", "oteryn:npc.arito"),
    ):
        path = f"content/quests/npc-{kind}/nomads-land.json"
        document = json.loads((ROOT / path).read_bytes())
        document[kind] = [row for row in document[kind] if row[identity] != disabled]
        if kind == "callbacks":
            validate_callbacks(document)
        generated[path] = encoded(document)
    generated.update(elane_gloves(inputs))
    greetings = reward_greetings(
        callbacks
        + battle_mage_completions
        + m39_callbacks
        + ready_callbacks
        + l13_callbacks
        + programme_callbacks,
        inputs,
    )
    generated["content/dialogues/quests/shared-reward-greetings.json"] = encoded(
        {"schema": 1, "npcs": greetings}
    )
    quest_shops = questl2_ready.djinn.shops(inputs, PIN)
    ape_and_rook = json.loads(
        (
            ROOT / "content/quests/authoring/ape-city-and-beginning-shops.json"
        ).read_bytes()
    )
    quest_shops.extend(ape_and_rook["shops"])
    quest_shops.append(lost_brother_shop())
    quest_shops.append(
        json.loads(
            (ROOT / "tools/content-schema/quest-authoring/cael-shop.json").read_bytes()
        )
    )
    service = "oteryn:service.trade.black_bert"
    trade = json.loads(
        (ROOT / "content/services/trade/trade-00000-00323.json").read_bytes()
    )
    stock = next(
        row["declaration"]["offers"]
        for row in trade["records"]
        if row["declaration"]["identity"]["key"] == service
    )
    bert_source = inputs["data-global/npc/black_bert.lua"]
    # The corpus copy is byte-identical to the chosen 201189f8 shop source.
    if (
        sha(bert_source)
        != "d4e83b34b3f20b62f0e1f06a764c6dadea0010a61f252048300c44eda27e6e0c"
    ):
        raise ValueError("Black Bert shop source changed")
    bert_shop = lua_tables.as_python(
        lua_tables.assignments(bert_source.decode(), {"shop"})["shop"]
    )
    # Keep source stock already admitted by the Item catalogue and the two
    # previously reviewed Global variants (special flask and lump of clay).
    prices = {
        f"oteryn:item.tibia.i{row['clientid']}": row["buy"]
        for row in bert_shop
        if "buy" in row
    }
    stock_keys = prices.keys() | {"oteryn:item.tibia.i125", "oteryn:item.tibia.i11329"}
    retained = [
        row | {"unit_price": prices.get(row["item"]["key"], row["unit_price"])}
        for row in stock
        if row["direction"] == "SellToPlayer" and row["item"]["key"] in stock_keys
    ]
    requirement = {
        "quest": "oteryn:quest.the_thieves_guild_quest",
        "track": "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/questline",
        "initial": 0,
        "minimum": 9,
        "maximum": 9,
    }
    quest_shops.append(
        {
            "npc": "oteryn:npc.black_bert",
            "service": service,
            "offers": [row | {"requirements": [requirement]} for row in retained],
            "sources": [
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/dorian.lua:211-219",
                f"crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/black_bert.lua:23-154:sha256:{sha(bert_source)}",
            ],
        }
    )
    quest_shops.sort(key=lambda shop: shop["npc"])
    generated["content/quests/shared-quest-shops.json"] = encoded(
        {"schema": 1, "shops": quest_shops}
    )
    generated["content/quests/questl2-djinn-world.json"] = encoded(
        questl2_ready.djinn.world_callbacks(inputs, PIN)
    )
    access = load_module(
        "qengine_l1_access", "tools/content-schema/quest-authoring/qengine_access.py"
    )
    worlds = []
    for name, quest, path, at, appearance, states, outfit in [
        (
            "l1-warmaster-wardrobe",
            "the_new_frontier_quest",
            "data-global/scripts/quests/the_new_frontier/action_outfit.lua",
            (33053, 31020, 7),
            5862,
            [
                storage(frontier + "questline", 26, 27),
                storage(frontier + "mission09/reward_door", 0, 1),
            ],
            "warmaster",
        ),
        (
            "l1-wayfarer-wardrobe",
            "wrath_of_the_emperor_quest",
            "data-global/scripts/quests/wrath_of_the_emperor/actions_mission12_just_rewards.lua",
            (33073, 31173, 8),
            11431,
            [
                storage(prefix + "u8_6/wrath_of_the_emperor/mission12", 0, 1),
                storage("oteryn:quest-progress/1150", -1, 1, 1),
            ],
            "wayfarer",
        ),
    ]:
        world = access.callback(
            name,
            quest,
            "use",
            path,
            at=access.position(*at),
            appearance=appearance,
            outfit_reward={"outfit": "oteryn:outfit." + outfit, "addons": 0},
        )
        world["storage"] = states
        world["sources"] = [f"crystal:{PIN}:{path}:sha256:{sha(inputs[path])}"]
        if outfit == "wayfarer":
            table = "data-global/startup/tables/item.lua"
            if not re.search(
                r"\[3200\]\s*=\s*\{\s*itemId\s*=\s*11431,\s*itemPos\s*=\s*\{\s*x\s*=\s*33073,\s*y\s*=\s*31173,\s*z\s*=\s*8",
                inputs[table].decode(),
            ):
                raise ValueError("Wayfarer source wardrobe binding drift")
            world["sources"].append(
                f"crystal:{PIN}:{table}:sha256:{sha(inputs[table])}"
            )
        worlds.append(world)
    generated["content/quests/shared-outfit-callbacks.json"] = encoded(
        {"schema": 1, "callbacks": worlds}
    )

    sys.path.insert(0, str(ROOT / "tools/content-migration"))
    import qengine_reward_item_profiles as reward_items
    import register_spell_families as native

    generated[reward_items.PROFILE] = (ROOT / reward_items.PROFILE).read_bytes()
    reward_items.admit(
        ROOT,
        generated,
        reward_items.PROFILE,
        "content/spells.manifest.json",
        native.provider_order,
        cosmetics.canonical,
    )
    import srx1_profile_updates

    updates = json.loads((ROOT / srx1_profile_updates.PATH).read_bytes())
    if updates["base_sha256"] != sha(generated[reward_items.PROFILE]):
        srx1_profile_updates.generate(
            ROOT,
            generated,
            reward_items.PROFILE,
            "content/spells.manifest.json",
            native.provider_order,
            cosmetics.canonical,
        )
    generated.update(lane.outputs(ROOT, inputs, PIN, load_module, lane_data))
    # The selected smith port owns these keys and the explicit cache placement.
    # Retain all other authored programmes; do not mount the older native-cache
    # variant beside its replacement.
    authored_path = "content/quests/authored-world-callbacks.json"
    authored = json.loads((ROOT / authored_path).read_bytes())
    smith_keys = {
        "oteryn:quest-callback/authored-smith-cache",
        "oteryn:quest-callback/authored-smith-cache-reward",
    }
    authored["callbacks"] = [
        c for c in authored["callbacks"] if c["key"] not in smith_keys
    ]
    generated[authored_path] = encoded(authored)
    return generated


def validate_callbacks(document):
    schema = json.loads(
        (
            ROOT
            / "tools/content-schema/quest-authoring/shared-reward-callbacks.schema.json"
        ).read_bytes()
    )
    Draft202012Validator(schema).validate(document)
    declarations = document.get("tracks", [])
    declared = set()
    readonly = {
        r["track"] for c in document["callbacks"] for r in c.get("requires", [])
    }
    owners = {}
    for c in document["callbacks"]:
        for owner, states in [(c["quest"], c["storage"] + c.get("state_ops", []))] + [
            (p["quest"], p["storage"] + p.get("state_ops", []))
            for p in c.get("participants", [])
        ]:
            for state in states:
                owners.setdefault(state["track"], set()).add(owner)
    written = set(owners)
    for track in declarations:
        writable = track.get("writable", False)
        if (
            track["key"] in declared
            or not track["min"] <= track["initial"] <= track["max"]
            or (writable and owners.get(track["key"]) != {track["quest"]})
            or (
                not writable
                and (track["key"] not in readonly or track["key"] in written)
            )
        ):
            raise ValueError("invalid reward track declaration " + track["key"])
        declared.add(track["key"])
    keys = set()
    for callback in document["callbacks"]:
        inventory = callback.get("inventory", [])
        if len({rule["item"] for rule in inventory}) != len(inventory) or any(
            rule["minimum"] > rule["maximum"] for rule in inventory
        ):
            raise ValueError("invalid inventory range " + callback["key"])
        reward = callback.get("give")
        weighted = callback.get("weighted_rewards", [])
        if weighted and (
            not reward
            or callback.get("world_claim") is not None
            or callback.get("carried_item") is not None
            or callback.get("counter") is not None
            or sum(variant["chance"] for variant in weighted) >= 100
            or any(
                variant["reward"].get("key_number") is not None for variant in weighted
            )
        ):
            raise ValueError("invalid weighted reward " + callback["key"])
        if callback.get("per_occurrence_mint", False) and (
            not reward
            or callback.get("outfit") is not None
            or callback.get("outfits")
            or callback.get("achievement") is not None
            or callback.get("achievement_progress")
        ):
            raise ValueError("invalid occurrence mint " + callback["key"])
        if (
            reward
            and reward.get("text") is not None
            and len(reward["text"].encode("utf-8")) > 7996
        ):
            raise ValueError("literal reward text exceeds UTF-8 byte bound")
        if callback["quest"] in callback.get("requires_completed_quests", []):
            raise ValueError("self-dependent reward completion " + callback["key"])
        storage = callback["storage"]
        source, target = callback.get("carried_item"), callback.get("carried_target")
        if target is not None and (
            source is None
            or source == target
            or not callback.get("give")
            or callback.get("peer_requires")
            or callback.get("creature_target")
            or any(
                {"item": item, "count": 1} not in callback["take"]
                for item in (source, target)
            )
        ):
            raise ValueError("invalid carried quest recipe " + callback["key"])
        if source is not None and (
            (
                target is None
                and (
                    callback["take"] != [{"item": source, "count": 1}]
                    or callback.get("give") is not None
                )
            )
            or callback.get("fee") is not None
            or callback.get("requires_actor_conditions")
        ):
            raise ValueError("invalid carried quest tool " + callback["key"])
        targets = (
            [callback["outfit"]]
            if callback.get("outfit") is not None
            else callback.get("outfits", [])
        )
        value_guard = any(
            t["outfit"] == g["outfit"]
            and t["addons"] & g["addons"]
            and (
                t.get("sex") is None
                or t.get("sex") == (g.get("sex") or callback.get("creation_sex"))
            )
            for t in targets
            for g in callback.get("forbids_addons", [])
        )
        value_guard = value_guard or any(
            t["addons"] == 0
            and t.get("sex") is None
            and t["outfit"] in callback.get("forbids_outfits", [])
            for t in targets
        )
        value_guard = value_guard or (
            bool(callback["take"]) and callback.get("give") is not None
        )
        value_guard = value_guard or (
            callback.get("per_occurrence_mint", False)
            and any(
                op["from"]["kind"] == "elapsed_at_least"
                and op["from"]["seconds"] > 0
                and op["effect"]["kind"] == "set_now"
                for op in callback.get("state_ops", [])
            )
        )
        value_guard = value_guard or (
            callback.get("repeatable", False)
            and bool(callback["take"])
            and 1 <= callback.get("experience", 0) <= 100000000
        )
        if callback.get("outfits") and callback.get("outfit") is not None:
            raise ValueError("mixed legacy and target-set grants " + callback["key"])
        participants = callback.get("participants", [])
        participant_storage = [s for p in participants for s in p["storage"]]
        participant_ops = [s for p in participants for s in p.get("state_ops", [])]
        operations = callback.get("state_ops", [])
        all_keys = [
            s["track"]
            for s in storage + operations + participant_storage + participant_ops
        ]
        counter = callback.get("counter")
        if counter is not None and (
            not 1 <= counter["maximum"] <= 1_000_000_000_000
            or callback.get("completes", False)
            or counter["track"] in all_keys
            or counter["track"] in {r["track"] for r in callback.get("requires", [])}
        ):
            raise ValueError("invalid owned reward counter " + callback["key"])
        value_guard = value_guard or (
            counter is not None and bool(storage or operations)
        )
        if (
            len(all_keys) + int(counter is not None) > 16
            or len(all_keys) != len(set(all_keys))
            or any(
                len(p["storage"]) + len(p.get("state_ops", [])) > 8
                for p in participants
            )
            or len({p["quest"] for p in participants}) != len(participants)
            or any(p["quest"] == callback["quest"] for p in participants)
            or any(s["minimum"] > s["maximum"] for s in participant_storage)
            or any(
                s["from"]["kind"] == "between"
                and s["from"]["minimum"] > s["from"]["maximum"]
                for s in operations + participant_ops
            )
            or {r["track"] for r in callback.get("requires", [])} & set(all_keys)
        ):
            raise ValueError("invalid reward participants " + callback["key"])
        requires = callback.get("requires", [])
        if (
            callback["key"] in keys
            or len({s["track"] for s in storage}) != len(storage)
            or len({p["key"] for p in callback.get("achievement_progress", [])})
            != len(callback.get("achievement_progress", []))
            or len({i["item"] for i in callback["take"]}) != len(callback["take"])
            or len({i["item"] for i in callback.get("present", [])})
            != len(callback.get("present", []))
            or (
                callback.get("fee")
                and any(
                    i["item"]
                    in (
                        "oteryn:item.tibia.i3031",
                        "oteryn:item.tibia.i3035",
                        "oteryn:item.tibia.i3043",
                    )
                    for i in callback["take"]
                )
            )
            or len({r["track"] for r in requires}) != len(requires)
            or {r["track"] for r in requires} & {s["track"] for s in storage}
            or any(r["minimum"] > r["maximum"] for r in requires)
            or any(s["minimum"] > s["maximum"] for s in storage)
            or (
                not value_guard
                and not callback.get("repeatable", False)
                and all(
                    s.get("preserve", False) or s["minimum"] <= s["set"] <= s["maximum"]
                    for s in storage + participant_storage
                )
            )
            or (
                callback.get("repeatable", False)
                and (
                    not (
                        (callback.get("fee") and callback.get("give"))
                        or (
                            bool(callback["take"])
                            and (
                                callback.get("give")
                                or 1 <= callback.get("experience", 0) <= 100000000
                            )
                        )
                    )
                    or callback.get("outfit")
                    or callback.get("outfits")
                    or callback.get("achievement")
                    or callback.get("completes", False)
                    or any(not s.get("preserve", False) for s in storage)
                )
            )
            or (
                (callback.get("give") or {}).get("key_number") is not None
                and (callback["give"]["count"] != 1 or callback["give"].get("contents"))
            )
            or any(k != k.lower() or not k.isascii() for k in callback["keywords"])
            or any(
                len(line.encode("utf-8")) > 512
                or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in line)
                for line in callback["reply"]
            )
            or any(len(source.encode("utf-8")) > 1024 for source in callback["sources"])
        ):
            raise ValueError("invalid reward callback " + callback["key"])
        keys.add(callback["key"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, raw in build().items():
        target = ROOT / path
        if args.check:
            if not target.is_file() or target.read_bytes() != raw:
                raise ValueError("stale QENGINE reward output: " + path)
        elif not target.is_file() or target.read_bytes() != raw:
            target.write_bytes(raw)
    retired = ROOT / questpc7_smith_programme.RETIRED_PROP_PATH
    if args.check:
        if retired.exists():
            raise ValueError(
                "retired QENGINE reward output: " + retired.relative_to(ROOT).as_posix()
            )
    else:
        retired.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
