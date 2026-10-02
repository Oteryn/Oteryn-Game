"""Closed successor8; reuse accepted documented-default source qualification."""

import argparse
import json
import os
from pathlib import Path

import appearance_membership as membership
import check_stack_default_historical_context as context

ROOT = Path(os.environ.get("OTERYN_DEFAULT8_ROOT", Path(__file__).resolve().parents[3]))

base = context.base
IDS = frozenset({23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339})
PROOF = (
    "docs/agents/evidence/"
    "OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json"
)
PROOF_SHA = "5f9a767cee631c73cd2ce534c04787dc82091ab15b8ad65add15e8b7cbc67bd4"
RECEIPT = (
    "docs/agents/evidence/"
    "OTV2-20261002-item-stack-default-successor8-current-parent-receipt-v1.json"
)
RECEIPT_SHA = "PENDING_ACTUAL_WEAPON103_ALLOCATION"
OUTPUT = (
    "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json"
)


def qualify(row, definition, binding, obj, routed, observations, pages, cutoff):
    source = row["source"]
    iid = source["source_item_id"]
    if (
        iid not in IDS
        or obj.get("id") != iid
        or binding != row["binding"]
        or definition.get("identity") != binding["target"]
        or base.sha(source["content"].encode()) != source["content_sha256"]
        or obj != row["official_object"]
    ):
        raise ValueError("successor8 literal source/current identity drift")
    why = base.reasons(
        source, definition, binding, obj, routed, observations, pages, cutoff
    )
    if why:
        raise ValueError(f"successor8 {iid}: {why}")
    return {
        "item_key": source["item_key"],
        "target": definition["identity"],
        "source_item_id": iid,
        "stackable": False,
        "binding": binding,
        "appearance_id": iid,
        "object_sha256": row["official_object_sha256"],
        "wiki": {
            k: source[k]
            for k in (
                "page_id",
                "revision_id",
                "revision_timestamp",
                "title",
                "content_sha256",
                "capture_sha256",
                "capture_url",
                "capture_time",
            )
        },
    }


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    receipt = json.loads(base.checked(root, RECEIPT, RECEIPT_SHA))
    rows = proof["records"]
    if (
        len(rows) != 8
        or {r["source"]["source_item_id"] for r in rows} != IDS
        or receipt["closed_ids"] != sorted(IDS)
        or receipt["historical_source_proof_sha256"] != PROOF_SHA
        or receipt["purpose"]
        != "CURRENT_PARENT_QUALIFICATION_NOT_HISTORICAL_NATIVE_AUTHORITY"
        or len(receipt["actual_published_parent"]) != 40
    ):
        raise ValueError("successor8 closed scope/actual parent receipt drift")
    for pin in proof["input_pins"] + proof["world_owner_inputs"]:
        base.checked(root, pin["path"], pin["sha256"])
    for pin in receipt["source_dependency_pins"]:
        base.checked(root, pin["path"], pin["sha256"])
    original, objects, bound, wiki, pages, definitions, routed = context.current_inputs(
        root
    )
    originals = {
        r["source_item_id"]
        for r in json.loads((root / base.OUTPUT.relative_to(base.ROOT)).read_bytes())[
            "promotions"
        ]
    }
    historical = json.loads(
        base.checked(root, context.historical.PROOF, context.historical.PROOF_SHA)
    )
    if IDS & (
        originals | {f["current"]["source_item_id"] for f in historical["frames"]}
    ):
        raise ValueError("successor8 overlaps immutable original cohorts")
    current_raw = {}
    for tag, raw in base.protobuf_fields(
        base.checked(root, base.CLIENT, base.CLIENT_SHA)
    ):
        if tag == 1:
            obj = base.decode_appearance_object(raw)
            if obj.get("id") in IDS:
                if obj["id"] in current_raw:
                    raise ValueError("duplicate successor8 official identity")
                current_raw[obj["id"]] = base.sha(raw)
    _, admitted = membership.load_admitted(
        root / "imports/official/appearance-membership"
    )
    members = {
        label: {entry[0]: entry for entry in table["entries"]}
        for label, table in admitted.items()
    }
    selected = {
        r["source_item_id"]: r
        for r in original["records"]
        if r["source_item_id"] in IDS
    }
    promotions = []
    for row in rows:
        source = row["source"]
        iid, key = source["source_item_id"], source["item_key"]
        projection = membership.sha256_hex(
            membership.canonical_bytes(membership.identity_projection(objects[iid]))
        )
        if (
            selected[iid] != source
            or current_raw[iid] != row["official_object_sha256"]
            or members["client-15.30"][iid] != row["current_member"]
            or members["crystal-ff7ede5"][iid] != row["crystal_member"]
            or projection != row["identity_projection_sha256"]
            or row["retained_observations"] != wiki[iid]
        ):
            raise ValueError("successor8 exact source/member/observation drift")
        promotions.append(
            qualify(
                row,
                definitions[key],
                bound[key],
                objects[iid],
                routed,
                wiki[iid],
                pages,
                original["qualification_cutoff"],
            )
        )
    return {
        "schema": "OTERYN_ITEM_STACK_FALSE_PROMOTION/v1",
        "source_policy": "DERIVED_DOCUMENTED_TEMPLATE_DEFAULT",
        "sources": {
            "source_proof": {"path": PROOF, "sha256": PROOF_SHA},
            "current_parent_receipt": {"path": RECEIPT, "sha256": RECEIPT_SHA},
            "actual_published_parent": receipt["actual_published_parent"],
            "compiler_sha256": base.sha(Path(__file__).read_bytes()),
        },
        "counts": {"promotions": 8, "holds": 0},
        "promotions": promotions,
        "holds": [],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = context.canonical(build())
    if args.check:
        if (ROOT / OUTPUT).read_bytes() != raw:
            raise SystemExit("successor8 packet drift")
    else:
        (ROOT / OUTPUT).write_bytes(raw)


if __name__ == "__main__":
    main()
