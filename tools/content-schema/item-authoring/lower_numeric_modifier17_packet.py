"""Explicit13+4 successor: whole17 vectors, existing Native37, no runtime bindings."""

import argparse
import json
import re
from pathlib import Path

import appearance_membership as membership
import lower_mantra_bond_modifier_packet as prior
import lower_wiki_movable_packet as frame

shared, base, ROOT = prior.shared, prior.base, prior.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py"
RECEIPT = (
    "docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json"
)
RECEIPT_SHA = "771a13b8b0c66095a346f12d200a4eb30339e7b1c6c143d053263fc58702ef05"
OUTPUT = (
    ROOT
    / "docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json"
)
IDS13 = frozenset(
    (
        36656,
        36657,
        36658,
        36659,
        36660,
        36661,
        36662,
        36666,
        39147,
        39148,
        39150,
        50169,
        50170,
    )
)
IDS4 = frozenset((36672, 36673, 45639, 45640))
IDS = IDS13 | IDS4
HEADERS = ("kind", "stack_class", "client_projection", "materializable")


def whole_vector(fields, order):
    raw = {k: v for k, v in fields.items() if k in shared.GROUP}
    if any(len(v) != 1 or not v[0] for v in raw.values()):
        return None
    entries, keep = {}, []
    for text in fields.get("attrib", [""])[0].split(","):
        text = text.strip().casefold()
        new = []
        if m := re.fullmatch(r"([0-9]+) damage reflection", text):
            if int(m[1]) >= 2**31:
                return None
            new = [("REFLECT_DAMAGE", "SIGNED_POINTS", int(m[1]))]
        elif m := re.fullmatch(r"cleave ([0-9]+)%", text):
            if int(m[1]) > 100:
                return None
            new = [
                (
                    "CLEAVE_PERCENT",
                    "RATIONAL_PERCENT",
                    {"numerator": int(m[1]), "denominator": 1},
                )
            ]
        elif m := re.fullmatch(r"perfect shot \+?([0-9]+) at range ([0-9]+)", text):
            if int(m[1]) >= 2**31 or int(m[2]) > 65535:
                return None
            new = [
                ("PERFECT_SHOT_DAMAGE", "SIGNED_POINTS", int(m[1])),
                ("PERFECT_SHOT_RANGE", "CELLS", int(m[2])),
            ]
        elif m := re.fullmatch(r"magic shield capacity \+([0-9]+) and ([0-9]+)%", text):
            if int(m[1]) >= 2**31 or int(m[2]) > 100:
                return None
            new = [
                ("MAGIC_SHIELD_CAPACITY_FLAT", "SIGNED_POINTS", int(m[1])),
                (
                    "MAGIC_SHIELD_CAPACITY_PERCENT",
                    "RATIONAL_PERCENT",
                    {"numerator": int(m[2]), "denominator": 1},
                ),
            ]
        else:
            keep.append(text)
        for kind, tag, value in new:
            if kind in entries:
                return None
            entries[kind] = {
                "kind": kind,
                "target_domain": {"state": "UNKNOWN"},
                "evaluation_phase": {"state": "UNKNOWN"},
                "priority": {"state": "UNKNOWN"},
                "parameter": {"state": "KNOWN", "value": {"kind": tag, "value": value}},
            }
    if not entries:
        return None
    remainder = {k: v for k, v in fields.items() if k != "attrib"}
    if keep:
        remainder["attrib"] = [", ".join(keep)]
    previous = prior.whole_vector(remainder, order)
    if any(k in remainder for k in shared.GROUP) and previous is None:
        return None
    for atom in previous["value"] if previous else []:
        if atom["kind"] in entries or (
            atom["kind"] == "ELEMENTAL_BOND"
            and atom["parameter"]["value"]["value"] not in {"EARTH", "ENERGY"}
        ):
            return None
        entries[atom["kind"]] = atom
    if len(entries) > 37 or not set(entries) <= set(order):
        return None
    return {
        "kind": "MODIFIERS",
        "value": [entries[k] for k in sorted(entries, key=order.__getitem__)],
    }


