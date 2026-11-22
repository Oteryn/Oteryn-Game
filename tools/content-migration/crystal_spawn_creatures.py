"""Stage the closed Crystal 201189f Creature bodies used by spawn and raid admission."""

import hashlib
import json
import re
import sys
from pathlib import Path

from creature_admission_stage import Mapper, Stage
from import_spell_families import canonical_bytes

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/monster-authoring"))
from validate_monster import validate

PATH = "imports/crystalserver/spawn-delta-creatures-201189f.json"
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
REVISION = "crystal-spawn-creatures:" + PIN
BATCH = "crystal-spawn-creatures-201189f-r1"
NAMES = frozenset(
    {
        "common_beetle",
        "cublarc_the_plunderer",
        "gorga",
        "grove_guardian",
        "herd_weevil",
        "rootthing_cocoon",
    }
)
KEYS = frozenset("oteryn:creature." + name for name in NAMES)


def stage(root, reference, payload):
    if (payload["schema"], payload["revision"]) != (
        "OTERYN_CRYSTAL_SPAWN_DELTA_CREATURES/v1",
        PIN,
    ):
        raise ValueError("Crystal spawn Creature capture revision")
    items = {
        row["identity"]["key"]
        for row in reference
        if row["identity"]["family"] == "Item"
    }
    bindings = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    item_map = {
        int(row["external_id"]): row["target"]["key"]
        for row in bindings["bindings"]
        if row["disposition"] == "EXACT"
        and row["external_id"].isdigit()
        and row["target"]["family"] == "Item"
        and row["target"]["key"] in items
    }
    cohort = Stage(Mapper(item_map))
    for row in payload["bundles"]:
        monster, dependencies, catalog, manifest = (
            row[field] for field in ("monster", "dependencies", "catalog", "manifest")
        )
        if manifest["sources"][0] != {
            "repository": "zimbadev/crystalserver",
            "revision": PIN,
        }:
            raise ValueError("Crystal spawn Creature donor identity")
        errors = validate(monster, dependencies, catalog, manifest)
        if errors:
            raise ValueError(
                "Crystal spawn Creature source is open: " + "; ".join(errors)
            )
        file = row["source"]["file"]
        cohort.stage_dependencies(dependencies, file)
        cohort.stage_monster(
            monster,
            file,
            {
                "source_key": "oteryn:source.crystalserver",
                "source_revision": REVISION,
                "identity_namespace": "crystalserver/monster-file",
                "external_id": file,
            },
        )
    if (
        len(cohort.bindings) != len(KEYS)
        or {row["target"]["key"] for row in cohort.bindings} != KEYS
    ):
        raise ValueError("Crystal spawn Creature membership")
    return cohort


def install(root, world):
    from cake_native_successor import source_profile

    raw = (root / PATH).read_bytes()
    payload = json.loads(raw)
    digest = hashlib.sha256(raw).hexdigest()
    locators = (
        "definitions/reference.json",
        "definitions/declarations.json",
        "provenance/sources.json",
        "provenance/imports.json",
    )
    docs = [json.loads((world / locator).read_bytes()) for locator in locators]
    reference, declarations, sources, imports = docs
    cohort = stage(root, reference["records"], payload)
    for rows, incoming, field in (
        (reference["records"], cohort.records.values(), "identity"),
        (declarations["authoring_profiles"], cohort.profiles.values(), "target"),
    ):
        occupied = {identity(row[field]): row for row in rows}
        for row in incoming:
            previous = occupied.get(identity(row[field]))
            if previous is None:
                rows.append(row)
            elif previous != (source_profile(row) if field == "target" else row):
                raise ValueError(
                    f"Crystal spawn Creature definition conflict: {row[field]}"
                )
    sources["source_identity_bindings"] = [
        row
        for row in sources["source_identity_bindings"]
        if row["source_revision"] != REVISION
    ] + cohort.bindings
    sources["sources"] = [
        row for row in sources["sources"] if row["import_batch_id"] != BATCH
    ] + [
        {
            "key": "oteryn:source.crystalserver",
            "import_batch_id": BATCH,
            "revision": REVISION,
            "sha256": digest,
            "evidence": "OtsHypothesisOnly",
        }
    ]
    imports["batches"] = [
        row for row in imports["batches"] if row["batch_id"] != BATCH
    ] + [
        {
            "batch_id": BATCH,
            "source_repository": "zimbadev/crystalserver",
            "source_revision": REVISION,
            "source_artifact_sha256": digest,
            "access_disposition": "PENDING",
            "source_generation_profile": payload["schema"],
            "importer": payload["schema"],
            "mapper": "creature_admission_stage.py",
            "mapper_revision": PIN,
            "mapper_sha256": hashlib.sha256(
                (
                    root / "tools/content-migration/creature_admission_stage.py"
                ).read_bytes()
            ).hexdigest(),
            "candidates": [],
            "reimport_states": [],
        }
    ]
    vocabulary = (
        root / "crates/gameplay/content/src/content/project/v2.rs"
    ).read_text()
    families = vocabulary.split("pub enum ProjectV2Family {", 1)[1].split("\n}", 1)[0]
    order = {
        {"Npc": "NPC", "ItemConversion": "item_conversion"}.get(name, name): index
        for index, name in enumerate(
            re.findall(r"^    ([A-Za-z]+),$", families, re.MULTILINE)
        )
    }
    reference["records"].sort(key=lambda row: identity(row["identity"]))
    declarations["authoring_profiles"].sort(
        key=lambda row: (
            order[row["target"]["family"]],
            row["target"]["key"],
            row["target"]["revision"],
        )
    )
    sources["sources"].sort(key=lambda row: (row["key"], row["revision"]))
    sources["source_identity_bindings"].sort(
        key=lambda row: tuple(
            row[field]
            for field in (
                "source_key",
                "source_revision",
                "identity_namespace",
                "external_id",
            )
        )
    )
    imports["batches"].sort(key=lambda row: row["batch_id"])
    for locator, document in zip(locators, docs, strict=True):
        (world / locator).write_bytes(canonical_bytes(document))


def identity(row):
    return row["family"], row["key"], row["revision"]


def admit(root, generated):
    from cake_native_successor import (
        CREATURES,
        PRESENTATIONS,
        read_project_document,
        union_profiles,
    )

    declarations = read_project_document(root, "definitions/declarations.json")[
        "authoring_profiles"
    ]
    profiles = {identity(row["target"]): row["data"] for row in declarations}
    reference = read_project_document(root, "definitions/reference.json")
    rows = [
        {"definition": row, "authoring": profiles[identity(row["identity"])]}
        for row in reference["records"]
        if row["identity"]["key"] in KEYS
    ]
    native, presentations, appended = union_profiles(
        json.loads(generated[CREATURES]),
        json.loads(generated[PRESENTATIONS]),
        rows,
        declarations,
        preserve_existing_display_aliases=True,
    )
    if len(appended) != len(KEYS) or {row["key"] for row in appended} != KEYS:
        raise ValueError("Crystal spawn Creature native membership")
    generated[CREATURES] = canonical_bytes(native)
    generated[PRESENTATIONS] = canonical_bytes(presentations)
    path = "content/spells.manifest.json"
    manifest = json.loads(generated[path])
    manifest["presentation_profiles"]["sha256"] = hashlib.sha256(
        generated[PRESENTATIONS]
    ).hexdigest()
    generated[path] = canonical_bytes(manifest)
    return generated
