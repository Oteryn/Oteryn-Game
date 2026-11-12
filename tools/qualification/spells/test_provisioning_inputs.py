"""Keep boot's bounded cosmetics closure compatible with the committed input."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from provisioning_inputs import MAX_AUTHORING_BYTES, select_cosmetics_inputs


class CosmeticsProvisioningTests(unittest.TestCase):
    def test_committed_cosmetics_closure(self):
        root = Path(__file__).resolve().parents[3]
        manifest = root / "content/spells.manifest.json"
        document = json.loads(manifest.read_bytes())
        locator = (manifest.parent / document["cosmetics"]["path"]).resolve()
        selected = {locator.relative_to(root).as_posix(): locator.read_bytes()}
        select_cosmetics_inputs(document, manifest, root, selected)
        descriptor = json.loads(locator.read_bytes())
        authoring = (locator.parent / descriptor["authoring"]["path"]).resolve()
        self.assertEqual(selected[authoring.relative_to(root).as_posix()], authoring.read_bytes())

    def test_authoring_still_has_a_finite_bound(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch).resolve()
            data = b" " * (MAX_AUTHORING_BYTES + 1)
            (root / "authoring.json").write_bytes(data)
            descriptor = {
                "schema": "OTERYN_NATIVE_COSMETICS_INPUT/v1",
                "authoring": {"path": "authoring.json", "sha256": hashlib.sha256(data).hexdigest()},
            }
            (root / "descriptor.json").write_text(json.dumps(descriptor))
            selected = {"descriptor.json": (root / "descriptor.json").read_bytes()}
            with self.assertRaisesRegex(ValueError, "exceeds provisioning bound"):
                select_cosmetics_inputs(
                    {"cosmetics": {"path": "descriptor.json"}},
                    root / "manifest.json", root, selected,
                )


if __name__ == "__main__":
    unittest.main()