def source_vector(entry, indexed, pages, cutoff, order):
    iid, seen, values = entry["item_id"], set(), []
    expected = {
        (pid, b["box_index"])
        for pid in indexed.get(iid, ())
        for b in pages[pid]["own_objects"]
        if any(str(iid) in re.findall(r"\d+", v) for v in b["raw_itemid_values"])
    }
    for box in entry["own_source_boxes"]:
        ref = box["reference"]
        token = (ref["page_id"], ref["box_index"])
        page = pages[ref["page_id"]]
        if (
            token in seen
            or page["revision_timestamp"] > cutoff
            or any(
                ref[k] != page[k]
                for k in (
                    "page_id",
                    "title",
                    "revision_id",
                    "revision_timestamp",
                    "content_sha256",
                )
            )
            or Path(ref["capture_path"]).name != page["source_part"]
            or ref["capture_sha256"] != page["source_part_sha256"]
            or ref["page_ordinal"] != page["source_page_ordinal"]
        ):
            raise ValueError("raw source coordinate/opposition drift")
        seen.add(token)
        if (
            not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
            or base.sha(box["raw"].encode()) != box["raw_sha256"]
        ):
            raise ValueError("raw own Object frame drift")
        fields = base.raw_parameters(box["raw"])
        own = next(b for b in page["own_objects"] if b["box_index"] == ref["box_index"])
        if (
            fields != box["parameter_values"]
            or fields.get("itemid") != [str(iid)]
            or own["raw_itemid_values"] != fields["itemid"]
            or any(
                len(fields.get(k, [])) != 1
                or fields[k][0].strip().casefold()
                != entry["official_source"]["official_name"].strip().casefold()
                for k in ("name", "actualname")
            )
        ):
            raise ValueError("source singleton identity/name drift")
        typed = whole_vector(fields, order)
        if typed is None or typed != entry["source_whole_vector"]:
            raise ValueError("whole raw modifier vector unsupported/disagrees")
        values.append(typed)
    if not seen or seen != expected or any(v != values[0] for v in values):
        raise ValueError("complete own-ID/all-present source agreement required")
    # Both independently checked own XML witnesses remain corroboration, not value authority.
    witnesses = entry["all_atom_own_xml_corroboration"]
    if [w["source"] for w in witnesses] != [
        "binding-selected-Crystal",
        "Canary-hypothesis",
    ]:
        raise ValueError("closed corroboration source drift")
    for witness in witnesses:
        atoms = witness["all_vector_atoms_checked"]
        if (
            not witness["all_agree"]
            or len(atoms) != len(values[0]["value"])
            or any(
                not checked["equal"]
                or checked["native_kind"] != atom["kind"]
                or checked["parameter_tag"] != atom["parameter"]["value"]["kind"]
                or checked["typed_source_value"] != atom["parameter"]["value"]["value"]
                for checked, atom in zip(atoms, values[0]["value"], strict=True)
            )
        ):
            raise ValueError("whole own XML corroboration drift")
    return values[0]["value"]


def qualify(entry, definition, binding, obj, members, routed, vector):
    iid, target = entry["item_id"], entry["current_target"]
    official = entry["official_source"]
    if (
        not definition
        or definition["identity"] != target
        or binding != official["binding"]
        or not binding
        or binding["external_id"] != str(iid)
        or binding["target"] != target
        or obj.get("id") != iid
        or target["key"] in routed
        or target["family"] != "Item"
        or target["revision"] != "definition-r1"
    ):
        raise ValueError("numeric modifier exact/native/World identity drift")
    if {k: definition[k] for k in HEADERS} != entry["current_published_scope_receipt"][
        "native_headers"
    ]:
        raise ValueError("four scoped structural headers drift")
    flags = obj.get("flags", {})
    if (
        obj.get("name") != official["official_name"]
        or obj.get("object_sha256") != official["official_raw_object_sha256"]
        or flags.get("flags.take") is not True
        or any(
            flags.get(k) is True
            for k in shared.CORPSE_FLAGS
            + shared.GROUND_OR_BORDER_FLAGS
            + ("flags.unmove", "flags.liquidpool")
        )
    ):
        raise ValueError("official raw name/portable source drift")
    label = prior.SOURCE_LABELS.get(binding["source_revision"])
    projection = membership.sha256_hex(
        membership.canonical_bytes(membership.identity_projection(obj))
    )
    if (
        members["client-15.30"].get(iid) != official["current_member"]
        or members.get(label, {}).get(iid) != official["bound_old_member"]
        or projection != official["current_member"][1]
        or projection != official["bound_old_member"][1]
        or label != official["bound_old_member_label"]
    ):
        raise ValueError("current/bound member identity continuity drift")
    name = frame.leaf(definition, "presentation.name")
    if (
        name["state"] != "KNOWN"
        or name["value"].strip().casefold() != obj["name"].strip().casefold()
    ):
        raise ValueError("normal native name guard")
    if frame.leaf(definition, "skill_modifiers.modifiers") not in (
        {"state": "UNKNOWN"},
        {"state": "KNOWN", "value": vector},
    ):
        raise ValueError("whole native modifier vector blocked/conflicting")
    return {
        "source_item_id": iid,
        "target": target,
        "headers": {k: definition[k] for k in HEADERS},
        "expected_name": obj["name"],
        "modifiers": vector,
    }


