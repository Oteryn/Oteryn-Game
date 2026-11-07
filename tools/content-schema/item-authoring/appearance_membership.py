"""Digest-bound membership manifests for every admitted CipSoft appearance file (A12 §4.1).

Decision `A12-ITEM-IDENTITY-TIBIA-ID-V1` makes the canonical Item key the Tibia id
(`oteryn:item.tibia.i<id>`) for every id in the *admitted CipSoft id set*: the union of
the appearance object ids of every admitted CipSoft `appearances.dat`. This tool emits one
manifest per admitted file, derived from its exact bytes:

- every appearance object id (family 1, `object`), each with two SHA-256 digests:
  - `identity projection`: canonical JSON of the identity-stable fields, the object class
    (always `object` here: outfits, effects and missiles are other families) and the
    CipSoft object name (`null` when the file carries none);
  - `record`: the object's exact protobuf message bytes, used only to note evolution;
- bound to the file's size and SHA-256, with an `entries_sha256` over the entry array.

`admitted.json` lists the manifests in admission order with each manifest's own digest,
and derives the union, the current set (the newest file) and the retired ids.

Only object ids and digests are written, never names, flags or sprites. The 15.30 file is
in the repository (`content/assets/files`); the three engine-pinned files are read from a
local checkout of the pinned engine revision (`--source <label>=<path to appearances.dat>`).
`--check` regenerates every manifest whose source is available (always 15.30) and verifies
every committed manifest's own digests and the index.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields

ROOT = Path(__file__).resolve().parent
REPO_ROOT = ROOT.parents[2]
OUT_DIR = REPO_ROOT / "imports/official/appearance-membership"
INDEX_NAME = "admitted.json"

MANIFEST_SCHEMA = "OTERYN_CIPSOFT_APPEARANCE_MEMBERSHIP_MANIFEST/v1"
INDEX_SCHEMA = "OTERYN_CIPSOFT_ADMITTED_APPEARANCE_SET/v1"
PROJECTION_PROFILE = "OTERYN_APPEARANCE_IDENTITY_PROJECTION/v1"
DECISION = "A12-ITEM-IDENTITY-TIBIA-ID-V1"

# Admission order: oldest first. The last entry is the newest admitted client, whose
# manifest is the current set. Each file is pinned by the corpus named in `pinned_by`.
ADMITTED = (
    {
        "label": "crystal-ff7ede5",
        "sha256": "6adb790d1064c2d31ffb2e5ce1a7aef376942ba672edea2adb6cafc620dd18f1",
        "bytes": 4_862_378,
        "pinned_by": "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a:data/items/appearances.dat",
    },
    {
        "label": "canary-47dfd51f",
        "sha256": "aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50",
        "bytes": 4_862_287,
        "pinned_by": "opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446:data/items/appearances.dat",
    },
    {
        "label": "crystal-donor-00ce02a5",
        "sha256": "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1",
        "bytes": 5_002_660,
        "pinned_by": "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f:data/items/appearances.dat",
    },
    {
        "label": "client-15.30",
        "sha256": "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2",
        "bytes": 5_017_996,
        "pinned_by": "client 15.30 (B3 pin, imports/official/client-assets/15.30/manifest.json)",
    },
)
IN_REPO_SOURCES = {
    "client-15.30": REPO_ROOT
    / "content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat",
}


def canonical_bytes(value) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "utf-8"
    )


def sha256_hex(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def manifest_name(spec) -> str:
    return f"appearances-{spec['sha256']}.json"


def identity_projection(record) -> dict:
    return {"class": "object", "name": record.get("name")}


def manifest_entries(data: bytes) -> list[list]:
    """`[[id, identity_projection_sha256, record_sha256], ...]` sorted by id."""
    entries = {}
    for number, value in protobuf_fields(data):
        if number != 1:
            continue
        record = decode_appearance_object(value)
        object_id = record.get("id")
        if not isinstance(object_id, int):
            raise SystemExit("appearance object without an id")
        if object_id in entries:
            raise SystemExit(f"duplicate appearance object id {object_id}")
        entries[object_id] = [
            object_id,
            sha256_hex(canonical_bytes(identity_projection(record))),
            sha256_hex(value),
        ]
    return [entries[object_id] for object_id in sorted(entries)]


def entries_digest(entries) -> str:
    return sha256_hex(json.dumps(entries, separators=(",", ":")).encode("utf-8"))


def manifest_bytes(spec, data: bytes) -> bytes:
    if len(data) != spec["bytes"] or sha256_hex(data) != spec["sha256"]:
        raise SystemExit(f"{spec['label']}: appearances.dat does not match its pin")
    entries = manifest_entries(data)
    return canonical_bytes(
        {
            "schema": MANIFEST_SCHEMA,
            "decision": DECISION,
            "appearances_sha256": spec["sha256"],
            "appearances_bytes": spec["bytes"],
            "pinned_by": spec["pinned_by"],
            "identity_projection_profile": PROJECTION_PROFILE,
            "entry_columns": ["id", "identity_projection_sha256", "record_sha256"],
            "object_count": len(entries),
            "entries_sha256": entries_digest(entries),
            "entries": entries,
        }
    )


def verify_manifest(spec, payload: bytes) -> dict:
    document = json.loads(payload)
    if canonical_bytes(document) != payload:
        raise SystemExit(f"{spec['label']}: manifest is not canonical JSON")
    entries = document["entries"]
    ids = [entry[0] for entry in entries]
    if (
        document["schema"] != MANIFEST_SCHEMA
        or document["appearances_sha256"] != spec["sha256"]
        or document["appearances_bytes"] != spec["bytes"]
        or document["object_count"] != len(entries)
        or document["entries_sha256"] != entries_digest(entries)
        or ids != sorted(set(ids))
    ):
        raise SystemExit(f"{spec['label']}: manifest self-check failed")
    return document


def index_bytes(manifests: dict[str, bytes]) -> bytes:
    id_sets = []
    files = []
    for order, spec in enumerate(ADMITTED, start=1):
        payload = manifests[spec["label"]]
        document = verify_manifest(spec, payload)
        id_sets.append({entry[0] for entry in document["entries"]})
        files.append(
            {
                "order": order,
                "label": spec["label"],
                "appearances_sha256": spec["sha256"],
                "manifest": manifest_name(spec),
                "manifest_sha256": sha256_hex(payload),
                "object_count": document["object_count"],
            }
        )
    union = set().union(*id_sets)
    current = id_sets[-1]
    return canonical_bytes(
        {
            "schema": INDEX_SCHEMA,
            "decision": DECISION,
            "key_rule": "oteryn:item.tibia.i<id>",
            "newest": ADMITTED[-1]["label"],
            "files": files,
            "union_count": len(union),
            "current_count": len(current),
            "retired_ids": sorted(union - current),
        }
    )


def load_admitted(out_dir: Path = OUT_DIR) -> tuple[dict, dict[str, dict]]:
    """Verified `(index, {label: manifest})` for callers such as the alias table tool."""
    manifests = {}
    payloads = {}
    for spec in ADMITTED:
        payload = (out_dir / manifest_name(spec)).read_bytes()
        manifests[spec["label"]] = verify_manifest(spec, payload)
        payloads[spec["label"]] = payload
    index_payload = (out_dir / INDEX_NAME).read_bytes()
    if index_payload != index_bytes(payloads):
        raise SystemExit("admitted.json does not match the committed manifests")
    return json.loads(index_payload), manifests


def parse_sources(values) -> dict[str, Path]:
    sources = dict(IN_REPO_SOURCES)
    labels = {spec["label"] for spec in ADMITTED}
    for value in values or []:
        label, _, path = value.partition("=")
        if label not in labels or not path:
            raise SystemExit(
                f"--source expects <label>=<path>, label in {sorted(labels)}"
            )
        sources[label] = Path(path)
    return sources


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--source", action="append", help="<label>=<path to appearances.dat>"
    )
    parser.add_argument("--out-dir", type=Path, default=OUT_DIR)
    parser.add_argument(
        "--check", action="store_true", help="verify instead of writing"
    )
    args = parser.parse_args()
    sources = parse_sources(args.source)

    payloads = {}
    regenerated = []
    for spec in ADMITTED:
        path = args.out_dir / manifest_name(spec)
        source = sources.get(spec["label"])
        if source is not None:
            payloads[spec["label"]] = manifest_bytes(spec, source.read_bytes())
            regenerated.append(spec["label"])
            if args.check and path.read_bytes() != payloads[spec["label"]]:
                raise SystemExit(f"manifest drift: {path}")
        elif args.check or path.is_file():
            payloads[spec["label"]] = path.read_bytes()
        else:
            raise SystemExit(f"{spec['label']}: no committed manifest and no --source")
    index = index_bytes(payloads)
    index_path = args.out_dir / INDEX_NAME
    if args.check:
        if index_path.read_bytes() != index:
            raise SystemExit(f"index drift: {index_path}")
    else:
        args.out_dir.mkdir(parents=True, exist_ok=True)
        for label, payload in payloads.items():
            spec = next(spec for spec in ADMITTED if spec["label"] == label)
            (args.out_dir / manifest_name(spec)).write_bytes(payload)
        index_path.write_bytes(index)
    print(json.dumps({"check": args.check, "regenerated": regenerated, "ok": True}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
