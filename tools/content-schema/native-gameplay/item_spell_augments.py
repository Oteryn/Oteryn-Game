"""Normalize items.xml augments into server Item attributes (SPELL-AUGMENT-1).

Pinned Canary percentages become permyriad; pinned Crystal already uses
permyriad. Cooldown is milliseconds in both. Values not published by Global
are reference values, not verified against Global. No Wheel data is consumed.
"""
KINDS = {
    "base damage": "base_damage", "base healing": "base_healing",
    "critical hit chance": "critical_hit_chance",
    "critical extra damage": "critical_extra_damage",
    "mana leech": "mana_leech", "life leech": "life_leech",
    "cooldown": "cooldown", "manaleech": "mana_leech",
}


def parse_item_spell_augments(node, source="canary"):
    if source not in ("canary", "crystal"):
        raise ValueError("unqualified augment source")
    groups = node.findall("attribute[@key='augments']") if node is not None else []
    if len(groups) > 1:
        raise ValueError("duplicate augments group")
    result = []
    for group in groups:
        if group.get("value", "0").lower() not in ("1", "true"):
            continue
        for child in group:
            name = child.get("key", "").lower()
            kind = child.get("value", "").lower()
            # Crystal's Base covers both damage and healing. Emit both typed
            # stages; only the stage matching the resolved effect can apply.
            kinds = ["base_damage", "base_healing"] if source == "crystal" and kind == "base" else [KINDS.get(kind)]
            values = child.findall("attribute[@key='value']")
            if not name or len(name) > 128 or any(c not in "abcdefghijklmnopqrstuvwxyz '" for c in name) or None in kinds or len(values) != 1:
                raise ValueError("unsupported item spell augment")
            raw = int(values[0].get("value", ""))
            value = raw * (100 if source == "canary" and kind != "cooldown" else 1)
            if not 0 <= value <= (86400000 if kind == "cooldown" else 100000):
                raise ValueError("item spell augment bounds")
            for normalized in kinds:
                result.append({"spell": name, "kind": normalized, "value": value})
    if len(result) > 16:
        raise ValueError("too many item spell augments")
    return result


def pinned_item_spell_augments():
    """Read the retained numeric projection of the task's exact reference pins."""
    import hashlib
    import json
    from pathlib import Path
    raw = Path(__file__).with_name("item-spell-augment-sources.json").read_bytes()
    if hashlib.sha256(raw).hexdigest() != "e88ea99211d6586c1898e46ebca084e8db881c2eeb19c3988bc2b2f9529c8695":
        raise ValueError("item augment source packet digest")
    document = json.loads(raw)
    source = document["sources"][0]
    if source["revision"] != "47dfd51f45280a59a1d3e50ba7edd573d7234446":
        raise ValueError("item augment source revision")
    return {row["item_id"]: row["augments"] for row in source["records"]}


def main():
    """Reproduce the retained projection from the exact two owner-pinned XML files."""
    import argparse
    import hashlib
    import json
    from pathlib import Path
    import xml.etree.ElementTree as ET
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary-xml", required=True, type=Path)
    parser.add_argument("--crystal-xml", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    pinned_item_spell_augments()  # Authenticate the source metadata before using it.
    document = json.loads(Path(__file__).with_name("item-spell-augment-sources.json").read_bytes())
    for source in document["sources"]:
        path = args.canary_xml if source["server"] == "canary" else args.crystal_xml
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != source["sha256"]:
            raise ValueError("item augment XML source digest")
        records = []
        for item_node in ET.fromstring(raw):
            augments = parse_item_spell_augments(item_node, source["server"])
            if augments:
                records.append({"item_id": int(item_node.get("id")), "augments": augments})
        source["records"] = records
    args.out.write_text(json.dumps(document, indent=2) + "\n")


if __name__ == "__main__":
    main()
