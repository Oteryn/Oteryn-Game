"""Compose the reviewed Crystal 201189f Creature and schedule declarations."""

import hashlib
import json
from collections import Counter
from copy import deepcopy

from import_spell_families import canonical_bytes
from nrwicked_profiles import ROLES as ARBAZILOTH_ENCOUNTER_ROLES

PACKET = "imports/crystalserver/creature-parity-201189f.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCE = "oteryn:source.crystalserver"
SOURCE_REVISION = "crystalserver-creature-parity:" + REVISION
BATCH = "creature-parity-crystal-201189f-r1"
CREATURES = "content/creatures/definitions/spell-native-profiles.json"
PRESENTATIONS = "content/presentations/definitions/spell-native-profiles.json"
RACES = "content/creatures/definitions/creature-races.json"
ACTIVE_ENCOUNTER_KEYS = frozenset(
    f"oteryn:creature.{role}" for role in ARBAZILOTH_ENCOUNTER_ROLES
)


def identity(ref):
    return (ref["family"], ref["key"], ref["revision"])


def read_packet(root):
    raw = (root / PACKET).read_bytes()
    packet = json.loads(raw)
    if (
        packet["schema"] != "OTERYN_CRYSTAL_CREATURE_PARITY/v1"
        or packet["revision"] != REVISION
    ):
        raise ValueError("unexpected Crystal Creature parity packet")
    return packet, hashlib.sha256(raw).hexdigest()


def extend(index, row, field, loot_corrections):
    from cake_native_successor import source_profile

    key = identity(row[field])
    existing = index.setdefault(key, row)
    if existing == row:
        return
    if key[0] == "Loot":
        # The source correction names the same donor roll under its verified
        # Item identity. Keep prior roll order and check the complete multiset.
        corrected = deepcopy(existing)
        for entry in corrected["entries"]:
            item = entry["item"]
            replacement = loot_corrections.get(item["key"])
            if replacement is not None:
                entry["item"] = replacement
        items = lambda value: Counter(
            canonical_bytes(entry) for entry in value["entries"]
        )
        if items(corrected) == items(row) and {
            k: v for k, v in corrected.items() if k != "entries"
        } == {k: v for k, v in row.items() if k != "entries"}:
            index[key] = corrected
            return
    if (
        field == "target"
        and key[0] == "Creature"
        and source_profile(existing) == source_profile(row)
    ):
        return
    raise ValueError(f"conflicting Crystal parity declaration: {key}")


def prior_cohort_keys(root):
    from crystal_spawn_creatures import KEYS
    from cyclopedia_creatures import SOURCE as CYCLOPEDIA_SOURCE

    cyclopedia = json.loads((root / CYCLOPEDIA_SOURCE).read_bytes())
    return KEYS | {row["target"]["key"] for row in cyclopedia["bindings"]}


def add_schedule(behavior, addition):
    schedule = addition["schedule"]
    slots = behavior.setdefault(addition["group"], [])
    matching = [slot for slot in slots if slot["ability"] == schedule["ability"]]
    if matching and matching != [schedule]:
        raise ValueError(f"conflicting Crystal schedule: {schedule['ability']}")
    if not matching:
        # Existing source callback owners use the retained slots by index.
        slots.append(schedule)


def extend_abilities(profile, schedule):
    ability = schedule["ability"]
    abilities = profile.setdefault("abilities", [])
    if ability not in abilities:
        abilities.append(ability)
        abilities.sort(key=identity)


