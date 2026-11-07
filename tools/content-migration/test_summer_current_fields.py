"""Regress current published fields through the real Summer profile transform."""
import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load_owner():
    import sys
    sys.path.insert(0, str(ROOT / "tools/content-migration"))
    spec = importlib.util.spec_from_file_location(
        "summer_source_qualification", ROOT / "tools/content-migration/summer_source_qualification.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class CurrentSummerFieldsTests(unittest.TestCase):
    def test_actual_canonical_profiles_keep_only_published_overrides(self):
        owner = load_owner()
        with tempfile.TemporaryDirectory(prefix="srv7-known-summer-") as directory:
            root = Path(directory)
            (root / "tools/content-schema/native-gameplay").mkdir(parents=True)
            (root / "content/world").mkdir(parents=True)
            (root / "content/world/definitions").symlink_to(
                ROOT / "content/world/definitions", target_is_directory=True
            )
            document = json.loads((ROOT / owner.INPUT).read_bytes())
            keys = {"oteryn:creature.mimar_haffar", "oteryn:creature.phosphorus_final"}
            document["records"] = [row for row in document["records"] if row["creature"] in keys]
            (root / owner.INPUT).write_text(json.dumps(document))
            current = "tools/content-schema/native-gameplay/summer-current-published-fields.json"
            (root / current).write_bytes((ROOT / current).read_bytes())
            provider = json.loads((ROOT / owner.PROVIDER).read_bytes())
            provider["records"] = [
                row for row in provider["records"] if row["profile"]["target"]["key"] in keys
            ]
            declarations = json.loads(
                (ROOT / "content/world/definitions/declarations.json").read_bytes()
            )
            for record in provider["records"]:
                record.pop("monster_melee", None)
                # The owning generator supplies this canonical profile before the final projection.
                record["profile"] = copy.deepcopy(next(
                    row for row in declarations["authoring_profiles"]
                    if row["target"] == record["profile"]["target"]
                ))
            generated = {
                owner.PROVIDER: json.dumps(provider).encode(),
                owner.MANIFEST: (ROOT / owner.MANIFEST).read_bytes(),
            }
            owner.apply_native(root, generated, lambda value: json.dumps(value).encode())
            records = {
                row["profile"]["target"]["key"]: row
                for row in json.loads(generated[owner.PROVIDER])["records"]
            }
            self.assertEqual(
                records["oteryn:creature.mimar_haffar"]["profile"]["data"]["profile"]["health"],
                460000,
            )
            final = records["oteryn:creature.phosphorus_final"]
            self.assertEqual(final["profile"]["data"]["profile"]["experience"], 16000000)
            # Unknown current HP retains the explicit provisional donor value and limitation.
            self.assertEqual(final["profile"]["data"]["profile"]["health"], 90000)
            self.assertIn("not verified against Global", final["source_qualification"])
            closure = next(
                row for row in final["attack_definitions"]["profiles"]
                if row["target"] == final["profile"]["target"]
            )
            self.assertEqual(closure["data"], final["profile"]["data"])
            allowed = {"profile", "presentation", "behavior", "monster_melee",
                       "source_item_abilities", "attack_definitions", "source_qualification"}
            self.assertFalse(set(final) - allowed)


if __name__ == "__main__":
    unittest.main()
