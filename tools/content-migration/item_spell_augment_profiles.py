"""SPELL-AUGMENT-1: attach pinned XML augment facts to already admitted Items."""
import hashlib
import importlib.util
import json


def apply_native(root, generated, path, manifest_path, canonical_bytes):
    spec = importlib.util.spec_from_file_location(
        "item_spell_augments", root / "tools/content-schema/native-gameplay/item_spell_augments.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    facts = module.pinned_item_spell_augments()
    document = json.loads(generated[path])
    for record in document["records"]:
        item_id = int(record["production_binding"]["external_id"])
        if item_id in facts:
            record["attributes"]["spell_augments"] = facts[item_id]
    data = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    generated[path] = data
    manifest = json.loads(generated[manifest_path])
    manifest["item_profiles"]["sha256"] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical_bytes(manifest)
