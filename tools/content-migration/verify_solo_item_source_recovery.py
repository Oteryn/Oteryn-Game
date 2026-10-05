"""Verify a quarantined item-source package without applying it to the server."""

import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path

DEFAULT_PACKAGE = (
    Path(__file__).resolve().parents[2]
    / "imports/ots-native-admission/solo-source-item-recovery-20261005"
)
CHUNK = 65536
MAX_RAW_FILE = 192 * 1024**2
MAX_RAW_TOTAL = 1024**3


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def safe_path(root, relative):
    path = Path(relative)
    require(not path.is_absolute() and bool(path.parts), "absolute or empty path")
    require(all(part not in ("..", ".git") for part in path.parts), "unsafe path")
    result = root / path
    require(result.resolve().is_relative_to(root.resolve()), "path escapes package")
    require(not result.is_symlink(), "symlink member")
    return result


def file_sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def recovered_digest(path, declared_bytes):
    require(
        type(declared_bytes) is int and 0 <= declared_bytes <= MAX_RAW_FILE, "raw bound"
    )
    digest, total = hashlib.sha256(), 0
    with gzip.open(path, "rb") as stream:
        while True:
            data = stream.read(min(CHUNK, declared_bytes - total + 1))
            if not data:
                break
            total += len(data)
            require(total <= declared_bytes, "decompression exceeds declared size")
            digest.update(data)
    require(total == declared_bytes, "truncated decompressed member")
    return digest.hexdigest()


def verify_recovery(package):
    manifest = json.loads((package / "recovery-manifest.json").read_bytes())
    rows = manifest["files"]
    require(len(rows) == manifest["total_recovery_files"], "recovery inventory count")
    require(
        sum(row["origin"] == "UNCOMMITTED_WORKING_TREE" for row in rows) == 149,
        "missing original working files",
    )
    seen, total = set(), 0
    for row in rows:
        require(row["path"] not in seen, "duplicate recovery target")
        safe_path(package, row["path"])
        seen.add(row["path"])
        archive = safe_path(package, row["archive"])
        require(archive.stat().st_size == row["archive_bytes"], "compressed size")
        require(file_sha(archive) == row["archive_sha256"], "compressed digest")
        require(
            recovered_digest(archive, row["raw_bytes"]) == row["raw_sha256"],
            "recovered digest",
        )
        total += row["raw_bytes"]
        require(total <= MAX_RAW_TOTAL, "whole recovery bound")
    require(total == manifest["raw_bytes"], "whole recovery size")
    return {"files": len(rows), "uncommitted_files": 149, "raw_bytes": total}


OLD_REGISTRY = "apps/game-server/src/content/source-definition-registry.json"
CHARGE_KIND = "CHARGES_AND_LEVEL_DOOR"


def read_recovered_json(package, original_path):
    manifest = json.loads((package / "recovery-manifest.json").read_bytes())
    matches = [row for row in manifest["files"] if row["path"] == original_path]
    require(len(matches) == 1, "recovered member missing or duplicated")
    row = matches[0]
    require(
        type(row["raw_bytes"]) is int and 0 <= row["raw_bytes"] <= MAX_RAW_FILE,
        "raw bound",
    )
    with gzip.open(safe_path(package, row["archive"]), "rb") as stream:
        data = stream.read(row["raw_bytes"] + 1)
    require(len(data) == row["raw_bytes"], "recovered member size")
    require(hashlib.sha256(data).hexdigest() == row["raw_sha256"], "recovered digest")
    return json.loads(data)


def verify_definition_successor(package, charge_records):
    """Check the published successor against its census, charge batch and old registry."""
    census = json.loads(
        (package / "source-batches/definition-successor-census.json").read_bytes()
    )
    with gzip.open(
        package / "source-batches/definition-registry-successor.json.gz", "rt"
    ) as stream:
        successor = json.load(stream)
    old = read_recovered_json(package, OLD_REGISTRY)
    schema = "OTERYN_SOURCE_DEFINITION_OBSERVATIONS_CLOSED/v1"
    require(successor["schema"] == old["schema"] == schema, "successor schema")
    new_targets, old_targets = successor["targets"], old["targets"]
    require(old_targets.keys() <= new_targets.keys(), "successor dropped an old target")
    require(census["old_registry_inverse"] == "EXACT", "census inverse claim")
    observations, max_observations, max_assignments, added = 0, 0, 0, 0
    for key, revisions in new_targets.items():
        require(list(revisions) == ["definition-r1"], "successor revision domain")
        require(key != "oteryn:item.tibia.i901", "protected item 901")
        values = revisions["definition-r1"]
        prior = old_targets.get(key, {}).get("definition-r1", [])
        require(values[: len(prior)] == prior, "old observations not preserved")
        require(
            all(row["parameter"]["kind"] != CHARGE_KIND for row in prior),
            "old registry holds charge observations",
        )
        suffix = values[len(prior) :]
        require(suffix == charge_records.get(key), "suffix differs from charge batch")
        observations += len(values)
        added += len(suffix)
        max_observations = max(max_observations, len(values))
        max_assignments = max(
            max_assignments, *(len(row["ordered_assignments"]) for row in values)
        )
    require(
        old_targets.keys() == {key for key in old_targets if key in new_targets},
        "old target set",
    )
    require(added == sum(map(len, charge_records.values())), "suffix not exhaustive")
    require(
        census["targets"] == len(new_targets)
        and census["observations"] == observations
        and census["new_observations"] == added
        and census["max_observations"] == max_observations
        and census["max_assignments"] == max_assignments
        and observations - added
        == sum(len(v["definition-r1"]) for v in old_targets.values()),
        "successor census disagrees with data",
    )
    return {"targets": len(new_targets), "observations": observations, "added": added}


