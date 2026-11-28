"""Qualify the physical Items required by shared quest exchanges.

Uses the existing native source importer and exact canonical identity bindings.
Known canonical facts win. No use effect, subtype or custom script is
inferred; the selected corpse chain retains its exact source timers, and the canonical Item admission records remain unchanged.
"""

import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
import m07_lion_flower_profile as lion_flower
import qmech_m39
import questl2_ready

IDS = {
    21897,
    21898,
    21899,
    21901,
    21902,
    21905,
    21906,
    30168,
    30169,
    37002,
    37003,
    768,
    769,
    770,
    3723,
    15698,
    16103,
    16165,
    16167,
    16242,
    12517,
    12413,
    16128,
    16129,
    20275,
    20276,
    5903,
    5947,
    133,
    139,
    875,
    3244,
    9041,
    10198,
    10199,
    11700,
    11701,
    4867,
    5886,
    5924,
    5937,
    5938,
    5939,
    5940,
    5921,
    6107,
    5945,
    7289,
    6099,
    6100,
    6101,
    6102,
    6126,
    12599,
    12601,
    12803,
    22516,
    22721,
    28792,
    28793,
}
# Knightwatch leader confirmations grant these two shields alongside the documents.
IDS |= {6390, 6432}
# Ferumbras chest: the Rift Tapestry and Teddy need portable reward custody.
IDS |= {22731, 22775}
# M51 arena trophy custody is qualified from the same pinned physical sources.
CUSTODY_PROMOTIONS = {5805, 5806}
CUSTODY_PROMOTION_KEYS = {f"oteryn:item.tibia.i{item}" for item in CUSTODY_PROMOTIONS}
# Closed source qualification admits zero speed only for these three native Items.
SOURCE_KNOWN_SPEED_IDS = frozenset({2833, 5805, 5806})
# Rottin Wood: carried hammer, handcrafted ribbon and rabbit's foot lucky charm.
IDS |= {3460, 12171, 12173}
# The merchant nets are carried; the rare Lucky Loop reward is a hunting horn.
IDS |= {12186, 12260}
# M12 selected daily harvest: existing Juicy Roots identity, exact portable facts.
IDS |= {21291}
# Black Bert sells the source-priced portable Dust of Arthei.
IDS |= {8720}
# Storkus takes stackable lord tokens and grants a portable Vampire Doll.
IDS |= {8191, 8192}
# The Pale Count's reward table closes as a whole, including its other yields.
IDS |= {18935, 18936, 19373, 19374}
# M13: source-pinned carried Guardcatcher and each Perfume Gatherer stage.
IDS |= {17669, 17670, 17672, 17673, 17674, 17675, 7253}
# M13: native Slug corpse closure; Ground-only source durations remain authoritative.
CORPSE_IDS = {12526, 12533, 12534, 12535}
IDS |= CORPSE_IDS
# Retain questpc4's qualified native offerings and source tool through this owner.
IDS |= {3537, 3543, 3587, 3589, 3590, 3593, 11544}
# M36: exact Rock Steady collection hand-ins from the pinned NPC source.
IDS |= {14021, 14022, 14023}
# Vascalir's source-authored mission 11 grants this ordinary flask with 3054.
IDS |= {12785}
# QN4 Cursed Crystal actions_MedusaOil.lua:2-4 uses these plain carried ingredients.
IDS |= {21504, 21505, 21506, 21507, 9106}
# Padreia's vermin poison and Boozer's empty and filled quest bottles.
IDS |= {135, 136, 138}
# M56: exact carried Molten Wax consumption through the existing quest input owner.
IDS |= {9106}
# Crystal 201189f8 Forgotten Knowledge consumes the Thorn Knight's Forbidden Fruit.
IDS |= {24966}
# QN3 Gnomus measurement report (Crystal 201189f gnomus.lua:88-97) grants this jar.
IDS |= {27654}
IDS |= qmech_m39.ITEMS
IDS |= {21504, 21505}
# Lion trials: exact holy-water vessel plus separately qualified one-hour flower duration.
IDS |= {21466, 21389}
# M07 retained Realm key and source bricklayers kit for the assigned repair stages.
IDS |= {14682, 7785}
# Wind-Up Key portable custody, matching the Global item definition and client flags.
IDS |= {37397}
# M07 deterministic siege burning uses the existing Fire Bug as a retained tool.
IDS |= {5467}
# Tower Defence delivers the Watchtower parcel; Troll Sabotage exchanges Jerom's
# family necklace for the Elvenhair Rope through the existing NPC custody owner.
IDS |= {140, 646, 7754}
# M07 Too Hot cleaning, column and floor-repair tools from the same pinned source.
IDS |= {36543, 36544, 36545}
# ENGNPC2: the registered Goblin Merchant Slug and Rotworm exchanges consume
# a gatherer and mint its next variant. Only physical custody is qualified
# here; the exact corpse/live-target callbacks remain the use authority.
IDS |= {17670, 17672, 17673, 17674}
# Rafzan supplies this plain quest bow; capture policy remains a separate owner.
IDS |= {17669}
# Siflind grants one plain paint vial; the World owner controls its use.
IDS |= {7253}
# M22: Star Herb and Antlers already have native owners; qualify only the missing Talisman.
IDS |= {22008}
# M01c Fanfare: copper key is numbered by the existing key MINT owner.
IDS |= {2973}
# Hunter Outfits: the four source music sheets are dropped by cultists and used
# from carried custody to learn their respective verses.
IDS |= {6087, 6088, 6089, 6090}
# M01c Grimvale: harvested blossoms, distinct from either planted nightshade.
IDS |= {22082}
# M01c Stampede: Terrified Elephant drops Tusk Shield in Crystal 201189f8
# (data-global/monster/mammals/terrified_elephant.lua:86). Keep canonical defense 36.
IDS |= {3443}
# M44: source flower harvest and the two remaining native pagoda offerings.
IDS |= {21389, 21466}
IDS |= questl2_ready.ITEMS
# Crystal 201189f8 Explorer Society tools and terminal backpack. The callback
# provider owns the timed sample and filled sample weights, and the corpse
# provider owns the Ground-only butterfly decay chain.
EXPLORER_ITEMS = {4834, 4844, 4845, 4847, 4863, 4872, 9605}
EXPLORER_PROPS = {3874, 4055, 4994}
IDS |= EXPLORER_ITEMS | EXPLORER_PROPS

