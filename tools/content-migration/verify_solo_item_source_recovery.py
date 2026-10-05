"""Verify a quarantined item-source package without applying it to the server."""

import argparse
import gzip
import hashlib
import json
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
    with gzip.open(ability_path, "rt", encoding="utf8") as stream:
        for line in stream:
            require(
                len(line.encode("utf8")) <= receipt["maxima"]["serialized_bytes"],
                "ability row byte bound",
            )
            row = json.loads(line)
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
    with gzip.open(
        package / "source-batches/charges-leveldoor-observations.json.gz", "rt"
    ) as stream:
        packet = json.load(stream)
    require(
        packet["input_rows"] == 45721 and not packet["held"], "charge held/missing rows"
    )
    charge_targets, charge_rows = set(), 0
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
        cuts = set()
        for value in row["observations"]:
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
    return {
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
