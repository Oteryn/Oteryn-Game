"""The Golem overlay must remain loadable through the production manifest."""
import hashlib
import json
from pathlib import Path
import unittest

import seasonal_creature_profiles

ROOT = Path(__file__).resolve().parents[2]


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


class GolemManifestBindingTests(unittest.TestCase):
    def test_golem_admission_rebinds_every_modified_native_manifest_input(self):
        # Start from an independently pinned predecessor; both providers change.
        providers = {
            "creature_profiles": "content/creatures/definitions/spell-native-profiles.json",
            "presentation_profiles": "content/presentations/definitions/spell-native-profiles.json",
        }
        generated = {path: canonical({"records": []}) for path in providers.values()}
        generated["content/spells.manifest.json"] = canonical({
            name: {"sha256": hashlib.sha256(generated[path]).hexdigest()}
            for name, path in providers.items()
        })
        seasonal_creature_profiles.admit(ROOT, generated, canonical)
        manifest = json.loads(generated["content/spells.manifest.json"])
        for name, path in providers.items():
            with self.subTest(provider=name):
                self.assertEqual(manifest[name]["sha256"], hashlib.sha256(generated[path]).hexdigest())
                self.assertTrue(json.loads(generated[path])["records"])


if __name__ == "__main__":
    unittest.main()
