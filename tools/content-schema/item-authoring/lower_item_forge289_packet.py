"""Closed new BR crosswalk and literal Forge pairs into existing Item source owners."""

import argparse
import json
import re
import sys
from collections import Counter, defaultdict

import lower_item_forge3332_packet as old
from engine_items import decode_appearance_object, protobuf_fields
from lower_client_market_packet import WORLD_FLAGS
from lower_wiki_stack_default_packet import raw_parameters

ROOT = old.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_forge289_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-forge289-source-qualification-v1.json"
PROOF_SHA = "e76b35928af82b02cc60eea1904b343e88588bd6765436acde39b70370b499a1"
SNAPSHOT = "imports/tibiawiki/facts/forge289-source-snapshot.json"
SNAPSHOT_SHA = "cff4df43b6e38255656d56fd1f53a47cf30f6174f09c01e18450cf5e49db32c2"
PROPOSED = (
    "docs/agents/evidence/OTV2-20261002-item-forge289-original-proposed-packet-v1.json"
)
PROPOSED_SHA = "0c934577617136c5af4fbd0bea4b454db1a6cec05dcee8ab281a1c1ef3c588f2"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json"
PATHS = {
    "attack": ("weapon", "attack"),
    "defense": ("weapon", "defense"),
    "defensemod": ("weapon", "extra_defense"),
    "range": ("weapon", "range"),
    "armor": ("protection", "armor"),
}
FANDOM_FIELDS = {"defensemod": ("extradefense", "extra_defense")}
PARSER_PINS = {
    "tools/content-census/item_wiki_family_capture.py": "0abb6b0180eeef98bfa397ca3b7c9192d47806a7e97aa6423159220e82c36f03",
    "tools/content-schema/item-authoring/lower_wiki_stack_default_packet.py": "70d6244253ec7fcc557b29f8203abc8492cd261a62563213bd1a3298bc212dba",
}


def same(a, b):
    return json.dumps(a, sort_keys=True, separators=(",", ":")) == json.dumps(
        b, sort_keys=True, separators=(",", ":")
    )


def br_fields(text):
    starts = list(
        re.finditer(r"\{\{\s*Infobox[ _]Item\s*(?=[|}])", text, re.IGNORECASE)
    )
    if len(starts) != 1 or text.rfind("<!--", 0, starts[0].start()) > text.rfind(
        "-->", 0, starts[0].start()
    ):
        raise ValueError("one uncommented own BR infobox required")
    start, depth, i = starts[0].start(), 0, starts[0].start()
    while i < len(text):
        token = text[i : i + 2]
        if token in ("{{", "}}"):
            depth += 1 if token == "{{" else -1
            i += 2
            if depth == 0:
                break
        else:
            i += 1
    if depth:
        raise ValueError("unbalanced BR infobox")
    # Reuse the accepted balanced-template splitter; the retained full article
    # and singleton own-name are verified independently of discovery titles.
    sys.path.insert(0, str(ROOT / "tools/content-census"))
    from item_wiki_family_capture import split_template_params

    fields = defaultdict(list)
    for token in split_template_params(text[start + 2 : i - 2])[1:]:
        key, sep, value = token.partition("=")
        if sep:
            if not re.fullmatch(r"[A-Za-z0-9_]+", key.strip()):
                raise ValueError("ambiguous BR template parameter key")
            fields[key.strip().casefold()].append(value.strip())
    if any(len(values) != 1 for values in fields.values()):
        raise ValueError("duplicate BR parameter")
    return dict(fields)