CHARGE_CUTS = {"CANARY_47DF", "CRYSTAL_FF7", "CRYSTAL_00CE"}
CHARGE_PHASE = "FRESH_CPP_PROTOBUF_THEN_FULL_ORDERED_XML_BEFORE_LUA"
CHARGE_ORIGINS = {"OWN_CPP_INITIALIZER", "EXPLICIT_ORDERED_XML"}
CHARGE_OBSERVATION_KEYS = {
    "current_world_owner_pointers",
    "external_item_id",
    "ordered_assignments",
    "parameter",
    "phase",
    "prototype_message_sha256",
    "source_cut",
    "source_group",
    "xml_record_ordinal",
    "xml_record_sha256",
}
CHARGE_PARAMETER_KEYS = {
    "charges_default_u32",
    "charges_origin",
    "kind",
    "level_door_origin",
    "level_door_u32",
}
OWNER_FAMILIES = {"Terrain", "WorldObject"}
SHA256_HEX = re.compile(r"[0-9a-f]{64}")


def is_uint(value, limit):
    return type(value) is int and 0 <= value <= limit


def verify_charge_observation(value):
    """Check one observation against the closed charge/level-door shape."""
    require(set(value) == CHARGE_OBSERVATION_KEYS, "charge observation keys")
    require(is_uint(value["external_item_id"], 4294967295), "charge item id")
    require(value["source_cut"] in CHARGE_CUTS, "charge source cut")
    require(value["phase"] == CHARGE_PHASE, "charge phase")
    require(isinstance(value["source_group"], str), "charge source group")
    require(is_uint(value["xml_record_ordinal"], 4294967295), "charge xml ordinal")
    for field in ("prototype_message_sha256", "xml_record_sha256"):
        digest = value[field]
        require(
            isinstance(digest, str) and SHA256_HEX.fullmatch(digest), "charge digest"
        )
    for pointer in value["current_world_owner_pointers"]:
        require(
            set(pointer) == {"family", "key", "revision"}
            and pointer["family"] in OWNER_FAMILIES
            and isinstance(pointer["key"], str)
            and pointer["revision"] == "definition-r1",
            "charge owner pointer",
        )
    parameter = value["parameter"]
    require(set(parameter) == CHARGE_PARAMETER_KEYS, "charge parameter keys")
    require(parameter["kind"] == CHARGE_KIND, "charge kind")
    for field in ("charges_default_u32", "level_door_u32"):
        require(is_uint(parameter[field], 4294967295), "own uint32 domain")
    origins = {parameter["charges_origin"], parameter["level_door_origin"]}
    require(origins <= CHARGE_ORIGINS, "charge origin")
    assignments = value["ordered_assignments"]
    require(bool(assignments) == ("EXPLICIT_ORDERED_XML" in origins), "charge origin")
    for assignment in assignments:
        require(
            set(assignment) == {"attribute_ordinal", "key", "value_lexeme"}
            and is_uint(assignment["attribute_ordinal"], 65535)
            and isinstance(assignment["key"], str)
            and isinstance(assignment["value_lexeme"], str),
            "charge assignment",
        )


def lexeme_bytes(value):
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "value_lexeme":
                yield len(item.encode("utf8"))
            else:
                yield from lexeme_bytes(item)
    elif isinstance(value, list):
        for item in value:
            yield from lexeme_bytes(item)


def ability_row_sizes(row, line):
    parameter = row["parameter"]
    return {
        "events": len(parameter["ordered_events"]),
        "direct_assignments": len(parameter["ordered_assignments"]),
        "lexeme_UTF8_bytes": max(lexeme_bytes(parameter), default=0),
        "serialized_bytes": len(line.encode("utf8")),
    }


