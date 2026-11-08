"""Author one exact-before display-name repair from pinned Crystal source.

The packet is materialized and pinned by regenerate_content.py afterwards.
No dialogue, travel fare, eligibility or placement is changed here.
"""
import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
PACKET = "tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json"
SOURCES = {
    "harlow_vengoth": ("Harlow", "25ee2a79a96c2df5f803445e7e916aa3a437e32adf9a387d43a4d18442e74ff5"),
    "maris_mistrock": ("Maris", "4bcd7342494cb23d49db54e1dfd0026034a8df39e445aefb16f3a185d91d97b1"),
    "ferryman_kamil_meluna": ("Ferryman Kamil", "8d9b6467a86adf0a16da177fa4050ab1ed90b821d85fa414e2855ac1577aef24"),
    "junkar_mines": ("Junkar", "4bcccb2dde2e93c3fb144e6019e63f7df66cc9e6bf761f5d18c6a6312643839d"),
}


def author(slug, raw, root=ROOT):
    name, digest = SOURCES[slug]
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError("Crystal source digest differs")
    match = re.search(rb'local internalNpcName = "([^"]+)"', raw)
    if match is None or match[1].decode() != name:
        raise ValueError("Crystal displayed identity differs")
    key = "oteryn:npc." + slug
    packet_path = root / PACKET
    packet = json.loads(packet_path.read_bytes())
    if any(row["after"]["identity"]["key"] == key for row in packet["repairs"]):
        raise ValueError("NPC already has an authored repair")
    records = []
    for path in sorted((root / "content/npcs/definitions").glob("npcs-*.json")):
        records.extend(row["declaration"] for row in json.loads(path.read_bytes())["records"])
    selected = [row for row in records if row["identity"]["key"] == key]
    if len(selected) != 1:
        raise ValueError("NPC identity is absent or duplicated")
    before = selected[0]
    after = copy.deepcopy(before)
    field = "oteryn:source.npc.display_name"
    if any(row["field_path"] == field for row in before["fields"]):
        raise ValueError("NPC already has a displayed identity")
    after["fields"].append({"field_path": field, "value": {"type": "Text", "value": name}})
    after["fields"].sort(key=lambda row: row["field_path"])
    packet["repairs"].append({
        "before": before,
        "after": after,
        "research": [
            f"OTS_HYPOTHESIS_ONLY: zimbadev/crystalserver@{REVISION}:data-global/npc/{slug}.lua:1; sha256:{digest}; internalNpcName={name}. Existing audited retail anchor retained. Displayed identity only."
        ],
    })
    packet_path.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--npc", choices=sorted(SOURCES), required=True)
    parser.add_argument("--source", type=Path, required=True)
    args = parser.parse_args()
    author(args.npc, args.source.read_bytes())


if __name__ == "__main__":
    main()