def qualify(source, page, native, obj, owners, routed, cutoff, policy):
    q, f = source["qualification"], source["own_fandom"]
    target, witness = q["target"], q["BR_source_witness"]
    if (
        native["identity"] != target
        or target["key"] in routed
        or native["kind"] != "Item"
    ):
        raise ValueError("current exact Item/domain drift")
    headers = ("identity", "kind", "stack_class", "materializable", "client_projection")
    if not same(
        {k: native[k] for k in headers}, {k: q["native_definition"][k] for k in headers}
    ):
        raise ValueError("current scoped native Item headers drift")
    name = obj["name"]
    if (
        obj["id"] != q["source_item_id"]
        or obj["flags"].get("flags.take") is not True
        or any(obj["flags"].get(k) is True for k in (*WORLD_FLAGS, "flags.unmove"))
    ):
        raise ValueError("current portable official source identity drift")
    n = native["semantics"]["presentation"]
    if n["state"] != "KNOWN" or n["value"]["name"] != {"state": "KNOWN", "value": name}:
        raise ValueError("current Known native source name required")
    text = page["raw_full_article"]
    if (
        page["page_id"] != witness["BR_page_id"]
        or page["revision_id"] != witness["BR_revision_id"]
        or page["revision_timestamp"] != witness["BR_revision_timestamp"]
        or page["revision_timestamp"] > cutoff
        or old.sha(text.encode()) != page["raw_content_sha256"]
        or old.sha(text.encode()) != witness["BR_raw_content_sha256"]
        or len(text.encode()) != page["raw_content_bytes"]
    ):
        raise ValueError("BR own raw revision/hash/cutoff drift")
    bf = br_fields(text)
    if bf != witness["BR_literal_fields"] or [
        x.casefold() for x in bf.get("name", [])
    ] != [name.casefold()]:
        raise ValueError("BR own literal name/parameters drift")
    if not re.fullmatch(r"[1-4]", bf.get("classificacao", [""])[0]) or not re.fullmatch(
        r"[1-9][0-9]*", bf.get("max_tier", [""])[0]
    ):
        raise ValueError("explicit class/max scalar required")
    forge = {
        "classification": int(bf["classificacao"][0]),
        "max_tier": int(bf["max_tier"][0]),
    }
    if (
        not same(forge, q["forge"])
        or forge["max_tier"] > 255
        or type(obj["flags"].get("upgradeclassification.upgrade_classification"))
        is not int
        or obj["flags"].get("upgradeclassification.upgrade_classification")
        != forge["classification"]
    ):
        raise ValueError("literal Forge/official class disagreement")
    if (
        not f["positive_exact_infobox_object_match"]
        or not f["balanced"]
        or f["inside_comment"]
        or old.sha(f["raw"].encode()) != f["raw_sha256"]
        or f["source_frame"]["revision_timestamp"] > cutoff
    ):
        raise ValueError("own Fandom raw template/frame drift")
    fp = raw_parameters(f["raw"])
    if (
        any(len(values) != 1 for values in fp.values())
        or fp.get("itemid") != [str(q["source_item_id"])]
        or [x.casefold() for x in fp.get("actualname", fp.get("name", []))]
        != [name.casefold()]
        or any(
            values[0].casefold() != name.casefold()
            for key, values in fp.items()
            if key in {"actualname", "name"}
        )
    ):
        raise ValueError("own singleton Fandom numeric ID/name conflict")
    stats = []
    for fact in witness["matched_non_title_stats"]:
        field = fact["field"]
        group, key = PATHS[field]
        raw = bf[field][0]
        value = native["semantics"].get(group, {})
        value = (
            value.get("value", {}).get(key, {}) if value.get("state") == "KNOWN" else {}
        )
        literals = [
            v for alias in FANDOM_FIELDS.get(field, (field,)) for v in fp.get(alias, [])
        ]
        if (
            not re.fullmatch(r"[+-]?[0-9]+", raw)
            or len(literals) != 1
            or not re.fullmatch(r"[+-]?[0-9]+", literals[0])
            or value.get("state") != "KNOWN"
            or type(value.get("value")) is not int
            or value["value"] != int(raw)
            or int(raw) != int(literals[0])
            or int(raw) != fact["current_native_known"]
        ):
            raise ValueError("independent typed source-stat/current native mismatch")
        stats.append({"group": group, "property": key, "value": int(raw)})
    if len({(s["group"], s["property"]) for s in stats}) < 2:
        raise ValueError("two distinct typed comparable fields required")
    wt = native["semantics"].get("weapon", {}).get("value", {}).get("weapon_type", {})
    if not same(
        wt,
        q["native_definition"]["semantics"]
        .get("weapon", {})
        .get("value", {})
        .get("weapon_type", {}),
    ):
        raise ValueError("current scoped native weapon type drift")
    if wt.get("state") == "KNOWN":
        brtype = (
            policy["BR_explicit_replica_type"].get(bf.get("type", [""])[0])
            if bf.get("primarytype") == ["Réplicas"]
            else policy["BR_primary"].get(bf.get("primarytype", [""])[0])
        )
        if (
            wt["value"] != brtype
            or fp.get("primarytype")
            and policy["Fandom"].get(fp["primarytype"][0]) != wt["value"]
        ):
            raise ValueError("source/native weapon type contradiction")
    existing = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(existing) > 1 or any(
        o["item"] != target
        or o.get("forge") is not None
        and not same(o["forge"], forge)
        for o in existing
    ):
        raise ValueError("current authoring owner conflict")
    return {"item": target, "forge": forge}, {
        "target": target,
        "official_name": name,
        "headers": {k: native[k] for k in headers},
        "weapon_type": wt,
        "stats": stats,
    }


