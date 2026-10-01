#!/usr/bin/env python3
"""StarterKit authoring: populate content/starter/ from the authored starter templates.

STARTER-CONTENT-1 (#162), under STARTER-BACKPACK-0 §4 and §4.2 (#1387) and the architect's family
ruling on #162. Tree-first, like RewardClaim: this tool writes the StarterKit family and its
registration in content/project.json, content/manifest.json and content/content.lock.json.
The family is data only; the server does not read content/ yet.

Rules:
- A record names a key, an Item definition key and revision, a quantity and a destination
  (§4). It belongs to a starter template label (#162 Q3 b). STARTER-1 binds each configured
  `starter_template_revision` to a label and fails closed on an unknown label.
- Every Item is an A12 key (`oteryn:item.tibia.i<id>`) that resolves in content/items, is
  materializable with a known stack class, and fits the quantity. The `container_slot`
  destination takes one non-stackable Item with a known capacity and a container-slot
  equipment pattern (D114), at most once per template.
- §4.2: a label is immutable once it has records. `sealed-templates.json` is append-only: each
  label with records must match its sealed digest, and a sealed label never disappears. A new
  set of records goes into a new label, sealed with `seal --label`.

`content` writes the family and registration (`--check` verifies it byte for byte);
`committed_errors()` runs the rules on the committed files.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE_REL = "tools/content-schema/starter-kit-authoring/templates.json"
SEALS_REL = "tools/content-schema/starter-kit-authoring/sealed-templates.json"
ITEMS_GLOB = "content/items/definitions/items-*.json"
FAMILY = "StarterKit"
CONTENT_DIR = "content/starter/"
INDEX_PATH = CONTENT_DIR + "index.json"
SHARD_SIZE = 100
REVISION = "starter-kit-r1"
ITEM_REVISION = "definition-r1"
INDEX_SCHEMA = "OTERYN_FAMILY_INDEX/v1"
SHARD_SCHEMA = "OTERYN_STARTER_KIT_SHARD/v1"
SOURCE_SCHEMA = "OTERYN_STARTER_KIT_AUTHORING/v1"
SEALS_SCHEMA = "OTERYN_STARTER_KIT_SEALS/v1"
A12_ITEM = re.compile(r"^oteryn:item\.tibia\.i[1-9][0-9]*$")
RECORD_KEY = re.compile(r"^oteryn:starter\.[a-z0-9_]+$")
LABEL = re.compile(r"^oteryn:starter-template\.[a-z0-9_-]+$")
DESTINATIONS = ("container_slot",)
KNOWN_STACK = ("NonStackable", "StackCapable")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def compact(payload) -> str:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    )


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_items() -> dict[str, dict]:
    items = {}
    for path in sorted(ROOT.glob(ITEMS_GLOB)):
        for record in load_json(path)["records"]:
            definition = record["definition"]
            items[definition["identity"]["key"]] = definition
    return items


def known(field: dict | None):
    """The value of a KNOWN semantic field, else None."""
    if isinstance(field, dict) and field.get("state") == "KNOWN":
        return field["value"]
    return None


def container_slot_problem(definition: dict) -> str | None:
    """Why `definition` cannot be a main backpack (D114), or None."""
    semantics = definition.get("semantics", {})
    container = known(semantics.get("container"))
    if not container or not known(container.get("capacity")):
        return "container_slot Item needs a known container capacity"
    equipment = known(semantics.get("equipment"))
    patterns = known(equipment.get("patterns")) if equipment else None
    if not patterns or not any(
        known(pattern.get("primary_slot")) == "CONTAINER" for pattern in patterns
    ):
        return "container_slot Item needs a container-slot equipment pattern"
    if definition.get("stack_class") != "NonStackable":
        return "container_slot Item must be non-stackable"
    return None


def admission_problem(record: dict, items: dict) -> str | None:
    """Why `record` fails admission (§4), or None."""
    ref = record["item"]
    if ref["family"] != "Item" or not A12_ITEM.match(ref["key"]):
        return f"{ref['key']} is not an A12 Item key"
    if ref["revision"] != ITEM_REVISION:
        return f"{ref['key']} revision {ref['revision']} is unknown"
    definition = items.get(ref["key"])
    if not definition:
        return f"{ref['key']} has no Item record in content/items"
    if (
        not definition.get("materializable")
        or definition.get("stack_class") not in KNOWN_STACK
    ):
        return f"{ref['key']} is not materializable with a known stack class"
    if not isinstance(record["quantity"], int) or record["quantity"] < 1:
        return "quantity must be a positive integer"
    if definition["stack_class"] == "NonStackable" and record["quantity"] != 1:
        return "a NonStackable Item takes quantity 1"
    if record["destination"] not in DESTINATIONS:
        return f"unknown destination {record['destination']}"
    if record["destination"] == "container_slot":
        if record["quantity"] != 1:
            return "the container slot holds one Item"
        return container_slot_problem(definition)
    return None


def template_digest(records: list) -> str:
    """The seal of one label: its records in key order, as emitted."""
    rows = sorted(
        (row["definition"] for row in records), key=lambda r: r["identity"]["key"]
    )
    return sha256(compact(rows).encode())


def build_records(source: dict) -> list:
    records = []
    for template in sorted(source["templates"], key=lambda t: t["label"]):
        for row in sorted(template["records"], key=lambda r: r["key"]):
            records.append(
                {
                    "definition": {
                        "destination": row["destination"],
                        "identity": {"key": row["key"], "revision": REVISION},
                        "item": {
                            "family": "Item",
                            "key": row["item"]["key"],
                            "revision": row["item"]["revision"],
                        },
                        "quantity": row["quantity"],
                        "template": template["label"],
                    }
                }
            )
    return records


def by_label(records: list) -> dict[str, list]:
    labels: dict[str, list] = {}
    for row in records:
        labels.setdefault(row["definition"]["template"], []).append(row)
    return labels


def validate(records: list, items: dict, seals: dict) -> list[str]:
    """Rules on the records; each error names the record or label."""
    errors = []
    seen = set()
    slots: dict[str, int] = {}
    for row in records:
        record = row["definition"]
        key, label = record["identity"]["key"], record["template"]
        where = f"{label} {key}"
        if not LABEL.match(label):
            errors.append(f"{where}: label is not oteryn:starter-template.<name>")
        if not RECORD_KEY.match(key):
            errors.append(f"{where}: key is not oteryn:starter.<name>")
        if (label, key) in seen:
            errors.append(f"{where}: a key appears once per template")
        seen.add((label, key))
        problem = admission_problem(record, items)
        if problem:
            errors.append(f"{where}: {problem}")
        if record["destination"] == "container_slot":
            slots[label] = slots.get(label, 0) + 1
    for label, count in sorted(slots.items()):
        if count > 1:
            errors.append(f"{label}: at most one container_slot record per template")
    errors += seal_errors(records, seals)
    return errors


def seal_errors(records: list, seals: dict) -> list[str]:
    """§4.2: each label with records matches its seal; sealed labels never go away."""
    errors = []
    labels = by_label(records)
    for label, rows in sorted(labels.items()):
        seal = seals.get(label)
        if seal is None:
            errors.append(f"{label}: has records but is not sealed (seal --label)")
        elif seal != {
            "record_count": len(rows),
            "records_sha256": template_digest(rows),
        }:
            errors.append(
                f"{label}: records differ from its seal; a label with records is immutable"
                " (§4.2), author a new label instead"
            )
    for label in sorted(set(seals) - set(labels)):
        errors.append(f"{label}: sealed label lost its records (§4.2)")
    return errors


def load_seals() -> dict:
    document = load_json(ROOT / SEALS_REL)
    if document.get("schema") != SEALS_SCHEMA:
        raise ValueError(f"{SEALS_REL}: schema")
    seals = {}
    for row in document["seals"]:
        if row["label"] in seals:
            raise ValueError(f"{SEALS_REL}: {row['label']} sealed twice")
        seals[row["label"]] = {
            "record_count": row["record_count"],
            "records_sha256": row["records_sha256"],
        }
    return seals


def load_source() -> tuple[bytes, dict]:
    source_bytes = (ROOT / SOURCE_REL).read_bytes()
    source = json.loads(source_bytes)
    if source.get("schema") != SOURCE_SCHEMA:
        raise ValueError(f"{SOURCE_REL}: schema")
    return source_bytes, source


def content_files(source_bytes: bytes, records: list) -> dict[str, str]:
    files, shards = {}, []
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{CONTENT_DIR}starter-kits-{start:05d}-{end:05d}.json"
        shards.append(path)
        files[path] = compact(
            {
                "family": FAMILY,
                "records": chunk,
                "schema": SHARD_SCHEMA,
                "shard": {
                    "count": len(chunk),
                    "end": end,
                    "index": index,
                    "start": start,
                },
            }
        )
    files[INDEX_PATH] = compact(
        {
            "authoring_source": {
                "path": SOURCE_REL,
                "schema": SOURCE_SCHEMA,
                "sha256": sha256(source_bytes),
            },
            "family": FAMILY,
            "population_state": "POPULATED",
            "record_count": len(records),
            "schema": INDEX_SCHEMA,
            "shards": shards,
            "templates": {
                label: len(rows) for label, rows in sorted(by_label(records).items())
            },
        }
    )
    return files


def registered(
    project: dict, manifest: dict, lock: dict, count: int, paths: list[str]
) -> tuple:
    """The three registration documents with the StarterKit family registered."""
    project, manifest, lock = map(copy.deepcopy, (project, manifest, lock))
    if FAMILY not in project["migrated_families"]:
        project["migrated_families"] = project["migrated_families"] + [FAMILY]
    manifest["families"][FAMILY] = {"records": count, "index": INDEX_PATH}
    managed = {row["path"] for row in manifest["managed_files"]}
    managed = {p for p in managed if not p.startswith(CONTENT_DIR)} | set(paths)
    manifest["managed_files"] = [{"path": p} for p in sorted(managed)]
    lock["family_counts"][FAMILY] = count
    return project, manifest, lock


def committed_errors() -> list[str]:
    """The rules on the committed family, against the current content/items."""
    index = load_json(ROOT / INDEX_PATH)
    records = [
        row for shard in index["shards"] for row in load_json(ROOT / shard)["records"]
    ]
    errors = validate(records, load_items(), load_seals())
    if len(records) != index["record_count"]:
        errors.append(
            f"index record_count {index['record_count']} != {len(records)} records"
        )
    return errors


def generate() -> dict[str, str]:
    source_bytes, source = load_source()
    records = build_records(source)
    errors = validate(records, load_items(), load_seals())
    if errors:
        raise ValueError("\n".join(errors))
    outputs = content_files(source_bytes, records)
    names = ("project", "manifest", "content.lock")
    docs = [load_json(ROOT / f"content/{name}.json") for name in names]
    for name, doc in zip(
        names, registered(*docs, len(records), sorted(outputs)), strict=True
    ):
        outputs[f"content/{name}.json"] = compact(doc)
    return outputs


def content_command(check: bool) -> int:
    try:
        outputs = generate()
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    stale = []
    for rel, text in sorted(outputs.items()):
        path = ROOT / rel
        if path.is_file() and path.read_text(encoding="utf-8") == text:
            continue
        stale.append(rel)
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8", newline="\n")
    extra = (
        sorted(
            p.relative_to(ROOT).as_posix()
            for p in (ROOT / CONTENT_DIR).glob("*.json")
            if p.relative_to(ROOT).as_posix() not in outputs
        )
        if (ROOT / CONTENT_DIR).is_dir()
        else []
    )
    if check:
        ok = not stale and not extra
        print(
            "starter kit content check: "
            + ("ok" if ok else f"FAIL, stale {stale} extra {extra}")
        )
        return 0 if ok else 1
    for rel in extra:
        (ROOT / rel).unlink()
    print(f"starter kit content: wrote {stale}, removed {extra}")
    return 0


def seal_command(label: str) -> int:
    """Append the seal of a new label; a sealed label is never resealed (§4.2)."""
    _, source = load_source()
    rows = by_label(build_records(source)).get(label)
    if not rows:
        print(f"{label}: no records to seal", file=sys.stderr)
        return 1
    path = ROOT / SEALS_REL
    document = load_json(path)
    if any(row["label"] == label for row in document["seals"]):
        print(f"{label}: already sealed; author a new label instead", file=sys.stderr)
        return 1
    document["seals"].append(
        {
            "label": label,
            "record_count": len(rows),
            "records_sha256": template_digest(rows),
        }
    )
    path.write_text(
        json.dumps(document, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"sealed {label}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    content = sub.add_parser(
        "content", help="write (or --check) the family and registration"
    )
    content.add_argument("--check", action="store_true")
    seal = sub.add_parser("seal", help="seal a new template label (append-only)")
    seal.add_argument("--label", required=True)
    args = parser.parse_args()
    if args.command == "content":
        return content_command(args.check)
    if args.command == "seal":
        return seal_command(args.label)
    return 2


if __name__ == "__main__":
    sys.exit(main())