# Already present in main f5b584ef1's native provider; the existing cohort
# cardinality includes these six shells, even when a source fact is refined.
PREVIOUSLY_COUNTED_IDS = {3029, 3035, 3205, 3212, 3234, 3607}
# Reused from qengine2 5a6de28c1. Tereban's fixed eight source materials use
# the same physical qualifier; XML item type never activates a USE/teleport rule.
SIDE_IDS = (
    {
        133,
        139,
        875,
        3244,
        9041,
        10198,
        10199,
        11700,
        11701,
        12599,
        12601,
        12803,
        19083,
        19151,
        21897,
        22074,
        22082,
        22516,
        22721,
        23986,
        27565,
        28792,
        28793,
        3560,
        5941,
        5886,
    }
    | set(range(11545, 11553))
    | {
        768,
        769,
        770,
        16128,
        16129,
        133,
        4867,
        12803,
        37002,
        37003,
        3723,
        139,
        5903,
        5921,
        5924,
        16165,
        16167,
        3244,
        5937,
        5938,
        20275,
        20276,
        11700,
        11701,
        5939,
        5940,
        5945,
        12599,
        5947,
        12601,
        22721,
        9041,
        15698,
        6099,
        6100,
        6101,
        10198,
        10199,
        30168,
        30169,
        6102,
        6107,
        28793,
        12517,
        16103,
        875,
        6126,
        16242,
        22516,
        28792,
        7289,
        12413,
        5886,
    }
)
SIDE_IDS |= {
    17668,
    17669,
    17670,
    17671,
    17672,
    17673,
    17674,
    17675,
    17676,
    3120,
}  # Rafzan's five source-priced portable tools.
# Bushwhacking scenery is Ground-only; the wave owner schedules removal.
MERCHANT_SCENERY = {12187, 12189}
STATIC_IDS = MERCHANT_SCENERY | {
    5858,  # Arito's placed scimitar.
    7181,  # Arito's opened cave entrance.
    8995,
    7845,  # Crystal Brotherhood lever 3047 creates immovable lava.
    8995,
    8996,
    9527,
    17677,
    17678,
    17679,
    17680,
    17681,
    21573,
    23163,
    32173,
    32537,
}
SIDE_IDS |= STATIC_IDS | {
    3114,
    3456,
    3661,
    32197,
    32225,
    32226,
    32227,
    32228,
    32582,
    32583,
    32584,
    32585,
    32602,
    32757,
}
SIDE_IDS |= {3537, 3543, 3587, 3589, 3590, 3593, 11544}

