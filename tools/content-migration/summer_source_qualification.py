"""Carry decision 22's donor limitation into the actual pinned Creature provider."""
import hashlib
import json

INPUT = "tools/content-schema/native-gameplay/summer-source-qualification.json"
PROVIDER = "content/creatures/definitions/spell-native-profiles.json"
MANIFEST = "content/spells.manifest.json"
NOTE = "not verified against Global"


def attack_definitions(root, record):
    """Retain the canonical executable closure; never lower references into guessed bodies."""
    references = json.loads((root / "content/world/definitions/reference.json").read_bytes())
    declarations = json.loads((root / "content/world/definitions/declarations.json").read_bytes())
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
    if (profiles[identity(record["profile"]["target"])] != record["profile"]
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
    provider = json.loads(generated[PROVIDER])
    seen = set()
    for record in provider["records"]:
        key = record["profile"]["target"]["key"]
        if key in by_key:
            row = by_key[key]
            record["source_qualification"] = row["qualification"]
            if key in {"oteryn:creature.radiant_acolyte", "oteryn:creature.radiant_paragon",
                       "oteryn:creature.radiant_templar"}:
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
            seen.add(key)
    if seen != set(by_key):
        raise ValueError("Summer source qualification names a missing runtime Creature")
    generated[PROVIDER] = encode(provider)
    manifest = json.loads(generated[MANIFEST])
    manifest["creature_profiles"]["sha256"] = hashlib.sha256(generated[PROVIDER]).hexdigest()
    generated[MANIFEST] = encode(manifest)
