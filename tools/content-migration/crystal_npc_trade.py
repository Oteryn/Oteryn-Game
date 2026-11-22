"""Reconcile Crystal prices and item identities in the staged WorldProject."""

import json

from questpc7_world_seal import refresh


def install_crystal_npc_trade(root, package):
    source = json.loads((root / "content/services/trade/crystal-npc-trade.authoring.json").read_bytes())
    path = package / "definitions/declarations.json"
    document = json.loads(path.read_bytes())
    services = {row["identity"]["key"]: row for row in document["records"] if row["kind"] == "Service"}
    for shop in source["shops"]:
        service = services[f"oteryn:service.trade.{shop['npc']}"]
        corrected = {}
        replaced = set()
        for row in shop["offers"]:
            key = f"oteryn:item.tibia.i{row['item']}"
            direction = row["direction"]
            assert (key, direction) not in corrected, "duplicate Crystal offer"
            offer = {
                "item": {"family": "Item", "key": key, "revision": "definition-r1"},
                "direction": direction,
                "unit_price": row["unit_price"],
            }
            if "sub_type" in row:
                offer["sub_type"] = row["sub_type"]
            corrected[key, direction] = offer
            replaced.add((key, direction))
            if "replaces_item" in row:
                replaced.add((f"oteryn:item.tibia.i{row['replaces_item']}", direction))
        service["offers"] = [
            row for row in service.get("offers", [])
            if (row["item"]["key"], row["direction"]) not in replaced
        ] + list(corrected.values())
        # Match ProjectV2ServiceOffer's derived order before the typed reader.
        service["offers"].sort(key=lambda row: (
            row["item"]["key"], row["item"]["revision"], row["direction"],
            row["unit_price"], row.get("currency", {}).get("key", ""),
            row.get("currency", {}).get("revision", ""), row.get("count", -1),
            row.get("sub_type", -1), row.get("parity_pending", False),
        ))

    path.write_text(
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    refresh(root, package)
