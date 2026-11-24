"""Admit Crystal Store offers against exact native cosmetic looks."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "rulesets/economy/store/cosmetics.json"
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
STORE = "data/modules/scripts/gamestore/gamestore.lua"
MOUNTS = "data/XML/mounts.xml"
DIGESTS = {
    STORE: "6affeb63a5d10fbdff1da5311d6db9e37e2097e6f13a2b14e9d2b8e87bca5df7",
    MOUNTS: "e4ad7d0f7b712a77160484f4c70ef83c66af1c798950b3853acf23f4897c397f",
}
SPELLINGS = {"Artic Unicorn": "Arctic Unicorn", "Hailtorm Fury": "Hailstorm Fury"}
OFFER = re.compile(
    r'name = "([^"]+)",\s+price = (\d+),\s+'
    r'(?:id = (\d+),|sexId = \{ female = (\d+), male = (\d+) \},\s+addon = 3,)'
    r'[^\n]*\n[^\n]*\n\s+type = GameStore.OfferTypes.OFFER_TYPE_(MOUNT|OUTFIT),'
)


def read(path, source):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != DIGESTS[source]:
        raise ValueError(f"{source} differs from Crystal {REVISION}")
    return raw.decode()


def normalize(name):
    return re.sub(r"[^a-z0-9]", "", name.lower())


def build(existing, document, store, mount_xml):
    outfit_looks = {k: v for k, v in existing["source"]["outfit_looks"].items()
                    if k == "oteryn:outfit.field_surgeon"}
    mount_looks = {}
    offers = [row for row in existing["offers"] if row["key"] == "store:outfit:field-surgeon"]
    if len(offers) != 1:
        raise ValueError("retained Field Surgeon offer missing")
    mounts = {row.get("id"): row.attrib for row in ET.fromstring(mount_xml)}
    native_mounts = {row["look"]["id"]: row["key"] for row in document["mounts"]}
    native_outfits = {(row["male"]["id"], row["female"]["id"]): row["key"]
                      for row in document["outfits"] if row["addon_count"] == 2}
    seen = {row["unlock_key"] for row in offers}
    for match in OFFER.finditer(store):
        name, price, mount_id, female, male, kind = match.groups()
        if kind == "MOUNT" and mount_id:
            row = mounts[mount_id]
            if row["type"] != "store" or normalize(SPELLINGS.get(name, name)) != normalize(row["name"]):
                continue
            look = int(row["clientid"])
            key = native_mounts.get(look)
            if key is None:
                continue
            mount_looks[key] = look
            family, preview = "mount", 3
        elif kind == "OUTFIT" and female:
            looks = (int(male), int(female))
            key = native_outfits.get(looks)
            if key is None or key in outfit_looks:
                continue
            outfit_looks[key] = list(looks)
            family, preview = "outfit", 2
        else:
            continue
        if key in seen:
            raise ValueError(f"duplicate Crystal cosmetic offer: {key}")
        seen.add(key)
        offers.append({
            "key": f"store:{family}:" + key.rsplit(".", 1)[1].replace("_", "-"),
            "category": f"cosmetics.{family}s",
            "title": name,
            "price": int(price),
            "currency": "TIBIA_COIN",
            "unlock_key": key,
            "preview": {"kind": preview, "key": key},
            "source_line": store.count("\n", 0, match.start()) + 1,
        })
    source = existing["source"] | {
        "crystal": {"repository": "zimbadev/crystalserver", "revision": REVISION, "sources": DIGESTS},
        "outfit_looks": outfit_looks,
        "mount_looks": mount_looks,
    }
    return existing | {"source": source, "offers": sorted(offers, key=lambda row: row["key"])}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--store", type=Path, required=True)
    parser.add_argument("--mounts", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    document = json.loads((ROOT / "content/cosmetics/outfits/authoring.json").read_bytes())["document"]
    output = build(json.loads(OUT.read_bytes()), document, read(args.store, STORE), read(args.mounts, MOUNTS))
    encoded = json.dumps(output, indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if OUT.read_text() != encoded:
            raise SystemExit("cosmetic store catalogue needs regeneration")
    else:
        OUT.write_text(encoded)


if __name__ == "__main__":
    main()
