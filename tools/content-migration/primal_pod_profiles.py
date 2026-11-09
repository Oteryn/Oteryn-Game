"""Carry the pinned source Primal Pod through the existing native Item provider."""
import hashlib
import json

SOURCE = "tools/content-schema/native-gameplay/primal-pod-item-profile.json"
KEY = "oteryn:item.tibia.i39176"


def admit(root, generated, destination, manifest_path, order, canonical):
    source = json.loads((root / SOURCE).read_bytes())
    if (source["schema"] != "OTERYN_NATIVE_PRIMAL_POD_ITEM_PROFILE/v1"
            or source["source_revision"] != "201189f871d23597b82caf88fd611454a2ddefb8"
            or len(source["records"]) != 1):
        raise ValueError("PRIMAL_POD_SOURCE_PROFILE")
    row = source["records"][0]
    if (row["production_definition"] != {"family": "Item", "production_key": KEY, "revision_ref": "definition-r1"}
            or row["admission"] != {"materializable": True, "stack_class": "NonStackable", "legal_destinations": ["Ground"]}
            or any(row["attributes"][name] is not False for name in
                   ("blocks_movement", "blocks_projectile", "immovable_block_solid"))
            or row["semantics"]["physical"]["value"]["movable"] != {"state": "KNOWN", "value": False}
            or row["semantics"]["physical"]["value"]["pickupable"] != {"state": "KNOWN", "value": False}
            or row["semantics"]["temporal"] != {"state": "NOT_APPLICABLE"}):
        raise ValueError("PRIMAL_POD_SOURCE_POLICY")
    document = json.loads(generated[destination])
    existing = [entry for entry in document["records"] if entry["production_definition"]["production_key"] == KEY]
    if existing and existing != [row]:
        raise ValueError("PRIMAL_POD_PROFILE_CONFLICT")
    if not existing:
        document["records"].append(row)
    document["records"].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)
