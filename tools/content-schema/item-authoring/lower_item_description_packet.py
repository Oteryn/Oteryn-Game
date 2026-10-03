"""Closed114 own in-game flavor literals; no admission or sibling changes."""

import argparse
import json

import appearance_membership
import lower_item_name_packet as names
import lower_wiki_movable_packet as movable
import lower_wiki_stack_default_packet as base
from source_field_catalogs import fandom_rule

ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_description_packet.py"
PROOF = (
    "docs/agents/evidence/OTV2-20261002-item-description-source-qualification-v1.json"
)
PROOF_SHA = "5bfd667737017475b13d475f7d087969c134646ef12bdc72a3cb835b770d9156"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-description-promotion-v1.json"
HEADERS = ("kind", "client_projection", "materializable", "stack_class")
# D316: main's quest-reward admission overlay reaches content/items after promotion, so the
# Native guards compare against the pre-overlay stage that the Rust promotion applies to.
PRE_OVERLAY = "content/world/definitions/reference.json"


def pre_overlay_definitions(root, current):
    """Pre-overlay Item definitions; exactly the current Item identities, no others."""
    definitions = {}
    for row in json.loads((root / PRE_OVERLAY).read_text())["records"]:
        if row["identity"]["family"] != "Item":
            continue
        key = row["identity"]["key"]
        if key in definitions:
            raise ValueError("duplicate pre-overlay Item identity")
        definitions[key] = row
    if set(definitions) != set(current):
        raise ValueError("pre-overlay/current Item identity set drift")
    return definitions


def same(a, b):
    return json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)


def flavor(raw):
    """Existing formal flavor property, unchanged plain UTF-8 literal only."""
    if (
        not raw
        or raw != raw.strip()
        or len(raw.encode()) > 200
        or any(ord(c) < 32 or ord(c) == 127 for c in raw)
        or any(x in raw for x in ("{{", "}}", "[[", "]]", "<", ">", "&", "''"))
        or fandom_rule("flavortext")["allowed_destinations"]
        != ["/item/presentation/flavor_text"]
    ):
        raise ValueError("unsupported flavor literal/formal route")
    return raw


def qualify(source, historical, definition, owners, params):
    target, text = source["target"], flavor(source["description"])
    if (
        target != historical["target"]
        or source["source_item_id"] != historical["source_item_id"]
        or source["binding"] != historical["binding"]
        or text != historical["description"]
        or any(
            w["description_values"] != [text]
            for w in historical["xml_description_witnesses"]
        )
        or not historical["xml_description_witnesses"]
        or historical["holds"]
        or definition.get("identity") != target
        or not same(
            {k: definition.get(k) for k in HEADERS}, source["native_header_guard"]
        )
        or movable.leaf(definition, "presentation.name") != source["native_name_guard"]
        or source["native_name_guard"]["state"] != "KNOWN"
    ):
        raise ValueError("historical literal/current identity/name/four-header drift")
    old = movable.leaf(definition, "presentation.description")
    if old not in ({"state": "UNKNOWN"}, {"state": "KNOWN", "value": text}):
        raise ValueError("description blocked or conflicting")
    matching = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(matching) > 1 or any(
        o["item"] != target or o.get("presentation") is not None for o in matching
    ):
        raise ValueError("GameOwned presentation/duplicate owner conflict")
    values = []
    for fields in params:
        if any(
            len(fields.get(k, [])) > 1
            for k in ("itemid", "name", "actualname", "primarytype", "flavortext")
        ):
            raise ValueError("duplicate own identity/flavor parameter")
        if "flavortext" in fields:
            if fields["flavortext"] != [text]:
                raise ValueError("empty/opposing own flavor literal")
            values.append(flavor(fields["flavortext"][0]))
    if not values:
        raise ValueError("no explicit own flavor literal")
    return {
        "target": target,
        "headers": source["native_header_guard"],
        "name": source["native_name_guard"]["value"],
        "description": text,
    }


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    old = proof["historical_source"]
    historical = json.loads(base.checked(root, old["path"], old["sha256"]))
    if (
        historical["facts"] != 114
        or proof["current_parent_receipt"]["parent_commit"]
        != proof["authoring_baseline"]
    ):
        raise ValueError("historical source/current parent receipt scope drift")
    definitions, owners, maps, indexed, pages = names.load_inputs(root, proof)
    definitions = pre_overlay_definitions(root, definitions)
    admitted, manifests = appearance_membership.load_admitted(
        out_dir=root / "imports/official/appearance-membership"
    )
    if (
        admitted["newest"] != "client-15.30"
        or admitted["files"][-1]["appearances_sha256"] != base.CLIENT_SHA
    ):
        raise ValueError("latest official admitted frame drift")
    members = {label: {r[0]: r for r in m["entries"]} for label, m in manifests.items()}
    aliases = json.loads((root / "content/items/aliases.json").read_text())["entries"]
    wiki = json.loads((root / base.WIKI).read_text())["records"]
    rows, seen = [], set()
    for source in proof["records"]:
        iid, target = source["source_item_id"], source["target"]
        ordinal = source["source_row_ordinal"]
        if iid in seen or ordinal not in range(114):
            raise ValueError("duplicate/out-of-range closed source")
        seen.add(iid)
        prior = historical["records"][ordinal]
        if not any(
            a["state"] == "ALIAS" and a.get("target") == target["key"] for a in aliases
        ):
            raise ValueError("canonical alias bridge missing")
        current = members["client-15.30"].get(iid)
        label = (
            "crystal-donor-00ce02a5"
            if source["binding"]["source_revision"].startswith("00ce")
            else "crystal-ff7ede5"
        )
        bound = members[label].get(iid)
        if (
            not current
            or not bound
            or current[1] != bound[1]
            or current[2] != source["official_object_sha256"]
        ):
            raise ValueError("current/bound membership identity drift")
        observations = (wiki.get(target["key"]) or wiki.get(str(iid)) or {}).get(
            "observations", []
        )
        if not observations or any(
            not any(
                all(
                    o.get(a) == w.get(b)
                    for a, b in (
                        ("page_id", "page_id"),
                        ("wiki_title", "title"),
                        ("revision_id", "revision_id"),
                        ("revision_timestamp", "revision_timestamp"),
                        ("content_sha256", "content_sha256"),
                    )
                )
                for w in source["sources"]
            )
            for o in observations
        ):
            raise ValueError("current imported own Wiki coordinate drift")
        params = names.witness_params(
            source, indexed, pages, proof["qualification_cutoff"]
        )
        rows.append(qualify(source, prior, definitions[target["key"]], owners, params))
    if (
        len(seen) != 114
        or seen != {r["source_item_id"] for r in historical["records"]}
        or {r["target"]["key"] for r in rows if r["headers"]["materializable"]}
        != {"oteryn:item.tibia.i237", "oteryn:item.tibia.i239"}
    ):
        raise ValueError("closed114/112False/2True scope drift")
    return {
        "schema": "OTERYN_ITEM_DESCRIPTION_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "historical_source": old,
        },
        "world_owner_inputs": maps,
        "counts": {"items": 114, "fields": 114},
        "promotions": rows,
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
            raise SystemExit("description packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("closed114 literal descriptions; all other fields/admission unchanged")


if __name__ == "__main__":
    main()
