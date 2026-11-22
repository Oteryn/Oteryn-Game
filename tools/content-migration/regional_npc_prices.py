"""Install regional gold prices and clear travel arrivals into the WorldProject."""

import argparse
import json
from pathlib import Path

from questpc7_world_seal import refresh

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "tools/content-schema/npc-authoring/regional-offer-prices.json"
TRAVEL = "crates/gameplay/content/src/content/npc_catalogue/travel_routes.json"


def install(root: Path, package: Path, *, check: bool = False) -> None:
    packet = json.loads((root / SOURCE).read_bytes())
    if packet["schema"] != "OTERYN_REGIONAL_OFFER_PRICES/v1":
        raise ValueError("invalid regional NPC price source")

    declarations = package / "definitions/declarations.json"
    document = json.loads(declarations.read_bytes())
    records = {row["identity"]["key"]: row for row in document["records"]}
    for key, evidence in packet["services"].items():
        service = records[key]
        npc = records[evidence["npc"]]
        if service["kind"] != "Service" or not any(ref["key"] == key for ref in npc["services"]):
            raise ValueError(f"{key}: source NPC does not own the service")
        for correction in evidence["prices"]:
            rows = [
                offer for offer in service["offers"]
                if offer["item"]["key"] == correction["item"]
                and offer["direction"] == correction["direction"]
                and offer.get("count", 1) == 1
                and offer.get("currency") is None
                and offer.get("sub_type") is None
            ]
            price = correction["price"]
            if len(rows) != 1 or type(price) is not int or price <= 0:
                raise ValueError(f"{key}: expected a unique ordinary gold offer for {correction}")
            if check and rows[0]["unit_price"] != price:
                raise ValueError(f"{key}: installed price differs from {correction}")
            rows[0]["unit_price"] = price

    # Harava occupies the donor Issavi arrival. Use Pelagia's clear arrival on
    # the same ship floor for both inbound routes that shared the donor cell.
    for correction in json.loads((root / TRAVEL).read_bytes()):
        if "world_arrival" not in correction:
            continue
        service, key = correction["key"].rsplit("/", 1)
        route, = (row for row in records[service]["routes"] if row["key"] == key)
        fields = ("x", "y", "floor")
        destination = route["destination"]
        current = tuple(destination[field] for field in fields)
        original = tuple(correction[field] for field in fields)
        arrival = tuple(correction["world_arrival"])
        expected = (arrival,) if check else (original, arrival)
        if current not in expected:
            raise ValueError(f"{service}/{key}: unexpected travel arrival {current}")
        if not check:
            destination.update(zip(fields, arrival, strict=True))

    if not check:
        declarations.write_text(
            json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        refresh(root, package)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    install(ROOT, ROOT / "content/world", check=args.check)
