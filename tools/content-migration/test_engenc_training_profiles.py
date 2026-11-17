"""Closed source admission for the Child of Destiny training spear."""

import copy
import hashlib
import json
import unittest

import engenc_training_profiles as training

PROFILE = "native-items"
MANIFEST = "native-manifest"


class TrainingProfiles(unittest.TestCase):
    def generated(self):
        return {
            PROFILE: b'{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}',
            MANIFEST: b'{"item_profiles":{"sha256":"pending"}}',
        }

    def admit(self, generated):
        training.admit(
            training.ROOT,
            generated,
            PROFILE,
            MANIFEST,
            lambda record: record["production_definition"]["production_key"],
            lambda value: json.dumps(value).encode(),
        )

    def test_source_admits_one_exact_portable_stackable_spear(self):
        generated = self.generated()
        self.admit(generated)
        records = json.loads(generated[PROFILE])["records"]
        self.assertEqual(len(records), 1)
        record = records[0]
        self.assertEqual(
            record["production_definition"]["production_key"], training.KEY
        )
        self.assertEqual(record["production_binding"]["disposition"], "EXACT")
        self.assertEqual(
            record["admission"],
            {
                "materializable": True,
                "stack_class": "StackCapable",
                "legal_destinations": ["CharacterInventory", "Ground"],
            },
        )
        self.assertEqual(
            record["semantics"]["weapon"]["value"]["attack"],
            {"state": "KNOWN", "value": 15},
        )
        self.assertEqual(
            record["semantics"]["equipment"]["value"]["patterns"]["value"][0][
                "vocations"
            ],
            {"state": "KNOWN", "value": ["NONE"]},
        )
        self.assertEqual(
            record["semantics"]["stack"]["value"]["stack_max"],
            {"state": "KNOWN", "value": 100},
        )
        self.assertEqual(
            json.loads(generated[MANIFEST])["item_profiles"]["sha256"],
            hashlib.sha256(generated[PROFILE]).hexdigest(),
        )

    def test_duplicate_native_owner_is_rejected(self):
        generated = self.generated()
        self.admit(generated)
        with self.assertRaisesRegex(ValueError, "ENGENC_TRAINING_DUPLICATE"):
            self.admit(generated)

    def test_changed_source_identity_stack_and_collision_are_rejected(self):
        original = json.loads((training.ROOT / training.FACTS).read_bytes())
        xml = (training.ROOT / training.XML).read_bytes()
        for field in ["identity", "stack", "collision", "source"]:
            facts = copy.deepcopy(original)
            if field == "identity":
                facts["item_id"] += 1
            elif field == "stack":
                facts["flags"]["flags.cumulative"] = False
            elif field == "collision":
                facts["physical"]["block_solid"] = True
            else:
                facts["source_sha256"] = "0" * 64
            with (
                self.subTest(field=field),
                self.assertRaisesRegex(ValueError, "ENGENC_TRAINING_SOURCE_FACTS"),
            ):
                training.validate(facts, xml)

    def test_unknown_vocation_constraint_still_refuses(self):
        native = training.importer(training.ROOT)
        facts = json.loads((training.ROOT / training.FACTS).read_bytes())
        node = native.xml_index((training.ROOT / training.XML).read_bytes())[17110]
        node.find("attribute[@key='script']/attribute[@key='vocation']").set(
            "value", "Unknown"
        )
        bindings = native.binding_index(
            (training.ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
        )
        with self.assertRaisesRegex(ValueError, "unknown source vocation constraint"):
            native.profile(
                17110,
                node,
                {"flags": facts["flags"]},
                facts["physical"],
                bindings,
                None,
            )


if __name__ == "__main__":
    unittest.main()
