"""Portable Item admissions for the first 323 Creature IDs from the 3e099164 loot pin."""

import hashlib
import json
from pathlib import Path

from grizzly_shop_profiles import accepted_first

NATIVE = Path("tools/content-schema/native-gameplay")
COHORT = NATIVE / "loot-slice-1-creatures.txt"
XML = NATIVE / "loot-slice-1-crystal-items.xml"
SPIRITED_SOIL = NATIVE / "loot-slice-1-spirited-soil-binding.json"
GLITTERSCALE = "oteryn:creature.glitterscale"
GLITTERSCALE_BODY = 11589
CORPSE_ROOTS = {5972, GLITTERSCALE_BODY, 22742}


def creatures(root):
    cohort = (root / COHORT).read_text().splitlines()
    if len(cohort) != 323 or sorted(set(cohort)) != cohort:
        raise ValueError("Invalid first loot slice")
    return set(cohort) | {GLITTERSCALE}


# Crystal201189f8 corrupted_ghost.lua, corrupted_skeleton.lua and dharalion.lua
# each declare one roll for the named Item. The stale numeric IDs resolve to
# scenery or earth. Replace their alternate-source duplicates with the donor roll.
ITEM_ROLLS = (
    ("corrupted_ghost", 2643, 3565, 30_400),
    ("corrupted_ghost", 7618, 266, 213_900),
    ("corrupted_ghost", 7620, 268, 200_100),
    ("corrupted_skeleton", 7618, 266, 262_800),
    ("dharalion", 3257, 3103, 79_200),
)


def project_tables(definitions, tables, facts):
    for creature, stale, portable, chance in ITEM_ROLLS:
        reference = definitions[f"oteryn:creature.{creature}"]["loot"]
        table = tables[reference["key"]]
        stale_item = facts[f"oteryn:item.tibia.i{stale}"]["item"]
        item = facts[f"oteryn:item.tibia.i{portable}"]["item"]
        (entry,) = [row for row in table["entries"] if row["item"] == stale_item]
        table["entries"] = [row for row in table["entries"] if row["item"] != item]
        entry.update(item=item, probability_ppm=chance)

    # Crystal201189f8 data-global/monster/mammals/clomp.lua:86.
    clomp = tables[definitions["oteryn:creature.clomp"]["loot"]["key"]]
    (shield,) = [
        row
        for row in clomp["entries"]
        if row["item"]["key"] == "oteryn:item.tibia.i3443"
    ]
    shield["probability_ppm"] = 4_000

    # Crystal201189f8 data-global/monster/bosses/glitterscale.lua:70–72.
    definition = definitions[GLITTERSCALE]
    key = "oteryn:loot.creature.glitterscale"
    if definition["loot"] is not None or key in tables:
        raise ValueError("Glitterscale already has canonical loot")
    reference = {"family": "Loot", "key": key, "revision": "definition-r1"}
    definition["loot"] = reference
    tables[key] = {
        "identity": reference,
        "algorithm": "IndependentBernoulliPpm",
        "entries": [
            {
                "item": facts["oteryn:item.tibia.i11550"]["item"],
                "min_count": 1,
                "max_count": 1,
                "probability_ppm": 700_000,
            },
        ],
    }


