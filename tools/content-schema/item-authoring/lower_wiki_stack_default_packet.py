"""Lower a closed cohort using the documented Infobox Object stackable default."""

import argparse
import hashlib
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

from engine_items import decode_appearance_object, protobuf_fields

ROOT = Path(__file__).resolve().parents[3]
COMPILER = "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py"
PROOF = (
    "docs/agents/evidence/OTV2-20261001-item-stack-default-source-qualification-v1.json"
)
PROOF_SHA = "945eb2c1e7fe74f3ae081a2fb6315105c2a082af8d95e833e887e9d7b8d8cf17"
DECODER = "tools/content-schema/item-authoring/engine_items.py"
DECODER_SHA = "28644c0fcc88f364992670489f7d458f827a9cd1db023b7f5ce0a9ae32d08c1a"
PARSER = "tools/content-census/item_wiki_family_capture.py"
PARSER_SHA = "0abb6b0180eeef98bfa397ca3b7c9192d47806a7e97aa6423159220e82c36f03"
CLIENT_SHA = "2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2"
CLIENT = f"content/assets/files/appearances-{CLIENT_SHA}.dat"
BINDINGS = "imports/crystalserver/bindings/items.json"
WIKI = "imports/tibiawiki/facts/items-stats.json"
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261001-item-stack-default-promotion-v1.json"
)
QUOTE = 'Include if the object is stackable. Use "yes" if it is and "no" if it isn\'t (which is the default).'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def checked(root, path, digest):
    data = (root / path).read_bytes()
    if sha(data) != digest:
        raise ValueError(f"source digest drift: {path}")
    return data


def raw_parameters(content):
    """Read the complete balanced template; retain empty and duplicate parameters."""
    sys.path.insert(0, str(ROOT / "tools/content-census"))
    from item_wiki_family_capture import split_template_params

    matches = list(
        re.finditer(r"\{\{\s*Infobox[ _]Object\s*(?=[|}])", content, re.IGNORECASE)
    )
    if len(matches) != 1:
        raise ValueError("ambiguous or absent Object infobox")
    start = matches[0].start()
    if content.rfind("<!--", 0, start) > content.rfind("-->", 0, start):
        raise ValueError("commented Object infobox")
    depth, end = 0, start
    while end < len(content):
        pair = content[end : end + 2]
        if pair in {"{{", "}}"}:
            depth += 1 if pair == "{{" else -1
            end += 2
            if depth == 0:
                break
        else:
            end += 1
    if depth:
        raise ValueError("unclosed Object infobox")
    fields = defaultdict(list)
    for token in split_template_params(content[start + 2 : end - 2])[1:]:
        key, sep, value = token.partition("=")
        if sep:
            key = key.strip()
            if not re.fullmatch(r"[A-Za-z0-9_]+", key):
                raise ValueError("ambiguous template parameter key")
            fields[key.casefold()].append(value.strip())
    return dict(fields)


def reasons(source, definition, binding, obj, routed, observations, page_ids, cutoff):
    fields = raw_parameters(source["content"])
    iid, key = source["source_item_id"], source["item_key"]
    result = []
    if "stackable" in fields:
        result.append("STACKABLE_ARGUMENT_PRESENT")
    if fields.get("itemid") != [str(iid)]:
        result.append("ACTUAL_SINGLE_ITEM_ID_REQUIRED")
    if source["revision_timestamp"] > cutoff:
        result.append("REVISION_AFTER_QUALIFICATION_CUTOFF")
    if not binding or binding["external_id"] != str(iid):
        result.append("NO_EXACT_SOURCE_BINDING")
    if not definition:
        result.append("NO_NATIVE_ITEM")
    if binding and binding["target"] != definition.get("identity"):
        result.append("BINDING_TARGET_NATIVE_IDENTITY_DRIFT")
    presentation = definition.get("semantics", {}).get(
        "presentation", {"state": "UNKNOWN"}
    )
    name = presentation.get("value", {}).get("name", {"state": "UNKNOWN"})
    if presentation["state"] not in {"UNKNOWN", "KNOWN"} or name["state"] not in {
        "UNKNOWN",
        "KNOWN",
    }:
        result.append("BLOCKED_PRESENTATION_EVIDENCE")
    elif (
        name["state"] == "KNOWN"
        and name["value"].strip().casefold()
        != (obj.get("name") or "").strip().casefold()
    ):
        result.append("KNOWN_PRESENTATION_NAME_CONFLICT")
    if key in routed:
        result.append("EXISTING_WORLD_OWNER")
    flags = obj.get("flags", {})
    if flags.get("flags.take") is not True:
        result.append("DOMAIN_HOLD_NO_AFFIRMATIVE_TAKE")
    if any(
        flags.get(k)
        for k in (
            "flags.clip",
            "flags.corpse",
            "flags.player_corpse",
            "flags.liquidpool",
            "flags.bank",
        )
    ):
        result.append("AFFIRMATIVE_WORLD_DOMAIN_HOLD")
    if flags.get("flags.cumulative") is True:
        result.append("CLIENT_CUMULATIVE_TRUE")
    names = fields.get("actualname", []) + fields.get("name", [])
    if (
        not obj.get("name")
        or not any(names)
        or any(
            n.strip().casefold() != obj["name"].strip().casefold() for n in names if n
        )
    ):
        result.append("EXACT_OFFICIAL_NAME_REQUIRED")
    if any(
        len(fields.get(k, [])) > 1
        for k in ("itemid", "name", "actualname", "primarytype")
    ):
        result.append("DUPLICATE_IDENTITY_PARAMETER")
    if not observations or any(page_ids[o["page_id"]] != {iid} for o in observations):
        result.append("SHARED_OR_MISSING_RETAINED_PAGE")
    coordinates = {
        "page_id": "page_id",
        "revision_id": "revision_id",
        "content_sha256": "content_sha256",
        "revision_timestamp": "revision_timestamp",
        "wiki_title": "title",
    }
    if len(observations) != 1 or any(
        observations[0].get(k) != source.get(v) for k, v in coordinates.items()
    ):
        result.append("PINNED_RAW_SNAPSHOT_COORDINATE_DRIFT")
    if any("stackable" in o["fields"] for o in observations):
        result.append("PRESENT_RETAINED_STACKABLE_ARGUMENT")
    stack = definition.get("semantics", {}).get("stack", {"state": "UNKNOWN"})
    old = stack.get("value", {}).get("stackable", {"state": "UNKNOWN"})
    if stack["state"] not in {"UNKNOWN", "KNOWN"} or old["state"] not in {
        "UNKNOWN",
        "KNOWN",
    }:
        result.append("BLOCKED_STACK_EVIDENCE")
    if definition.get("stack_class") == "StackCapable" or old.get("value") is True:
        result.append("KNOWN_STACKABLE_CONFLICT")
    return sorted(set(result))