def install_documents(root, package_root):
    """Apply exact source rows to the staged inputs of the native World writer."""
    from questpc7_world_seal import refresh

    packet, digest = read_packet(root)
    paths = {
        name: package_root / relative
        for name, relative in (
            ("reference", "definitions/reference.json"),
            ("declarations", "definitions/declarations.json"),
            ("sources", "provenance/sources.json"),
            ("imports", "provenance/imports.json"),
        )
    }
    documents = {name: json.loads(path.read_bytes()) for name, path in paths.items()}
    records = {
        identity(row["identity"]): row for row in documents["reference"]["records"]
    }
    source_profiles = documents["declarations"]["authoring_profiles"]
    families = {row["target"]["family"]: i for i, row in enumerate(source_profiles)}
    profiles = {identity(row["target"]): row for row in source_profiles}
    sources = documents["sources"]
    bindings = {
        (
            row["source_key"],
            row["source_revision"],
            row["identity_namespace"],
            row["external_id"],
        ): row
        for row in sources["source_identity_bindings"]
    }

    for entry in [*packet["definitions"], *packet["schedules"]]:
        corrections = {
            f"oteryn:item.tibia.i{fix['donor_id']}": fix["item"]
            for fix in entry.get("source_loot_identity_corrections", [])
        }
        for row in entry["records"]:
            extend(records, row, "identity", corrections)
        for row in entry["profiles"]:
            extend(profiles, row, "target", {})
    for entry in packet["definitions"]:
        row = entry["binding"]
        key = (
            row["source_key"],
            row["source_revision"],
            row["identity_namespace"],
            row["external_id"],
        )
        if bindings.setdefault(key, row) != row:
            raise ValueError(f"conflicting Crystal Creature binding: {key}")
    for addition in packet["schedules"]:
        behavior = profiles[identity(addition["behavior"])]["data"]["profile"]
        creature = profiles[identity(addition["creature"])]["data"]["profile"]
        add_schedule(behavior, addition)
        extend_abilities(creature, addition["schedule"])

    documents["reference"]["records"] = [row for _, row in sorted(records.items())]
    documents["declarations"]["authoring_profiles"] = [
        profiles[key]
        for key in sorted(profiles, key=lambda key: (families[key[0]], *key[1:]))
    ]
    sources["source_identity_bindings"] = [row for _, row in sorted(bindings.items())]
    sources["sources"] = [
        row for row in sources["sources"] if row["import_batch_id"] != BATCH
    ]
    sources["sources"].append(
        {
            "key": SOURCE,
            "import_batch_id": BATCH,
            "revision": SOURCE_REVISION,
            "sha256": digest,
            "evidence": "OtsHypothesisOnly",
        }
    )
    sources["sources"].sort(key=lambda row: (row["key"], row["revision"]))
    imports = documents["imports"]
    imports["batches"] = [row for row in imports["batches"] if row["batch_id"] != BATCH]
    imports["batches"].append(
        {
            "batch_id": BATCH,
            "source_repository": "zimbadev/crystalserver",
            "source_revision": SOURCE_REVISION,
            "source_artifact_sha256": digest,
            "access_disposition": "PENDING",
            "source_generation_profile": packet["schema"],
            "importer": packet["schema"],
            "mapper": "creature_admission_stage.py",
            "mapper_revision": REVISION,
            "mapper_sha256": hashlib.sha256(
                (
                    root / "tools/content-migration/creature_admission_stage.py"
                ).read_bytes()
            ).hexdigest(),
            "candidates": [],
            "reimport_states": [],
        }
    )
    imports["batches"].sort(key=lambda row: row["batch_id"])
    for name, document in documents.items():
        paths[name].write_bytes(canonical_bytes(document))
    refresh(root, package_root)


def apply_native(root, generated):
    """Bind the canonical cohort before the existing attack and loot closure builders."""
    from bind_monster_source_definitions import explicit_reference_levels
    from build_canonical_monster_profiles import union_profiles
    from build_presentation_bindings import dumps

    packet, _ = read_packet(root)
    index = json.loads((root / "content/creatures/definitions/index.json").read_bytes())
    source_rows = {identity(row["identity"]): row for row in packet["definitions"]}
    expected = {
        key: row
        for key, row in source_rows.items()
        if key[1] not in ACTIVE_ENCOUNTER_KEYS
    }
    prior = prior_cohort_keys(root)
    additions = {key for key in expected if key[1] not in prior}
    rows = [
        row
        for path in index["shards"]
        for row in json.loads((root / path).read_bytes())["records"]
        if identity(row["definition"]["identity"]) in additions
    ]
    declarations = json.loads(
        (root / "content/world/definitions/declarations.json").read_bytes()
    )
    explicit_reference_levels(declarations)
    creatures, presentations, appended = union_profiles(
        json.loads(generated[CREATURES]),
        json.loads(generated[PRESENTATIONS]),
        rows,
        declarations["authoring_profiles"],
        preserve_existing_display_aliases=True,
        # Crystal's registry name differs from the display name. The spawn
        # resolver consumes the exact key for this source variant.
        source_display_aliases={
            "oteryn:creature.elderbloodjaw": "oteryn:creature.elder_bloodjaw",
        },
    )
    if {identity(ref) for ref in appended} != additions:
        missing = additions - {identity(ref) for ref in appended}
        raise ValueError(f"incomplete Crystal Creature admission: {sorted(missing)}")
    for record in creatures["records"]:
        entry = expected.get(identity(record["profile"]["target"]))
        if entry and "melee" in entry:
            record["monster_melee"] = entry["melee"]
    by_creature = {
        identity(row["profile"]["target"]): row for row in creatures["records"]
    }
    for addition in packet["schedules"]:
        record = by_creature[identity(addition["creature"])]
        if record["behavior"]["target"] != addition["behavior"]:
            raise ValueError("Crystal schedule target differs from native Behavior")
        add_schedule(record["behavior"]["data"]["profile"], addition)
        extend_abilities(record["profile"]["data"]["profile"], addition["schedule"])
        if addition.get("owner") == "source_item":
            ability = next(
                row
                for row in addition["profiles"]
                if row["target"] == addition["schedule"]["ability"]
            )
            record.setdefault("source_item_abilities", []).append(ability)
    creatures["records"].sort(key=lambda row: identity(row["profile"]["target"]))
    presentations["records"].sort(key=lambda row: identity(row["target"]))
    generated[CREATURES] = canonical_bytes(creatures)
    generated[PRESENTATIONS] = canonical_bytes(presentations)
    manifest = json.loads(generated["content/spells.manifest.json"])
    manifest["presentation_profiles"]["sha256"] = hashlib.sha256(
        generated[PRESENTATIONS]
    ).hexdigest()
    generated["content/spells.manifest.json"] = canonical_bytes(manifest)

    generated[RACES] = dumps(race_catalogue(root, source_rows.values())).encode()