def corpse_profiles(root, source, raw_xml, client, appearances, bindings):
    """Qualify the exact death bodies and every source decay target."""
    import quest_corpse_profiles as corpses

    xml = source.xml_index(raw_xml)
    selected = corpses.decay_closure(CORPSE_ROOTS, xml)
    physics = {
        row["server_item_id"]: row for row in source.policies(client, raw_xml, selected)
    }
    definitions = corpses.definitions(root, selected)
    worlds = corpses.world_objects(root, selected)
    for item in sorted(selected):
        definition = definitions[f"oteryn:item.tibia.i{item}"]
        if item != GLITTERSCALE_BODY:
            yield corpses.qualify(
                item,
                definition,
                worlds.get(item),
                xml[item],
                appearances[item],
                physics[item],
                bindings,
                item in CORPSE_ROOTS,
            )
            continue
        # The exact Glitterscale Lua uses this fixed appearance as its body.
        # Personal rewards allow the body to retain its source non-container form.
        record = source.profile(
            item, xml[item], appearances[item], physics[item], bindings, None
        )
        world = worlds[item]
        if (
            world["provenance"]["item_pointer"] != definition["identity"]
            or world["client_projection"] != source.K({"appearance_id": item})
            or world["movable"] != source.K(False)
            or record["semantics"]["container"] != source.NA
            or record["admission"]["legal_destinations"] != ["Ground"]
        ):
            raise ValueError("GLITTERSCALE_PLAIN_BODY_SOURCE")
        yield from source.bind_profiles(
            {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": [record]},
            bindings,
            {item},
        )["records"]


def admit_native(root, generated, profile_path, starter_path, order, canonical):
    """Reuse existing carried custody and fill missing facts from exact Item sources."""
    import encounter_loot_item_admissions as donor
    import house_decoration_actions
    from build_full_spell_artifact import native_loot_yield_facts
    from writing_item_admission import ids as writing_ids

    source = donor.source
    client = (root / donor.CLIENT).read_bytes()
    canary_xml = (root / donor.XML).read_bytes()
    if (
        hashlib.sha256(client).hexdigest() != donor.CLIENT_SHA
        or hashlib.sha256(canary_xml).hexdigest() != donor.XML_SHA
    ):
        raise ValueError("Loot slice Item source digest mismatch")
    raw_xml = (root / XML).read_bytes()
    xml = source.xml_index(raw_xml)
    soil_binding = json.loads((root / SPIRITED_SOIL).read_bytes())
    soil = int(soil_binding["external_id"])
    # Crystal201189f8 has the exact appearance and monster loot name but no XML
    # override. Canary's retained XML and TibiaWiki r1181885 agree on 2.50 oz.
    xml[soil] = source.xml_index(canary_xml)[soil]
    identities = source.binding_index((root / donor.BINDINGS).read_bytes())
    if str(soil) in identities:
        raise ValueError("Spirited Soil binding already exists")
    identities[str(soil)] = soil_binding
    # SRX-1 owns the source-qualified writing attributes. Letter 3505 already
    # has its writing profile in the base provider, but needs the donor's loot
    # facts there too. The merge below keeps its accepted writing semantics.
    selected = set(xml) - (set(writing_ids(root)) - {3505})
    appearances = source.engine_items.load_appearance_objects(client)
    physics = {
        row["server_item_id"]: row for row in source.policies(client, raw_xml, selected)
    }
    keys = {identities[str(item)]["target"]["key"] for item in selected}
    definitions = {
        row["definition"]["identity"]["key"]: row["definition"]
        for path in (root / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    }
    top_order = json.loads((root / house_decoration_actions.PROFILES).read_bytes())
    document = json.loads(generated[profile_path])
    existing = {
        row["production_definition"]["production_key"]: row
        for row in document["records"]
    }
    starters = {
        row["production_definition"]["production_key"]: row
        for row in json.loads(generated[starter_path])["records"]
    }
    for item in sorted(selected):
        binding = identities[str(item)]
        target = binding["target"]
        key = target["key"]
        definition = definitions[key]
        if definition["identity"] != target:
            raise ValueError(f"Loot slice Item identity mismatch: {item}")
        if key in starters:
            if native_loot_yield_facts(target, starters[key]) is None:
                raise ValueError(f"Starting loadout loot Item is not portable: {item}")
            continue
        prior = existing.get(key)
        if prior is not None and native_loot_yield_facts(target, prior) is not None:
            continue
        observed = source.profile(
            item, xml[item], appearances[item], physics[item], identities, None
        )
        observed.update(
            production_binding=binding,
            production_binding_qualification=None,
            production_definition={
                "family": "Item",
                "production_key": key,
                "revision_ref": target["revision"],
            },
        )
        observed["semantics"] = accepted_first(
            definition.get("semantics", {}), observed["semantics"]
        )
        if prior is not None:
            if (
                prior["production_binding"] != binding
                or prior["admission"]["stack_class"]
                != observed["admission"]["stack_class"]
            ):
                raise ValueError(f"Loot slice native Item conflict: {item}")
            observed["authoring"] = prior["authoring"]
            observed["semantics"] = accepted_first(
                prior["semantics"], observed["semantics"]
            )
            if "speed_bonus" not in prior["attributes"] and "speed" not in source.attrs(
                xml[item]
            ):
                # Loot custody preserves the accepted absence of equipment speed.
                observed["attributes"].pop("speed_bonus")
            observed["attributes"].update(prior["attributes"])
        if item == 6300:
            # Crystal and Canary agree on the death ring's 480-second budget.
            # Keep the accepted active-time mode and paired forms.
            observed["semantics"]["temporal"]["value"]["duration"] = source.K(
                int(source.attrs(xml[item])["duration"]) * 1_000
            )
        if native_loot_yield_facts(target, observed) is None:
            raise ValueError(f"Loot slice Item is not portable: {item}")
        house_decoration_actions.apply_top_order([observed], top_order)
        if prior is None:
            document["records"].append(observed)
        else:
            prior.update(observed)
    for body in corpse_profiles(
        root, source, canary_xml, client, appearances, identities
    ):
        key = body["production_definition"]["production_key"]
        house_decoration_actions.apply_top_order([body], top_order)
        prior = existing.get(key)
        if prior is None:
            document["records"].append(body)
        elif prior != body:
            raise ValueError(
                f"Loot slice corpse conflicts with its native owner: {key}"
            )
    document["records"].sort(key=order)
    generated[profile_path] = canonical(document)
