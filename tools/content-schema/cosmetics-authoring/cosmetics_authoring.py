"""D1093 canonical permanent-cosmetics authoring/materialization; no implicit source promotion.

Build only from an explicit complete authored document and promoted, value-bound field evidence.
No source-name/sex/starter/fallback inference, network access, registry edit or default population.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SCHEMA = HERE / "cosmetics.schema.json"
AUTHORING = "content/cosmetics/outfits/authoring.json"
OUTPUT = "content/cosmetics/outfits/native-cosmetics.json"
MANIFEST = "imports/cipsoft-appearances/outfits/manifest.json"
MAX_DOCUMENT = 8 * 1024 * 1024
MAX_SOURCE = 32 * 1024 * 1024
MAX_SECTION = 16 * 1024 * 1024


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate cosmetics member: " + key)
        result[key] = value
    return result


def read(path: Path, limit=MAX_DOCUMENT) -> bytes:
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if not data or len(data) > limit:
        raise ValueError("cosmetics input bounds: " + str(path))
    return data


def parse(data: bytes):
    def no_float(_):
        raise ValueError("cosmetics requires integer JSON numbers")
    return json.loads(data, object_pairs_hook=unique, parse_float=no_float, parse_constant=no_float)


def promotion_paths(document):
    result = {"/source_manifest_sha256", "/source_appearances_sha256", "/palette",
              "/fallbacks/female", "/fallbacks/male"}
    if "grant_mappings" in document:
        result.add("/grant_mappings")
    for family, fields in [
        ("outfits", ["key", "revision", "female", "male", "addon_count", "starter_female", "starter_male",
                     "starter_addons_female", "starter_addons_male", "premium", "unlock_sources",
                     "source"]),
        ("mounts", ["key", "revision", "look", "speed_bonus", "premium", "source"]),
    ]:
        for index in range(len(document[family])):
            result.update(f"/{family}/{index}/{field}" for field in fields)
    return result


def pointer(document, path):
    for part in path.lstrip("/").split("/"):
        document = document[int(part)] if isinstance(document, list) else document[part]
    return document


def validate(authoring):
    Draft202012Validator(parse(SCHEMA.read_bytes())).validate(authoring)
    def ascii_strings(value):
        if isinstance(value, float):
            raise ValueError("canonical cosmetics requires integer JSON numbers")
        if isinstance(value, str):
            if (any(not 32 <= ord(c) <= 126 for c in value) or value.upper() == "UNKNOWN"):
                raise ValueError("canonical cosmetics requires known printable ASCII text")
        elif isinstance(value, list):
            for child in value:
                ascii_strings(child)
        elif isinstance(value, dict):
            for child in value.values():
                ascii_strings(child)
    def atom(value):
        parts = re.split(r"[:./_-]", value.lower())
        if (not value or any(not 33 <= ord(c) <= 126 for c in value)
                or any(x in value.lower() for x in ["fixture", "synthetic", "evidence", "test-only"])
                or "test" in parts):
            raise ValueError("native cosmetics identity/revision contains nonproduction marker")
    ascii_strings(authoring)
    atom(authoring["profile"]["revision"])
    document = authoring["document"]
    for identity in [document["palette"], *document["outfits"], *document["mounts"]]:
        atom(identity["key"])
        atom(identity["revision"])
    expected = promotion_paths(document)
    seen = set()
    for promotion in authoring["promotions"]:
        path = promotion["path"]
        if (path not in expected or path in seen
                or canonical(promotion["value"]) != canonical(pointer(document, path))):
            raise ValueError("missing, duplicated or mismatched cosmetic promotion")
        seen.add(path)
    if seen != expected:
        raise ValueError("cosmetics has incomplete per-field promotion")
    keys = set()
    outfits = {}
    for row in document["outfits"]:
        if row["key"] in keys:
            raise ValueError("duplicate cosmetics key")
        keys.add(row["key"])
        outfits[row["key"]] = row
        allowed = (1 << row["addon_count"]) - 1
        if (row["starter_addons_female"] | row["starter_addons_male"]) & ~allowed:
            raise ValueError("starter addon mask exceeds authored addon count")
    for row in document["mounts"]:
        if row["key"] in keys:
            raise ValueError("duplicate cosmetics key")
        keys.add(row["key"])
        if row["speed_bonus"] != authoring["profile"]["mount_speed_bonus"]:
            raise ValueError("Reference ruleset mount speed disagreement")
    grants = set()
    mounts = {row["key"] for row in document["mounts"]}
    for mapping in document.get("grant_mappings", []):
        if mapping["grant_key"] in grants:
            raise ValueError("duplicate cosmetics grant key")
        grants.add(mapping["grant_key"])
        target = mapping["target"]
        if target["kind"] == "Mount":
            valid = target["mount_key"] in mounts
        else:
            outfit = outfits.get(target["outfit_key"])
            valid = outfit is not None and (target["kind"] == "Outfit"
                    or target["addon"] <= outfit["addon_count"])
        if not valid:
            raise ValueError("unknown cosmetics grant target")
    for sex in ["female", "male"]:
        fallback = document["fallbacks"][sex]
        row = outfits.get(fallback["outfit_key"])
        if row is None or not row["starter_" + sex]:
            raise ValueError("fallback must name an explicit starter for its sex")


def printable(value, limit):
    return (type(value) is str and 0 < len(value.encode()) <= limit
            and not any(ord(c) < 32 or 127 <= ord(c) < 160 for c in value))


def manifest_shape(manifest):
    """The exact staged manifest shape the Rust `Manifest` decoder requires (deny unknown, all required)."""
    keys = {"schema", "authority", "client_version", "family", "identity_namespace", "source",
            "top_level_field", "record_count", "excluded_top_level", "inputs", "files"}
    inputs = manifest.get("inputs") if isinstance(manifest, dict) else None
    appearances = inputs.get("appearances") if isinstance(inputs, dict) else None
    files = manifest.get("files") if isinstance(manifest, dict) else None
    if (not isinstance(manifest, dict) or set(manifest) != keys
            or not all(type(manifest[k]) is str for k in
                       ["schema", "authority", "client_version", "family", "identity_namespace", "source"])
            or type(manifest["top_level_field"]) is not int or type(manifest["record_count"]) is not int
            or set(inputs) != {"appearances"} or not isinstance(appearances, dict)
            or set(appearances) != {"file", "sha256"} or not printable(appearances["file"], 1024)
            or type(appearances["sha256"]) is not str
            or not isinstance(files, list)
            or any(not isinstance(f, dict) or set(f) != {"path", "records", "sha256"}
                   or type(f["path"]) is not str or type(f["records"]) is not int
                   or type(f["sha256"]) is not str for f in files)):
        raise ValueError("cosmetics source manifest shape")


def observation_shape(observation):
    """The exact staged shard shape the Rust `Observations`/`Observation` decoders require."""
    record_keys = {"source_id", "source_index", "source_record_sha256", "flags", "frame_groups"}
    if (not isinstance(observation, dict) or set(observation) != {"schema", "family", "records"}
            or type(observation["schema"]) is not str or type(observation["family"]) is not str
            or not isinstance(observation["records"], list)
            or any(not isinstance(r, dict) or set(r) != record_keys for r in observation["records"])):
        raise ValueError("cosmetics source observation shape")


def build(authoring, manifest_path: Path) -> bytes:
    """Materialize exact source bytes with the authored facts; no filesystem output."""
    validate(authoring)
    document = authoring["document"]
    raw = read(manifest_path)
    manifest = parse(raw)
    manifest_shape(manifest)
    if (sha(raw) != document["source_manifest_sha256"]
            or manifest["schema"] != "OTERYN_CIPSOFT_APPEARANCE_OUTFIT_SOURCE_MANIFEST/v1"
            or manifest["family"] != "Outfit" or manifest["top_level_field"] != 2
            or manifest["authority"] != "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys or gameplay promotion"
            or manifest["identity_namespace"] != "cipsoft/appearance-outfits"
            or manifest["source"] != "CIPSOFT_CLIENT_ASSETS"
            or manifest["client_version"] != authoring["client_version"]
            or manifest["inputs"]["appearances"]["sha256"] != document["source_appearances_sha256"]):
        raise ValueError("cosmetics source manifest identity/digest disagreement")
    looks = {}
    names = set()
    shards = []
    total = len(raw)
    ordinal = 0
    for pin in manifest["files"]:
        name = pin["path"]
        if (not re.fullmatch(r"[A-Za-z0-9_.-]+\.json", name) or name in names):
            raise ValueError("invalid or duplicate cosmetics source shard")
        names.add(name)
        data = read(manifest_path.parent / name)
        total += len(data)
        if total > MAX_SOURCE or sha(data) != pin["sha256"]:
            raise ValueError("cosmetics source shard digest/bounds")
        observation = parse(data)
        observation_shape(observation)
        if (observation["schema"] != "OTERYN_CIPSOFT_APPEARANCE_OUTFIT_SOURCE_OBSERVATIONS/v1"
                or observation["family"] != "Outfit" or type(pin["records"]) is not int
                or len(observation["records"]) != pin["records"]):
            raise ValueError("cosmetics source record count/family")
        for record in observation["records"]:
            identity = record["source_id"]
            if (type(identity) is not int or not 1 <= identity <= 65535
                    or identity in looks or type(record["source_index"]) is not int
                    or record["source_index"] != ordinal):
                raise ValueError("duplicate, invalid or reordered cosmetics source ID")
            digest = record["source_record_sha256"]
            if (type(digest) is not str or not re.fullmatch(r"[0-9a-f]{64}", digest)
                    or digest == "0" * 64):
                raise ValueError("cosmetics source record digest must be lowercase nonzero sha256")
            looks[identity] = digest
            ordinal += 1
        shards.append({"path": name, "json": data.decode("utf-8")})
    if (not names or type(manifest["record_count"]) is not int
            or not 1 <= ordinal == manifest["record_count"] <= 65535):
        raise ValueError("cosmetics source manifest total")
    refs = [row[sex] for row in document["outfits"] for sex in ["female", "male"]]
    refs += [row["look"] for row in document["mounts"]]
    if any(looks.get(ref["id"]) != ref["source_record_sha256"] for ref in refs):
        raise ValueError("cosmetic look does not match exact source record")
    result = canonical({"schema": "OTERYN_NATIVE_COSMETICS/v1", "availability": "authored",
                        "authoring": authoring, "source_manifest": raw.decode("utf-8"),
                        "source_shards": shards})
    if len(result) > MAX_SECTION:
        raise ValueError("cosmetics native section exceeds 16MiB")
    return result


def materialized_outputs(root: Path):
    """Called by the real tree materializer. Missing facts create no content, pin or fallback."""
    source = root / AUTHORING
    if not source.exists():
        if (root / OUTPUT).exists():
            raise ValueError("native cosmetics output has missing authoring source")
        return {}
    return {OUTPUT: build(parse(read(source)), root / MANIFEST)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authoring", required=True, type=Path)
    parser.add_argument("--manifest", default=ROOT / MANIFEST, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = build(parse(read(args.authoring)), args.manifest)
    if args.check:
        if not args.output.is_file() or args.output.read_bytes() != result:
            raise ValueError("cosmetics output missing or different")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(result)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError) as error:
        sys.exit(str(error))