def race_catalogue(root, parity_rows):
    races = json.loads((root / RACES).read_bytes())
    records = {identity(row["creature"]): row for row in races["records"]}
    records.update(reviewed_race_records(root, parity_rows))
    races["records"] = [records[key] for key in sorted(records)]
    races["record_count"] = len(records)
    return races


def reviewed_race_records(root, parity_rows):
    from cyclopedia_creatures import SOURCE as CYCLOPEDIA_SOURCE

    rows = list(parity_rows)
    cyclopedia = json.loads((root / CYCLOPEDIA_SOURCE).read_bytes())
    bindings = {row["external_id"]: row["target"] for row in cyclopedia["bindings"]}
    rows.extend(
        {
            "identity": bindings[row["path"]],
            "race": row["race"],
            "race_disposition": row["race_disposition"],
            "source_path": row["path"],
            "source_sha256": row["sha256"],
        }
        for row in cyclopedia["sources"]
        if "race" in row
    )
    return {
        identity(row["identity"]): {
            "creature": row["identity"],
            "race": row["race"],
            "disposition": row["race_disposition"],
            "source": {
                "repository": "zimbadev/crystalserver",
                "path": row["source_path"],
                "sha256": row["source_sha256"],
            },
        }
        for row in rows
    }


def reviewed_schedule_preimage(record, additions):
    """Recover the existing callback owner's body after checking the new slots."""
    from copy import deepcopy

    additions = [
        row for row in additions if row["creature"] == record["profile"]["target"]
    ]
    if not additions:
        return record
    earlier = deepcopy(record)
    for addition in additions:
        schedule = addition["schedule"]
        earlier["behavior"]["data"]["profile"][addition["group"]].remove(schedule)
        earlier["profile"]["data"]["profile"]["abilities"].remove(schedule["ability"])
        if addition.get("owner") == "source_item":
            body = next(
                row
                for row in addition["profiles"]
                if row["target"] == schedule["ability"]
            )
            earlier["source_item_abilities"].remove(body)
            if not earlier["source_item_abilities"]:
                del earlier["source_item_abilities"]

    # Drop a dependency only when the earlier Creature and Behavior no longer
    # reach it. Other schedules may reuse an Ability, Effect, or Formula.
    closure = earlier["attack_definitions"]
    profiles = {identity(row["target"]): row for row in closure["profiles"]}
    profiles[identity(earlier["profile"]["target"])] = earlier["profile"]
    profiles[identity(earlier["behavior"]["target"])] = earlier["behavior"]
    records = {identity(row["identity"]): row for row in closure["records"]}
    reachable = set()

    def visit(value):
        if isinstance(value, dict):
            if set(value) == {"family", "key", "revision"}:
                key = identity(value)
                if key in profiles and key not in reachable:
                    reachable.add(key)
                    visit(profiles[key])
                    visit(records[key])
            else:
                for child in value.values():
                    visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(earlier["profile"]["target"])
    visit(earlier["behavior"]["target"])
    for role in ("attacks", "defenses"):
        profile = earlier["behavior"]["data"]["profile"]
        if profile.get(role) == []:
            del profile[role]
    closure["profiles"] = [profiles[key] for key in sorted(reachable)]
    closure["records"] = [records[key] for key in sorted(reachable)]
    return earlier