def exact_bindings(records, revisions):
    bound, external_ids = {}, set()
    for b in records:
        if b["disposition"] != "EXACT":
            continue
        if (
            b["source_key"] != "oteryn:source.crystalserver"
            or b["source_revision"] not in revisions
            or b["identity_namespace"] != "ots/item_server_id"
            or b["target"]["family"] != "Item"
            or b["target"]["revision"] != "definition-r1"
            or b["target"]["key"] in bound
            or b["external_id"] in external_ids
        ):
            raise ValueError("unqualified or duplicate exact source binding")
        external_ids.add(b["external_id"])
        bound[b["target"]["key"]] = b
    return bound


def build(root=ROOT):
    proof = json.loads(checked(root, PROOF, PROOF_SHA))
    checked(root, DECODER, DECODER_SHA)
    checked(root, PARSER, PARSER_SHA)
    checked(root, proof["bridge_proof"]["path"], proof["bridge_proof"]["sha256"])
    doc = proof["documentation"]
    if (
        sha(doc["content"].encode()) != doc["raw_wikitext_sha256"]
        or QUOTE not in doc["content"]
        or (doc["page_id"], doc["revision_id"]) != (94083, 1204204)
    ):
        raise ValueError("documented default source drift")
    objects = {}
    for tag, raw in protobuf_fields(checked(root, CLIENT, CLIENT_SHA)):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate client object identity")
            objects[obj["id"]] = obj | {"object_sha256": sha(raw)}
    bound = exact_bindings(
        json.loads((root / BINDINGS).read_text())["bindings"],
        proof["bridge"]["source_revisions"],
    )
    snapshot = json.loads((root / WIKI).read_text())
    digest = sha(
        json.dumps(
            snapshot["records"],
            sort_keys=True,
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    )
    if digest != snapshot["snapshot_sha256"]:
        raise ValueError("retained snapshot digest drift")
    wiki, pages = {}, defaultdict(set)
    for record in snapshot["records"].values():
        wiki[record["item_id"]] = record["observations"]
        for obs in record["observations"]:
            pages[obs["page_id"]].add(record["item_id"])
    definitions = {
        r["definition"]["identity"]["key"]: r["definition"]
        for shard in json.loads((root / "content/items/index.json").read_text())[
            "shards"
        ]
        for r in json.loads((root / shard).read_text())["records"]
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
    for source in proof["records"]:
        iid, key = source["source_item_id"], source["item_key"]
        if iid in seen or sha(source["content"].encode()) != source["content_sha256"]:
            raise ValueError("duplicate or substituted raw page")
        seen.add(iid)
        if key not in proof["world_owner_baseline"]["candidate_absence_keys"]:
            raise ValueError("missing frozen World domain exclusion")
        obj = objects.get(iid, {})
        why = reasons(
            source,
            definitions.get(key, {}),
            bound.get(key),
            obj,
            routed,
            wiki.get(iid, []),
            pages,
            proof["qualification_cutoff"],
        )
        handle = {
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
        }
        row = {"item_key": key, "source_item_id": iid, "wiki": handle}
        if why:
            holds.append(row | {"reasons": why})
        else:
            rows.append(
                row
                | {
                    "stackable": False,
                    "binding": bound[key],
                    "appearance_id": iid,
                    "object_sha256": obj["object_sha256"],
                }
            )
    if len(seen) != 1651 or len(rows) != 1487:
        raise ValueError(
            f"closed default cohort drift: {len(seen)} sources, {len(rows)} qualified"
        )
    return {
        "schema": "OTERYN_ITEM_STACK_FALSE_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": sha((root / COMPILER).read_bytes()),
            "parser_sha256": PARSER_SHA,
            "decoder_sha256": DECODER_SHA,
        },
        "source_policy": "DERIVED_DOCUMENTED_TEMPLATE_DEFAULT",
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "client_path": CLIENT,
            "client_sha256": CLIENT_SHA,
            "bindings_path": BINDINGS,
            "bindings_sha256": sha((root / BINDINGS).read_bytes()),
            "wiki_path": WIKI,
            "snapshot_sha256": digest,
            "map_owner_inputs": maps,
        },
        "counts": {"promotions": len(rows), "holds": len(holds)},
        "promotions": rows,
        "holds": holds,
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
            raise SystemExit("default-no packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(json.dumps(packet["counts"]))


if __name__ == "__main__":
    main()