# Crystal 201189f Jorge (jorge.lua:60-97) and Jean Pierre's upstairs books.
COOKBOOK_IDS = {9093, 11541}
JORGE_REPLICA_IDS = {
    19150,
    19151,
    19152,
    19153,
    19154,
    19155,
    19156,
    22026,
    22027,
    22028,
    23676,
    23677,
    23678,
    23679,
    23680,
    23681,
    23682,
    23683,
    25979,
    25980,
    25981,
    25982,
    28894,
    28895,
    28896,
    28897,
    31265,
    31266,
    31267,
    31268,
    34210,
    37058,
    37059,
    37060,
    43510,
    43511,
    43517,
}
# Source corpse stages for the active Silver Raid Token bosses. The three
# dead-wasp stages also support carried custody in the pinned appearances.
RAID_CORPSE_IDS = {
    5989,
    *range(7282, 7286),
    *range(7320, 7324),
    *range(18118, 18122),
    *range(18262, 18266),
    *range(18970, 18986),
}
WILLI_WASP_DECAY = {4080, 4081, 4082}
RAID_LOOT_IDS = {
    6300,
    7459,
    8044,
    19356,
    19357,
    19361,
    19362,
    19363,
    19365,
    19366,
    19369,
    19371,
    19391,
}
IDS |= COOKBOOK_IDS | JORGE_REPLICA_IDS | RAID_CORPSE_IDS | WILLI_WASP_DECAY | RAID_LOOT_IDS

# Existing Dworc and Merlkin loot yields require portable native custody.
IDS |= {3348, 3403}
IDS |= SIDE_IDS
PROOF = "tools/content-schema/native-gameplay/qengine-reward-item-profiles.json"
PROFILE = "content/items/definitions/spell-native-profiles.json"
XML = "imports/canary/items-xml/items.xml"
XML_SHA = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"


# (Accepted canonical weight, retained source weight).
CANONICAL_WEIGHTS = {
    19361: (150, 850),
    19371: (1700, 170),
    23677: (2700, 200),
    32228: (20, 250),
    34210: (6660, 6659),
}


def reconcile_canonical_physical(item_id, definition, record):
    if item_id not in CANONICAL_WEIGHTS:
        return
    # Preserve the canonical Item weight across the portable-custody admission.
    canonical_weight, donor_weight = CANONICAL_WEIGHTS[item_id]
    known = lambda value: {"state": "KNOWN", "value": value}
    canonical = definition["semantics"]["physical"]["value"]["weight"]
    donor = record["semantics"]["physical"]["value"]["weight"]
    if canonical != known(canonical_weight) or donor != known(donor_weight):
        raise ValueError(f"QENGINE_REWARD_ITEM_WEIGHT_DECISION_DRIFT:{item_id}")
    record["semantics"]["physical"]["value"]["weight"] = canonical.copy()


