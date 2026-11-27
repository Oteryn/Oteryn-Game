"""Generate native profiles for source-backed carried grants omitted by other owners."""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

from inventory_grant_profiles import PREFIX, ROOT, inventory_admissible
from native_profile_composition import encode_native_profile

sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
import build_source_item_profiles as native

CAPTURE = "tools/content-schema/native-gameplay/inventory-grant-item-profiles.json"
CANARY_XML = "imports/canary/items-xml/items.xml"
CRYSTAL = "201189f871d23597b82caf88fd611454a2ddefb8"
CANARY_SHA = "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2"
CRYSTAL_XML_SHA = "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
CRYSTAL_APPEARANCE_SHA = "17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1"
SCHEMA = "OTERYN_NATIVE_INVENTORY_GRANT_PROFILES/v1"
IDS = frozenset({
    3119, 3398, 8055, 8195, 8819, 8822, 9171, 12557, 12802, 13992,
    15696, 15703, 15794, 15802, 15817, 15828, 15832, 16277, 52759,
})


def read_pinned(path, expected):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != expected:
        raise ValueError(f"Inventory grant source digest changed: {path}")
    return raw


def definitions(root):
    wanted = {f"{PREFIX}{item}" for item in IDS}
    return {
        definition["identity"]["key"]: definition
        for path in (root / "content/items/definitions").glob("items-*.json")
        for entry in json.loads(path.read_bytes())["records"]
        if (definition := entry["definition"])["identity"]["key"] in wanted
    }


def capture(root, crystal_xml, appearances):
    from grizzly_shop_profiles import accepted_first

    canary = read_pinned(root / CANARY_XML, CANARY_SHA)
    crystal = read_pinned(crystal_xml, CRYSTAL_XML_SHA)
    dat = read_pinned(appearances, CRYSTAL_APPEARANCE_SHA)
    bindings = native.binding_index((root / "imports/crystalserver/bindings/items.json").read_bytes())
    xml = native.xml_index(crystal)
    # The Stone's profile reads its presentation and weight from the in-tree
    # Canary XML. The exact physical flags come from the pinned appearance.
    xml[16277] = native.xml_index(canary)[16277]
    objects = native.engine_items.load_appearance_objects(dat)
    physical = {entry["server_item_id"]: entry for entry in native.policies(dat, crystal, IDS)}
    canonical = definitions(root)
    records = []
    for item in sorted(IDS):
        binding = bindings[str(item)]
        definition = canonical[f"{PREFIX}{item}"]
        if binding["disposition"] != "EXACT" or binding["target"] != definition["identity"]:
            raise ValueError(f"Inventory grant has no exact binding: {item}")
        row = native.profile(item, xml[item], objects[item], physical[item], bindings, None)
        row["semantics"] = accepted_first(definition.get("semantics", {}), row["semantics"])
        facts = row["semantics"]["physical"]["value"]
        if (not inventory_admissible(row)
                or any(facts[field] != native.K(True) for field in ("movable", "pickupable"))
                or row["semantics"]["stack"]["value"]["stackable"] != native.K(
                    row["admission"]["stack_class"] == "StackCapable")):
            raise ValueError(f"Inventory grant has incompatible physical facts: {item}")
        row.update(production_binding=copy.deepcopy(binding), production_binding_qualification=None,
                   production_definition={"family": "Item", "production_key": definition["identity"]["key"],
                                          "revision_ref": definition["identity"]["revision"]})
        records.append(row)
    return {"schema": SCHEMA, "source_revision": CRYSTAL, "sources": {
        CANARY_XML: CANARY_SHA, "data/items/items.xml": CRYSTAL_XML_SHA,
        "data/items/appearances.dat": CRYSTAL_APPEARANCE_SHA,
    }, "records": records}


def admit(root, document, order):
    capture = json.loads((root / CAPTURE).read_bytes())
    records = capture["records"]
    keys = {row["production_definition"]["production_key"] for row in records}
    if (capture["schema"] != SCHEMA or capture["source_revision"] != CRYSTAL
            or keys != {f"{PREFIX}{item}" for item in IDS} or len(records) != len(IDS)
            or not all(inventory_admissible(row) for row in records)):
        raise ValueError("Inventory grant source capture disagreement")
    current = {row["production_definition"]["production_key"]: row for row in document["records"]}
    for row in records:
        key = row["production_definition"]["production_key"]
        previous = current.get(key)
        if previous is None:
            document["records"].append(row)
        elif (previous["production_definition"] != row["production_definition"]
              or not inventory_admissible(previous)):
            raise ValueError(f"Inventory grant conflicts with an existing provider: {key}")
    document["records"].sort(key=order)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-xml", type=Path, required=True)
    parser.add_argument("--appearances", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = ROOT / CAPTURE
    expected = encode_native_profile(capture(ROOT, args.crystal_xml, args.appearances))
    if args.check:
        if output.read_bytes() != expected:
            raise SystemExit("Inventory grant source capture is stale")
    else:
        output.write_bytes(expected)


if __name__ == "__main__":
    main()
