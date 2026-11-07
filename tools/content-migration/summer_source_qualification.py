"""Carry decision 22's donor limitation into the actual pinned Creature provider."""
import copy
import hashlib
import json

from bind_monster_source_definitions import explicit_reference_levels

INPUT = "tools/content-schema/native-gameplay/summer-source-qualification.json"
PROVIDER = "content/creatures/definitions/spell-native-profiles.json"
MANIFEST = "content/spells.manifest.json"
NOTE = "not verified against Global"
CURRENT_FIELDS = "tools/content-schema/native-gameplay/summer-current-published-fields.json"


def attack_definitions(root, record):
    """Retain the canonical executable closure; never lower references into guessed bodies."""
    references = json.loads((root / "content/world/definitions/reference.json").read_bytes())
    declarations = json.loads((root / "content/world/definitions/declarations.json").read_bytes())
    explicit_reference_levels(declarations)
    identity = lambda value: (value["family"], value["key"], value["revision"])
    records = {identity(row["identity"]): row for row in references["records"]}
    profiles = {identity(row["target"]): row for row in declarations["authoring_profiles"]}
    selected = set()

    def visit(value):
        if isinstance(value, list):
            for child in value:
                visit(child)
        elif isinstance(value, dict):
            if (set(value) == {"family", "key", "revision"}
                    and value["family"] in {"Creature", "Behavior", "Ability", "Effect", "Formula"}):
                key = identity(value)
                if key not in selected:
                    selected.add(key)
                    visit(records[key])
                    visit(profiles[key])
            else:
                for child in value.values():
                    visit(child)

    visit(record["profile"]["target"])
    creature_profile = copy.deepcopy(record["profile"])
    explicit_reference_levels(creature_profile)
    if (profiles[identity(record["profile"]["target"])] != creature_profile
            or profiles[identity(record["behavior"]["target"])] != record["behavior"]):
        raise ValueError("Summer attack closure differs from admitted Creature or Behavior")
    return {"records": [records[key] for key in sorted(selected)],
            "profiles": [profiles[key] for key in sorted(selected)]}


