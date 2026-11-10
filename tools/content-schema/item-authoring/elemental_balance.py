"""Read-only, conservative Elemental Resistance candidates from Oteryn Item sources.

This is an authoring *proposal*, not a runtime or canonical Item migration.
The four damage types stay distinct; only a future reviewed Item change may opt in.
No Item schema, generated pins, or source facts are changed by this tool.

Usage:
  python3 tools/content-schema/item-authoring/elemental_balance.py --output /tmp/elemental-report.json
  python3 -m unittest discover -s tools/content-schema/item-authoring -p test_elemental_balance.py
"""
from __future__ import annotations

import argparse
import json
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ELEMENTS = ("FIRE", "ICE", "EARTH", "ENERGY")
PINNED = ROOT / "docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json"


def propose(resistances: dict[str, Fraction]) -> dict | None:
    """Conservative equal-exposure budget; does not erase a vulnerability or field rule.

    Four identical positives collapse losslessly. Otherwise require at least two
    positive direct elements and discount the old four-element budget by 10%.
    A recommendation is not a parity claim for any individual monster or hunt.
    """
    if any(kind.endswith("_FIELD") for kind in resistances):
        return None
    values = [resistances.get(kind, Fraction(0)) for kind in ELEMENTS]
    if any(value < 0 or value.denominator != 1 for value in values):
        return None
    active = [value for value in values if value > 0]
    if len(active) < 2:
        return None
    old = sum(active)
    if len(active) == 4 and len(set(active)) == 1:
        amount = int(active[0])
        mode = "exact"
    else:
        amount = (int(old) * 9) // 40
        mode = "review"
    if amount <= 0:
        return None
    return {
        "mode": mode,
        "percent": amount,
        "old_budget": int(old),
        "new_budget": amount * 4,
        "largest_element_gain": max(amount - int(value) for value in values),
        "largest_element_loss": max(int(value) - amount for value in values),
    }


def known(field: object) -> object | None:
    if isinstance(field, dict) and field.get("state") == "KNOWN":
        return field.get("value")
    return None


def decode_resistances(entries: list[dict]) -> dict[str, Fraction]:
    result: dict[str, Fraction] = {}
    for entry in entries:
        kind = entry["kind"]
        raw = known(entry["percent"])
        if not isinstance(raw, dict) or kind in result:
            raise ValueError("unknown percent or duplicate resistance kind")
        result[kind] = Fraction(raw["numerator"], raw["denominator"])
    return result


def load_items() -> tuple[dict[str, dict], dict]:
    index = json.loads((ROOT / "content/items/index.json").read_text(encoding="utf-8"))
    items: dict[str, dict] = {}
    stats = {"indexed": 0, "known_resistances": 0, "overlay_filled": 0, "conflicts": 0}
    for shard in index["shards"]:
        document = json.loads((ROOT / shard).read_text(encoding="utf-8"))
        for record in document["records"]:
            definition = record["definition"]
            key = definition["identity"]["key"]
            stats["indexed"] += 1
            semantics = definition.get("semantics", {})
            presentation = known(semantics.get("presentation"))
            name = known(presentation.get("name")) if isinstance(presentation, dict) else None
            protection = known(semantics.get("protection"))
            rows = known(protection.get("resistances")) if isinstance(protection, dict) else None
            if rows is not None:
                stats["known_resistances"] += 1
            items[key] = {
                "name": name if isinstance(name, str) else None,
                "resistances": decode_resistances(rows) if rows is not None else None,
                "source": "item_shard" if rows is not None else None,
            }
    if stats["indexed"] != index["record_count"]:
        raise ValueError("Item index count drift")
    packet = json.loads(PINNED.read_text(encoding="utf-8"))
    for fact in packet["facts"]:
        if fact["field_path"] != "protection.resistances":
            continue
        key = fact["item_key"]
        if key not in items:
            raise ValueError(f"pinned Item absent from index: {key}")
        current = decode_resistances(fact["value"])
        original = items[key]["resistances"]
        if original is None:
            items[key]["resistances"] = current
            items[key]["source"] = "equip_abilities_v1"
            stats["overlay_filled"] += 1
        elif original != current:
            # The pinned materializer fills Unknown leaves only. Do not resolve
            # a disagreement by silently choosing the later fact.
            items[key]["source"] = "source_conflict"
            stats["conflicts"] += 1
    return items, stats


def report() -> dict:
    items, stats = load_items()
    candidates = []
    for key, item in sorted(items.items()):
        if item["source"] == "source_conflict" or item["resistances"] is None:
            continue
        result = propose(item["resistances"])
        if result is None:
            continue
        candidates.append({
            "item_key": key,
            "name": item["name"],
            "source": item["source"],
            "old_direct": {kind: str(item["resistances"].get(kind, 0)) for kind in ELEMENTS},
            **result,
        })
    return {
        "schema": "OTERYN_ELEMENTAL_BALANCE_PROPOSAL/v1",
        "source_scope": ["content/items/index.json shards", str(PINNED.relative_to(ROOT))],
        "not_included": ["runtime content overlays", "imbuements", "other equipment slots",
                         "monster damage distribution", "player builds", "source-held conflicts"],
        "formula": "floor(0.9 * sum(fire,ice,earth,energy)/4); exact for four equal",
        "stats": {**stats, "proposals": len(candidates),
                  "exact": sum(c["mode"] == "exact" for c in candidates),
                  "needs_review": sum(c["mode"] == "review" for c in candidates)},
        "candidates": candidates,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="write JSON proposal; never changes Item sources")
    args = parser.parse_args()
    text = json.dumps(report(), ensure_ascii=False, sort_keys=True, indent=2) + "\n"
    if args.output:
        args.output.write_text(text, encoding="utf-8")
        print(f"Saved balance proposal to {args.output}")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
