"""Select genuine historical source frames for seven documented stack defaults."""

import argparse
import json
from collections import defaultdict

import lower_wiki_stack_default_packet as base
from engine_items import load_appearance_objects

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_wiki_stack_historical_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261001-item-stack-historical-source-frame-v1.json"
PROOF_SHA = "0ecaedfe954961a3a26859a2cb05ecfed9ca9a513610e44dfba1a0260f1eff29"
BASE_SHA = "5969b009ed326991e6db310b3e3c48640ae5a5ac361c9c34f5619740bff8ce2e"
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261001-item-stack-historical-promotion-v1.json"
)
REVISIONS = {
    3337: 1202769,
    3365: 1147406,
    50239: 1177239,
    51271: 1178841,
    51272: 1178839,
    51273: 1178838,
    51274: 1178842,
}
COORDS = ("page_id", "revision_id", "content_sha256", "revision_timestamp")


def qualify(
    frame, definition, binding, obj, routed, current_observations, pages, cutoff
):
    current, historical = frame["current"], frame["historical"]
    for source in (current, historical):
        if base.sha(source["content"].encode()) != source["content_sha256"]:
            raise ValueError("raw source digest drift")
    if (
        historical["revision_timestamp"] > cutoff
        or current["revision_timestamp"] <= cutoff
    ):
        raise ValueError("historical/current source frame cutoff mismatch")
    if any(
        historical[k] != current[k]
        for k in ("page_id", "source_item_id", "item_key", "title")
    ):
        raise ValueError("historical/current source identity drift")
    current_fields, historical_fields = (
        base.raw_parameters(s["content"]) for s in (current, historical)
    )
    if any(
        current_fields.get(k) != historical_fields.get(k)
        for k in ("itemid", "name", "actualname")
    ):
        raise ValueError("historical/current own identity fields drift")
    # This checks the actual imported current observation, including all five coordinates.
    current_reasons = base.reasons(
        current, definition, binding, obj, routed, current_observations, pages, cutoff
    )
    if current_reasons != ["REVISION_AFTER_QUALIFICATION_CUTOFF"]:
        raise ValueError(f"unqualified current source frame: {current_reasons}")
    # Separate historical observation: coordinates and fields come only from genuine raw.
    historical_observation = {k: historical[k] for k in COORDS} | {
        "wiki_title": historical["title"],
        "fields": {k: v[0] for k, v in historical_fields.items()},
    }
    historical_reasons = base.reasons(
        historical,
        definition,
        binding,
        obj,
        routed,
        [historical_observation],
        pages,
        cutoff,
    )
    if historical_reasons:
        raise ValueError(f"unqualified historical source frame: {historical_reasons}")
    return {
        "item_key": current["item_key"],
        "stackable": False,
        "current": {k: current[k] for k in (*COORDS, "title")},
        "selected_historical": {k: historical[k] for k in (*COORDS, "title")},
        "historical_observation": historical_observation,
        "binding": binding,
        "appearance_id": obj["id"],
    }


def build(root=ROOT):
    base.checked(root, base.COMPILER, BASE_SHA)
    source_packet = base.build(
        root
    )  # Verifies decoder, client, current snapshot and source bridge.
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    parent = proof["default_source_proof"]
    inherited = json.loads(base.checked(root, parent["path"], parent["sha256"]))
    pinned = {s["source_item_id"]: s for s in inherited["records"]}
    objects = load_appearance_objects(base.checked(root, base.CLIENT, base.CLIENT_SHA))
    bound = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        inherited["bridge"]["source_revisions"],
    )
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for shard in json.loads((root / "content/items/index.json").read_text())[
            "shards"
        ]
        for r in json.loads((root / shard).read_text())["records"]
    }
    routed = {
        r["provenance"]["item_pointer"]["key"]
        for path in source_packet["sources"]["map_owner_inputs"]
        for r in json.loads((root / path).read_text()).get("records", [])
        if r.get("provenance", {}).get("item_pointer")
    }
    observations, pages = {}, defaultdict(set)
    for r in json.loads((root / base.WIKI).read_text())["records"].values():
        observations[r["item_id"]] = r["observations"]
        for o in r["observations"]:
            pages[o["page_id"]].add(r["item_id"])
    rows, seen = [], set()
    for frame in proof["frames"]:
        current, historical = frame["current"], frame["historical"]
        iid, key = current["source_item_id"], current["item_key"]
        if (
            iid in seen
            or current != pinned[iid]
            or historical["revision_id"] != REVISIONS.get(iid)
        ):
            raise ValueError("closed historical source scope drift")
        seen.add(iid)
        rows.append(
            qualify(
                frame,
                definitions[key],
                bound.get(key),
                objects[iid],
                routed,
                observations[iid],
                pages,
                proof["qualification_cutoff"],
            )
        )
    if seen != set(REVISIONS):
        raise ValueError("historical seven scope drift")
    return {
        "schema": "OTERYN_ITEM_STACK_FALSE_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
            "parent_compiler_sha256": BASE_SHA,
        },
        "source_policy": "DERIVED_DOCUMENTED_TEMPLATE_DEFAULT_SEPARATE_HISTORICAL_FRAME",
        "sources": source_packet["sources"]
        | {"historical_proof_path": PROOF, "historical_proof_sha256": PROOF_SHA},
        "counts": {"promotions": 7, "holds": 0},
        "promotions": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    packet = build()
    data = (
        json.dumps(packet, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("historical stack packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
