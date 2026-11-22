"""Stage and serve the exact Creature definitions needed by the Crystal placement delta."""

import hashlib
import json
import re
import sys
from pathlib import Path

from creature_admission_stage import Mapper, Stage
from questpc7_world_seal import refresh

ROOT = Path(__file__).resolve().parents[2]
PACKET = "imports/crystalserver/spawn-delta-creatures-201189f8.json"
BATCH = "crystal-spawn-delta-creatures-201189f8-r1"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCE_REVISION = f"crystal-spawn-creatures:{REVISION}"
CREATURES = "content/creatures/definitions/spell-native-profiles.json"
PRESENTATIONS = "content/presentations/definitions/spell-native-profiles.json"


def encode(document):
    return (
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n"
    ).encode()


def read(root, path):
    return json.loads((root / path).read_bytes())


def stage(root, packet):
    if (
        packet["schema"] != "OTERYN_CRYSTAL_SPAWN_CREATURES/v1"
        or packet["repository"] != "zimbadev/crystalserver"
        or packet["revision"] != REVISION
    ):
        raise ValueError("Crystal spawn Creature source identity")
    bindings = read(root, "imports/crystalserver/bindings/items.json")["bindings"]
    items = {
        int(row["external_id"]): row["target"]["key"]
        for row in bindings
        if row["disposition"] == "EXACT" and row["external_id"].isdecimal()
    }
    result = Stage(Mapper(items))
    for entry in packet["records"]:
        path = entry["path"]
        result.stage_dependencies(entry["dependencies"], path)
        result.stage_monster(
            entry["monster"],
            path,
            {
                "source_key": "oteryn:source.crystalserver",
                "source_revision": SOURCE_REVISION,
                "identity_namespace": "crystalserver/monster-file",
                "external_id": path,
            },
        )
    return result


def install(root, world):
    raw = (root / PACKET).read_bytes()
    packet = json.loads(raw)
    staged = stage(root, packet)
    digest = hashlib.sha256(raw).hexdigest()
    repository = packet["repository"]
    reference_path = "definitions/reference.json"
    declarations_path = "definitions/declarations.json"
    sources_path = "provenance/sources.json"
    imports_path = "provenance/imports.json"
    reference, declarations, sources, imports = (
        read(world, path)
        for path in (reference_path, declarations_path, sources_path, imports_path)
    )
    # The native writer expects typed family order for authoring profiles and wire
    # family order for reference records, as in the existing Creature stage.
    families = (root / "crates/gameplay/content/src/content/project/v2.rs").read_text()
    family_order = {
        {"Npc": "NPC", "ItemConversion": "item_conversion"}.get(name, name): ordinal
        for ordinal, name in enumerate(
            re.findall(
                r"^    ([A-Za-z]+),$",
                families.split("pub enum ProjectV2Family {", 1)[1].split("\n}", 1)[0],
                re.M,
            )
        )
    }

    def add_exact(rows, authored, field, family_key):
        by_id = {
            (row[field]["family"], row[field]["key"], row[field]["revision"]): row
            for row in rows
        }
        for row in authored:
            ref = row[field]
            identity = (ref["family"], ref["key"], ref["revision"])
            if identity in by_id and by_id[identity] != row:
                raise ValueError(
                    f"Crystal spawn Creature identity collision: {identity}"
                )
            by_id[identity] = row
        return sorted(
            by_id.values(),
            key=lambda row: (
                family_key(row[field]["family"]),
                row[field]["key"],
                row[field]["revision"],
            ),
        )

    reference["records"] = add_exact(
        reference["records"], staged.records.values(), "identity", str
    )
    declarations["authoring_profiles"] = add_exact(
        declarations["authoring_profiles"],
        staged.profiles.values(),
        "target",
        family_order.__getitem__,
    )
    sources["source_identity_bindings"] = sorted(
        [
            row
            for row in sources["source_identity_bindings"]
            if row["source_revision"] != SOURCE_REVISION
        ]
        + staged.bindings,
        key=lambda row: tuple(
            row[key]
            for key in (
                "source_key",
                "source_revision",
                "identity_namespace",
                "external_id",
            )
        ),
    )
    sources["sources"] = sorted(
        [row for row in sources["sources"] if row["import_batch_id"] != BATCH]
        + [
            {
                "key": "oteryn:source.crystalserver",
                "import_batch_id": BATCH,
                "revision": SOURCE_REVISION,
                "sha256": digest,
                "evidence": "OtsHypothesisOnly",
            }
        ],
        key=lambda row: (row["key"], row["revision"]),
    )
    imports["batches"] = sorted(
        [row for row in imports["batches"] if row["batch_id"] != BATCH]
        + [
            {
                "batch_id": BATCH,
                "source_repository": repository,
                "source_revision": SOURCE_REVISION,
                "source_artifact_sha256": digest,
                "access_disposition": "PENDING",
                "source_generation_profile": packet["schema"],
                "importer": packet["schema"],
                "mapper": "creature_admission_stage.py",
                "mapper_revision": REVISION,
                "mapper_sha256": hashlib.sha256(
                    Path(__file__).with_name("creature_admission_stage.py").read_bytes()
                ).hexdigest(),
                "candidates": [],
                "reimport_states": [],
            }
        ],
        key=lambda row: row["batch_id"],
    )
    for path, document in [
        (reference_path, reference),
        (declarations_path, declarations),
        (sources_path, sources),
        (imports_path, imports),
    ]:
        (world / path).write_bytes(encode(document))
    refresh(root, world)


def admit(root, generated):
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    from build_canonical_monster_profiles import union_profiles

    staged = stage(root, read(root, PACKET))
    keys = {key for family, key in staged.records if family == "Creature"}
    rows = [
        row
        for path in sorted(
            (root / "content/creatures/definitions").glob("creatures-*.json")
        )
        for row in json.loads(path.read_bytes())["records"]
        if row["definition"]["identity"]["key"] in keys
    ]
    declarations = read(root, "content/world/definitions/declarations.json")[
        "authoring_profiles"
    ]
    creatures, presentations, appended = union_profiles(
        json.loads(generated[CREATURES]),
        json.loads(generated[PRESENTATIONS]),
        rows,
        declarations,
        preserve_existing_display_aliases=True,
    )
    if {row["key"] for row in appended} != keys:
        raise ValueError("Crystal spawn Creature cohort is incomplete")
    generated[CREATURES] = encode(creatures)
    generated[PRESENTATIONS] = encode(presentations)
    manifest_path = "content/spells.manifest.json"
    manifest = json.loads(generated[manifest_path])
    manifest["presentation_profiles"]["sha256"] = hashlib.sha256(
        generated[PRESENTATIONS]
    ).hexdigest()
    generated[manifest_path] = encode(manifest)
