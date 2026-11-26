"""Item custody for the middle third of the base creature loot audit.

The retained Item XML is a bounded extract from Crystal 201189f (items.xml
SHA256 8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509).
Use exact client physics and existing production bindings. Keep admitted Item
owners and use TibiaWiki revision facts for three Items absent from that XML.
"""

import hashlib
import json
from decimal import Decimal

import build_source_item_profiles as source

NATIVE = "tools/content-schema/native-gameplay"
CRYSTAL_EXTRACT = "794af765f23c16f0ddeed71425f6d128faff040afda42cc5af7a18c83e529ec1"
ITEM_PREFIX = "oteryn:item.tibia.i"

# Crystal Muglex Clan Chief Lua names health and mana potions. Its numeric IDs
# 7618 and 7620 refer to obsolete appearances. Use the exact 15.30 potion IDs.
# Crystal Yselda Lua lacks the Transcendence Potion. TibiaWiki's Yselda loot and
# Transcendence Potion page identify 49271, corroborated by Crystal items.xml.
# Preserve the authored rolls and replace only these tables' stale identities.
REPLACEMENTS = {
    # Crystal Lloyd rolls 3100 (data-global/monster/quests/forgotten_knowledge/
    # bosses/lloyd.lua:78). src/items/item.cpp:56,107-109 creates the inactive 3098.
    "oteryn:loot.creature.lloyd": {3100: 3098},
    "oteryn:loot.creature.megasylvan_yselda": {51302: 49271},
    "oteryn:loot.creature.muglex_clan_chief": {7618: 266, 7620: 268},
}


def item_ids(root):
    ids = json.loads((root / NATIVE / "cloot2-item-ids.json").read_bytes())
    if ids != sorted(set(ids)):
        raise ValueError("CLOOT2_ITEM_IDS")
    return set(ids)


def source_profiles(root):
    from encounter_loot_item_admissions import BINDINGS, CLIENT, CLIENT_SHA

    xml = (root / NATIVE / "cloot2-crystal-item-facts.xml").read_bytes()
    bank = (root / CLIENT).read_bytes()
    if (
        hashlib.sha256(xml).hexdigest() != CRYSTAL_EXTRACT
        or hashlib.sha256(bank).hexdigest() != CLIENT_SHA
    ):
        raise ValueError("CLOOT2_ITEM_SOURCE_DIGEST")
    nodes = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(bank)
    bindings = source.binding_index((root / BINDINGS).read_bytes())
    wiki_bindings = {
        int(b["external_id"]): b
        for b in json.loads(
            (root / NATIVE / "cloot2-wiki-item-bindings.json").read_bytes()
        )
    }
    if (
        nodes.keys() & wiki_bindings.keys()
        or set(map(str, wiki_bindings)) & bindings.keys()
    ):
        raise ValueError("CLOOT2_DUPLICATE_SOURCE_ID")
    wiki = json.loads((root / "imports/tibiawiki/facts/items-stats.json").read_bytes())[
        "records"
    ]
    ids = nodes.keys() | wiki_bindings.keys()
    physical = {p["server_item_id"]: p for p in source.policies(bank, xml, ids)}
    for item in sorted(ids):
        key = f"{ITEM_PREFIX}{item}"
        binding = wiki_bindings.get(item) or bindings[str(item)]
        target = {"family": "Item", "key": key, "revision": "definition-r1"}
        if binding["disposition"] != "EXACT" or binding["target"] != target:
            raise ValueError(f"CLOOT2_ITEM_BINDING:{item}")
        profile = source.profile(
            item, nodes.get(item), objects[item], physical[item], bindings, None
        )
        if item in wiki_bindings:
            obj = objects[item]
            (observation,) = (
                o
                for o in wiki[key]["observations"]
                if binding["source_revision"] == f"fandom-r{o['revision_id']}"
                and o["wiki_title"].casefold() == obj["name"].casefold()
            )
            if (
                wiki[key]["item_id"] != item
                or obj["flags"].get("cyclopediaitem.cyclopedia_type") != item
            ):
                raise ValueError(f"CLOOT2_WIKI_ITEM_ID:{item}")
            profile["semantics"]["physical"]["value"]["weight"] = source.K(
                int(Decimal(observation["fields"]["weight"]) * 100)
            )
            if item in (44432, 44433):
                # TibiaWiki confirms eventual disintegration, with no measured
                # duration. Leave the temporal field unknown until it is sourced.
                profile["semantics"]["temporal"] = source.U
        profile["production_binding"] = binding
        profile["production_definition"] = {
            "family": "Item",
            "production_key": key,
            "revision_ref": "definition-r1",
        }
        yield item, profile