def reconcile_active_physical(existing, record):
    key = record["production_definition"]["production_key"]
    external_id = record["production_binding"]["external_id"]
    if not external_id.isdecimal() or int(external_id) not in CANONICAL_WEIGHTS:
        return
    canonical_weight, donor_weight = CANONICAL_WEIGHTS[int(external_id)]
    known = lambda value: {"state": "KNOWN", "value": value}
    before = existing["semantics"]["physical"]["value"]["weight"]
    after = record["semantics"]["physical"]["value"]["weight"]
    if before not in (known(donor_weight), known(canonical_weight)) or after != known(
        canonical_weight
    ):
        raise ValueError(f"QENGINE_REWARD_ITEM_ACTIVE_WEIGHT_DRIFT:{key}")
    existing["semantics"]["physical"]["value"]["weight"] = after.copy()


def _source_known_speed(item_id, speed, prior=None):
    if item_id not in SOURCE_KNOWN_SPEED_IDS:
        return None
    if type(speed) is not int or speed != 0:
        raise ValueError("QENGINE_REWARD_ITEM_KNOWN_SPEED_SOURCE")
    previous = prior.get("attributes", {}).get("speed_bonus") if prior is not None else None
    if previous is not None and (type(previous) is not int or previous != speed):
        raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:attributes.speed_bonus")
    return speed


def _known_speed_identity(record):
    definition = record.get("production_definition") or {}
    binding = record.get("production_binding") or {}
    binding_target = binding.get("target") or {}
    item_id = next(
        (
            item_id
            for item_id in SOURCE_KNOWN_SPEED_IDS
            if definition.get("production_key") == f"oteryn:item.tibia.i{item_id}"
            or binding.get("external_id") == str(item_id)
            or binding_target.get("key") == f"oteryn:item.tibia.i{item_id}"
        ),
        None,
    )
    if item_id is None:
        return None
    key = f"oteryn:item.tibia.i{item_id}"
    target = {"family": "Item", "key": key, "revision": "definition-r1"}
    if (
        definition != {"family": "Item", "production_key": key, "revision_ref": "definition-r1"}
        or binding.get("disposition") != "EXACT"
        or binding.get("external_id") != str(item_id)
        or binding.get("identity_namespace") != "ots/item_server_id"
        or binding.get("source_key") != "oteryn:source.crystalserver"
        or binding.get("source_revision") != "ff7ede593c69d4c658b382c97443e8155926924a"
        or binding.get("target") != target
    ):
        raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:known_speed_identity")
    return item_id


def _admitted_known_speed(record, prior):
    item_id = _known_speed_identity(record)
    if item_id is None:
        return None
    if prior is not None:
        for field in ("production_definition", "production_binding"):
            if prior.get(field) != record[field]:
                raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:" + field)
    return _source_known_speed(item_id, record.get("attributes", {}).get("speed_bonus"), prior)


