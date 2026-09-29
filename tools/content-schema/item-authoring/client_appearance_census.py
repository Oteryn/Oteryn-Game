"""Census the client appearance ids that no pinned engine defines (task B3).

Reads the owner's local 15.30 client `appearances.dat` (reference evidence only: the file
itself is never committed, only the facts extracted here) and every pinned engine
`items.xml` this lane reads -- Crystal `ff7ede5`, the Crystal donor `00ce02a5` and
Canary `47dfd51f`. For each appearance object id that none of those `items.xml` define,
it records what Oteryn's own rules can decide from the appearance alone, reusing
`engine_items`'s helpers unchanged:

- `routed`: a non-Item owner (`non_item_route`: corpse, placeholder slot;
  `immovable_non_item_route`: Terrain ground/border, WorldObject immovable) -- listed
  as sorted id arrays per owner/reason;
- `pickupable_candidate`: `flags.take` is set, so a portable Item is plausible, but no
  engine defines its family, weight or any attribute;
- `unclassified`: everything else.

It mints no identity. Rows carry a provisional, clearly non-canonical key
(`client_key`); assigning Oteryn keys, a family and facts beyond the appearance is a
later, separately reviewed step. Upstream engines are sources of facts only.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

from donor_census import DONOR_APPEARANCES_SHA256, DONOR_COMMIT, DONOR_ITEMS_XML_SHA256
from engine_items import (
    ENGINES,
    default_artifact_digests,
    immovable_non_item_route,
    load_appearance_objects,
    load_items_xml,
    non_item_route,
    read_verified_artifact,
)

ROOT = Path(__file__).resolve().parent
DEFAULT_SAMPLE = ROOT / "samples" / "client-appearance-census-15-30-2dfa943b.json"

# The owner's local 15.30 client appearances, as pinned by the owner-supplied client
# asset manifest (`OTERYN_CLIENT_ASSET_MANIFEST/v1`, 2026-09-27).
CLIENT_VERSION = "15.30"
CLIENT_APPEARANCES_SHA256 = (
    "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
)
CLIENT_APPEARANCES_BYTES = 5_017_996
# The 15.30 client's own new appearance-id range (donor census, owner-measured).
NEW_APPEARANCE_FIRST_ID = 52977
FACT_FLAGS = (
    "flags.take",
    "flags.cumulative",
    "flags.container",
    "flags.usable",
    "flags.unmove",
    "flags.unpass",
    "flags.hang",
    "flags.corpse",
    "flags.player_corpse",
    "clothes.slot",
    "market.category",
)


def client_key(item_id, digest=CLIENT_APPEARANCES_SHA256):
    """Provisional, clearly non-canonical key: never `oteryn:*`."""
    return f"client:tibia@{CLIENT_VERSION}-{digest[:8]}:item/{item_id}"


def load_client_appearances(path, digest=CLIENT_APPEARANCES_SHA256, size=None):
    data = Path(path).read_bytes()
    expected_size = CLIENT_APPEARANCES_BYTES if size is None else size
    if len(data) != expected_size:
        raise SystemExit(
            f"{path}: {len(data)} bytes, expected {expected_size} (pinned client file)"
        )
    actual = hashlib.sha256(data).hexdigest()
    if actual != digest:
        raise SystemExit(f"{path}: sha256 {actual}, expected {digest}")
    return load_appearance_objects(data)


def pinned_items_xml_digest(engine):
    profile = ENGINES[engine]["profile"]
    return default_artifact_digests(profile, engine)["data/items/items.xml"]


def engine_defined_ids(crystal_source, donor_source, canary_source, digests=None):
    """Union of every pinned `items.xml` id (Crystal, donor, Canary). `digests`
    overrides the pinned SHA-256 per engine; used only by fixture tests."""
    digests = digests or {}
    inputs = (
        ("crystal", crystal_source, digests.get("crystal")),
        ("donor", donor_source, digests.get("donor", DONOR_ITEMS_XML_SHA256)),
        ("canary", canary_source, digests.get("canary")),
    )
    inputs = tuple(
        (name, source, digest or pinned_items_xml_digest(name))
        for name, source, digest in inputs
    )
    defined = set()
    counts = {}
    for name, source, digest in inputs:
        data, _mode = read_verified_artifact(source, "data/items/items.xml", digest)
        ids = set(load_items_xml(data.decode("utf-8")))
        counts[name] = len(ids)
        defined |= ids
    return defined, counts


def classify(appearance):
    flags = dict(appearance["flags"])
    route = non_item_route(None, {}, flags) or immovable_non_item_route(flags)
    if route is not None:
        return {"outcome": "routed", "owner": route[0], "reason": route[1]}
    if flags.get("flags.take"):
        return {"outcome": "pickupable_candidate"}
    return {"outcome": "unclassified"}


def fact_row(item_id, appearance, result, digest):
    flags = appearance["flags"]
    row = {"client_key": client_key(item_id, digest), "outcome": result["outcome"]}
    if appearance.get("name"):
        row["name"] = appearance["name"]
    if appearance.get("description"):
        row["description"] = appearance["description"]
    row["flags"] = {flag: flags[flag] for flag in FACT_FLAGS if flag in flags}
    return row


def build_census(appearances, defined, engine_counts, digest=CLIENT_APPEARANCES_SHA256):
    undefined = sorted(set(appearances) - defined)
    routed = defaultdict(list)
    rows = {}
    outcome = Counter()
    by_range = Counter()
    for item_id in undefined:
        result = classify(appearances[item_id])
        outcome[result["outcome"]] += 1
        by_range["new_15_30" if item_id >= NEW_APPEARANCE_FIRST_ID else "older"] += 1
        if result["outcome"] == "routed":
            routed[f"{result['owner']}:{result['reason']}"].append(item_id)
        else:
            rows[str(item_id)] = fact_row(item_id, appearances[item_id], result, digest)
    return {
        "schema": "OTERYN_CLIENT_APPEARANCE_CENSUS/v1",
        "client": {
            "version": CLIENT_VERSION,
            "appearances_sha256": digest,
            "note": (
                "Owner's local client file, reference evidence only; never committed. "
                "Only the facts below are extracted."
            ),
        },
        "engines": {
            "crystal": ENGINES["crystal"]["revision"],
            "donor": DONOR_COMMIT,
            "canary": ENGINES["canary"]["revision"],
            "items_xml_ids": dict(sorted(engine_counts.items())),
            "donor_appearances_sha256": DONOR_APPEARANCES_SHA256,
        },
        "scope": (
            "Every client appearance object id that none of the pinned items.xml "
            "define. No Oteryn identity is minted; client_key is provisional."
        ),
        "totals": {
            "client_appearance_ids": len(appearances),
            "engine_defined_ids": len(defined),
            "undefined_ids": len(undefined),
            "undefined_by_range": dict(sorted(by_range.items())),
        },
        "outcome": dict(sorted(outcome.items())),
        "routed": {
            name: {"items": len(ids), "ids": ids}
            for name, ids in sorted(routed.items())
        },
        "items": rows,
    }


def census_document_bytes(result):
    return (
        json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    ).encode("utf-8")


MEMBERSHIP_SCHEMA = "OTERYN_CLIENT_APPEARANCE_MEMBERSHIP/v1"


def membership_document_bytes(
    ids, digest=CLIENT_APPEARANCES_SHA256, size=CLIENT_APPEARANCES_BYTES, name=None
):
    """Deterministic id-only membership manifest: object ids and digests, nothing else."""
    ids = sorted(set(ids))
    ids_bytes = json.dumps(ids, sort_keys=True, separators=(",", ":")).encode("utf-8")
    document = {
        "schema": MEMBERSHIP_SCHEMA,
        "client_version": CLIENT_VERSION,
        "appearances_file": name or f"appearances-{digest}.dat",
        "appearances_sha256": digest,
        "appearances_bytes": size,
        "object_count": len(ids),
        "max_id": ids[-1] if ids else None,
        "ids_sha256": hashlib.sha256(ids_bytes).hexdigest(),
        "ids": ids,
    }
    return (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode(
        "utf-8"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--appearances", type=Path, required=True)
    parser.add_argument("--crystal-source", type=Path)
    parser.add_argument("--donor-source", type=Path)
    parser.add_argument("--canary-source", type=Path)
    parser.add_argument(
        "--membership-out",
        type=Path,
        help="write only the id-only membership manifest (needs no engine sources)",
    )
    parser.add_argument("--out", type=Path, default=DEFAULT_SAMPLE)
    parser.add_argument(
        "--check",
        action="store_true",
        help="regenerate in memory and diff against --out; exits 1 on drift",
    )
    args = parser.parse_args()

    appearances = load_client_appearances(args.appearances)
    if args.membership_out is not None:
        args.membership_out.write_bytes(
            membership_document_bytes(appearances, name=args.appearances.name)
        )
        print(json.dumps({"membership_out": str(args.membership_out)}))
        return
    if not (args.crystal_source and args.donor_source and args.canary_source):
        parser.error(
            "--crystal-source, --donor-source and --canary-source are required "
            "unless --membership-out is given"
        )
    defined, counts = engine_defined_ids(
        args.crystal_source, args.donor_source, args.canary_source
    )
    candidate = census_document_bytes(build_census(appearances, defined, counts))
    if args.check:
        if args.out.read_bytes() != candidate:
            raise SystemExit(f"client appearance census drift against {args.out}")
        print(json.dumps({"check": "ok", "out": str(args.out)}))
        return
    args.out.write_bytes(candidate)
    print(json.dumps({"out": str(args.out)}))


if __name__ == "__main__":
    main()