def apply_native(root, generated, encode):
    document = json.loads((root / INPUT).read_bytes())
    if document["schema"] != "OTERYN_SUMMER_SOURCE_QUALIFICATION/v1":
        raise ValueError("Summer source qualification schema")
    rows = document["records"]
    by_key = {row["creature"]: row for row in rows}
    if len(by_key) != len(rows) or any(NOTE not in row["qualification"] for row in rows):
        raise ValueError("Summer source qualification must retain every limitation")
    current = current_published_fields(root)
    provider = json.loads(generated[PROVIDER])
    seen = set()
    for record in provider["records"]:
        key = record["profile"]["target"]["key"]
        if key in by_key:
            row = by_key[key]
            record["source_qualification"] = row["qualification"]
            if key in {"oteryn:creature.radiant_acolyte", "oteryn:creature.radiant_paragon",
                       "oteryn:creature.radiant_templar", "oteryn:creature.phosphorus_final"}:
                record["attack_definitions"] = attack_definitions(root, record)
            if "attacks" in row:
                melee = [(index, attack["fields"]) for index, attack in
                         enumerate(row["attacks"]["entries"], 1)
                         if attack["fields"].get("name") == "melee"]
                if len(melee) != 1 or record.get("monster_melee"):
                    raise ValueError("Summer melee source count or competing binding")
                index, fields = melee[0]
                ability_key = f"oteryn:ability.creature.{key.rsplit('.', 1)[1]}.attack-{index}"
                attacks = record["behavior"]["data"]["profile"]["attacks"]
                schedule = next((a for a in attacks if a["ability"]["key"] == ability_key), None)
                if (schedule is None or fields["minDamage"] > 0 or fields["maxDamage"] >= 0
                        or schedule["interval_ms"] != fields["interval"]
                        or schedule["chance_ppm"] != fields["chance"] * 10_000):
                    raise ValueError("Summer melee native schedule differs from pinned source")
                source = row["source"]
                record["monster_melee"] = {
                    "ability": schedule["ability"],
                    "interval_ms": fields["interval"],
                    "chance_ppm": fields["chance"] * 10_000,
                    "minimum": abs(fields["minDamage"]),
                    "maximum": abs(fields["maxDamage"]),
                    "source_repository": "zimbadev/crystalserver",
                    "source_revision": source["crystal_revision"],
                    "source_path": source["path"],
                    "source_sha256": source["sha256"],
                    "execution_status": "primaryphysicalonly_omits_secondary_conditions",
                }
            if "fallback_health" in row:
                source = row["source"]
                if (source["crystal_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
                        or len(source["sha256"]) != 64
                        or type(row["fallback_health"]) is not int
                        or row["fallback_health"] <= 0
                        or type(row["fallback_experience"]) is not int
                        or row["fallback_experience"] < 0):
                    raise ValueError("Summer boss fallback source or health")
                profile = record["profile"]["data"]["profile"]
                profile["health"] = row["fallback_health"]
                profile["experience"] = row["fallback_experience"]
                record["source_qualification"] = (
                    "Decision 22: pinned Crystal " + source["crystal_revision"]
                    + " provisional base HP/XP; not verified against Global. "
                    "Scalable encounter difficulty is owned by BOSSENC-1; "
                    "these base values do not implement difficulty scaling."
                )
                if "/bosses/" in source["path"]:
                    record["source_qualification"] += (
                        " W16_ADMISSION_AND_PHASE_SOURCE_MISSING: admission anchors, party gates, "
                        "cooldowns and normal-to-final transitions are unavailable."
                    )
                if key in current:
                    record["profile"]["data"]["profile"].update(current[key]["facts"])
                    record["source_qualification"] += (
                        " Current pinned Wiki published fields override provisional donor values: "
                        + ", ".join(sorted(current[key]["facts"]))
                        + "; source " + current[key]["source"]["revision"]
                        + "/" + current[key]["source"]["path"]
                        + " SHA256 " + current[key]["source"]["sha256"]
                        + ". Unpublished health, attacks, scaling and encounter admission remain unverified."
                    )
                if key == "oteryn:creature.phosphorus_final":
                    # The closure keeps decision 22's active HP/XP projection, rather than
                    # reviving the definition's pre-override final-form XP (Crystal final.lua:5).
                    for profile in record["attack_definitions"]["profiles"]:
                        if profile["target"] == record["profile"]["target"]:
                            profile["data"] = copy.deepcopy(record["profile"]["data"])
            seen.add(key)
    if seen != set(by_key):
        raise ValueError("Summer source qualification names a missing runtime Creature")
    generated[PROVIDER] = encode(provider)
    manifest = json.loads(generated[MANIFEST])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(generated[PROVIDER]).hexdigest()
    generated[MANIFEST] = encode(manifest)


def current_published_fields(root):
    """Only the exact source-captured numeric facts may supersede provisional values."""
    expected = {
        "oteryn:creature.mimar_haffar": (
            "mimar-haffar-87d7d",
            "13abfd2147b15959c8ece3557ff9eaef8bf0875ef930d618a3f0c18bb474e293",
            {"health": 460000, "experience": 3500000}),
        "oteryn:creature.phosphorus_final": (
            "phosphorus-final-ec835",
            "d14c1e74e4addc338e2db498fbe2bd20d955ac4c957e578ac026e89fb14cd607",
            {"experience": 16000000}),
    }
    doc = json.loads((root / CURRENT_FIELDS).read_bytes())
    if doc["schema"] != "OTERYN_CURRENT_SUMMER_PUBLISHED_FIELDS/v1":
        raise ValueError("Summer published fields schema")
    rows = {}
    for row in doc["records"]:
        key = row["creature"]
        if key not in expected or key in rows:
            raise ValueError("Unexpected or duplicate Summer current field")
        slug, digest, facts = expected[key]
        source = row["source"]
        path = "quests/data/wiki/" + slug + ".json"
        revision = "acc9994e2ad8868696bc10ef08070108e2baba92"
        if (source != {"repository": "Lavovskii/grimorio", "revision": revision,
                       "path": path, "sha256": digest,
                       "url": "https://raw.githubusercontent.com/Lavovskii/grimorio/"
                              + revision + "/" + path}
                or row["facts"] != facts
                or any(type(value) is not int for value in row["facts"].values())):
            raise ValueError("Summer current fields differ from exact captured source")
        rows[key] = row
    if set(rows) != set(expected):
        raise ValueError("Missing Summer current source fields")
    return rows