def capture(root=ROOT, ids=None, *, item_ids=None):
    if item_ids is not None:
        if ids is not None:
            raise ValueError("choose ids or item_ids")
        ids = item_ids
    selected_ids = IDS if ids is None else frozenset(ids)
    selected = selected_ids | CUSTODY_PROMOTIONS
    xml_raw = (root / XML).read_bytes()
    client_raw = (root / CLIENT).read_bytes()
    if (
        hashlib.sha256(xml_raw).hexdigest() != XML_SHA
        or hashlib.sha256(client_raw).hexdigest() != CLIENT_SHA
    ):
        raise ValueError("QENGINE_REWARD_ITEM_SOURCE_PIN")
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client_raw)
    physical = {r["server_item_id"]: r for r in source.policies(client_raw, xml_raw, selected)}
    bindings = source.binding_index(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    keys = {f"oteryn:item.tibia.i{i}" for i in selected}
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for path in sorted((root / "content/items/definitions").glob("items-*.json"))
        for r in json.loads(path.read_bytes())["records"]
        if r["definition"]["identity"]["key"] in keys
    }
    if definitions.keys() != keys:
        raise ValueError("QENGINE_REWARD_ITEM_CANONICAL_SET")
    admitted = {
        r["production_definition"]["production_key"]: r
        for r in json.loads((root / PROFILE).read_bytes())["records"]
    }
    records = []
    ground_only_ids = STATIC_IDS | CORPSE_IDS | RAID_CORPSE_IDS | EXPLORER_PROPS
    timed = (
        CORPSE_IDS
        | RAID_CORPSE_IDS
        | WILLI_WASP_DECAY
        | MERCHANT_SCENERY
        | {4837, 6300}
    )
    for item_id in sorted(selected):
        node = xml[item_id]
        attributes = source.attrs(node)
        flower = lion_flower.admitted(item_id, attributes)
        temporal_keys = {"charges", "duration", "decayto", "stopduration"} & attributes.keys()
        if item_id == 17678:
            if {key: attributes[key] for key in temporal_keys} != {
                "duration": "240",
                "decayto": "17677",
            }:
                raise ValueError("QENGINE_REWARD_SMASHED_TRUNK_TIMER_DRIFT")
        elif (
            not flower
            and item_id not in timed
            and temporal_keys
        ):
            raise ValueError("QENGINE_REWARD_ITEM_INSTANCE_OWNER_REQUIRED")
        flags = objects[item_id]["flags"]
        if flags.get("flags.liquidcontainer") or flags.get("flags.liquidpool"):
            raise ValueError("QENGINE_REWARD_ITEM_FLUID_OWNER_REQUIRED")
        record = source.profile(item_id, node, objects[item_id], physical[item_id], bindings, None)
        known_speed = _source_known_speed(
            item_id,
            record["attributes"].get("speed_bonus"),
            admitted.get(f"oteryn:item.tibia.i{item_id}"),
        )
        definition = definitions[f"oteryn:item.tibia.i{item_id}"]
        reconcile_canonical_physical(item_id, definition, record)
        if item_id == 6300:
            # The canonical Death Ring has a 450-second equipped budget.
            record["semantics"]["temporal"] = copy.deepcopy(definition["semantics"]["temporal"])
        if item_id in COOKBOOK_IDS:
            # Crystal's explicit cookbook actions grant read-only text. The
            # native literal inscription owner measures up to 4000 characters.
            known = lambda value: {"state": "KNOWN", "value": value}
            record["semantics"]["readable_writeable"] = known(
                {
                    "readable": known(True),
                    "writeable": known(False),
                    "distance_read": known(False),
                    "max_text_length": known(4000),
                    "write_once_target": {"state": "UNKNOWN"},
                }
            )
        if flower:
            record = lion_flower.temporal(record, definition, accepted.known_conflict)
        # These exact Ground-only props and corpse stages have canonical
        # identity-only rows; their physical facts come from the pinned sources.
        ground_only = item_id in ground_only_ids
        identity_only = ground_only or item_id in WILLI_WASP_DECAY
        semantics = definition.get("semantics") if identity_only else definition["semantics"]
        if identity_only and (
            definition.get("kind") != "Item" or definition["identity"]["family"] != "Item"
        ):
            raise ValueError("QENGINE_REWARD_STATIC_IDENTITY")
        for group in ("physical", "stack", "container", "temporal"):
            if semantics is not None and accepted.known_conflict(
                semantics[group], record["semantics"][group]
            ):
                raise ValueError(
                    "QENGINE_REWARD_ITEM_CANONICAL_CONFLICT:" + group + ":" + str(item_id)
                )
        if definition["stack_class"] not in (
            "Unknown",
            record["admission"]["stack_class"],
        ):
            raise ValueError("QENGINE_REWARD_ITEM_STACK_CONFLICT")
        expected_destinations = (
            ["Ground"] if ground_only else ["CharacterInventory", "Ground"]
        )
        if record["admission"]["legal_destinations"] != expected_destinations:
            raise ValueError("QENGINE_REWARD_ITEM_DESTINATION_DRIFT")
        if item_id in STATIC_IDS and record["admission"]["stack_class"] != "NonStackable":
            raise ValueError("QENGINE_REWARD_STATIC_STACK_DRIFT")
        if semantics is not None:
            record["semantics"] = accepted.accepted_first(semantics, record["semantics"])
        prior = admitted.get(f"oteryn:item.tibia.i{item_id}")
        if prior is not None:
            refined = accepted.accepted_first(prior["semantics"], record["semantics"])
            record["semantics"] = (
                accepted.accepted_first(semantics, refined) if semantics is not None else refined
            )
            record["attributes"] = accepted.accepted_first(
                prior["attributes"], record["attributes"]
            )
            record["authoring"] = copy.deepcopy(prior["authoring"])
            # The proof and native admission share the same custody boundary:
            # physical qualification cannot invent an owner's unknown mechanic.
            import questpc7_physical_custody_join

            record = questpc7_physical_custody_join.observation(
                prior, record, accepted.accepted_first
            )
        if known_speed is not None:
            record["attributes"]["speed_bonus"] = known_speed
        records.append(record)
    records = source.bind_profiles(
        {"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": records},
        bindings,
        selected,
    )["records"]
    return {
        "schema": "OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1",
        "producer": "tools/content-migration/qengine_reward_item_profiles.py",
        "sources": [
            {
                "path": XML,
                "revision": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
                "sha256": XML_SHA,
            },
            {"path": CLIENT, "sha256": CLIENT_SHA},
        ],
        "scope": "Source-qualified portable quest custody, fixed Ground-only props and exact source corpse timers; no custom use or script activation.",
        "records": [
            r for r in records if int(r["production_binding"]["external_id"]) in selected_ids
        ],
        "custody_promotions": [
            r for r in records if int(r["production_binding"]["external_id"]) in CUSTODY_PROMOTIONS
        ],
    }