def verify_batches(package):
    from jsonschema import Draft202012Validator

    schema = json.loads(
        (
            package / "source-batches/ability-literal-configuration.schema.json"
        ).read_bytes()
    )
    validator = Draft202012Validator(schema)
    catalogs = {}
    for path in (package / "source-batches/catalogs").glob("*-constructor.json"):
        catalogs[path.name.removesuffix("-constructor.json")] = file_sha(path)
    ability_path = package / "source-batches/ability-literal-observations.jsonl.gz"
    receipt = json.loads((package / "source-batches/ability-receipt.json").read_bytes())
    require(file_sha(ability_path) == receipt["output_sha256"], "ability output digest")
    targets, rows = set(), set()
    events, rows_with_events = 0, 0
    measured = dict.fromkeys(
        ("events", "direct_assignments", "lexeme_UTF8_bytes", "serialized_bytes"), 0
    )
    with gzip.open(ability_path, "rt", encoding="utf8") as stream:
        for line in stream:
            row = json.loads(line)
            for name, size in ability_row_sizes(row, line).items():
                measured[name] = max(measured[name], size)
            parameter, target, header = row["parameter"], row["target"], row["header"]
            validator.validate(parameter)
            require(
                target["family"] == "Item" and target["revision"] == "definition-r1",
                "ability identity domain",
            )
            require(target == row["own_binding"]["target"], "ability exact binding")
            require(
                target["key"] == f"oteryn:item.tibia.i{header['external_item_id']}",
                "ability external identity",
            )
            require(header["external_item_id"] != 901, "protected item 901")
            cut = parameter["source_cut"]
            require(
                parameter["constructor_catalog_sha256"] == catalogs[cut],
                "constructor catalog digest",
            )
            coordinate = (target["key"], target["revision"], cut)
            require(coordinate not in rows, "duplicate own ability row")
            rows.add(coordinate)
            targets.add(target["key"])
            events += len(parameter["ordered_events"])
            rows_with_events += bool(parameter["ordered_events"])
    require(len(rows) == 45721 and len(targets) == 33975, "ability closed cohort")
    require(receipt["maxima"] == measured, "ability receipt maxima")
    with gzip.open(
        package / "source-batches/charges-leveldoor-observations.json.gz", "rt"
    ) as stream:
        packet = json.load(stream)
    require(
        packet["input_rows"] == 45721 and not packet["held"], "charge held/missing rows"
    )
    charge_targets, charge_rows = set(), 0
    charge_records = {}
    for row in packet["records"]:
        target = row["target"]
        require(
            target["key"] not in charge_targets
            and target["key"] != "oteryn:item.tibia.i901",
            "duplicate/protected charge target",
        )
        require(
            target["family"] == "Item" and target["revision"] == "definition-r1",
            "charge identity domain",
        )
        charge_targets.add(target["key"])
        charge_records[target["key"]] = row["observations"]
        cuts = set()
        require(set(row) == {"observations", "target"}, "charge record keys")
        for value in row["observations"]:
            verify_charge_observation(value)
            require(value["source_cut"] not in cuts, "duplicate own charge cut")
            cuts.add(value["source_cut"])
            require(
                target["key"] == f"oteryn:item.tibia.i{value['external_item_id']}",
                "charge external identity",
            )
            require(
                value["parameter"]["kind"] == "CHARGES_AND_LEVEL_DOOR", "charge kind"
            )
            for field in ("charges_default_u32", "level_door_u32"):
                number = value["parameter"][field]
                require(
                    type(number) is int and 0 <= number <= 4294967295,
                    "own uint32 domain",
                )
            charge_rows += 1
    require(charge_targets == targets and charge_rows == 45721, "charge closed cohort")
    successor = verify_definition_successor(package, charge_records)
    return {
        "definition_successor": successor,
        "targets": len(targets),
        "ability_rows": len(rows),
        "ability_declaration_events": events,
        "ability_rows_with_events": rows_with_events,
        "charge_rows": charge_rows,
        "Native_promotions": 0,
    }


def verify_inventory(package):
    inventory = json.loads((package / "package-inventory.json").read_bytes())
    seen = set()
    for row in inventory:
        require(row["path"] not in seen, "duplicate package member")
        seen.add(row["path"])
        path = safe_path(package, row["path"])
        require(
            path.stat().st_size == row["bytes"] and file_sha(path) == row["sha256"],
            "package member digest",
        )
    actual = {
        str(path.relative_to(package)) for path in package.rglob("*") if path.is_file()
    }
    require(
        actual == seen | {"package-inventory.json"}, "unlisted/missing package member"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, default=DEFAULT_PACKAGE)
    args = parser.parse_args()
    package = args.package.resolve()
    verify_inventory(package)
    print(
        json.dumps(
            {
                "status": "VERIFIED_SOURCE_RECOVERY_NOT_NATIVE_ADMISSION",
                "recovery": verify_recovery(package),
                "source_batches": verify_batches(package),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
