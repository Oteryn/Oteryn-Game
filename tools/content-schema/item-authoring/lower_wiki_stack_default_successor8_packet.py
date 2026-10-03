"""Closed successor8; reuse accepted documented-default source qualification."""

import argparse
import json
from pathlib import Path

import appearance_membership as membership
import check_stack_default_historical_context as context

ROOT = Path(__file__).resolve().parents[3]

base = context.base
IDS = frozenset({23229, 23230, 23231, 23232, 23295, 23299, 23335, 23339})
PROOF = (
    "docs/agents/evidence/"
    "OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json"
)
PROOF_SHA = "f54f066cc186cce0829346664485d3a993d80ed9fc291096c2641008755fa990"
RECEIPT = (
    "docs/agents/evidence/"
    "OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json"
)
RECEIPT_SHA = "05f5ef964ff51b067b3d420227995556c74331522076e40a43848e4dcb168149"
OUTPUT = (
    "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json"
)


def own_id_frame(root, proof, frame):
    pin = frame["index"]
    if (
        pin["path"]
        != "imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json"
    ):
        raise ValueError(
            "own-ID index must use closed repository-relative replay route"
        )
    if (
        pin["sha256"] != proof["complete_own_id_index"]["sha256"]
        or pin["original_capture_path"] != proof["complete_own_id_index"]["path"]
    ):
        raise ValueError("complete own-ID index source transport drift")
    index = json.loads(base.checked(root, pin["path"], pin["sha256"]))
    if (
        index["status"] != "COMPLETE_CUTOFF_OWN_ID_INDEX"
        or index["cutoff"] != proof["qualification_cutoff"]
    ):
        raise ValueError("complete own-ID cutoff/frame drift")
    parts = {}
    for pin in frame["parts"]:
        if (
            pin["path"]
            != "imports/tibiawiki/source-evidence/stack-default-successor8/global-infobox-object-source-part-098.json"
        ):
            raise ValueError(
                "own-ID part must use closed repository-relative replay route"
            )
        original_path = pin["original_capture_path"]
        if original_path in parts:
            raise ValueError("duplicate captured part transport")
        parts[original_path] = (
            pin["sha256"],
            json.loads(base.checked(root, pin["path"], pin["sha256"])),
        )
    used = set()
    for row in proof["records"]:
        source = row["source"]
        refs = index["by_own_itemid_integer_mention"].get(
            str(source["source_item_id"]), []
        )
        if len(refs) != 1 or context.canonical(refs) != context.canonical(
            row["complete_selected_own_id_references"]
        ):
            raise ValueError("own-ID complete reference uniqueness/coordinates drift")
        ref = refs[0]
        digest, part = parts[ref["capture_path"]]
        used.add(ref["capture_path"])
        if digest != ref["capture_sha256"]:
            raise ValueError("own-ID source part digest drift")
        page = part["pages"][ref["page_ordinal"]]
        box = page["infobox_objects"][ref["box_index"]]
        coordinates = (
            "page_id",
            "revision_id",
            "revision_timestamp",
            "content_sha256",
            "title",
        )
        fields = base.raw_parameters(source["content"])
        if (
            any(ref[k] != source[k] or page[k] != source[k] for k in coordinates)
            or source["revision_timestamp"] > proof["qualification_cutoff"]
            or base.sha(source["content"].encode()) != source["content_sha256"]
            or not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
            or context.canonical(box) != context.canonical(row["selected_own_raw_box"])
            or base.sha(box["raw"].encode()) != row["selected_own_raw_box_sha256"]
            or source["content"][box["offset"] : box["offset"] + len(box["raw"])]
            != box["raw"]
            or context.canonical(base.raw_parameters(box["raw"]))
            != context.canonical(fields)
            or context.canonical(fields) != context.canonical(box["parameter_values"])
            or fields.get("itemid") != [str(source["source_item_id"])]
            or "stackable" in fields
        ):
            raise ValueError("own-ID actual part/whole article/own raw box drift")
    if used != set(parts):
        raise ValueError("closed8 source part scope drift")


def qualify(
    row, definition, binding, obj, routed, observations, pages, cutoff, baseline=None
):
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
    baseline = baseline if baseline is not None else row["current_native_definition"]
    header_keys = ("kind", "stack_class", "materializable", "client_projection")
    presentation = definition.get("semantics", {}).get("presentation", {})
    name = presentation.get("value", {}).get("name", {})
    if (
        context.canonical({k: definition.get(k) for k in header_keys})
        != context.canonical({k: baseline.get(k) for k in header_keys})
        or presentation.get("state") != "KNOWN"
        or name.get("state") != "KNOWN"
    ):
        raise ValueError("successor8 current name/admission/class header drift")
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
    own_id_frame(root, proof, receipt["own_id_source_frame"])
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
    baseline = {r["item_key"]: r for r in receipt["selected_native_definitions"]}
    if len(baseline) != 8 or set(baseline) != {r["source"]["item_key"] for r in rows}:
        raise ValueError("successor8 baseline closed target drift")
    for row in rows:
        source = row["source"]
        witness = baseline[source["item_key"]]
        definition = witness["definition"]
        if (
            witness["source_item_id"] != source["source_item_id"]
            or base.sha(context.canonical(definition)) != witness["definition_sha256"]
            or definition["identity"] != row["binding"]["target"]
            or definition["semantics"]["stack"] != {"state": "UNKNOWN"}
        ):
            raise ValueError(
                "successor8 actual parent identity/unknown stack witness drift"
            )
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
                baseline[key]["definition"],
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
