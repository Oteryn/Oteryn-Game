#!/usr/bin/env python3
"""Normalize the closed native world-change item families from retained local sources.

No network or raw-source publication. The output is a supplemental native overlay;
register_spell_families owns aggregate provider/manifest regeneration.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name("world-change-item-profiles.json")
IDS = frozenset({3583, 3990, 4103, 4104, 5963, 12059, 12060, 12061, 12062, 12063, 12065, 12077, 12368, 12369, 13853, 13854, 13855, 13856, 13974, 13976, 14018, 14020, 14051, 14052, 14053, 14054, 14055, 14064, 14074, 14163, 14164, 14255, 21752, 21803, 21804, 21805, 21812, 21893, 21894, 21895, 21896})
RESERVED = 12064
SCHEMA = "OTERYN_NATIVE_WORLD_CHANGE_ITEM_PROFILES/v1"
CORPUS = "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
XML_PATH = "data/items/items.xml"
XML_REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
XML_SHA256 = "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb"
APPEARANCES_SHA256 = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
APPEARANCES = f"content/assets/files/appearances-{APPEARANCES_SHA256}.dat"
MANIFEST = "imports/official/client-assets/15.30/manifest.json"


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def verify_sources(xml: bytes, appearances: bytes) -> None:
    if digest(xml) != XML_SHA256:
        raise ValueError("WORLD_CHANGE_XML_DIGEST")
    if digest(appearances) != APPEARANCES_SHA256:
        raise ValueError("WORLD_CHANGE_APPEARANCE_DIGEST")


def read_xml(source_root: Path) -> bytes:
    with tarfile.open(source_root / CORPUS, "r:gz") as archive:
        manifest = archive.getmember("corpus-manifest.json")
        if manifest.size > 32 * 1024 * 1024:
            raise ValueError("WORLD_CHANGE_CORPUS_MANIFEST_BOUND")
        stream = archive.extractfile(manifest)
        if stream is None:
            raise ValueError("WORLD_CHANGE_CORPUS_MANIFEST")
        files = json.load(stream)["files"]
        matches = [row for row in files if row["source"] == "crystalserver"
                   and row["revision"] == XML_REVISION and row["path"] == XML_PATH]
        if len(matches) != 1 or matches[0]["sha256"] != XML_SHA256:
            raise ValueError("WORLD_CHANGE_XML_SOURCE")
        member = archive.getmember(matches[0]["cache_path"])
        if member.size > 32 * 1024 * 1024:
            raise ValueError("WORLD_CHANGE_XML_BOUND")
        stream = archive.extractfile(member)
        if stream is None:
            raise ValueError("WORLD_CHANGE_XML_SOURCE")
        return stream.read()


def build(source_root: Path = ROOT) -> dict:
    # --source-root lets a sparse authoring checkout read immutable full source inputs.
    # OUTPUT always remains beside this script, never under the source-root override.
    sys.path.insert(0, str(source_root / "tools/content-schema/native-gameplay"))
    native = importlib.import_module("build_source_item_profiles")
    xml_raw = read_xml(source_root)
    appearance_raw = (source_root / APPEARANCES).read_bytes()
    verify_sources(xml_raw, appearance_raw)
    manifest_raw = (source_root / MANIFEST).read_bytes()
    manifest = json.loads(manifest_raw)
    matching = [row for row in manifest["files"]
                if row["name"] == Path(APPEARANCES).name]
    if len(matching) != 1 or matching[0]["sha256"] != APPEARANCES_SHA256:
        raise ValueError("WORLD_CHANGE_OFFICIAL_MANIFEST")
    xml = native.xml_index(xml_raw)
    objects = native.engine_items.load_appearance_objects(appearance_raw)
    if RESERVED in objects or xml[RESERVED].get("name") != "RESERVED SPRITE":
        raise ValueError("WORLD_CHANGE_RESERVED_SPRITE_SOURCE")
    if IDS - set(xml) or IDS - set(objects):
        raise ValueError("WORLD_CHANGE_SOURCE_ITEM_MISSING")
    physical = {row["server_item_id"]: row for row in
                native.policies(appearance_raw, xml_raw, IDS)}
    binding_raw = native.BINDINGS.read_bytes()
    bindings = native.binding_index(binding_raw)
    records = [native.profile(number, xml[number], objects[number], physical[number],
                              bindings, None) for number in sorted(IDS)]
    bound = native.bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",
                                  "records": records}, bindings, IDS)
    return {
        "schema": SCHEMA,
        "producer": "tools/content-schema/native-gameplay/build_world_change_item_profiles.py",
        "sources": [
            {"role": "crystal_items", "archive": CORPUS, "path": XML_PATH,
             "revision": XML_REVISION, "sha256": XML_SHA256},
            {"role": "official_appearances", "path": APPEARANCES,
             "sha256": APPEARANCES_SHA256, "manifest": MANIFEST,
             "manifest_sha256": digest(manifest_raw)},
        ],
        "binding_table_sha256": digest(binding_raw),
        "reserved_ids_excluded": [RESERVED],
        "source_rows": [
            {"id": number, "xml_name": xml[number].get("name"),
             "xml_attributes": native.attrs(xml[number]),
             "appearance_flags": objects[number]["flags"],
             "projected_physical": physical[number]}
            for number in sorted(IDS)
        ],
        "records": bound["records"],
    }


def encoded(document: dict) -> bytes:
    return (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = encoded(build(args.source_root))
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("WORLD_CHANGE_ITEM_PROFILE_DRIFT")
    else:
        OUTPUT.write_bytes(data)
