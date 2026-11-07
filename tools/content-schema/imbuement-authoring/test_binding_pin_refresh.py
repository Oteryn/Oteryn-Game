"""Canonical hash refresh keeps reviewed identities and source evidence fixed."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import binding_evidence as evidence


class BindingPinRefreshTests(unittest.TestCase):
    def test_only_canonical_definition_hashes_can_change(self):
        packet = {"item_ref": {"key": "oteryn:item.tibia.i10275"},
                  "evidence": {"canonical": {"definition_sha256": "old", "name": "Orc Tooth"},
                               "primary_client": {"id": 10275}}}
        previous = json.dumps(packet).encode()
        for kind in ("definition", "identity", "source", "baseline"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                output = root / "bindings.json"
                pin = root / "imbuement_authoring.py"
                digest = hashlib.sha256(previous).hexdigest()
                pin.write_text('EVIDENCE_PINS = {"imbuement-bindings.json": "' + digest + '",}\n')
                changed = copy.deepcopy(packet)
                changed["evidence"]["canonical"]["definition_sha256"] = "new"
                if kind == "identity":
                    changed["item_ref"]["key"] = "oteryn:item.tibia.i10276"
                elif kind == "source":
                    changed["evidence"]["primary_client"]["id"] = 10276
                candidate = json.dumps(changed).encode()
                baseline = previous if kind != "baseline" else b'{}'
                with patch.object(evidence, "HERE", root), patch.object(evidence, "ROOT", root), \
                     patch.object(evidence, "OUTPUT", output), \
                     patch.object(evidence.subprocess, "check_output", return_value=baseline):
                    if kind == "definition":
                        evidence.refresh_definition_pin("reviewed", candidate)
                        self.assertEqual(output.read_bytes(), candidate)
                        self.assertIn(hashlib.sha256(candidate).hexdigest(), pin.read_text())
                    else:
                        original_pin = pin.read_text()
                        with self.assertRaises(ValueError):
                            evidence.refresh_definition_pin("reviewed", candidate)
                        self.assertFalse(output.exists())
                        self.assertEqual(pin.read_text(), original_pin)


if __name__ == "__main__":
    unittest.main()