def build(root=ROOT):
    proof = json.loads(old.checked(root, PROOF, PROOF_SHA))
    snapshot = json.loads(old.checked(root, SNAPSHOT, SNAPSHOT_SHA))
    proposed = json.loads(old.checked(root, PROPOSED, PROPOSED_SHA))
    for path, digest in PARSER_PINS.items():
        old.checked(root, path, digest)
    for path, digest in proof["immutable_input_hashes"].items():
        old.checked(root, path, digest)
    index = json.loads(
        (root / "imports/official/appearance-membership/admitted.json").read_bytes()
    )
    descriptor = proof["newest_official_descriptor"]
    if (
        index["newest"] != descriptor["label"]
        or max(index["files"], key=lambda x: x["order"]) != descriptor
    ):
        raise ValueError("actual latest admitted official frame drift")
    raw = old.checked(
        root,
        f"content/assets/files/appearances-{descriptor['appearances_sha256']}.dat",
        descriptor["appearances_sha256"],
    )
    objects, names = {}, defaultdict(list)
    for tag, body in protobuf_fields(raw):
        if tag != 1:
            continue
        obj = decode_appearance_object(body)
        if obj["id"] in objects:
            raise ValueError("duplicate official object")
        objects[obj["id"]] = (obj, old.sha(body))
        if obj.get("name"):
            names[obj["name"].casefold()].append(obj["id"])
    membership = {}
    for label in {descriptor["label"], "crystal-ff7ede5"}:
        d = next(x for x in index["files"] if x["label"] == label)
        membership[label] = {
            x[0]: x
            for x in json.loads(
                old.checked(
                    root,
                    "imports/official/appearance-membership/" + d["manifest"],
                    d["manifest_sha256"],
                )
            )["entries"]
        }
    pages = {p["page_id"]: p for p in snapshot["pages"]}
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_bytes())["shards"]:
        for row in json.loads((root / shard).read_bytes())["records"]:
            d = row["definition"]
            if d["identity"]["key"] in definitions:
                raise ValueError("duplicate current canonical Item")
            definitions[d["identity"]["key"]] = d
    crystal = json.loads(
        (root / "imports/crystalserver/bindings/items.json").read_bytes()
    )["bindings"]
    br = json.loads((root / "imports/tibiawiki/bindings/items.json").read_bytes())[
        "bindings"
    ]
    draft = json.loads((root / "content/world/provenance/sources.json").read_bytes())[
        "source_identity_bindings"
    ]
    owners = json.loads(
        (root / "content/world/definitions/declarations.json").read_bytes()
    )["item_authoring"]
    routed = {
        r["provenance"]["item_pointer"]["key"]
        for family in ("objects", "terrain")
        for p in (root / f"content/world/{family}").glob("*.json")
        for r in json.loads(p.read_bytes()).get("records", [])
        if r.get("provenance", {}).get("item_pointer")
    }
    bindings = {b["target"]["key"]: b for b in proposed["bindings"]}
    if (
        len(bindings) != len(proof["records"])
        or len(bindings) != 289
        or len(pages) != 289
    ):
        raise ValueError("closed289 source scope drift")
    promotions, guards = [], []
    for source in proof["records"]:
        q = source["qualification"]
        iid, target = q["source_item_id"], q["target"]
        obj, digest = objects[iid]
        if (
            names[obj["name"].casefold()] != [iid]
            or digest != q["official_object_sha256"]
            or membership[descriptor["label"]].get(iid) != q["current_membership"]
            or membership["crystal-ff7ede5"].get(iid) != q["historical_membership"]
        ):
            raise ValueError("official global unique name/object/membership drift")
        old.exact_binding(crystal, q["crystal_binding"], target)
        binding = bindings[target["key"]]
        expected = {
            "source_key": proposed["source"]["key"],
            "source_revision": proposed["source"]["revision"],
            "identity_namespace": "mediawiki/page_id",
            "external_id": str(q["BR_source_witness"]["BR_page_id"]),
            "target": target,
            "disposition": "EXACT",
        }
        if not same(binding, expected):
            raise ValueError("closed proposed BR binding mismatch")
        for current in (br, draft):
            hits = [
                b
                for b in current
                if b["source_key"] == binding["source_key"]
                and (
                    b["target"] == target or b["external_id"] == binding["external_id"]
                )
            ]
            if hits and not same(hits, [binding]):
                raise ValueError("opposed or duplicate existing BR identity binding")
        row, guard = qualify(
            source,
            pages[int(binding["external_id"])],
            definitions[target["key"]],
            obj,
            owners,
            routed,
            proof["qualification_cutoff"],
            proof["source_type_corroboration"],
        )
        promotions.append(row)
        guards.append(guard)
    if not same(promotions, proposed["promotions"]):
        raise ValueError("immutable original289 Forge facts drift")
    distribution = Counter(
        f"{p['forge']['classification']}/{p['forge']['max_tier']}" for p in promotions
    )
    if distribution != {"1/1": 135, "2/2": 55, "3/3": 18, "4/10": 81}:
        raise ValueError("closed literal Forge distribution drift")
    digest = old.sha((root / COMPILER).read_bytes())
    import_batch = {
        "batch_id": proposed["source"]["import_batch_id"],
        "source_repository": "tibiawiki.com.br",
        "source_revision": proposed["source"]["revision"],
        "source_artifact_sha256": SNAPSHOT_SHA,
        "access_disposition": "PENDING",
        "source_generation_profile": snapshot["schema"],
        "importer": "OTERYN_ITEM_FORGE289_SOURCE_IMPORT/v1",
        "mapper": "OTERYN_ITEM_FORGE289_SOURCE_IMPORT/v1",
        "mapper_revision": f"forge289-source-mapper-v1:{digest}",
        "mapper_sha256": digest,
        "candidates": [],
        "reimport_states": [],
    }
    return {
        "schema": "OTERYN_ITEM_FORGE289_SOURCE_IMPORT/v1",
        "compiler": {"path": COMPILER, "sha256": digest},
        "compiler_dependencies": PARSER_PINS,
        "proof": {"path": PROOF, "sha256": PROOF_SHA},
        "snapshot": {"path": SNAPSHOT, "sha256": SNAPSHOT_SHA},
        "counts": {
            "items": 289,
            "fields": 578,
            "bindings": 289,
            "literal_distribution": dict(distribution),
        },
        "source": proposed["source"],
        "import_batch": import_batch,
        "bindings": proposed["bindings"],
        "promotions": promotions,
        "native_guards": guards,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--check", action="store_true")
    args = p.parse_args()
    raw = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("Forge289 packet drift")
    else:
        OUTPUT.write_bytes(raw)
    print("Forge289:289 source bindings/578 literal Forge fields; Native unchanged")


if __name__ == "__main__":
    main()
