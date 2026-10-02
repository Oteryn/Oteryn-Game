"""Closed1515 own Wiki-primary flavor literals; differing XML stays unchanged."""

import argparse
import json

import lower_item_description_packet as strict

ROOT = strict.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_description_wiki_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-description-wiki-source-qualification-v1.json"
PROOF_SHA = "e914832a1b4a9cbb8ccddd7925d54541ff0fc48faadf797d106c04b60bd87757"
OUTPUT = (
    ROOT / "docs/agents/evidence/OTV2-20261002-item-description-wiki-promotion-v1.json"
)


def qualify(source, historical, definition, owners, params):
    target, text = source["target"], strict.flavor(source["description"])
    if (
        target != historical["target"]
        or source["source_item_id"] != historical["source_item_id"]
        or source["binding"] != historical["binding"]
        or text != historical["description"]
        or not text.endswith(".")
        or not historical["xml_description_witnesses"]
        or historical["matching_pinned_ots_sources"]
        or any(
            w["description_values"] != [text[:-1]]
            for w in historical["xml_description_witnesses"]
        )
        or set(historical["holds"])
        != {
            "NO_PINNED_OTS_EXACT_DESCRIPTION_CORROBORATION",
            "PRESENT_PINNED_OTS_DESCRIPTION_OPPOSITION",
        }
        or definition.get("identity") != target
        or not strict.same(
            {k: definition.get(k) for k in strict.HEADERS},
            source["native_header_guard"],
        )
        or strict.movable.leaf(definition, "presentation.name")
        != source["native_name_guard"]
        or source["native_name_guard"]["state"] != "KNOWN"
    ):
        raise ValueError(
            "closed Wiki-primary source/current identity/name/header drift"
        )
    # This compares source observations only; Native receives the FULL Wiki text.
    old = strict.movable.leaf(definition, "presentation.description")
    if old not in ({"state": "UNKNOWN"}, {"state": "KNOWN", "value": text}):
        raise ValueError("Wiki description blocked or conflicting")
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
                raise ValueError("empty/opposing full own Wiki literal")
            values.append(strict.flavor(fields["flavortext"][0]))
    if not values:
        raise ValueError("no explicit own Wiki flavor")
    return {
        "target": target,
        "headers": source["native_header_guard"],
        "name": source["native_name_guard"]["value"],
        "description": text,
    }


def build(root=ROOT):
    proof = json.loads(strict.base.checked(root, PROOF, PROOF_SHA))
    old = proof["historical_source"]
    historical = json.loads(strict.base.checked(root, old["path"], old["sha256"]))
    excluded = proof["strict114_excluded"]
    original = json.loads(
        strict.base.checked(root, excluded["path"], excluded["sha256"])
    )
    if (
        len(historical["records"]) != 1515
        or proof["accepted_source_policy"]
        != "CLOSED1515_OWN_WIKI_PRIMARY_LITERAL_INCLUDING_TERMINAL_PERIOD_XML_DIFFERING_LOWER_AUTHORITY_OBSERVATIONS_UNCHANGED"
        or proof["current_parent_receipt"]["parent_commit"]
        != proof["authoring_baseline"]
    ):
        raise ValueError("closed1515 source policy/current parent receipt drift")
    definitions, owners, maps, indexed, pages = strict.names.load_inputs(root, proof)
    admitted, manifests = strict.appearance_membership.load_admitted(
        out_dir=root / "imports/official/appearance-membership"
    )
    if (
        admitted["newest"] != "client-15.30"
        or admitted["files"][-1]["appearances_sha256"] != strict.base.CLIENT_SHA
    ):
        raise ValueError("latest official admitted frame drift")
    members = {label: {r[0]: r for r in m["entries"]} for label, m in manifests.items()}
    aliases = json.loads((root / "content/items/aliases.json").read_text())["entries"]
    wiki = json.loads((root / strict.base.WIKI).read_text())["records"]
    rows, seen = [], set()
    for source in proof["records"]:
        iid, target = source["source_item_id"], source["target"]
        ordinal = source["source_row_ordinal"]
        if iid in seen or ordinal not in range(1515):
            raise ValueError("duplicate/out-of-range closed Wiki source")
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
        params = strict.names.witness_params(
            source, indexed, pages, proof["qualification_cutoff"]
        )
        rows.append(qualify(source, prior, definitions[target["key"]], owners, params))
    if (
        len(seen) != 1515
        or seen != {r["source_item_id"] for r in historical["records"]}
        or seen & {r["source_item_id"] for r in original["records"]}
        or sum(r["headers"]["materializable"] for r in rows) != 11
    ):
        raise ValueError("closed disjoint1515/1504False/11True scope drift")
    return {
        "schema": "OTERYN_ITEM_DESCRIPTION_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": strict.base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "historical_source": old,
            "policy": proof["accepted_source_policy"],
        },
        "world_owner_inputs": maps,
        "counts": {"items": 1515, "fields": 1515},
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
            raise SystemExit("Wiki description packet drift")
    else:
        OUTPUT.write_bytes(data)
    print(
        "closed1515 full Wiki literals, differing XML retained; Native siblings unchanged"
    )


if __name__ == "__main__":
    main()