def build(root=ROOT):
    receipt = json.loads(base.checked(root, RECEIPT, RECEIPT_SHA))
    proofs = [
        json.loads(base.checked(root, p["path"], p["sha256"]))
        for p in receipt["source_proofs"]
    ]
    records = [r for p in proofs for r in p["records"]]
    successor = json.loads(
        base.checked(
            root,
            receipt["explicit_successor"]["path"],
            receipt["explicit_successor"]["sha256"],
        )
    )
    if {r["item_id"]: r for r in successor["records"]} != {
        r["item_id"]: r for r in records
    } or set(successor["closed_ids"]) != IDS:
        raise ValueError("explicit17 successor differs from immutable13 plus separate4")
    for path, digest in receipt["input_pins"].items():
        base.checked(root, path, digest)
    if (
        len(proofs) != 2
        or set(proofs[0]["closed_ids"]) != IDS13
        or set(proofs[1]["closed_ids"]) != IDS4
        or {r["item_id"] for r in records} != IDS
        or len(records) != 17
        or set(receipt["closed_ids"]) != IDS
    ):
        raise ValueError("explicit closed13+4 scope drift")
    for doc in [d for p in proofs for d in p.get("mechanics_source_witnesses", [])]:
        text = doc["full_raw_content"]
        if (
            base.sha(text.encode()) != doc["raw_content_sha256"]
            or len(text.encode()) != doc["raw_content_bytes"]
            or text.find(doc["quote"]) != doc["quote_char_offset"]
            or len(text[: doc["quote_char_offset"]].encode())
            != doc["quote_UTF8_offset"]
            or doc["revision_timestamp"] > receipt["cutoff"]
        ):
            raise ValueError("mechanic unit source drift")
    indexed, pages = frame.own_index(
        json.loads(
            base.checked(
                root,
                receipt["global_own_id_frame"]["path"],
                receipt["global_own_id_frame"]["sha256"],
            )
        )
    )
    bindings = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_bytes())["bindings"],
        set(prior.SOURCE_LABELS),
    )
    _, admitted = membership.load_admitted(
        root / "imports/official/appearance-membership"
    )
    members = {label: {r[0]: r for r in m["entries"]} for label, m in admitted.items()}
    objects = {}
    for tag, raw in shared.protobuf_fields(
        base.checked(root, base.CLIENT, base.CLIENT_SHA)
    ):
        if tag == 1:
            obj = shared.decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate official numeric object")
            objects[obj["id"]] = obj | {"object_sha256": base.sha(raw)}
    definitions, routed, maps = {}, set(), {}
    for path in json.loads((root / "content/items/index.json").read_bytes())["shards"]:
        shard = json.loads((root / path).read_bytes())
        if (
            shard["schema"] != "OTERYN_ITEM_AUTHORING_SHARD/v1"
            or shard["family"] != "Item"
        ):
            raise ValueError("Item shard structural headers drift")
        for row in shard["records"]:
            d = row["definition"]
            if d["identity"]["key"] in definitions:
                raise ValueError("duplicate current Item identity")
            definitions[d["identity"]["key"]] = d
    for family in ("objects", "terrain"):
        for path in sorted((root / f"content/world/{family}").glob("*.json")):
            maps[str(path.relative_to(root))] = base.sha(path.read_bytes())
            routed.update(
                r["provenance"]["item_pointer"]["key"]
                for r in json.loads(path.read_bytes()).get("records", [])
                if r.get("provenance", {}).get("item_pointer")
            )
    order, rows = shared.native_order(root), []
    for entry in records:
        vector = source_vector(entry, indexed, pages, receipt["cutoff"], order)
        rows.append(
            qualify(
                entry,
                definitions.get(entry["item_key"]),
                bindings.get(entry["item_key"]),
                objects[entry["item_id"]],
                members,
                routed,
                vector,
            )
        )
    if sum(len(r["modifiers"]) for r in rows) != 50:
        raise ValueError("closed50 atom count drift")
    return {
        "schema": "OTERYN_ITEM_NUMERIC_MODIFIER_PROMOTION/v1",
        "counts": {"items": 17, "vectors": 17, "atoms": 50},
        "promotions": sorted(rows, key=lambda r: r["target"]["key"]),
        "source_policy": "EXPLICIT13_PLUS4_WHOLE17_CONTEXT_UNKNOWN_NO_REFLECTION_ELEMENT_NO_GEAR_FORMULA",
        "sources": {
            "receipt": RECEIPT,
            "receipt_sha256": RECEIPT_SHA,
            "proofs": receipt["source_proofs"],
            "compiler_sha256": base.sha((root / COMPILER).read_bytes()),
            "world_owner_inputs": maps,
        },
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = (
        json.dumps(build(), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()
    if args.check:
        if OUTPUT.read_bytes() != data:
            raise SystemExit("numeric17 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("PASS numeric17/50", base.sha(data))


if __name__ == "__main__":
    main()