def reconcile(existing, observed):
    """Join pinned physical facts without replacing an established owner's identity."""
    # Only these two formerly Ground-only trophies are promoted, and only to
    # the exact portable facts captured above. No identity, use or text owner changes.
    key = existing.get("production_definition", {}).get("production_key")
    old_custody = {
        "materializable": False,
        "stack_class": "NonStackable",
        "legal_destinations": ["Ground"],
    }
    portable = {
        "materializable": True,
        "stack_class": "NonStackable",
        "legal_destinations": ["CharacterInventory", "Ground"],
    }
    if (
        key in CUSTODY_PROMOTION_KEYS
        and existing.get("admission") == old_custody
        and observed.get("admission") == portable
    ):
        existing = dict(existing, admission=portable)
    for field in ("production_definition", "production_binding", "admission"):
        if existing.get(field) != observed.get(field):
            raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:" + field)
    merged = dict(existing)
    semantics = accepted.accepted_first(existing["semantics"], observed["semantics"])
    if accepted.known_conflict(existing["semantics"], semantics) or accepted.known_conflict(
        observed["semantics"], semantics
    ):
        raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:semantics")
    merged["semantics"] = {
        field: value
        for field, value in semantics.items()
        if field in existing["semantics"] or value != {"state": "UNKNOWN"}
    }
    attributes = dict(existing.get("attributes", {}))
    compact_attributes = {"field_condition", "field_replaceable", "rune_consumption"}
    for key, value in observed.get("attributes", {}).items():
        if key in compact_attributes and key not in attributes and value is None:
            continue
        previous = attributes.get(key)
        if previous is not None and value is not None and previous != value:
            raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:attributes." + key)
        if previous is None:
            attributes[key] = value
    merged["attributes"] = attributes
    optional = {"production_binding_qualification"}
    if existing.keys() - optional != observed.keys() - optional:
        raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:shape")
    identity_fields = (existing.keys() | observed.keys()) - {"authoring", "attributes", "semantics"}
    for key in identity_fields:
        if existing.get(key) != observed.get(key):
            raise ValueError("QENGINE_REWARD_ITEM_NATIVE_CONFLICT:" + key)
    return merged


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    generated[PROOF] = (json.dumps(packet, ensure_ascii=False, indent=2) + "\n").encode()
    admit_records(
        root,
        generated,
        profile_path,
        manifest_path,
        order,
        canonical,
        packet["records"] + packet["custody_promotions"],
    )

    import s06_golden_box_profile

    s06_golden_box_profile.admit(root, generated, profile_path, manifest_path, order, canonical)


