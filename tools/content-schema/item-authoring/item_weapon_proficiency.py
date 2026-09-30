#!/usr/bin/env python3
"""ITEM-PROF-1: join the 15.30 client weapon proficiency staging to Item keys.

Inputs are the digest-pinned client files (`content/assets/files/proficiencies-7fea90ec...json`
and `appearances-2dfa943b...dat`, read through their committed staging under
`imports/cipsoft-staticdata/{proficiencies,weapon-proficiency-bindings}/`, produced by
`tools/content-census/stage_proficiencies.py`). Every staged shard is checked against its
manifest SHA-256 and the manifests against the pins below before any record is read.

Output (`samples/item-weapon-proficiency-15-30-7fea90ec.json`) is a normalized staging
artifact: the 443 client profiles with their raw level/perk records, the 666 weapon bindings
keyed `oteryn:item.tibia.i<client id>`, and the threshold class of each profile.
Perk `Type`, `SkillId`, `AugmentType`, `ElementId` and `DamageType` stay raw values in the
profiles; the owner-accepted D199 meaning and unit table is emitted beside them as
`perk_mapping`. The threshold class is keyed per binding (item), never per profile: bolt
ammunition means crossbow (D198), Knight applies only to knight-restricted sword/axe/club
weapons (D200), and any binding without in-repo evidence is `unknown`, never guessed.
Nothing is an Oteryn gameplay definition. `--check` diffs an in-memory regeneration.

ITEM-PROF-1b: the weapon vocation of each binding comes, in source-policy order, from the
pinned TibiaWiki stat snapshot (`imports/tibiawiki/facts/items-stats.json`, `vocrequired`,
read by numeric item id; PROVEN), else from the Crystal ff7ede5 `items.xml` weapon
`vocation` attribute staged by `--stage-crystal` into
`imports/crystalserver/facts/items-weapon-vocations.json` (DERIVED hypothesis; a weapon node
without the attribute is `unrestricted`), else it is UNKNOWN with a reason.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STAGING = ROOT / "imports" / "cipsoft-staticdata"
ITEM_DEFINITIONS = ROOT / "content" / "items" / "definitions"
DEFAULT_OUTPUT = (
    Path(__file__).resolve().parent
    / "samples"
    / ("item-weapon-proficiency-15-30-7fea90ec.json")
)

SCHEMA = "OTERYN_ITEM_WEAPON_PROFICIENCY_STAGING/v1"
PROFICIENCIES_SHA256 = (
    "7fea90ec1cfd472f4b5978f4456b430d3271598b4e545b7919d692641411e015"
)
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
EXPECTED_PROFILES = 443
EXPECTED_BINDINGS = 666

# In-repo per-item evidence. Both files are digest-pinned before use.
EVIDENCE = ROOT / "docs" / "agents" / "evidence"
CRYSTAL_CATALOG = (
    EVIDENCE / "OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
)
CRYSTAL_CATALOG_SHA256 = (
    "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7"
)
# TibiaWiki stat snapshot (ITEM-SEM-2a). Read by numeric item id, not by record key, and
# not byte-pinned: its record keys and snapshot digest may change on a re-key. A value
# change still shows up as a `--check` difference in the regenerated artifact.
WIKI_STATS = ROOT / "imports" / "tibiawiki" / "facts" / "items-stats.json"
WIKI_STATS_SCHEMA = "OTERYN_ITEM_WIKI_STATS_SNAPSHOT/v1"
# Crystal items.xml weapon vocation facts, staged by `--stage-crystal` (ITEM-PROF-1b).
CRYSTAL_ITEMS_XML = {
    "repository": "zimbadev/crystalserver",
    "revision": "ff7ede593c69d4c658b382c97443e8155926924a",
    "path": "data/items/items.xml",
    "sha256": "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb",
}
CRYSTAL_VOCATIONS = (
    ROOT / "imports" / "crystalserver" / "facts" / "items-weapon-vocations.json"
)
CRYSTAL_VOCATIONS_SCHEMA = "OTERYN_CRYSTAL_ITEM_WEAPON_VOCATION_FACTS/v1"
CRYSTAL_VOCATIONS_SHA256 = (
    "048572bc4eab032b6c274fadf4c87cbe9256e43917c2040a7c5a2a866eb65fbe"
)

# Owner-accepted evidence: #162 comments 5905884086 (table) and 5905899852 (decision).
THRESHOLD_SOURCE = {
    "url": "https://tibia.fandom.com/wiki/Weapon_Proficiency",
    "revid": 1192598,
    "fetched": "2026-09-30",
    "acceptance": "owner decision #162 5905899852",
}
# Progress needed for levels 1-7 and Mastery levels 8-9.
THRESHOLDS = {
    "standard": [
        1750,
        25000,
        100000,
        400000,
        2000000,
        8000000,
        30000000,
        60000000,
        90000000,
    ],
    "knight": [
        1250,
        20000,
        80000,
        300000,
        1500000,
        6000000,
        20000000,
        40000000,
        60000000,
    ],
    "crossbow": [
        600,
        8000,
        30000,
        150000,
        650000,
        2500000,
        10000000,
        20000000,
        30000000,
    ],
}
MASTERY_OFFSET = 2  # Mastery is reached at the weapon's top level + 2.
THRESHOLD_DECISIONS = {
    "D198": "bolt ammunition (Crystal ff7ede5 items.xml ammotype=bolt, in-repo catalog "
    "OTV2-20260919 cw2-b1; TibiaWiki secondarytype=Crossbows) selects the Crossbow table "
    "per binding, including shared bow/crossbow profiles",
    "D198_wiki": "a ranged-family binding without ammotype uses TibiaWiki secondarytype: "
    "Crossbows = crossbow, Bows = standard",
    "D200": "Knight table only for knight-restricted sword/axe/club weapons (Canary "
    "getExperienceArray: weapon vocation includes knight); every other melee weapon, "
    "including fist (D197), is standard",
    "vocation": "weapon vocation per binding: TibiaWiki items-stats vocrequired by item id "
    "(PROVEN), else Crystal ff7ede5 items.xml weapon vocation attribute, a weapon node "
    "without one being unrestricted (DERIVED), else UNKNOWN",
    "unknown": "no per-item evidence (no ammotype or wiki secondarytype for a "
    "ranged-family profile, UNKNOWN vocation for a sword/axe/club): class stays unknown, "
    "never guessed",
}
CLASSES = ("crossbow", "knight", "standard", "unknown")
KNIGHT_WORDS = {"Sword", "Axe", "Club"}
RANGED_WORDS = {"Bow", "Crossbow", "Distance", "Arbalest"}

# D199 (owner, #162 2026-09-30): TibiaWiki Weapon_Proficiency_Tables revid 1206177
# (cross-match 3671/3671) and Canary src/enums/weapon_proficiency.hpp.
PERK_MAPPING_SOURCE = {
    "decision": "D199",
    "tibiawiki": "Weapon_Proficiency_Tables revid 1206177 (3671/3671 perks cross-matched)",
    "canary": "opentibiabr/canary src/enums/weapon_proficiency.hpp",
    "type_minus_1": "dropped (not present in the 15.30 file)",
}
PERK_TYPES = {
    0: ("attack", "flat"),
    1: ("defence", "flat"),
    2: ("weapon shield defence modifier", "flat"),
    3: ("combat skill (SkillId)", "flat"),
    4: ("specialised magic level (DamageType)", "flat"),
    5: ("spell augmentation (AugmentType, SpellId)", "fraction"),
    6: ("damage against bestiary family (BestiaryId)", "fraction"),
    7: ("damage against bosses and Sinister Embraced", "fraction"),
    8: ("critical hit chance", "fraction"),
    9: ("elemental critical hit chance (ElementId)", "fraction"),
    10: ("offensive rune critical hit chance", "fraction"),
    11: ("auto-attack critical hit chance", "fraction"),
    12: ("critical extra damage", "fraction"),
    13: ("elemental critical extra damage (ElementId)", "fraction"),
    14: ("offensive rune critical extra damage", "fraction"),
    15: ("auto-attack critical extra damage", "fraction"),
    16: ("mana leech", "fraction"),
    17: ("life leech", "fraction"),
    18: ("mana on hit", "flat"),
    19: ("hit points on hit", "flat"),
    20: ("mana on kill", "flat"),
    21: ("hit points on kill", "flat"),
    22: ("damage at range (Value is a distance in tiles)", "flat_tiles"),
    23: ("ranged hit chance", "fraction"),
    24: ("attack range", "flat"),
    25: ("skill-scaled extra damage for auto-attacks (SkillId)", "fraction"),
    26: ("skill-scaled extra damage for spells (SkillId)", "fraction"),
    27: ("skill-scaled extra healing (SkillId)", "fraction"),
    28: ("Alpha Strike: extra damage against targets above 95% HP", "fraction"),
    29: ("Omega Strike: extra damage against targets below 30% HP", "fraction"),
    30: ("armor penetration (1.0 = +100%)", "fraction"),
    31: ("elemental pierce (ElementId)", "fraction"),
    32: (
        "homing missile (ElementId, MissileId; Probability and Multiplier fractions)",
        "fraction",
    ),
}
PERK_UNIT_RULES = (
    "Value unit: flat for Types 0-4, 18-22 and 24 (Type 22 in tiles); fraction (x100 = %) "
    "for Types 6-17, 23 and 25-31, and for Type 5 except AugmentType 6 (negative seconds); "
    "Type 32 carries Probability and Multiplier fractions"
)
SKILL_IDS = {
    1: "Magic Level",
    6: "Shielding",
    7: "Distance",
    8: "Sword",
    9: "Club",
    10: "Axe",
    11: "Fist Fighting",
    13: "Fishing",
}
AUGMENT_TYPES = {
    2: "spell base damage",
    3: "spell healing",
    6: "spell cooldown (negative seconds)",
    14: "spell life leech",
    15: "spell mana leech",
    16: "spell critical extra damage",
    17: "spell critical hit chance",
}
COMBAT_TYPES = {
    1: "physical",
    8: "fire",
    16: "earth",
    32: "energy",
    64: "ice",
    128: "holy",
    256: "death",
    1048576: "healing",
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_family(directory: Path, family: str, expected: int) -> tuple[dict, list[dict]]:
    """Load one staged family, verifying the manifest and every shard digest first."""
    manifest_path = directory / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("family") != family:
        raise SystemExit(
            f"{manifest_path}: family {manifest.get('family')!r} != {family!r}"
        )
    records: list[dict] = []
    for entry in manifest["files"]:
        raw = (directory / entry["path"]).read_bytes()
        if sha256_bytes(raw) != entry["sha256"]:
            raise SystemExit(
                f"{directory / entry['path']}: digest differs from the manifest"
            )
        shard = json.loads(raw)
        if len(shard["records"]) != entry["records"]:
            raise SystemExit(f"{entry['path']}: record count differs from the manifest")
        records.extend(shard["records"])
    if len(records) != expected or manifest["record_count"] != expected:
        raise SystemExit(f"{family}: expected {expected} records, found {len(records)}")
    return manifest, records


def verify_pins(proficiency_manifest: dict, binding_manifest: dict) -> None:
    pins = {
        "proficiencies": PROFICIENCIES_SHA256,
        "appearances": APPEARANCES_SHA256,
    }
    for manifest in (proficiency_manifest, binding_manifest):
        for name, pin in pins.items():
            declared = manifest.get("inputs", {}).get(name, {}).get("sha256")
            if declared is not None and declared != pin:
                raise SystemExit(
                    f"manifest {name} digest {declared} is not the pin {pin}"
                )
    if binding_manifest["inputs"]["proficiencies"]["sha256"] != PROFICIENCIES_SHA256:
        raise SystemExit("binding manifest is not pinned to the proficiency file")
    # The client files are committed under content/assets/files; verify them when present.
    for manifest in (binding_manifest,):
        for name in ("appearances", "proficiencies"):
            path = ROOT / manifest["inputs"][name]["file"]
            if (
                path.is_file()
                and sha256_bytes(path.read_bytes())
                != manifest["inputs"][name]["sha256"]
            ):
                raise SystemExit(f"{path}: digest differs from its pin")


def read_pinned_json(path: Path, sha256: str):
    raw = path.read_bytes()
    if sha256_bytes(raw) != sha256:
        raise SystemExit(f"{path}: digest differs from the pin {sha256}")
    return json.loads(raw)


def crystal_ammotypes() -> dict[int, str | None]:
    """Crystal ff7ede5 items.xml ammotype per source item id (None = record without one)."""
    catalog = read_pinned_json(CRYSTAL_CATALOG, CRYSTAL_CATALOG_SHA256)[
        "semantic_catalog"
    ]
    nodes = {}
    for node in catalog["semantic_candidate_node_records"]:
        nodes[node["source_node_digest"]] = next(
            (
                f["source_value"]
                for f in node["candidate_fields"]
                if f["source_key"] == "ammotype"
            ),
            None,
        )
    return {
        record["source_item_id"]: nodes.get(record["source_node_digest"])
        for record in catalog["identity_records"]
    }


def xml_item_ids(item: ET.Element) -> list[int]:
    if "id" in item.attrib:
        return [int(item.attrib["id"])]
    if "fromid" in item.attrib:
        return list(range(int(item.attrib["fromid"]), int(item.attrib["toid"]) + 1))
    return []


def stage_crystal(xml_path: Path, ids: set[int]) -> dict:
    """Extract the weapon vocation facts of the binding ids from the pinned items.xml."""
    raw = xml_path.read_bytes()
    if sha256_bytes(raw) != CRYSTAL_ITEMS_XML["sha256"]:
        raise SystemExit(f"{xml_path}: digest differs from {CRYSTAL_ITEMS_XML}")
    records: dict[str, dict] = {}
    for item in ET.fromstring(raw).iter("item"):
        attributes = list(item.iter("attribute"))
        keys = [
            (a.attrib.get("key", "").lower(), a.attrib.get("value", ""))
            for a in attributes
        ]
        fact = {
            "name": item.attrib.get("name"),
            "weapon": any(
                k == "weapontype" or (k == "script" and "weapon" in v) for k, v in keys
            ),
            "vocation": [v for k, v in keys if k == "vocation"],
        }
        for item_id in xml_item_ids(item):
            if item_id in ids:
                if str(item_id) in records:
                    raise SystemExit(f"{xml_path}: item {item_id} listed twice")
                records[str(item_id)] = fact
    return {
        "schema": CRYSTAL_VOCATIONS_SCHEMA,
        "evidence": "OtsHypothesisOnly",
        "source": CRYSTAL_ITEMS_XML,
        "extracted_by": "tools/content-schema/item-authoring/item_weapon_proficiency.py "
        "--stage-crystal",
        "scope": "the client 15.30 weapon proficiency binding ids",
        "absent_ids": sorted(i for i in ids if str(i) not in records),
        "records": dict(sorted(records.items(), key=lambda kv: int(kv[0]))),
    }


def render_facts(document: dict) -> str:
    head = {k: v for k, v in document.items() if k != "records"}
    lines = ["{"]
    for key in sorted(head):
        lines.append(f"  {json.dumps(key)}: {json.dumps(head[key], sort_keys=True)},")
    lines.append('  "records": {')
    rows = [
        f"    {json.dumps(k)}: {json.dumps(v, sort_keys=True)}"
        for k, v in document["records"].items()
    ]
    lines.append(",\n".join(rows))
    lines.append("  }")
    lines.append("}")
    return "\n".join(lines) + "\n"


def crystal_vocations() -> dict[int, dict]:
    document = read_pinned_json(CRYSTAL_VOCATIONS, CRYSTAL_VOCATIONS_SHA256)
    if (
        document["schema"] != CRYSTAL_VOCATIONS_SCHEMA
        or document["source"] != CRYSTAL_ITEMS_XML
    ):
        raise SystemExit(f"{CRYSTAL_VOCATIONS}: schema or source pin differs")
    return {int(k): v for k, v in document["records"].items()}


def wiki_stats() -> tuple[dict, dict[int, dict[str, list[str]]]]:
    """TibiaWiki stat fields per numeric item id: field -> distinct observed values."""
    document = json.loads(WIKI_STATS.read_text(encoding="utf-8"))
    if document["schema"] != WIKI_STATS_SCHEMA:
        raise SystemExit(f"{WIKI_STATS}: schema {document['schema']!r}")
    out: dict[int, dict[str, list[str]]] = {}
    for key, record in document["records"].items():
        item_id = int(record.get("item_id", key.rsplit(".i", 1)[-1]))
        fields = out.setdefault(item_id, {})
        for observation in record["observations"]:
            for name in ("vocrequired", "secondarytype"):
                value = observation["fields"].get(name)
                if value and value not in fields.setdefault(name, []):
                    fields[name].append(value)
    meta = {
        "path": str(WIKI_STATS.relative_to(ROOT)),
        "batch_id": document["batch_id"],
        "read_by": "numeric item_id (robust to record re-keying)",
    }
    return meta, out


def single(values: list[str] | None) -> str | None:
    return values[0] if values and len(values) == 1 else None


def vocation_evidence(wiki: dict[str, list[str]], crystal: dict | None) -> dict:
    """PROVEN (TibiaWiki), DERIVED (Crystal items.xml) or UNKNOWN weapon vocation."""
    values = wiki.get("vocrequired", [])
    if len(values) == 1:
        return {"basis": "PROVEN", "source": "tibiawiki", "value": values[0]}
    if len(values) > 1:
        return {
            "basis": "UNKNOWN",
            "reason": f"TibiaWiki observations disagree: {values}",
        }
    if crystal is None:
        return {
            "basis": "UNKNOWN",
            "reason": "no TibiaWiki vocrequired; not in Crystal items.xml",
        }
    if crystal["vocation"]:
        return {
            "basis": "DERIVED",
            "source": "crystal_items_xml",
            "value": ", ".join(crystal["vocation"]),
        }
    if crystal["weapon"]:
        return {
            "basis": "DERIVED",
            "source": "crystal_items_xml",
            "value": "unrestricted",
        }
    return {
        "basis": "UNKNOWN",
        "reason": "no TibiaWiki vocrequired; Crystal items.xml has no weapon node",
    }


def threshold_class(
    profile_name: str,
    ammotype: str | None = None,
    vocation: str | None = None,
    secondarytype: str | None = None,
) -> str:
    """Per-binding class: crossbow (bolt), knight (knight sword/axe/club), standard, unknown.

    `ammotype` is the Crystal items.xml value, `vocation` the resolved weapon vocation and
    `secondarytype` the TibiaWiki value; all are per-item evidence, None means none.
    """
    words = set(profile_name.replace("-", " ").split())
    if ammotype == "bolt":
        return "crossbow"
    if words & RANGED_WORDS:
        # A bow/crossbow family profile can be shared; without ammunition or wiki
        # bow/crossbow evidence the class is not guessed.
        if ammotype == "arrow" or secondarytype == "Bows":
            return "standard"
        return "crossbow" if secondarytype == "Crossbows" else "unknown"
    if words & KNIGHT_WORDS:
        if vocation is None:
            return "unknown"
        return "knight" if "knight" in vocation.lower() else "standard"
    return "standard"


def item_keys() -> set[str]:
    keys: set[str] = set()
    for path in sorted(ITEM_DEFINITIONS.glob("items-*.json")):
        for record in json.loads(path.read_text(encoding="utf-8"))["records"]:
            keys.add(record["definition"]["identity"]["key"])
    return keys


def perk_enum_inventory(profiles: list[dict]) -> dict:
    """Raw enum values observed in the perks, with counts; no meaning is attached."""
    counters = {
        name: Counter()
        for name in (
            "Type",
            "SkillId",
            "AugmentType",
            "ElementId",
            "DamageType",
            "MissileId",
        )
    }
    for profile in profiles:
        for level in profile["levels"]:
            for perk in level["Perks"]:
                for name, counter in counters.items():
                    if name in perk:
                        counter[perk[name]] += 1
    return {
        name: [
            {"value": value, "perks": count} for value, count in sorted(counter.items())
        ]
        for name, counter in counters.items()
    }


def perk_mapping() -> dict:
    """The owner-accepted D199 meaning and unit of each perk enum value."""
    return {
        "source": PERK_MAPPING_SOURCE,
        "unit_rules": PERK_UNIT_RULES,
        "Type": [
            {"value": v, "meaning": m, "unit": u}
            for v, (m, u) in sorted(PERK_TYPES.items())
        ],
        "SkillId": [{"value": v, "meaning": m} for v, m in sorted(SKILL_IDS.items())],
        "AugmentType": [
            {"value": v, "meaning": m} for v, m in sorted(AUGMENT_TYPES.items())
        ],
        "ElementId_DamageType": [
            {"value": v, "meaning": m} for v, m in sorted(COMBAT_TYPES.items())
        ],
    }


def build() -> dict:
    proficiency_manifest, definitions = load_family(
        STAGING / "proficiencies", "Proficiency", EXPECTED_PROFILES
    )
    binding_manifest, bindings = load_family(
        STAGING / "weapon-proficiency-bindings",
        "WeaponProficiencyBinding",
        EXPECTED_BINDINGS,
    )
    verify_pins(proficiency_manifest, binding_manifest)
    known_items = item_keys()
    ammotypes = crystal_ammotypes()
    wiki_meta, wiki = wiki_stats()
    crystal = crystal_vocations()

    profiles = []
    by_id = {}
    for definition in definitions:
        level_count = len(definition["levels"])
        profile = {
            "proficiency_id": definition["source_id"],
            "name": definition["name"],
            "version": definition["version"],
            "top_level": level_count,
            "mastery_level": level_count + MASTERY_OFFSET,
            "levels": definition["levels"],
        }
        if profile["proficiency_id"] in by_id:
            raise SystemExit(f"duplicate proficiency id {profile['proficiency_id']}")
        by_id[profile["proficiency_id"]] = profile
        profiles.append(profile)
    profiles.sort(key=lambda p: p["proficiency_id"])

    out_bindings = []
    seen = set()
    for binding in bindings:
        key = f"oteryn:item.tibia.i{binding['source_id']}"
        if key in seen:
            raise SystemExit(f"duplicate item key {key}")
        seen.add(key)
        profile = by_id.get(binding["proficiency_id"])
        if profile is None:
            raise SystemExit(
                f"{key}: dangling proficiency id {binding['proficiency_id']}"
            )
        item_id = binding["source_id"]
        item_wiki = wiki.get(item_id, {})
        vocation = vocation_evidence(item_wiki, crystal.get(item_id))
        secondarytype = single(item_wiki.get("secondarytype"))
        evidence = {
            "ammotype": ammotypes.get(item_id),
            "secondarytype": secondarytype,
            "vocation": vocation,
        }
        out_bindings.append(
            {
                "item_key": key,
                "client_object_id": item_id,
                "client_name": binding["name"],
                "proficiency_id": binding["proficiency_id"],
                "threshold_class": threshold_class(
                    profile["name"],
                    evidence["ammotype"],
                    vocation.get("value"),
                    secondarytype,
                ),
                "threshold_evidence": evidence,
                "item_defined": key in known_items,
            }
        )
        conflict = {"bolt": "Bows", "arrow": "Crossbows"}.get(evidence["ammotype"])
        if conflict is not None and secondarytype == conflict:
            raise SystemExit(f"{key}: ammotype and TibiaWiki secondarytype disagree")
    out_bindings.sort(key=lambda b: b["client_object_id"])

    return {
        "schema": SCHEMA,
        "authority": "SOURCE_OBSERVATIONS_ONLY; no gameplay promotion; perk meanings per owner decision D199",
        "client_version": "15.30",
        "inputs": {
            "proficiencies_sha256": PROFICIENCIES_SHA256,
            "appearances_sha256": APPEARANCES_SHA256,
            "staging": [
                "imports/cipsoft-staticdata/proficiencies",
                "imports/cipsoft-staticdata/weapon-proficiency-bindings",
            ],
        },
        "threshold_source": THRESHOLD_SOURCE,
        "threshold_decisions": THRESHOLD_DECISIONS,
        "threshold_evidence_inputs": {
            "crystal_catalog": {
                "path": str(CRYSTAL_CATALOG.relative_to(ROOT)),
                "sha256": CRYSTAL_CATALOG_SHA256,
            },
            "wiki_stats": wiki_meta,
            "crystal_vocations": {
                "path": str(CRYSTAL_VOCATIONS.relative_to(ROOT)),
                "sha256": CRYSTAL_VOCATIONS_SHA256,
                "source": CRYSTAL_ITEMS_XML,
            },
        },
        "thresholds": THRESHOLDS,
        "mastery_offset": MASTERY_OFFSET,
        "counts": {
            "profiles": len(profiles),
            "bindings": len(out_bindings),
            "bindings_without_item_definition": sum(
                not b["item_defined"] for b in out_bindings
            ),
            "referenced_profiles": len({b["proficiency_id"] for b in out_bindings}),
            "bindings_by_threshold_class": dict(
                sorted(Counter(b["threshold_class"] for b in out_bindings).items())
            ),
            "bindings_by_vocation_basis": dict(
                sorted(
                    Counter(
                        b["threshold_evidence"]["vocation"]["basis"]
                        for b in out_bindings
                    ).items()
                )
            ),
        },
        "perk_mapping": perk_mapping(),
        "perk_raw_enums": perk_enum_inventory(profiles),
        "bindings": out_bindings,
        "profiles": profiles,
    }


def render(document: dict) -> str:
    """One record per line keeps the diff reviewable; keys sorted, LF endings."""
    head = {k: v for k, v in document.items() if k not in ("bindings", "profiles")}
    lines = ["{"]
    for key in sorted(head):
        lines.append(f"  {json.dumps(key)}: {json.dumps(head[key], sort_keys=True)},")
    for key in ("bindings", "profiles"):
        lines.append(f'  "{key}": [')
        rows = [
            json.dumps(row, sort_keys=True, separators=(", ", ": "))
            for row in document[key]
        ]
        lines.append(",\n".join("    " + row for row in rows))
        lines.append("  ]" + ("," if key == "bindings" else ""))
    lines.append("}")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="diff instead of writing")
    parser.add_argument(
        "--stage-crystal",
        type=Path,
        metavar="ITEMS_XML",
        help="stage the weapon vocation facts from the pinned Crystal items.xml",
    )
    args = parser.parse_args(argv)
    if args.stage_crystal:
        _, bindings = load_family(
            STAGING / "weapon-proficiency-bindings",
            "WeaponProficiencyBinding",
            EXPECTED_BINDINGS,
        )
        facts = stage_crystal(args.stage_crystal, {b["source_id"] for b in bindings})
        CRYSTAL_VOCATIONS.parent.mkdir(parents=True, exist_ok=True)
        CRYSTAL_VOCATIONS.write_text(
            render_facts(facts), encoding="utf-8", newline="\n"
        )
        print(f"wrote {CRYSTAL_VOCATIONS}")
        return 0
    text = render(build())
    json.loads(text)  # the rendering must stay valid JSON
    if args.check:
        current = (
            args.output.read_text(encoding="utf-8") if args.output.is_file() else None
        )
        if current != text:
            print(f"{args.output}: differs from a fresh regeneration", file=sys.stderr)
            return 1
        print(f"{args.output}: up to date")
        return 0
    args.output.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
