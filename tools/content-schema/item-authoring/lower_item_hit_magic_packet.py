"""Explicit relative hit modifiers and Rune ML in existing field owners."""

import argparse
import json
import re
from fractions import Fraction

import lower_item_name_packet as names

base = names.base
ROOT = base.ROOT
COMPILER = "tools/content-schema/item-authoring/lower_item_hit_magic_packet.py"
PROOF = "docs/agents/evidence/OTV2-20261002-item-hit-magic-source-qualification-v1.json"
PROOF_SHA = "3090e4ca05386c61baa71cb3b04d8bf8f4a99460295fa5f4656af759d6e1938d"
OUTPUT = ROOT / "docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json"


def parse_sources(source, indexed, pages, cutoff):
    values = {}
    for params in names.witness_params(source, indexed, pages, cutoff):
        for parameter, member in (
            ("hit_mod", "hit_chance"),
            ("mlrequired", "required_magic_level"),
        ):
            if member not in source["facts"] or parameter not in params:
                continue
            raw = params[parameter]
            if len(raw) != 1 or not re.fullmatch(
                r"[+-]?[0-9]+" if member == "hit_chance" else r"[0-9]+", raw[0]
            ):
                raise ValueError("present empty/malformed hit/ML field")
            n = int(raw[0])
            if member == "hit_chance":
                if not -(2**63) <= n < 2**63:
                    raise ValueError("hit source-unit native i64 bounds")
                ratio = Fraction(n, 100)
                if not (
                    -(2**63) <= ratio.numerator < 2**63 and ratio.denominator < 2**64
                ):
                    raise ValueError("hit rational native bounds")
                value = {"numerator": ratio.numerator, "denominator": ratio.denominator}
            elif 0 <= n <= 65535:
                value = n
            else:
                raise ValueError("authored magic level bounds")
            if member in values and values[member] != value:
                raise ValueError("hit/ML all-present disagreement")
            values[member] = value
    if values != source["facts"]:
        raise ValueError("closed hit/ML values drift")
    return values


def qualify(source, definition, owners, facts):
    target = source["target"]
    owner = [o for o in owners if o["item"]["key"] == target["key"]]
    if len(owner) > 1 or any(o["item"] != target for o in owner):
        raise ValueError("hit/ML authoring target/duplicate")
    name = names.global_frame.leaf(definition, "presentation.name")
    if name["state"] not in {"UNKNOWN", "KNOWN"} or (
        name["state"] == "KNOWN"
        and name["value"].strip().casefold()
        != source["official_name"].strip().casefold()
    ):
        raise ValueError("normal native name guard")
    if "hit_chance" in facts:
        leaf = names.global_frame.leaf(definition, "weapon.hit_chance")
        if leaf != {"state": "UNKNOWN"} and leaf != {
            "state": "KNOWN",
            "value": facts["hit_chance"],
        }:
            raise ValueError("blocked/conflicting hit leaf/group")
    if "required_magic_level" in facts and any(
        o.get("required_magic_level") not in (None, facts["required_magic_level"])
        for o in owner
    ):
        raise ValueError("conflicting authored magic level")
    hit = facts.get("hit_chance")
    return {
        "target": target,
        "expected_name": source["official_name"],
        "facts": facts,
        "hit_percentage_points": hit["numerator"] * 100 // hit["denominator"]
        if hit is not None
        else None,
    }


def build(root=ROOT):
    proof = json.loads(base.checked(root, PROOF, PROOF_SHA))
    definitions, owners, maps, indexed, pages = names.load_inputs(root, proof)
    rows, seen, counts = [], set(), {"hit_chance": 0, "required_magic_level": 0}
    for source in proof["records"]:
        iid, target = source["source_item_id"], source["target"]
        if iid in seen or len(source["facts"]) != 1:
            raise ValueError("closed hit/ML identity/scope drift")
        seen.add(iid)
        facts = parse_sources(source, indexed, pages, proof["qualification_cutoff"])
        for member in facts:
            counts[member] += 1
        rows.append(qualify(source, definitions[target["key"]], owners, facts))
    if len(rows) != 67 or counts != {"hit_chance": 28, "required_magic_level": 39}:
        raise ValueError("closed67 hit/ML scope drift")
    return {
        "schema": "OTERYN_ITEM_HIT_MAGIC_PROMOTION/v1",
        "compiler": {
            "path": COMPILER,
            "sha256": base.sha((root / COMPILER).read_bytes()),
        },
        "sources": {
            "proof_path": PROOF,
            "proof_sha256": PROOF_SHA,
            "world_owner_inputs": maps,
        },
        "counts": {"fields": 67, "items": 67},
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
            raise SystemExit("hit/ML packet drift")
    else:
        OUTPUT.write_bytes(data)
    print("closed67 hit/ML fields/67Items")


if __name__ == "__main__":
    main()
