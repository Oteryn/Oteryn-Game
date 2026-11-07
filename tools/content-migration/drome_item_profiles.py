"""W09: source-qualified native profiles for the Cube and all twenty rewards.

The immutable Crystal 201189f source is decoded by the existing native Item
importer. Canonical identities and bindings are retained; this does not promote
or modify the historical World reference records. No raw appearance assets are
redistributed. The captured overlay contains normalized server facts only.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CUBE_ID = 36827
REWARD_IDS = tuple(range(36723, 36743))
ITEM_IDS = (*REWARD_IDS, CUBE_ID)
KEYS = tuple(f"oteryn:item.tibia.i{item}" for item in ITEM_IDS)
PATH = "tools/content-schema/native-gameplay/drome-item-profiles.json"
BINDINGS_PATH = "imports/crystalserver/bindings/items.json"
BINDINGS_SHA = "1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a"
SOURCE_PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
# GitHub tree/file reads on SOURCE_PIN; capture re-proves each complete blob.
SOURCE_BLOBS = {
    "data/items/appearances.dat": "37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0",
    "data/items/items.xml": "ea4196beae8f77be898f05de89db81de134886b6",
    "data/scripts/actions/items/drome_cube.lua": "b9a020b79a08a54501c7e2b35be53323770f7c3d",
    "src/items/items.cpp": "bdb7b4b48aadfeb9e031311ca107aadb4ee6e30b",
    "src/items/items.hpp": "b502ec621701f1cebfe7e509207649081ee3e470",
    "src/protobuf/appearances.proto": "7069b5ead685929c0a79cb6e8b0664bbe8f19a3a",
}
SCHEMA = "OTERYN_NATIVE_DROME_ITEM_PROFILES/v1"
NA = {"state": "NOT_APPLICABLE"}


def _fail(key: str, reason: str) -> None:
    raise ValueError(f"DROME_ITEM_PROFILE_{reason}: {key}")


def _canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def _known(value: object, key: str, field: str) -> object:
    if not isinstance(value, dict) or value.get("state") != "KNOWN" or "value" not in value:
        _fail(key, f"UNKNOWN_{field}")
    return value["value"]


def verify_blob(path: str, raw: bytes) -> None:
    actual = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
    if actual != SOURCE_BLOBS.get(path):
        _fail(path, "SOURCE_BLOB")


def verify_pool(raw: bytes) -> None:
    text = raw.decode("utf-8")
    cube = re.findall(r"local\s+dromeCube\s*=\s*(\d+)", text)
    rewards = re.findall(r"\{\s*id\s*=\s*(\d+)\s*,\s*count\s*=\s*(\d+)\s*\}", text)
    if cube != [str(CUBE_ID)] or rewards != [(str(item), "1") for item in REWARD_IDS]:
        _fail("pool", "SOURCE_POOL")


def validate_records(records: list[dict], bindings: dict[str, dict]) -> None:
    """Validate qualified native admission, never infer it from identity membership."""
    if [r.get("production_definition", {}).get("production_key") for r in records] != list(KEYS):
        _fail("pool", "CLOSED_SET")
    for item, key, record in zip(ITEM_IDS, KEYS, records):
        binding = bindings.get(str(item))
        identity = {"family": "Item", "key": key, "revision": "definition-r1"}
        definition = {"family": "Item", "production_key": key, "revision_ref": "definition-r1"}
        if (
            binding is None or binding.get("target") != identity
            or binding.get("disposition") != "EXACT"
            or record.get("production_binding") != binding
            or record.get("production_definition") != definition
            or record.get("production_binding_qualification") is not None
        ):
            _fail(key, "IDENTITY_OR_BINDING")
        admission = record.get("admission", {})
        if (admission.get("materializable") is not True
                or admission.get("legal_destinations") != ["CharacterInventory", "Ground"]):
            _fail(key, "NOT_MATERIALIZABLE")
        semantics = record["semantics"]
        physical = _known(semantics.get("physical"), key, "PHYSICAL")
        if not isinstance(physical, dict):
            _fail(key, "PHYSICAL_SHAPE")
        for field in ("movable", "pickupable"):
            if _known(physical.get(field), key, field.upper()) is not True:
                _fail(key, "NOT_PORTABLE")
        weight = _known(physical.get("weight"), key, "WEIGHT")
        if type(weight) is not int or not 0 <= weight <= 0xFFFF_FFFF:
            _fail(key, "WEIGHT_BOUND")
        if semantics.get("container") != NA or semantics.get("temporal") != NA:
            _fail(key, "NOT_A_PORTABLE_LEAF")
        stack_class = admission.get("stack_class")
        stack = _known(semantics.get("stack"), key, "STACK")
        if not isinstance(stack, dict):
            _fail(key, "STACK_SHAPE")
        stackable = _known(stack.get("stackable"), key, "STACKABLE")
        if stack_class == "StackCapable":
            maximum = _known(stack.get("stack_max"), key, "STACK_MAXIMUM")
            if stackable is not True or type(maximum) is not int or not 1 <= maximum <= 255:
                _fail(key, "STACK_BOUND")
        elif (stack_class != "NonStackable" or stackable is not False
              or stack.get("stack_max") != NA):
            _fail(key, "STACK_CLASS")
        presentation = _known(semantics.get("presentation"), key, "PRESENTATION")
        if not isinstance(presentation, dict):
            _fail(key, "PRESENTATION_SHAPE")
        name = _known(presentation.get("name"), key, "NAME")
        if not isinstance(name, str) or not name or len(f"You have received a {name}!".encode()) > 255:
            _fail(key, "NAME_BOUND")


def validate_overlay(document: dict, bindings: dict[str, dict]) -> list[dict]:
    if (document.get("schema") != SCHEMA or document.get("source_revision") != SOURCE_PIN
            or document.get("identity_binding_sha256") != BINDINGS_SHA):
        _fail("overlay", "SOURCE_PIN")
    sources = document.get("sources", [])
    if ([row.get("path") for row in sources] != list(SOURCE_BLOBS)
            or any(row.get("git_blob") != SOURCE_BLOBS[row["path"]]
                   or re.fullmatch(r"[0-9a-f]{64}", row.get("sha256", "")) is None
                   for row in sources)):
        _fail("overlay", "SOURCE_CLOSURE")
    records = document["records"]
    validate_records(records, bindings)
    if hashlib.sha256(_canonical(records)).hexdigest() != document.get("records_sha256"):
        _fail("overlay", "RECORDS_DIGEST")
    return records


def capture(crystal: Path, root: Path = ROOT) -> dict:
    """Read only immutable source objects; the checkout HEAD is never changed."""
    sys.path.insert(0, str(root / "tools/content-schema/native-gameplay"))
    import build_source_item_profiles as source
    captured = {}
    for path in SOURCE_BLOBS:
        raw = subprocess.check_output(["git", "-C", str(crystal), "show", f"{SOURCE_PIN}:{path}"])
        verify_blob(path, raw)
        captured[path] = raw
    verify_pool(captured["data/scripts/actions/items/drome_cube.lua"])
    # These verified primary loader defaults are exactly the generic mapper's inputs.
    hpp = captured["src/items/items.hpp"].decode()
    cpp = captured["src/items/items.cpp"].decode()
    if (any(text not in hpp for text in ("int32_t weight = 0", "uint8_t stackSize = 100",
                                       "uint16_t maxItems = 8", "uint32_t decayTime = 0"))
            or any(text not in cpp for text in ("iType.stackable = object.flags().cumulative()",
                       "iType.pickupable = object.flags().take()",
                       "iType.movable = object.flags().unmove() == false"))):
        _fail("loader", "SOURCE_DEFAULTS")
    xml = captured["data/items/items.xml"]
    appearances = captured["data/items/appearances.dat"]
    index = source.xml_index(xml)
    if not set(ITEM_IDS) <= index.keys():
        _fail("pool", "MISSING_XML")
    binding_raw = (root / BINDINGS_PATH).read_bytes()
    bindings = source.binding_index(binding_raw, expected_sha=BINDINGS_SHA)
    objects = source.engine_items.load_appearance_objects(appearances)
    physical = {r["server_item_id"]: r for r in source.policies(appearances, xml, set(ITEM_IDS))}
    rows = [source.profile(item, index[item], objects[item], physical[item], bindings, None)
            for item in ITEM_IDS]
    # bind_profiles requires qualified XML membership for every materializable item.
    bound = source.bind_profiles({"schema": "OTERYN_NATIVE_ITEM_PROFILES/v1", "records": rows},
                                 bindings, set(index))
    records = bound["records"]
    document = {
        "schema": SCHEMA,
        "producer": "tools/content-migration/drome_item_profiles.py",
        "source_revision": SOURCE_PIN,
        "identity_binding_sha256": hashlib.sha256(binding_raw).hexdigest(),
        "sources": [{"path": path, "git_blob": blob,
                     "sha256": hashlib.sha256(captured[path]).hexdigest()}
                    for path, blob in SOURCE_BLOBS.items()],
        "records_sha256": hashlib.sha256(_canonical(records)).hexdigest(),
        "records": records,
    }
    validate_overlay(document, bindings)
    return document


def admit(root: Path, generated: dict[str, bytes], profile_path: str,
          manifest_path: str, order, canonical) -> None:
    """Join the ordinary registration pipeline; preserve agreeing richer profiles."""
    raw_bindings = (root / BINDINGS_PATH).read_bytes()
    if hashlib.sha256(raw_bindings).hexdigest() != BINDINGS_SHA:
        _fail("bindings", "BINDING_DIGEST")
    bindings = {row["external_id"]: row for row in json.loads(raw_bindings)["bindings"]}
    incoming = validate_overlay(json.loads((root / PATH).read_bytes()), bindings)
    document = json.loads(generated[profile_path])
    present = {}
    for record in document["records"]:
        key = record["production_definition"]["production_key"]
        if key in KEYS:
            if key in present:
                _fail(key, "DUPLICATE_NATIVE")
            present[key] = record
    additions = []
    for record in incoming:
        key = record["production_definition"]["production_key"]
        existing = present.get(key)
        if existing is None:
            additions.append(copy.deepcopy(record))
            continue
        if (any(existing.get(field) != record[field] for field in
                ("production_definition", "production_binding", "admission"))
                or any(existing["semantics"].get(field) != record["semantics"][field]
                       for field in ("physical", "stack", "container", "temporal", "presentation"))):
            _fail(key, "NATIVE_CONFLICT")
    document["records"] = sorted(document["records"] + additions, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(raw).hexdigest()
    manifest_raw = canonical(manifest)
    generated[profile_path] = raw
    generated[manifest_path] = manifest_raw


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + "\n").encode()
    destination = ROOT / PATH
    if args.check:
        if destination.read_bytes() != raw:
            raise SystemExit("DROME_ITEM_PROFILE_CAPTURE_DRIFT")
    else:
        destination.write_bytes(raw)
    print(f"PASS Drome native source profiles={len(ITEM_IDS)} source={SOURCE_PIN}")
