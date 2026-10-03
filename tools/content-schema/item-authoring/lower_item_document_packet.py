"""Closed explicit readability metadata; independent bool/u32 facts, no defaults."""

import argparse
import json
import re
from collections import Counter
from pathlib import Path

import d289_holds
from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS
from lower_wiki_stack_default_packet import checked, exact_bindings, raw_parameters, sha

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_item_document_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261001-item-document-source-qualification-v1.json"
PROOF_SHA = "a4399548f320a23086c03a21388f24f5479c15e35310c3d017bae69efbc43e4a"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261001-item-document-promotion-v1.json"
FIELDS = {"writable": "writeable", "writechars": "max_text_length"}


def wiki_values(source, iid, name, cutoff, requested):
    raw = source["raw_infobox"]
    if sha(raw.encode()) != source["raw_infobox_sha256"]:
        raise ValueError("document raw source digest drift")
    if source["revision_timestamp"] > cutoff:
        raise ValueError("document source after cutoff")
    params = raw_parameters(raw)
    ids = params.get("itemid", [])
    if (
        len(ids) != 1
        or not re.fullmatch(r"[1-9][0-9]*(?:\s*,\s*[1-9][0-9]*)*", ids[0])
        or iid not in [int(v.strip()) for v in ids[0].split(",")]
    ):
        raise ValueError("document own identity")
    if not (set(requested) & set(FIELDS.values())):
        return {}  # Official readability is independent; shared-page scalars stay held.
    if ids != [str(iid)] or any(
        len(params.get(k, [])) > 1 for k in ("name", "actualname", *FIELDS)
    ):
        raise ValueError("document own identity/duplicate parameter")
    names = (
        params.get("actualname")
        if any(params.get("actualname", []))
        else params.get("name")
    )
    if not names or any(n.strip().casefold() != name.strip().casefold() for n in names):
        raise ValueError("document actual/official name conflict")
    values = {}
    for parameter, member in FIELDS.items():
        if member not in requested or parameter not in params:
            continue
        raw_value = params[parameter][0]
        value = (
            {"yes": True, "no": False}.get(raw_value)
            if parameter == "writable"
            else (
                int(raw_value)
                if re.fullmatch(r"[0-9]+", raw_value)
                and 0 < int(raw_value) <= 2**32 - 1
                else None
            )
        )
        if value is None:
            raise ValueError("document present empty/malformed value")
        values[member] = value
    return values


def qualify(source, definition, binding, obj, routed, cutoff):
    iid, target, facts = source["source_item_id"], source["target"], source["facts"]
    if (
        not definition
        or binding != source["binding"]
        or not binding
        or (
            binding["target"] != target
            or definition["identity"] != target
            or binding["external_id"] != str(iid)
            or obj.get("id") != iid
            or target["family"] != "Item"
            or target["revision"] != "definition-r1"
            or target["key"] in routed
        )
    ):
        raise ValueError("document exact/native/World identity drift")
    flags = obj["flags"]
    if flags.get("flags.take") is not True or any(
        flags.get(k) is True for k in (*WORLD_FLAGS, "flags.unmove")
    ):
        raise ValueError("document positive portable source required")
    if not obj.get("name") or obj["name"] != source["official_name"]:
        raise ValueError("document official name drift")
    presentation = definition.get("semantics", {}).get(
        "presentation", {"state": "UNKNOWN"}
    )
    name = presentation.get("value", {}).get("name", {"state": "UNKNOWN"})
    if (
        presentation["state"] not in {"UNKNOWN", "KNOWN"}
        or name["state"] not in {"UNKNOWN", "KNOWN"}
        or (
            name["state"] == "KNOWN"
            and name["value"].strip().casefold() != obj["name"].strip().casefold()
        )
    ):
        raise ValueError("document native name blocked/conflict")
    parsed = [
        wiki_values(s, iid, obj["name"], cutoff, facts) for s in source["wiki_sources"]
    ]
    expected = {}
    if flags.get("flags.write") is True or flags.get("flags.write_once") is True:
        expected["readable"] = True
    for member in FIELDS.values():
        present = [p[member] for p in parsed if member in p]
        if present:
            if any(v != present[0] for v in present):
                raise ValueError("document all-present source disagreement")
            expected[member] = present[0]
    if facts != expected:
        raise ValueError("document closed fact/source scope drift")
    group = definition.get("semantics", {}).get(
        "readable_writeable", {"state": "UNKNOWN"}
    )
    if group["state"] not in {"UNKNOWN", "KNOWN"}:
        raise ValueError("document blocked native group")
    for member, value in facts.items():
        leaf = group.get("value", {}).get(member, {"state": "UNKNOWN"})
        if leaf["state"] not in {"UNKNOWN", "KNOWN"} or (
            leaf["state"] == "KNOWN" and leaf["value"] != value
        ):
            raise ValueError("document blocked/conflicting native leaf")
    if (
        facts.get("writeable") is True
        and group.get("value", {}).get("readable", {}).get("value") is False
    ):
        raise ValueError("document known readability coherence conflict")
    return {"target": target, "facts": facts}


