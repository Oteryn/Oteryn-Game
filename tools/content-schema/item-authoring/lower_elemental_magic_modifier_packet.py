"""Closed whole-raw modifier26; retain empty/shared source holds and old419 packet."""

import argparse
import json
import re

import lower_wiki_stack_default_packet as base
import lower_wiki_stats_packet as stats
from engine_items import (
    CORPSE_FLAGS,
    GROUND_OR_BORDER_FLAGS,
    decode_appearance_object,
    protobuf_fields,
)
from lower_wiki_movable_packet import leaf

ROOT = base.ROOT
COMPILER = (
    "tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py"
)
PROOF = "docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json"
PROOF_SHA = "c800852f41264107087e23935081932b3072265cf5d858e1adf2c99164c70608"
OUTPUT = (
    ROOT
    / "docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json"
)
ENUM = "apps/game-server/src/content/reference_playable.rs"
IDS = frozenset(
    {
        36664,
        36665,
        37611,
        39152,
        39154,
        39182,
        39185,
        39188,
        40594,
        43881,
        43884,
        43886,
        43887,
        44622,
        44623,
        44624,
        49534,
        51263,
        51264,
        52348,
        52353,
        53219,
        53220,
        53221,
        53222,
        53230,
    }
)
ALIASES = {
    element + " magic level": element.upper() + "_MAGIC_LEVEL_POINTS"
    for element in ("death", "earth", "energy", "fire", "healing", "holy", "ice")
}
GROUP = ("attrib", *stats.MODIFIER_PERCENTS, "mantra", "elementalbond")


def native_order(root=ROOT):
    body = (
        (root / ENUM)
        .read_text()
        .split("item_enum!(ReferenceSkillModifierKind {", 1)[1]
        .split("});", 1)[0]
    )
    return {
        re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).upper(): int(value)
        for name, value in re.findall(r"(\w+)\s*=\s*(\d+)", body)
    }


def whole_vector(fields, order):
    raw = {k: v for k, v in fields.items() if k in GROUP}
    if not raw or any(len(v) != 1 or not v[0] for v in raw.values()):
        return None
    other = {k: v[0] for k, v in raw.items() if k != "attrib"}
    _, typed = stats.modifiers(other)
    if typed == "MALFORMED":
        return None
    entries = {e["kind"]: e for e in typed["value"]} if typed else {}
    if "attrib" in raw:
        for part in raw["attrib"][0].lower().split(","):
            match = re.fullmatch(r"([a-z ]+) ([+-]?[0-9]+)", part.strip())
            kind = (stats.MODIFIER_POINTS | ALIASES).get(match[1]) if match else None
            if kind is None or kind in entries or not -(2**31) <= int(match[2]) < 2**31:
                return None
            entries[kind] = {
                "kind": kind,
                "target_domain": {"state": "UNKNOWN"},
                "evaluation_phase": {"state": "UNKNOWN"},
                "priority": {"state": "UNKNOWN"},
                "parameter": {
                    "state": "KNOWN",
                    "value": {"kind": "SIGNED_POINTS", "value": int(match[2])},
                },
            }
    if not entries or len(entries) > 37:
        return None
    return {
        "kind": "MODIFIERS",
        "value": [entries[k] for k in sorted(entries, key=order.__getitem__)],
    }


