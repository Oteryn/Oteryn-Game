"""Closed49 native vectors from57 source-qualified Mantra/Bond vectors; no runtime context or name promotion."""

import argparse
import json
import re

import appearance_membership as membership
import lower_elemental_magic_modifier_packet as shared

base, stats, ROOT = shared.base, shared.stats, shared.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py"
PROOF = (
    "docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json"
)
PROOF_SHA = "f1ec2ba4fad50815906c3f76723020e7d3804d513f152d195a45c1a6a6fe5116"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json"
SOURCE_IDS = frozenset(
    {
        3289,
        10389,
        10391,
        17828,
        50146,
        50154,
        50160,
        50164,
        50165,
        50166,
        50167,
        50176,
        50181,
        50182,
        50183,
        50184,
        50185,
        50186,
        50187,
        50188,
        50189,
        50190,
        50193,
        50194,
        50195,
        50239,
        50250,
        50254,
        50255,
        50257,
        50258,
        50260,
        50261,
        50262,
        50263,
        50264,
        50268,
        50270,
        50271,
        50272,
        50273,
        50274,
        50275,
        50276,
        50277,
        50278,
        50279,
        50280,
        50289,
        50291,
        51267,
        51294,
        52335,
        52349,
        53223,
        53224,
        53233,
    }
)
CARRIER_MISSING = frozenset({10389, 17828, 50164, 50165, 50181, 50239, 50273, 52335})
IDS = SOURCE_IDS - CARRIER_MISSING
HELD = frozenset(
    {
        50147,
        50156,
        51261,
        50157,
        50158,
        50159,
        50161,
        50162,
        50169,
        50170,
        50251,
        50252,
        50253,
    }
)
SOURCE_LABELS = {
    "ff7ede593c69d4c658b382c97443e8155926924a": "crystal-ff7ede5",
    "00ce02a57ca5a12e48f32a3476e37471167e4c3f": "crystal-donor-00ce02a5",
}


def whole_vector(fields, order):
    """Interpret the entire group, retaining every empty/duplicate as a whole hold."""
    raw = {k: v for k, v in fields.items() if k in shared.GROUP}
    if not raw or any(len(v) != 1 or not v[0] for v in raw.values()):
        return None
    baseline = {k: v for k, v in fields.items() if k not in {"mantra", "elementalbond"}}
    typed = shared.whole_vector(baseline, order)
    if (
        any(k in raw for k in shared.GROUP if k not in {"mantra", "elementalbond"})
        and typed is None
    ):
        return None
    entries = {e["kind"]: e for e in typed["value"]} if typed else {}
    for field, kind in (("mantra", "MANTRA"), ("elementalbond", "ELEMENTAL_BOND")):
        if field not in raw:
            continue
        value = raw[field][0]
        if field == "mantra":
            if (
                not re.fullmatch(r"[+-]?[0-9]+", value)
                or not -32768 <= int(value) <= 32767
            ):
                return None
            parameter = {"kind": "SIGNED_POINTS", "value": int(value)}
        else:
            if value.casefold() not in {"physical", "energy", "earth"}:
                return None
            parameter = {"kind": "ELEMENT", "value": value.upper()}
        if kind in entries:
            return None
        entries[kind] = {
            "kind": kind,
            "target_domain": {"state": "UNKNOWN"},
            "evaluation_phase": {"state": "UNKNOWN"},
            "priority": {"state": "UNKNOWN"},
            "parameter": {"state": "KNOWN", "value": parameter},
        }
    if not entries or len(entries) > 37 or not set(entries) <= set(order):
        return None
    return {
        "kind": "MODIFIERS",
        "value": [entries[k] for k in sorted(entries, key=order.__getitem__)],
    }


def qualified_membership(entry, binding, obj, members):
    if not binding:
        return False
    iid = entry["item_id"]
    label = SOURCE_LABELS.get(binding["source_revision"])
    current = members["client-15.30"].get(iid)
    old = members.get(label, {}).get(iid)
    projection = membership.sha256_hex(
        membership.canonical_bytes(membership.identity_projection(obj))
    )
    return (
        label == entry["binding_member_label"]
        and current == entry["current_member"]
        and old == entry["old_member"]
        and current is not None
        and old is not None
        and current[1] == old[1] == projection
        and current[2]
        == obj.get("object_sha256")
        == entry["official_raw_object_sha256"]
    )


