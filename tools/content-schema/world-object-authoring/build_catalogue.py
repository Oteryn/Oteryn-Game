"""Build the Terrain and WorldObject catalogues (WO-2) from the pinned Crystal sources.

Writes every record `world_objects.build_census` validates into `content/world/terrain/`
and `content/world/objects/`, in shards of 500 records ordered by source (Tibia) id, and
marks both directories POPULATED. Each record's key is the A12 §4.6 family key of its
Tibia Item key; the routed Item keeps its key (the `routed_to` pointer on the Item record
and the typed relation references are WO-2b).

`--check` rebuilds every file in memory and fails on any byte difference, missing or
extra shard. The census sample is rebuilt from the same run, so both always agree.

Architecture: docs/architecture/reviews/
OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

# isort: off
# world_objects installs the existing item-authoring helpers on sys.path.
import world_objects
import appearance_membership
import donor_census
# isort: on

ROOT = Path(__file__).resolve().parents[3]
SHARD = 500
CATALOGUES = {
    # family: (directory, shard prefix, marker notes)
    "Terrain": ("content/world/terrain", "terrain", "Terrain definitions"),
    "WorldObject": (
        "content/world/objects",
        "objects",
        "Doors, ladders, beds, corpses, furniture, decorations and other world objects",
    ),
}
BUILDER = "tools/content-schema/world-object-authoring/build_catalogue.py"


def marker(directory, family, count, source):
    notes = (
        f"{CATALOGUES[family][2]}: {count} records, one per Tibia id routed to {family} "
        f"by the item converter at {source['repository']}@{source['revision'][:7]} "
        f"(WO-0 D93/D94, A12 §4.6), built by {BUILDER}."
    )
    return {
        "schema": "OTERYN_GAME_TREE_DIRECTORY/v1",
        "path": f"{directory}/",
        "kind": "static_content",
        "owner": "WorldObject/LocalObject" if family == "WorldObject" else "Terrain",
        "repository": "Oteryn/Oteryn-Game",
        "population_state": "POPULATED",
        "contract": "docs/architecture/OTERYN_FULL_GAME_CONTENT_AND_RULESET_TREE_V1.md",
        "notes": notes,
    }


# Existing admitted donor routes, restricted to the reviewed ownerless Item keys.
DONOR_QUALIFICATION = (
    world_objects.ROOT / "samples/qualified-donor-routes-00ce02a5.json"
)
DONOR_QUALIFICATION_SHA256 = (
    "63bf279a03caf45f6564e9ce7292c342902ae3135228e14791ad344100f96d83"
)
DONOR_CENSUS = world_objects.ROOT / "samples/census-crystal-donor-00ce02a5.json"
DONOR_PROFILE = "OTERYN_ITEM_DONOR_CENSUS/v1"


def qualified_donor_routes():
    raw = DONOR_QUALIFICATION.read_bytes()
    if hashlib.sha256(raw).hexdigest() != DONOR_QUALIFICATION_SHA256:
        raise ValueError("DONOR_QUALIFICATION_DIGEST")
    document = json.loads(raw)
    source = document["source"]
    if source != {
        "repository": donor_census.DONOR_REPOSITORY,
        "revision": donor_census.DONOR_COMMIT,
        "items_xml_sha256": donor_census.DONOR_ITEMS_XML_SHA256,
        "appearances_sha256": donor_census.DONOR_APPEARANCES_SHA256,
    }:
        raise ValueError("DONOR_QUALIFICATION_SOURCE")
    ids = [row["source_item_id"] for row in document["records"]]
    if len(ids) != 125 or ids != sorted(set(ids)):
        raise ValueError("DONOR_QUALIFICATION_SCOPE")
    return document


def membership_projection(label, appearances_sha256):
    index = json.loads(
        (ROOT / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    if label == "client-15.30" and index["newest"] != label:
        raise ValueError("DONOR_CURRENT_MEMBERSHIP_GENERATION")
    spec = next(row for row in index["files"] if row["label"] == label)
    raw = (
        ROOT / "imports/official/appearance-membership" / spec["manifest"]
    ).read_bytes()
    document = json.loads(raw)
    if (
        spec["appearances_sha256"] != appearances_sha256
        or hashlib.sha256(raw).hexdigest() != spec["manifest_sha256"]
        or document["appearances_sha256"] != appearances_sha256
        or appearance_membership.entries_digest(document["entries"])
        != document["entries_sha256"]
    ):
        raise ValueError("DONOR_MEMBERSHIP_DIGEST")
    return {row[0]: row for row in document["entries"]}


def build_qualified_donor_record(
    sources, donor, entry, binding, definition, taxonomized, memberships
):
    item_id = entry["source_item_id"]
    key = entry["item_key"]
    expected_binding = {
        "disposition": "EXACT",
        "external_id": str(item_id),
        "identity_namespace": "ots/item_server_id",
        "source_key": "oteryn:source.crystalserver",
        "source_revision": donor_census.DONOR_COMMIT,
        "target": {"family": "Item", "key": key, "revision": "definition-r1"},
    }
    if (
        binding != expected_binding
        or (definition or {}).get("identity") != expected_binding["target"]
        or (definition or {}).get("materializable") is not False
    ):
        raise ValueError("DONOR_EXACT_IDENTITY_BINDING")
    old, current = (membership.get(item_id) for membership in memberships)
    if (
        old is None
        or current is None
        or old[1] != current[1]
        or key != f"oteryn:item.tibia.i{current[0]}"
        or old[1] != entry["identity_projection_sha256"]
        or old[2] != entry["donor_record_sha256"]
        or current[2] != entry["official_current_record_sha256"]
    ):
        raise ValueError("DONOR_APPEARANCE_IDENTITY_CONTINUITY")
    if taxonomized or item_id in sources["items"]:
        raise ValueError("DONOR_EXISTING_OWNER_PRECEDENCE")
    xml = donor["items"].get(item_id)
    appearance = donor["appearances"].get(item_id)
    if xml is None or appearance is None or xml["name"] != entry["name"]:
        raise ValueError("DONOR_SOURCE_NAME_OR_PRESENCE")
    merged = {**donor["items"], **sources["items"]}
    result = donor_census.classify_donor_item(
        item_id,
        donor,
        merged,
        sources["wiki_family_fallback"],
        sources["owner_family_decisions"],
        sources["identity_index"],
    )
    if (result.get("outcome"), result.get("owner"), result.get("reason")) != (
        "routed",
        entry["owner"],
        entry["reason"],
    ):
        raise ValueError("DONOR_REVIEWED_ROUTE_CHANGED")
    meta = {
        "engine": "crystal",
        "repository": donor_census.DONOR_REPOSITORY,
        "revision": donor_census.DONOR_COMMIT,
        "profile": DONOR_PROFILE,
    }
    record = world_objects.build_record(
        entry["owner"], item_id, key, entry["reason"], xml, appearance, meta
    )
    errors = world_objects.validate_record(record)
    pointer = world_objects.routed_item_pointer(key, entry["owner"])
    if errors or world_objects.validate_routed_item_pointer(pointer):
        raise ValueError(f"DONOR_RECORD_VALIDATION:{item_id}:{errors[:3]}")
    if (
        hashlib.sha256(world_objects.canonical_bytes(record)).hexdigest()
        != entry["validated_record_sha256"]
    ):
        raise ValueError("DONOR_REVIEWED_RECORD_CHANGED")
    return record


def donor_outputs(sources, donor_source, existing_keys, base_counts):
    qualification = qualified_donor_routes()
    donor = donor_census.load_donor_artifacts(donor_source)
    bindings = json.loads(
        (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
    )
    by_key = {}
    for binding in bindings["bindings"]:
        key = binding["target"]["key"]
        if key in by_key:
            raise ValueError("DONOR_DUPLICATE_BINDING")
        by_key[key] = binding
    definitions = {}
    for path in (ROOT / "content/items/definitions").glob("items-*.json"):
        for row in json.loads(path.read_bytes())["records"]:
            identity = row["definition"]["identity"]
            definitions[identity["key"]] = row["definition"]
    taxonomy = {
        row["target"]["key"]
        for row in json.loads(
            (ROOT / "content/items/taxonomy/items.json").read_bytes()
        )["records"]
    }
    memberships = (
        membership_projection(
            "crystal-donor-00ce02a5", donor_census.DONOR_APPEARANCES_SHA256
        ),
        membership_projection(
            "client-15.30", qualification["current_appearances_sha256"]
        ),
    )
    records = {family: [] for family in CATALOGUES}
    for entry in qualification["records"]:
        key = entry["item_key"]
        if key in existing_keys:
            raise ValueError("DONOR_EXISTING_WORLD_OWNER")
        record = build_qualified_donor_record(
            sources,
            donor,
            entry,
            by_key.get(key),
            definitions.get(key),
            key in taxonomy,
            memberships,
        )
        records[entry["owner"]].append(record)
    files = {}
    for family, rows in records.items():
        directory, prefix, _notes = CATALOGUES[family]
        files[
            f"{directory}/{prefix}-{base_counts[family]:05d}-{base_counts[family] + len(rows) - 1:05d}.json"
        ] = world_objects.canonical_bytes({"family": family, "records": rows}) + b"\n"
    all_records = sorted(
        (record for rows in records.values() for record in rows),
        key=lambda record: record["provenance"]["source_item_id"],
    )
    census = {
        "schema": "OTERYN_WORLD_OBJECT_TERRAIN_DONOR_CENSUS/v1",
        "source": all_records[0]["provenance"]["source"],
        "artifact_digests": donor["artifact_digests"],
        "qualification_sha256": DONOR_QUALIFICATION_SHA256,
        "records": {
            "total": len(all_records),
            "by_family": {family: len(rows) for family, rows in records.items()},
            "digest_sha256": hashlib.sha256(
                b"".join(
                    world_objects.canonical_bytes(row) + b"\n" for row in all_records
                )
            ).hexdigest(),
        },
    }
    return files, world_objects.canonical_bytes(census) + b"\n", records


def outputs(sources, donor_source=None, include_official=False):
    """Return ({relative path: bytes} for both catalogues, census bytes)."""
    records = {family: [] for family in CATALOGUES}
    result = world_objects.build_census(
        sources, on_record=lambda family, record: records[family].append(record)
    )
    files = {}
    for family, (directory, prefix, _notes) in CATALOGUES.items():
        rows = records[family]
        for start in range(0, len(rows), SHARD):
            chunk = rows[start : start + SHARD]
            name = f"{prefix}-{start:05d}-{start + len(chunk) - 1:05d}.json"
            document = {"family": family, "records": chunk}
            files[f"{directory}/{name}"] = (
                world_objects.canonical_bytes(document) + b"\n"
            )
        index = marker(directory, family, len(rows), result["source"])
        files[f"{directory}/index.json"] = (
            json.dumps(index, indent=2, ensure_ascii=False) + "\n"
        ).encode("utf-8")
    if donor_source is not None:
        existing_keys = {
            row["provenance"]["item_pointer"]["key"]
            for rows in records.values()
            for row in rows
        }
        supplemental, census, donor_records = donor_outputs(
            sources,
            donor_source,
            existing_keys,
            {family: len(rows) for family, rows in records.items()},
        )
        files.update(supplemental)
        files[DONOR_CENSUS.relative_to(ROOT).as_posix()] = census
        for family, (directory, _prefix, _notes) in CATALOGUES.items():
            index_path = f"{directory}/index.json"
            index = json.loads(files[index_path])
            index["notes"] += (
                f" Additionally {len(donor_records[family])} qualified donor records at "
                f"{donor_census.DONOR_REPOSITORY}@{donor_census.DONOR_COMMIT[:8]}; "
                f"total {len(records[family]) + len(donor_records[family])}."
            )
            files[index_path] = (
                json.dumps(index, indent=2, ensure_ascii=False) + "\n"
            ).encode("utf-8")
    if include_official:
        import official_corpses

        prior = [
            row
            for path, data in files.items()
            if path.startswith("content/world/")
            for row in json.loads(data).get("records", [])
        ]
        existing = {row["provenance"]["item_pointer"]["key"] for row in prior}
        count = sum(row["identity"]["family"] == "WorldObject" for row in prior)
        files.update(official_corpses.outputs(existing, count))
        index_path = "content/world/objects/index.json"
        index = json.loads(files[index_path])
        index["notes"] += (
            f" Additionally 40 qualified official client 15.30 corpse records; "
            f"total {count + 40} WorldObject records."
        )
        files[index_path] = (
            json.dumps(index, indent=2, ensure_ascii=False) + "\n"
        ).encode()
    return files, world_objects.census_document_bytes(result)


def committed_files():
    found = set()
    for directory, _prefix, _notes in CATALOGUES.values():
        for path in (ROOT / directory).iterdir():
            found.add(path.relative_to(ROOT).as_posix())
    return found


def donor_shards():
    baseline = json.loads(world_objects.DEFAULT_SAMPLE.read_bytes())["records"][
        "by_family"
    ]
    reviewed = qualified_donor_routes()["records"]
    limits = {
        family: baseline[family] + sum(row["owner"] == family for row in reviewed)
        for family in CATALOGUES
    }
    return {
        path
        for family, (directory, prefix, _notes) in CATALOGUES.items()
        for path in (ROOT / directory).glob(f"{prefix}-*.json")
        if path.stem.split("-")[1].isdigit()
        and baseline[family] <= int(path.stem.split("-")[1]) < limits[family]
    }


def official_shards():
    import official_corpses

    baseline = json.loads(world_objects.DEFAULT_SAMPLE.read_bytes())["records"][
        "by_family"
    ]
    start = baseline["WorldObject"] + sum(
        row["owner"] == "WorldObject" for row in qualified_donor_routes()["records"]
    )
    path = (
        ROOT
        / f"content/world/objects/objects-{start:05d}-{start + len(official_corpses.IDS) - 1:05d}.json"
    )
    return {path} if path.exists() else set()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--source", type=Path, required=True, help="pinned Crystal checkout (ff7ede5)"
    )
    parser.add_argument(
        "--donor-source", type=Path, help="pinned Crystal donor checkout (00ce02a5)"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="rebuild in memory and fail on any difference from the committed files",
    )
    parser.add_argument(
        "--official-client",
        action="store_true",
        help="include the closed 40-record official client 15.30 corpse corpus",
    )
    args = parser.parse_args(argv)

    if args.donor_source is None and (donor_shards() or DONOR_CENSUS.exists()):
        raise SystemExit(
            "--donor-source is required for the supplemental donor catalogue"
        )
    sources = world_objects.engine_items.load_engine_sources("crystal", args.source)
    import official_corpses

    if (
        official_corpses.CENSUS.exists() or official_shards()
    ) and not args.official_client:
        raise SystemExit(
            "--official-client is required for the official corpse catalogue"
        )
    files, census = outputs(sources, args.donor_source, args.official_client)
    census_path = world_objects.DEFAULT_SAMPLE
    if args.check:
        drift = sorted(
            path
            for path, data in files.items()
            if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
        )
        extra = sorted(committed_files() - set(files))
        if census_path.read_bytes() != census:
            drift.append(census_path.relative_to(ROOT).as_posix())
        if drift or extra:
            print(
                json.dumps({"drift": drift[:10], "extra": extra[:10]}), file=sys.stderr
            )
            return 1
        print(json.dumps({"check": "PASS", "files": len(files)}))
        return 0

    for path in committed_files() - set(files):
        (ROOT / path).unlink()
    for path, data in files.items():
        (ROOT / path).write_bytes(data)
    census_path.write_bytes(census)
    counts = {family: 0 for family in CATALOGUES}
    for path in files:
        if not path.endswith("index.json") and path.startswith("content/world/"):
            family = "Terrain" if "/terrain/" in path else "WorldObject"
            counts[family] += len(json.loads(files[path])["records"])
    print(json.dumps({"records": counts, "files": len(files)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