def build(root=ROOT):
    proof = json.loads(checked(root, PROOF, PROOF_SHA))
    for path, digest in proof["input_digests"].items():
        checked(root, path, digest)
    objects = {}
    for tag, raw in protobuf_fields(
        checked(root, proof["client_path"], proof["client_sha256"])
    ):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate official identity")
            objects[obj["id"]] = obj | {"object_sha256": sha(raw)}
    bindings = exact_bindings(
        json.loads((root / "imports/crystalserver/bindings/items.json").read_text())[
            "bindings"
        ],
        proof["source_revisions"],
    )
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for s in json.loads((root / "content/items/index.json").read_text())["shards"]
        for r in json.loads((root / s).read_text())["records"]
    }
    routed, maps = set(), {}
    for family in ("terrain", "objects"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_text()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    rows, holds, seen = [], [], set()
    absent = set(proof["world_owner_baseline"]["candidate_absence_keys"])
    for source in proof["records"]:
        iid, key = source["source_item_id"], source["target"]["key"]
        if (
            iid in seen
            or key not in absent
            or objects[iid]["object_sha256"] != source["official_object_sha256"]
        ):
            raise ValueError("document scope/object/World baseline drift")
        seen.add(iid)
        held = d289_holds.DOCUMENT_HOLDS.get(key)
        if held:
            # D289: the accepted Native leaf wins; the conflicting source fact is held.
            if (
                source["facts"] != held["source_facts"]
                or source["target"] != source["binding"]["target"]
            ):
                raise ValueError("document D289 held source drift")
            holds.append(
                {
                    "decision": d289_holds.DECISION,
                    "reason": held["reason"],
                    "source_facts": source["facts"],
                    "source_item_id": iid,
                    "target": source["target"],
                }
            )
            continue
        rows.append(
            qualify(
                source,
                definitions.get(key),
                bindings.get(key),
                objects[iid],
                routed,
                proof["qualification_cutoff"],
            )
        )
    d289_holds.require_hits(
        d289_holds.DOCUMENT_HOLDS, [h["target"]["key"] for h in holds], "document"
    )
    counts = Counter(member for row in rows for member in row["facts"])
    if (
        len(seen) != 98
        or len(rows) != 97
        or sum(counts.values()) != 207
        or counts != {"readable": 82, "writeable": 70, "max_text_length": 55}
    ):
        raise ValueError("document closed207/97 scope drift")
    return {
        "schema": "OTERYN_ITEM_DOCUMENT_PROMOTION/v1",
        "compiler": {"path": COMPILER, "sha256": sha((root / COMPILER).read_bytes())},
        "sources": {"proof_path": PROOF, "proof_sha256": PROOF_SHA},
        "world_owner_inputs": maps,
        "counts": {
            "fields": 207,
            "items": 97,
            "holds": len(holds),
            "by_field": dict(counts),
        },
        "promotions": rows,
        "holds": holds,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("document packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("document207 fields/97Items, D289 holds 1")


if __name__ == "__main__":
    main()
