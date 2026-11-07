"""SRV7 closed Summer NPC population and Doctor Marrow canonical admission.

Source literals/cells are normalized from Crystal201189f8; no Lua is executed.
The committed input retains individual file SHA-256 and source line evidence.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INPUT = ROOT / "tools/content-migration/summer_npcs.json"
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
BATCH = "srv7-summer-doctor-marrow-r1"
SOURCE = "oteryn:source.crystalserver"


def dump(value):
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    )


def data():
    value = json.loads(INPUT.read_text())
    assert value["schema"] == 1 and value["source_revision"] == PIN
    assert len(value["positions"]) == 20 and len(value["greetings"]) == 19
    assert len({row["npc"] for row in value["positions"]}) == 19
    assert (
        value["doctor"]["source"]["sha256"]
        == "af1ff19264525f2b0726211c05f54ec4e244eee797d4fa919ed8dfd82a874135"
    )
    return value


def append_exact(rows, entry, identity):
    prior = [row for row in rows if identity(row) == identity(entry)]
    if prior:
        if len(prior) != 1 or prior[0] != entry:
            raise ValueError("SUMMER_NPC_EXISTING_IDENTITY_CONFLICT")
    else:
        rows.append(copy.deepcopy(entry))


def refresh_controls(files):
    """Retoken the existing package using production.rs:299-326's exact preimage."""
    world = ROOT / "content/world"
    manifest = json.loads((world / "manifest.json").read_bytes())
    for row in manifest["documents"]:
        path = world / row["locator"]
        raw = files.get(str(path))
        raw = path.read_bytes() if raw is None else raw.encode()
        row["byte_length"] = len(raw)
        row["sha256"] = hashlib.sha256(raw).hexdigest()
    raw_manifest = dump(manifest).encode()
    manifest_digest = hashlib.sha256(raw_manifest).hexdigest()
    fields = [
        manifest[key]
        for key in (
            "package_key",
            "package_revision",
            "semantic_schema_version",
            "licensing_metadata",
        )
    ]
    fields.append(manifest_digest)
    preimage = b"".join(
        len(value.encode()).to_bytes(4, "big") + value.encode() for value in fields
    )
    provenance_digest = hashlib.sha256(preimage).hexdigest()
    lock = json.loads((world / "content.lock.json").read_bytes())
    matched = [
        row
        for row in lock["entries"]
        if row["package_key"] == manifest["package_key"]
        and row["package_revision"] == manifest["package_revision"]
    ]
    if len(matched) != 1:
        raise ValueError("SUMMER_NPC_ROOT_PACKAGE_LOCK_MISMATCH")
    matched[0]["package_provenance_digest"] = provenance_digest
    raw_lock = dump(lock).encode()
    project = json.loads((world / "project.json").read_bytes())
    project["manifest_sha256"] = manifest_digest
    project["content_lock_sha256"] = hashlib.sha256(raw_lock).hexdigest()
    files[str(world / "manifest.json")] = raw_manifest.decode()
    files[str(world / "content.lock.json")] = raw_lock.decode()
    files[str(world / "project.json")] = dump(project)