def admit(root, document, order):
    import house_decoration_actions
    from build_full_spell_artifact import native_loot_yield_facts
    from grizzly_shop_profiles import accepted_first
    from writing_item_admission import ids as writing_ids

    records = {r["production_binding"]["target"]["key"]: r for r in document["records"]}
    # SRX-1 admits these writing rows with their qualified attributes. The
    # final loot projection checks custody against the composed Item provider.
    ids = item_ids(root) - set(writing_ids(root))
    source_needed = {
        item
        for item in ids
        if (current := records.get(f"{ITEM_PREFIX}{item}")) is None
        or native_loot_yield_facts(current["production_binding"]["target"], current)
        is None
    }
    if not source_needed:
        return
    qualified = {
        item: profile
        for item, profile in source_profiles(root)
        if item in source_needed
    }
    if qualified.keys() != source_needed:
        raise ValueError(
            f"CLOOT2_MISSING_ITEM_FACTS:{sorted(source_needed - qualified.keys())}"
        )
    definitions = {}
    for path in sorted((root / "content/items/definitions").glob("items-*.json")):
        for row in json.loads(path.read_bytes())["records"]:
            definition = row["definition"]
            definitions[definition["identity"]["key"]] = definition

    added = []
    for item, observed in qualified.items():
        key = f"{ITEM_PREFIX}{item}"
        definition = definitions[key]
        target = observed["production_binding"]["target"]
        if definition["identity"] != target:
            raise ValueError(f"CLOOT2_ITEM_DEFINITION:{item}")
        current = records.get(key)
        semantics = accepted_first(
            definition.get("semantics", {}), observed["semantics"]
        )
        if current is None:
            current = observed
            current["semantics"] = semantics
            document["records"].append(current)
            added.append(current)
        else:
            if current["production_binding"] != observed["production_binding"]:
                raise ValueError(f"CLOOT2_ITEM_PROFILE_BINDING:{item}")
            current["semantics"] = accepted_first(current["semantics"], semantics)
        current["admission"] = observed["admission"]
        if native_loot_yield_facts(target, current) is None:
            raise ValueError(f"CLOOT2_ITEM_CUSTODY:{item}")
    # Compose the existing source drawing order in the base profile. The
    # bounded SRX-1 supplement is for its dispenser, potion and writing rows.
    house_decoration_actions.apply_top_order(
        added, json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    )
    document["records"].sort(key=order)


def apply_loot_facts(root, tables, facts, profiles):
    from build_full_spell_artifact import native_loot_yield_facts

    for key, replacements in REPLACEMENTS.items():
        for old, new in replacements.items():
            before, after = (
                facts[f"{ITEM_PREFIX}{item}"]["item"] for item in (old, new)
            )
            (entry,) = (
                entry for entry in tables[key]["entries"] if entry["item"] == before
            )
            entry["item"] = after
    for item in sorted(item_ids(root)):
        key = f"{ITEM_PREFIX}{item}"
        rows = profiles.get(key, ())
        admitted = (
            native_loot_yield_facts(facts[key]["item"], rows[0])
            if len(rows) == 1
            else None
        )
        if admitted is None:
            raise ValueError(f"CLOOT2_UNAVAILABLE_LOOT_ITEM:{item}")
        facts[key].update(admitted)
