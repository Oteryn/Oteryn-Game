"""Generate the E12 catalogue from exact Crystal 201189f sources, never from hand prices.

python3 tools/content-schema/store-authoring/build_hireling_catalogue.py --sources-json /tmp/hireling-donor-201189f.json
python3 tools/content-schema/store-authoring/build_hireling_catalogue.py --sources-json /tmp/hireling-donor-201189f.json --check
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "rulesets/economy/store/hirelings.json"
COMMIT = "201189f871d23597b82caf88fd611454a2ddefb8"
CATALOGUE = "data/modules/scripts/gamestore/gamestore.lua"
RULES = "data/libs/systems/hireling.lua"


def build(files: dict[str, str]) -> dict:
    lua = files[CATALOGUE]
    constants = {}
    for namespace in ("HIRELING_SKILLS", "HIRELING_OUTFITS"):
        table = re.search(rf"{namespace}\s*=\s*\{{(.*?)\n\}}", files[RULES], re.DOTALL)
        if table is None:
            raise ValueError(f"missing {namespace}")
        for name, number in re.findall(r"(\w+)\s*=\s*\{\s*(\d+)\s*,", table.group(1)):
            constants[f"{namespace}.{name}[1]"] = int(number)
    offers = []
    pattern = re.compile(
        r'name = "([^"]+)",\s*price = (\d+),\s*id = ([^,]+),(?:(?!type = GameStore\.OfferTypes\.).)*?type = GameStore\.OfferTypes\.(OFFER_TYPE_HIRELING\w*),',
        re.DOTALL,
    )
    for match in pattern.finditer(lua):
        name, price, source_id, kind = match.groups()
        # A preceding non-Hireling offer must not consume this block through its type.
        if (
            "type = GameStore.OfferTypes."
            in match.group(0)[: match.group(0).rfind("type =")]
        ):
            continue
        source_id = source_id.strip()
        identifier = int(source_id) if source_id.isdecimal() else constants[source_id]
        effect = {
            "OFFER_TYPE_HIRELING": "hire",
            "OFFER_TYPE_HIRELING_SKILL": "unlock",
            "OFFER_TYPE_HIRELING_OUTFIT": "unlock",
            "OFFER_TYPE_HIRELING_NAMECHANGE": "rename",
            "OFFER_TYPE_HIRELING_SEXCHANGE": "sex_change",
        }[kind]
        row = {
            "key": "store:hireling:" + str(identifier),
            "category": "houses.hirelings"
            if kind != "OFFER_TYPE_HIRELING_OUTFIT"
            else "houses.hireling-dresses",
            "title": name,
            "price": int(price),
            "currency": "TIBIA_COIN",
            "hireling": {"effect": effect, "source_id": identifier},
            "source_line": lua.count("\n", 0, match.start()) + 1,
        }
        if effect == "hire":
            row["item"] = {
                "key": "oteryn:item.tibia.i29432",
                "revision": "definition-r1",
                "charges": 0,
            }
        offers.append(row)
    offers.sort(key=lambda row: row["key"])
    if len(offers) != 16 or len({row["key"] for row in offers}) != 16:
        raise ValueError(f"expected 16 distinct donor offers, got {len(offers)}")
    return {
        "schema": "OTERYN_STORE_RUNTIME_CATALOGUE/v1",
        "catalog_revision": "store-hireling-crystal-201189f-v1",
        "source": {
            "repository": "zimbadev/crystalserver",
            "commit": COMMIT,
            "path": CATALOGUE,
            "sha256": hashlib.sha256(lua.encode()).hexdigest(),
        },
        "offers": offers,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sources-json", type=Path)
    parser.add_argument("--crystal", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.sources_json:
        source = json.loads(args.sources_json.read_text(encoding="utf-8"))
        if source["revision"] != COMMIT:
            raise ValueError("source revision mismatch")
        files = source["files"]
    elif args.crystal:
        import subprocess

        revision = subprocess.check_output(
            ["git", "-C", str(args.crystal), "rev-parse", "HEAD"], text=True
        ).strip()
        if revision != COMMIT:
            raise ValueError("Crystal checkout revision mismatch")
        files = {
            path: (args.crystal / path).read_text(encoding="utf-8")
            for path in (CATALOGUE, RULES)
        }
    else:
        parser.error("provide exact --sources-json or --crystal source")
    expected = json.dumps(build(files), indent=2) + "\n"
    if args.check:
        valid = OUTPUT.read_text(encoding="utf-8") == expected
        print(
            "HIRELING_STORE_CATALOGUE_PASS"
            if valid
            else "HIRELING_STORE_CATALOGUE_FAIL"
        )
        return 0 if valid else 1
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(expected, encoding="utf-8")
    print("Generated 16 Crystal Hireling offers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