def canonical_files(overrides=None):
    value = data()
    doctor = value["doctor"]
    output = {}
    for relative in [
        "definitions/declarations.json",
        "definitions/reference.json",
        "provenance/sources.json",
        "provenance/imports.json",
    ]:
        source = (overrides or {}).get(relative)
        output[relative] = json.loads(
            source
            if source is not None
            else (ROOT / "content/world" / relative).read_text()
        )
    declarations = output["definitions/declarations.json"]
    for row in doctor["records"]:
        append_exact(declarations["records"], row, lambda r: r["identity"]["key"])
    for row in doctor["profiles"]:
        append_exact(
            declarations["authoring_profiles"], row, lambda r: r["target"]["key"]
        )
    reference = output["definitions/reference.json"]["records"]
    for family, projection in [
        ("Behavior", "ServerOnly"),
        ("Presentation", "ClientSafe"),
    ]:
        entry = {
            "kind": "Generic",
            "identity": {
                "family": family,
                "key": f"oteryn:{family.lower()}.npc.doctor_marrow",
                "revision": "definition-r1",
            },
            "client_projection": projection,
        }
        append_exact(reference, entry, lambda r: r["identity"]["key"])
    sources = output["provenance/sources.json"]
    append_exact(
        sources["sources"],
        {
            "key": SOURCE,
            "import_batch_id": BATCH,
            "revision": PIN,
            "sha256": doctor["source"]["sha256"],
            "evidence": "OtsHypothesisOnly",
        },
        lambda r: (r["key"], r["revision"]),
    )
    binding = {
        "source_key": SOURCE,
        "source_revision": PIN,
        "identity_namespace": "crystalserver/npc-file",
        "external_id": "doctor_marrow",
        "target": {
            "family": "NPC",
            "key": "oteryn:npc.doctor_marrow",
            "revision": "definition-r1",
        },
        "disposition": "EXACT",
    }
    append_exact(
        sources["source_identity_bindings"],
        binding,
        lambda r: (
            r["source_key"],
            r["source_revision"],
            r["identity_namespace"],
            r["external_id"],
            r["target"]["family"],
        ),
    )
    batch = {
        "batch_id": BATCH,
        "source_repository": "zimbadev/crystalserver",
        "source_revision": PIN,
        "source_artifact_sha256": doctor["source"]["sha256"],
        "access_disposition": "PENDING",
        "source_generation_profile": "OTERYN_NPC_AUTHORING_CANDIDATE/v1",
        "importer": "OTERYN_NPC_PROMOTION_CANDIDATES/v1",
        "mapper": "OTERYN_NPC_ADMISSION_STAGE/v1",
        "mapper_revision": "srv7-summer-npc-r1",
        "mapper_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "candidates": [],
        "reimport_states": [],
    }
    batches = output["provenance/imports.json"]["batches"]
    batches[:] = [row for row in batches if row["batch_id"] != BATCH]
    batches.append(batch)
    # Project V2 declaration/profile order is the Rust enum order; reference
    # records use the distinct lexical (family,key) contract. Appending a new
    # Doctor must preserve both contracts before the real Project loader runs.
    families = [
        "Terrain",
        "Presentation",
        "LocalObject",
        "WorldObject",
        "Area",
        "Document",
        "Achievement",
        "Outfit",
        "Mount",
        "Charm",
        "Proficiency",
        "item_conversion",
        "Item",
        "Creature",
        "Ability",
        "Effect",
        "Formula",
        "Loot",
        "Behavior",
        "NPC",
        "Dialogue",
        "Service",
        "Interaction",
        "Quest",
        "Transition",
        "House",
        "Encounter",
    ]
    rank = {family: ordinal for ordinal, family in enumerate(families)}
    declarations["records"].sort(
        key=lambda row: (rank[row["kind"]], row["identity"]["key"])
    )
    declarations["authoring_profiles"].sort(
        key=lambda row: (
            rank[row["target"]["family"]],
            row["target"]["key"],
            row["target"]["revision"],
        )
    )
    reference.sort(key=lambda row: (row["identity"]["family"], row["identity"]["key"]))
    sources["sources"].sort(key=lambda row: (row["key"], row["revision"]))
    sources["source_identity_bindings"].sort(
        key=lambda row: (
            row["source_key"],
            row["source_revision"],
            row["identity_namespace"],
            row["external_id"],
        )
    )
    batches.sort(key=lambda row: row["batch_id"])
    files = {
        str(ROOT / "content/world" / relative): dump(content)
        for relative, content in output.items()
    }
    refresh_controls(files)
    greetings = {
        "schema": 1,
        "source_revision": PIN,
        "npcs": value["greetings"],
        "placeholder_source": value["placeholder_source"],
    }
    files[str(ROOT / "crates/gameplay/npc/src/npc_talk/summer.json")] = (
        json.dumps(greetings, indent=2, ensure_ascii=False) + "\n"
    )
    return files


def augment_placements(files, frame):
    """Called by the existing placement owner so regeneration retains its base shard."""
    index = json.loads(files["index.json"])
    existing = []
    for path in index["shards"]:
        existing.extend(json.loads(files[Path(path).name])["records"])
    placed = {r["declaration"]["npc"] for r in existing}
    existing_ids = {r["declaration"]["identity"]["key"] for r in existing}
    new = []
    for row in data()["positions"]:
        cell = row["cell"]
        key = f"oteryn:npc_placement.{row['npc'].removeprefix('oteryn:npc.')}.x{cell['x']}_y{cell['y']}_z{cell['floor']}"
        if key in existing_ids:
            continue
        if any(r["declaration"]["cell"] == cell for r in existing + new):
            raise ValueError("SUMMER_NPC_CELL_COLLISION")
        declaration = {
            "cell": cell,
            "direction": row["direction"],
            "identity": {"key": key, "revision": "definition-r1"},
            "npc": row["npc"],
            "provenance": [
                {
                    "arbitration": [],
                    "definition_sha256": row["definition_sha256"],
                    "origin": "crystal",
                    "repository": "zimbadev/crystalserver",
                    "revision": PIN,
                    "source_key": SOURCE,
                    "source_row": f"crystal:npc/{row['npc'].removeprefix('oteryn:npc.')}",
                }
            ],
        }
        new.append({"declaration": declaration})
        placed.add(row["npc"])
    new.sort(key=lambda r: r["declaration"]["identity"]["key"])
    assert len(new) == 17
    name = f"placements-{len(existing):05d}-{len(existing) + len(new) - 1:05d}.json"
    files[name] = dump(
        {"coordinate_frame": frame, "family": "Npc.Placement", "records": new}
    )
    index["shards"].append("content/world/npc-placements/" + name)
    index["record_count"] += len(new)
    index["npc_count"] = len(placed)
    index["inputs"]["summer_npcs_sha256"] = hashlib.sha256(
        INPUT.read_bytes()
    ).hexdigest()
    files["index.json"] = dump(index)
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, expected in canonical_files().items():
        if args.check:
            if Path(path).read_text() != expected:
                raise ValueError("SUMMER_NPC_CANONICAL_DRIFT:" + path)
        else:
            Path(path).parent.mkdir(parents=True, exist_ok=True)
            Path(path).write_text(expected)


if __name__ == "__main__":
    main()
