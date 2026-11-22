import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import rust_lottery_profiles as profiles


class RustLotteryProfiles(unittest.TestCase):
    def test_committed_capture_admits_shield_and_helmet_inputs(self):
        generated = {
            "profiles": b'{"records":[]}',
            "manifest": b'{"item_profiles":{}}',
        }
        profiles.admit(
            profiles.ROOT, generated, "profiles", "manifest",
            lambda row: row["production_definition"]["production_key"],
            lambda document: json.dumps(document).encode(),
        )
        keys = {row["production_definition"]["production_key"]
                for row in json.loads(generated["profiles"])["records"]}
        self.assertEqual({f"oteryn:item.tibia.i{item}" for item in profiles.IDS}, keys)

    def test_admission_requires_one_complete_capture(self):
        expanded = {8902, 8907, 8908}
        base = profiles.IDS - expanded
        with TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / profiles.PATH
            path.parent.mkdir(parents=True)

            def admit(ids):
                records = [
                    {"production_definition": {"production_key": f"oteryn:item.tibia.i{item}"}}
                    for item in ids
                ]
                path.write_text(json.dumps({
                    "schema": "OTERYN_NATIVE_RUST_LOTTERY_PROFILES/v1",
                    "records": records,
                }))
                generated = {
                    "profiles": b'{"records":[]}',
                    "manifest": b'{"item_profiles":{}}',
                }
                profiles.admit(
                    root, generated, "profiles", "manifest",
                    lambda row: row["production_definition"]["production_key"],
                    lambda document: json.dumps(document).encode(),
                )
                self.assertEqual(len(json.loads(generated["profiles"])["records"]), len(ids))

            admit(profiles.IDS)
            for ids in (base, base | {8902}):
                with self.subTest(ids=ids), self.assertRaisesRegex(ValueError, "RUST_LOTTERY_PROFILE_SET"):
                    admit(ids)
