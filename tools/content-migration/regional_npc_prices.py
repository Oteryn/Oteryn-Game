"""Install regional gold prices into the WorldProject."""

import argparse
import json
from pathlib import Path

from questpc7_world_seal import refresh

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "tools/content-schema/npc-authoring/regional-offer-prices.json"


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
        if service["kind"] != "Service" or not any(
            ref["key"] == key for ref in npc["services"]
        ):
            raise ValueError(f"{key}: source NPC does not own the service")
        if excluded := evidence.get("exclude_items"):
            offers = [
                offer
                for offer in service["offers"]
                if offer["item"]["key"] not in excluded
            ]
            if check and offers != service["offers"]:
                raise ValueError(
                    f"{key}: installed shop includes offers absent from Crystal"
                )
            if len(offers) != len(evidence["prices"]):
                raise ValueError(f"{key}: shop offer count differs from Crystal")
            service["offers"] = offers
        for correction in evidence["prices"]:
            rows = [
                offer
                for offer in service["offers"]
                if offer["item"]["key"] == correction["item"]
                and offer["direction"] == correction["direction"]
                and offer.get("count", 1) == 1
                and offer.get("currency") is None
                and offer.get("sub_type") is None
            ]
            price = correction["price"]
            if len(rows) != 1 or type(price) is not int or price <= 0:
                raise ValueError(
                    f"{key}: expected a unique ordinary gold offer for {correction}"
                )
            if check and rows[0]["unit_price"] != price:
                raise ValueError(f"{key}: installed price differs from {correction}")
            rows[0]["unit_price"] = price

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
