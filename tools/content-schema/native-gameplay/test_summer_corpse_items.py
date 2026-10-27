"""Keep Summer corpse admission within its Item ownership boundary."""

import json
import unittest

import summer_corpse_items as summer


class SummerItemOwnershipTests(unittest.TestCase):
    def test_loot_admission_excludes_existing_imbuement_and_refinement_owners(self):
        bindings = json.loads((summer.ROOT / (
            "tools/content-schema/imbuement-authoring/samples/imbuement-bindings.json"
        )).read_bytes())
        refinement = json.loads((summer.ROOT / (
            "imports/tibiawiki/facts/items-external-family18-20261001.json"
        )).read_bytes())
        owned = {51442, 51443}

        def collect(value):
            if isinstance(value, dict):
                for child in value.values():
                    collect(child)
            elif isinstance(value, list):
                for child in value:
                    collect(child)
            elif isinstance(value, str) and value.startswith("oteryn:item.tibia.i"):
                owned.add(int(value.rsplit("i", 1)[1]))

        collect(bindings)
        collect(refinement)
        self.assertFalse(summer.LOOT_IDS & owned)

    def test_utility_records_remain_unmaterializable_and_unchanged(self):
        path = summer.ROOT / "content/items/definitions/items-28000-28499.json"
        before = json.loads(path.read_bytes())
        outputs = summer.outputs()
        after = json.loads(outputs.get(path, path.read_bytes()))
        keys = {f"oteryn:item.tibia.i{i}" for i in (51442, 51443)}

        def utilities(shard):
            return {r["definition"]["identity"]["key"]: r for r in shard["records"]
                    if r["definition"]["identity"]["key"] in keys}

        records = utilities(before)
        self.assertEqual(set(records), keys)
        self.assertEqual(records, utilities(after))
        for record in records.values():
            self.assertFalse(record["definition"]["materializable"])
            self.assertNotEqual(record["definition"]["stack_class"], "StackCapable")

    def test_committed_shards_are_deterministic_output(self):
        for path, raw in summer.outputs().items():
            with self.subTest(path=path.relative_to(summer.ROOT)):
                self.assertEqual(path.read_bytes(), raw)


if __name__ == "__main__":
    unittest.main()
