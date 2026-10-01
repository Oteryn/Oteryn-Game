#!/usr/bin/env python3
"""Verify normalized evidence from Crystal's pinned imbuements branch, offline."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
PACKET = HERE / "samples/crystal-imbuements-evidence.json"
REVISION = "15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1"
# Canonical compact JSON digests of reviewed captures. Code/HTML are not
# redistributed: these pins keep offline replay bound to the captured facts.
CAPTURE_PINS = {
    "xml": "225d72971dc5ee7bcc71d13db37cc3444c18051903a7a117c1a16abbb280975d",
    "sources": "127e4a203109696f7f9980a398dfe064c2df0a9abcbebb2a3a1f7306ad8a7ebb",
    "engine_facts": "111b65be2f04c3dcfffa80e9c0a25e77fad0cfc9255dc25d66ecc4b39a949f6a",
    "custom_assistant_packages": "75f3b64e5fd444f5e8eb1e8039e277a8d1b1605ef38e20e1baf8dccbd1a28d16",
    "critical_modifier_public_evidence": "9729138159844ba79bed21a94b1b912243e1ea3fe4c01a3bc6b623e3ff19dee8",
    "corrected_previous_scroll_bindings": "e958925aac492b5a91997600aaf25f16efaebcb40f80bf358a844585fe2113cf",
    "original_capture_defects": "5bf19c7ca965caeb1a550d638bf63aa0078edf888bf3842349c54fa29773d429",
}


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True,
                                     separators=(",", ":")).encode()).hexdigest()


def keyed(rows, expected, key):
    keys = [key(row) for row in rows]
    if len(keys) != len(set(keys)) or set(keys) != expected:
        raise ValueError("duplicate, missing or unexpected record key")
    return dict(zip(keys, rows, strict=True))


def configured_effect(record):
    effect = record["effect"]
    kind = effect["type"]
    if kind == "damage":
        return {"kind": "elemental_conversion", "element": effect["combat"],
                "share_bps": int(effect["value"]) * 100}
    if kind == "reduction":
        return {"kind": "protection", "element": effect["combat"],
                "absorb_bps": int(effect["value"]) * 100}
    if kind == "speed":
        return {"kind": "speed_bonus", "amount": int(effect["value"])}
    if kind == "capacity":
        return {"kind": "capacity_bonus", "increase_bps": int(effect["value"]) * 100}
    if kind == "vibrancy":
        return {"kind": "paralysis_deflection", "chance_bps": int(effect["chance"]) * 100}
    if effect["value"] == "critical":
        # The intrinsic character baseline belongs to the combat profile. An
        # item modifier must never include it, or runtime would apply it twice.
        return {"kind": "critical", "chance_bps": int(effect["chance"]),
                "extra_damage_bps": int(effect["bonus"]),
                "value_semantics": "ADDITIVE_IMBUEMENT_MODIFIER"}
    if effect["value"] in ("lifeleech", "manaleech"):
        return {"kind": "leech", "chance_bps": int(effect["chance"]) * 100,
                "share_bps": int(effect["bonus"]),
                "resource": "health" if effect["value"] == "lifeleech" else "mana"}
    skill = {"dist": "distance", "shield": "shielding", "magicpoints": "magic_level"}
    return {"kind": "skill_bonus", "skill": skill.get(effect["value"], effect["value"]),
            "amount": int(effect["bonus"])}


def expected_comparison(xml):
    # Import only pure effect/source-fact functions: build() would recurse through
    # this validator. Exact source bytes and binding bytes are verified separately.
    import imbuement_authoring as authoring
    raw = (HERE / "samples/imbuement-bindings.json").read_bytes()
    if hashlib.sha256(raw).hexdigest() != authoring.EVIDENCE_PINS["imbuement-bindings.json"]:
        raise ValueError("identity evidence differs from its reviewed pin")
    bindings = json.loads(raw)
    wiki = {r["name"]: r for r in authoring.source_facts()["wiki_br"]["records"]}
    bases = {int(b["id"]): b for b in xml["bases"]}
    rows = []
    for record in xml["records"]:
        name, tier = record["name"], record["tier"]
        definition = wiki[name]
        if name == "Strike":
            # The original Wiki BR capture records isolated combined totals.
            # Revised primary/Fandom evidence supplies pure additive modifiers.
            selected = {"kind": "critical", "chance_bps": 500,
                        "extra_damage_bps": (500, 1500, 4000)[tier - 1],
                        "value_semantics": "ADDITIVE_IMBUEMENT_MODIFIER"}
        else:
            selected = authoring.effect(name, definition["tier_effect_numbers"][tier - 1])
        observed = configured_effect(record)
        materials = [{"source_item_id": int(bindings["material_bindings"][m["name"]]
                       ["item_ref"]["key"].split(".i")[-1]), "count": m["count"]}
                     for m in definition["incremental_materials"][:tier]]
        level = ("basic", "intricate", "powerful")[tier - 1]
        scroll = int(bindings["scroll_bindings"][name][level]["item_ref"]["key"].split(".i")[-1])
        rows.append({"name": name, "tier": tier,
                     "materials_match": record["materials"] == materials,
                     "configured_normalized_effect": observed, "selected_effect": selected,
                     "effect_configuration_matches": observed == selected,
                     "strength_numbers_match": observed["chance_bps"] == selected["remove_chance_bps"]
                         if name == "Vibrancy" else observed == selected,
                     "loader_applies_effect": name != "Vibrancy",
                     "scroll_id_matches": record["scroll_item_id"] == scroll,
                     "selected_scroll_item_id": scroll,
                     "apply_fee_matches": int(bases[tier]["price"]) == (7500, 60000, 250000)[tier - 1],
                     "clear_fee_matches": int(bases[tier]["removecost"]) == 15000,
                     "duration_matches": int(bases[tier]["duration"]) * 1000 == 72000000})
        if name == "Strike":
            rows[-1]["configured_effective_total_with_intrinsic_baseline"] = {
                "value_semantics": "ISOLATED_TOTAL_NOT_AN_IMBUEMENT_MODIFIER",
                "chance_bps": observed["chance_bps"] + 500,
                "extra_damage_bps": observed["extra_damage_bps"] + 1000,
                "other_critical_sources": "EXCLUDED_FROM_THIS_COMPARISON",
            }
    summary = {"records": len(rows)}
    for count, field in (("recipes_matching", "materials_match"),
                         ("effects_config_matching", "effect_configuration_matches"),
                         ("strength_numbers_matching", "strength_numbers_match"),
                         ("effects_loaded", "loader_applies_effect"),
                         ("scroll_ids_matching", "scroll_id_matches"),
                         ("fees_matching", "apply_fee_matches")):
        summary[count] = sum(row[field] for row in rows)
    return rows, summary


def parse_xml(raw):
    text = raw.decode("utf-8")
    root = ET.fromstring(text)
    starts = [i for i, line in enumerate(text.splitlines(), 1)
              if re.search(r"<imbuement\s", line)]
    records = []
    for node, line in zip(root.findall("imbuement"), starts, strict=True):
        effects = [dict(a.attrib) for a in node.findall("attribute")
                   if a.get("key") == "effect"]
        if len(effects) != 1:
            raise ValueError("exactly one explicit effect is required")
        records.append({
            "name": node.get("name"), "tier": int(node.get("base")),
            "category_id": int(node.get("category")),
            "premium": node.get("premium") == "1",
            "storage": int(node.get("storage")),
            "scroll_item_id": int(node.get("scrollid")),
            "effect": effects[0],
            "materials": [{"source_item_id": int(a.get("value")),
                           "count": int(a.get("count"))}
                          for a in node.findall("attribute") if a.get("key") == "item"],
            "source_line": line,
        })
    records.sort(key=lambda row: (row["name"], row["tier"]))
    return {
        "bases": [dict(n.attrib) for n in root.findall("base")],
        "categories": [dict(n.attrib) for n in root.findall("category")],
        "records": records,
    }


def parse_previous_scroll_bindings(raw):
    """Capture both XML encodings; missing scroll IDs remain explicit zero."""
    text = raw.decode("utf-8")
    root = ET.fromstring(text)
    starts = [i for i, line in enumerate(text.splitlines(), 1)
              if re.search(r"<imbuement\s", line)]
    rows = []
    for node, line in zip(root.findall("imbuement"), starts, strict=True):
        root_id = node.get("scrollid")
        child_ids = [n.get("value") for n in node if n.get("key") == "scroll"]
        if len(child_ids) > 1 or (root_id is not None and child_ids):
            raise ValueError("ambiguous source scroll encoding")
        value = root_id if root_id is not None else child_ids[0] if child_ids else "0"
        rows.append({"name": node.get("name"), "tier": int(node.get("base")),
                     "scroll_item_id": int(value), "source_line": line,
                     "representation": "ROOT_SCROLLID" if root_id is not None
                         else "CHILD_SCROLL_ATTRIBUTE" if child_ids else "ABSENT"})
    return sorted(rows, key=lambda row: (row["name"], row["tier"]))


def corrected_previous_references(packet):
    """Copy immutable old facts and repair only their normalized scroll lists."""
    import imbuement_authoring as authoring
    captures = packet["corrected_previous_scroll_bindings"]
    if digest(captures) != CAPTURE_PINS["corrected_previous_scroll_bindings"]:
        raise ValueError("corrected scroll capture differs from reviewed XML evidence")
    originals = authoring.source_facts()
    expected_keys = {(name, tier) for name in authoring.LAYOUT for tier in (1, 2, 3)}
    if set(captures) != {"canary", "crystal"}:
        raise ValueError("both pinned previous references are required")
    result = {}
    for name, capture in captures.items():
        source = originals[name]
        if any(capture[field] != source[field] for field in ("revision", "path", "sha256", "url")):
            raise ValueError("corrected scroll evidence must retain original raw source identity")
        rows = keyed(capture["records"], expected_keys, lambda row: (row["name"], row["tier"]))
        corrected = copy.deepcopy(source)
        for record in corrected["records"]:
            scroll_id = rows[record["name"], record["tier"]]["scroll_item_id"]
            record["scroll_item_ids"] = [scroll_id] if scroll_id else []
        result[name] = corrected["records"]
    return result


def verify_previous_xml(packet, source_name, raw):
    if source_name not in ("canary", "crystal"):
        raise ValueError("unknown previous XML reference")
    capture = packet["corrected_previous_scroll_bindings"][source_name]
    if hashlib.sha256(raw).hexdigest() != capture["sha256"]:
        raise ValueError("previous XML differs from its immutable captured source")
    if parse_previous_scroll_bindings(raw) != capture["records"]:
        raise ValueError("corrected scroll bindings differ from reparsed pinned XML")


def expected_previous_comparison(packet):
    """Recompute previous XML differences, including corrected scroll IDs."""
    import imbuement_authoring as authoring
    originals = authoring.source_facts()
    result = {}
    for name, records in corrected_previous_references(packet).items():
        source = originals[name]
        lookup = {(row["name"], row["tier"]): row for row in records}
        differences = []
        for row in packet["xml"]["records"]:
            old = lookup[row["name"], row["tier"]]
            for field in ("effect", "materials", "storage", "premium", "scroll_item_id"):
                previous = (old["scroll_item_ids"][0] if old["scroll_item_ids"] else 0) \
                    if field == "scroll_item_id" else old[field]
                if field == "storage":
                    previous = int(previous)
                if field == "premium":
                    previous = previous == "1"
                if row[field] != previous:
                    differences.append({"name": row["name"], "tier": row["tier"], "field": field,
                                        "new_branch": row[field], "previous_reference": previous})
        result[name] = {
            "revision": source["revision"],
            "new_base_prices": [int(base["price"]) for base in packet["xml"]["bases"]],
            "previous_base_prices": [int(base["price"]) for base in source["bases"]],
            "corrected_scroll_reference": f"corrected_previous_scroll_bindings.{name}",
            "record_differences": differences,
        }
    return result


def validate(packet):
    from imbuement_authoring import LAYOUT
    if packet["revision"] != REVISION or packet["role"] != "OTS_HYPOTHESIS_ONLY":
        raise ValueError("Crystal hypothesis must retain its exact provenance")
    records = packet["xml"]["records"]
    expected_keys = {(name, tier) for name in LAYOUT for tier in (1, 2, 3)}
    record_map = keyed(records, expected_keys, lambda row: (row["name"], row["tier"]))
    for field, pin in CAPTURE_PINS.items():
        if digest(packet[field]) != pin:
            raise ValueError(f"pinned source capture changed: {field}")
    if packet["previous_reference_comparison"] != expected_previous_comparison(packet):
        raise ValueError("previous-reference comparison omits or invents source differences")
    for record in records:
        if not record["materials"] or any(m["count"] < 1 for m in record["materials"]):
            raise ValueError("invalid recipe")
    comparison = packet["selected_catalogue_comparison"]
    keyed(comparison["records"], expected_keys, lambda row: (row["name"], row["tier"]))
    expected_rows, expected_summary = expected_comparison(packet["xml"])
    if comparison["records"] != expected_rows or comparison["summary"] != expected_summary:
        raise ValueError("source comparison differs from independently recomputed facts")
    levels = {"basic": 1, "intricate": 2, "powerful": 3}
    bundle_keys = {(name, level) for name in ("Strike", "Vampirism", "Void") for level in levels}
    bundles = keyed(packet["gold_token_bundles"], bundle_keys, lambda row: (row["name"], row["tier"]))
    for (name, level), bundle in bundles.items():
        tier = levels[level]
        if (bundle["gold_tokens"] != tier * 2 or bundle["token_item_id"] != 22721
                or bundle["materials"] != record_map[name, tier]["materials"]):
            raise ValueError("token bundle differs from the pinned recipe or token price")
    facts = packet["engine_facts"]
    if len({fact["id"] for fact in facts}) != len(facts):
        raise ValueError("duplicate engine fact")
    for fact in packet["engine_facts"]:
        if fact["confidence"] != "OTS_SOURCE_CODE_ONLY":
            raise ValueError("engine code cannot prove official Global behavior")
        for anchor in fact["anchors"]:
            source = packet["sources"][anchor["path"]]
            if not 1 <= anchor["line"] <= source["line_count"]:
                raise ValueError("source anchor is outside pinned file")
            if anchor["url"] != source["url"] + f'#L{anchor["line"]}':
                raise ValueError("source anchor URL does not match its pinned file and line")
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--xml", type=Path, help="Optionally reparse a downloaded pinned XML")
    parser.add_argument("--previous-xml", action="append", default=[], metavar="SOURCE=PATH",
                        help="Reparse corrected canary/crystal scroll captures from pinned raw XML")
    args = parser.parse_args()
    packet = json.loads(PACKET.read_text())
    validate(packet)
    if args.xml:
        raw = args.xml.read_bytes()
        source = packet["sources"]["data/XML/imbuements.xml"]
        if hashlib.sha256(raw).hexdigest() != source["sha256"]:
            raise ValueError("XML differs from the captured pinned source")
        if parse_xml(raw) != packet["xml"]:
            raise ValueError("normalized XML differs from the pinned source")
    for reference in args.previous_xml:
        name, separator, path = reference.partition("=")
        if not separator or not path:
            raise ValueError("previous XML argument must be SOURCE=PATH")
        verify_previous_xml(packet, name, Path(path).read_bytes())
    print("Crystal imbuements: 72 records, 9 token bundles, pinned OTS evidence verified")


if __name__ == "__main__":
    main()