def qualify(
    entry,
    definition,
    binding,
    obj,
    observations,
    routed,
    order,
    cutoff,
    vector_parser=whole_vector,
):
    iid, key = entry["item_id"], entry["item_key"]
    if (
        not definition
        or definition["identity"] != entry["binding"]["target"]
        or binding != entry["binding"]
        or not binding
        or binding["external_id"] != str(iid)
        or binding["target"]["key"] != key
        or key in routed
        or obj.get("id") != iid
        or obj.get("name") != entry["official_client_name"]
        or obj.get("object_sha256") != entry["official_raw_object_sha256"]
        or leaf(definition, "presentation.name")["state"] not in {"UNKNOWN", "KNOWN"}
        or (
            leaf(definition, "presentation.name")["state"] == "KNOWN"
            and leaf(definition, "presentation.name")["value"].strip().casefold()
            != obj["name"].strip().casefold()
        )
        or observations != entry["retained_observations"]
    ):
        return None
    flags = obj.get("flags", {})
    if flags.get("flags.take") is not True or any(
        flags.get(k) is True
        for k in CORPSE_FLAGS
        + GROUND_OR_BORDER_FLAGS
        + ("flags.unmove", "flags.liquidpool")
    ):
        return None
    refs, boxes = entry["complete_global_own_id_references"], entry["own_boxes"]
    if not boxes or len(boxes) != len(refs) or len(boxes) != len(observations):
        return None
    values = []
    for box, ref, observed in zip(
        boxes, refs, sorted(observations, key=lambda o: o["page_id"]), strict=True
    ):
        if (
            box["reference"] != ref
            or not box["balanced"]
            or box["inside_comment"]
            or not box["positive_exact_infobox_object_match"]
            or base.sha(box["raw"].encode()) != box["raw_sha256"]
            or box["revision_timestamp"] > cutoff
            or any(
                observed[k] != box[k]
                for k in ("page_id", "revision_id", "revision_timestamp")
            )
            or observed["wiki_title"] != box["title"]
            or observed["content_sha256"]
            != box["content_sha256_declared_fullarticle_coordinate"]
        ):
            return None
        fields = base.raw_parameters(box["raw"])
        if (
            fields != box["parameter_values"]
            or fields.get("itemid") != [str(iid)]
            or any(
                len(fields.get(k, [])) != 1
                or fields[k][0].strip().casefold() != obj["name"].strip().casefold()
                for k in ("name", "actualname")
            )
        ):
            return None
        typed = vector_parser(fields, order)
        if typed is None:
            return None
        values.append(typed)
    if any(v != values[0] for v in values):
        return None
    if leaf(definition, "skill_modifiers.modifiers") not in (
        {"state": "UNKNOWN"},
        {"state": "KNOWN", "value": values[0]["value"]},
    ):
        return None
    return {
        "item_key": key,
        "target": definition["identity"],
        "field_path": "skill_modifiers.modifiers",
        "typed_value": values[0],
    }


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    for pin in proof["input_pins"]:
        base.checked(root, pin["path"], pin["sha256"])
    if set(proof["closed_positive_ids"]) != IDS or len(proof["records"]) != 49:
        raise ValueError("closed modifier49/26 source scope drift")
    bound = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        {
            "ff7ede593c69d4c658b382c97443e8155926924a",
            "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
        },
    )
    objects = {}
    for tag, raw in protobuf_fields((root / base.CLIENT).read_bytes()):
        if tag == 1:
            obj = decode_appearance_object(raw)
            if obj["id"] in objects:
                raise ValueError("duplicate current appearance")
            objects[obj["id"]] = obj | {"object_sha256": base.sha(raw)}
    definitions, routed = stats.physical_field_inputs()
    wiki = {
        r["item_id"]: r["observations"]
        for r in json.loads((root / base.WIKI).read_text())["records"].values()
    }
    maps = {
        str(path.relative_to(root)): base.sha(path.read_bytes())
        for family in ("objects", "terrain")
        for path in sorted((root / f"content/world/{family}").glob("*.json"))
    }
    order, rows, holds = native_order(root), [], []
    for entry in proof["records"]:
        row = qualify(
            entry,
            definitions.get(entry["item_key"]),
            bound.get(entry["item_key"]),
            objects.get(entry["item_id"], {}),
            wiki.get(entry["item_id"], []),
            routed,
            order,
            proof["source_cutoff"],
        )
        if row and entry["item_id"] in IDS:
            rows.append(row)
        elif not row and entry["item_id"] not in IDS:
            holds.append(
                {"item_id": entry["item_id"], "reasons_preserved": entry["holds"]}
            )
        else:
            raise ValueError(f"closed qualified/held source changed:{entry['item_id']}")
    if (
        len(rows) != 26
        or sum(len(r["typed_value"]["value"]) for r in rows) != 63
        or len(holds) != 23
    ):
        raise ValueError("modifier26/63/23 count drift")
    return {
        "schema": "OTERYN_ITEM_STATS_PROMOTION/v2",
        "counts": {"fields": 26, "items": 26, "atoms": 63, "holds": 23},
        "promotions": sorted(rows, key=lambda r: r["item_key"]),
        "holds": holds,
        "source_policy": "WHOLE_RAW_CLOSED26_METADATA_CONTEXT_UNKNOWN",
        "sources": {
            "proof": PROOF,
            "proof_sha256": PROOF_SHA,
            "compiler_sha256": base.sha((root / COMPILER).read_bytes()),
            "input_pins": proof["input_pins"],
            "world_owner_inputs": maps,
            "native_enum_sha256": base.sha((root / ENUM).read_bytes()),
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
            raise SystemExit("modifier26 packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("PASS vectors26 atoms63 holds23", base.sha(data))


if __name__ == "__main__":
    main()