def qualify(
    entry, definition, binding, obj, observations, routed, order, cutoff, members
):
    if observations != entry["retained_stats_observations"] or not qualified_membership(
        entry, binding, obj, members
    ):
        return None
    row = shared.qualify(
        entry,
        definition,
        binding,
        obj,
        [
            {
                "page_id": b["page_id"],
                "revision_id": b["revision_id"],
                "revision_timestamp": b["revision_timestamp"],
                "wiki_title": b["title"],
                "content_sha256": b["content_sha256_declared_fullarticle_coordinate"],
            }
            for b in entry["own_boxes"]
        ],
        routed,
        order,
        cutoff,
        vector_parser=whole_vector,
    )
    if row and row["typed_value"]["value"] != entry["source_expected_vector"]:
        raise ValueError("whole source vector differs from sealed source qualification")
    return row


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    for pin in proof["input_pins"]:
        base.checked(root, pin["path"], pin["sha256"])
    record_ids = [e["item_id"] for e in proof["records"]]
    if (
        set(proof["closed_positive_ids"]) != SOURCE_IDS
        or len(record_ids) != 70
        or set(record_ids) != SOURCE_IDS | HELD
        or SOURCE_IDS & shared.IDS
        or set(proof["prior26_disjoint_ids"]) != shared.IDS
    ):
        raise ValueError("closed Mantra/Bond70/57/13 scope drift")
    bindings = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_bytes())["bindings"], set(SOURCE_LABELS)
    )
    _, admitted = membership.load_admitted(
        root / "imports/official/appearance-membership"
    )
    members = {label: {r[0]: r for r in m["entries"]} for label, m in admitted.items()}
    objects = {}
    for tag, raw in shared.protobuf_fields((root / base.CLIENT).read_bytes()):
        if tag == 1:
            obj = shared.decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate current appearance")
            objects[obj["id"]] = obj | {"object_sha256": base.sha(raw)}
    definitions, routed = stats.physical_field_inputs()
    wiki = {
        r["item_id"]: r["observations"]
        for r in json.loads((root / base.WIKI).read_bytes())["records"].values()
    }
    order, rows, holds = shared.native_order(root), [], []
    source_rows = []
    for entry in proof["records"]:
        row = qualify(
            entry,
            definitions.get(entry["item_key"]),
            bindings.get(entry["item_key"]),
            objects.get(entry["item_id"], {}),
            wiki.get(entry["item_id"], []),
            routed,
            order,
            proof["source_cutoff"],
            members,
        )
        if row and entry["item_id"] in SOURCE_IDS:
            source_rows.append(row)
            physical = any(
                e["kind"] == "ELEMENTAL_BOND"
                and e["parameter"]["value"] == {"kind": "ELEMENT", "value": "PHYSICAL"}
                for e in row["typed_value"]["value"]
            )
            if entry["item_id"] in IDS and not physical:
                rows.append(row)
            elif entry["item_id"] in CARRIER_MISSING and physical:
                holds.append(
                    {
                        "item_id": entry["item_id"],
                        "reason": "CARRIER_MISSING",
                        "carrier": "ReferenceModifierElement.PHYSICAL",
                        "whole_source_vector_preserved": row["typed_value"],
                        "policy": "WHOLE_VECTOR_HELD_NO_PARTIAL_PROMOTION",
                    }
                )
            else:
                raise ValueError("closed Physical carrier partition changed")
        elif not row and entry["item_id"] in HELD:
            holds.append(
                {"item_id": entry["item_id"], "reasons_preserved": entry["holds"]}
            )
        else:
            raise ValueError(f"closed qualified/held source changed:{entry['item_id']}")
    if (
        len(source_rows) != 57
        or sum(len(r["typed_value"]["value"]) for r in source_rows) != 130
        or len(rows) != 49
        or sum(len(r["typed_value"]["value"]) for r in rows) != 114
        or len(holds) != 21
    ):
        raise ValueError("Mantra/Bond source57/130 native49/114 holds21 count drift")
    maps = {
        str(p.relative_to(root)): base.sha(p.read_bytes())
        for f in ("objects", "terrain")
        for p in sorted((root / f"content/world/{f}").glob("*.json"))
    }
    return {
        "schema": "OTERYN_ITEM_STATS_PROMOTION/v2",
        "counts": {
            "fields": 49,
            "items": 49,
            "atoms": 114,
            "holds": 21,
            "source_qualified_vectors": 57,
            "source_qualified_atoms": 130,
            "source_holds": 13,
            "carrier_holds": 8,
        },
        "promotions": sorted(rows, key=lambda r: r["item_key"]),
        "holds": holds,
        "source_policy": "WHOLE_RAW_SOURCE57_NATIVE49_MANTRA_BOND_CONTEXT_UNKNOWN_PHYSICAL8_WHOLE_HELD",
        "sources": {
            "proof": PROOF,
            "proof_sha256": PROOF_SHA,
            "compiler_sha256": base.sha((root / COMPILER).read_bytes()),
            "input_pins": proof["input_pins"],
            "world_owner_inputs": maps,
            "native_enum_sha256": base.sha((root / shared.ENUM).read_bytes()),
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
            raise SystemExit("Mantra/Bond49 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("PASS source57/130 native49/114 holds13+8", base.sha(data))


if __name__ == "__main__":
    main()