def admit_records(root, generated, profile_path, manifest_path, order, canonical, records):
    """Merge qualified physical profiles while preserving existing native owners."""
    document = json.loads(generated[profile_path])
    for existing in document["records"]:
        _known_speed_identity(existing)
    present = {r["production_definition"]["production_key"]: r for r in document["records"]}
    for record in records:
        key = record["production_definition"]["production_key"]
        known_speed = _admitted_known_speed(record, present.get(key))
        if key in present:
            prior = present[key]
            if key in CUSTODY_PROMOTION_KEYS and "readable_writeable" in prior["semantics"]:
                # The text owner retains an unresolved write-once target for these trophies.
                record = {
                    **record,
                    "semantics": {
                        **record["semantics"],
                        "readable_writeable": prior["semantics"]["readable_writeable"],
                    },
                }
            reconcile_active_physical(prior, record)
            inert = prior["admission"] == {
                "materializable": False,
                "stack_class": record["admission"]["stack_class"],
                "legal_destinations": ["Ground"],
            }
            if (
                inert
                and prior["production_definition"] == record["production_definition"]
                and accepted.accepted_first(prior["semantics"], record["semantics"])
                == record["semantics"]
            ):
                document["records"][document["records"].index(prior)] = record
                continue
            import questpc7_quest_item_reconcile

            questpc7_quest_item_reconcile.preserve(prior, record)
            import questpc7_physical_custody_join

            physical = questpc7_physical_custody_join.observation(
                prior, record, accepted.accepted_first
            )
            if known_speed is not None:
                physical["attributes"]["speed_bonus"] = known_speed
            merged = reconcile(prior, physical)
            present[key].clear()
            present[key].update(merged)
        else:
            document["records"].append(record)
    document["records"].sort(key=order)
    raw = (
        json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    # Ordinary native regeneration must retain the promoted Quest cosmetics
    # section instead of falling back to a manifest with no appearance owner.
    cosmetics = "cosmetics/outfits/native-cosmetics.json"
    cosmetics_raw = generated.get("content/" + cosmetics)
    if cosmetics_raw is None and (root / "content" / cosmetics).exists():
        cosmetics_raw = (root / "content" / cosmetics).read_bytes()
    if cosmetics_raw is not None:
        if manifest["schema"] in {f"OTERYN_NATIVE_GAMEPLAY_MANIFEST/v{i}" for i in range(1, 6)}:
            manifest["schema"] = "OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6"
        manifest["cosmetics"] = {
            "path": cosmetics,
            "sha256": hashlib.sha256(cosmetics_raw).hexdigest(),
        }
    generated[manifest_path] = canonical(manifest)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = json.dumps(capture(), ensure_ascii=False, indent=2) + "\n"
    path = ROOT / PROOF
    if args.check:
        if path.read_text() != expected:
            raise SystemExit("QENGINE reward source capture is stale")
    else:
        path.write_text(expected)
