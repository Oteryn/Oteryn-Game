import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build_runtime_catalogue as runtime


class RuntimeCatalogueTests(unittest.TestCase):
    def test_committed_charm_entitlement_needs_no_item(self):
        self.assertEqual(runtime.check(), [])

    def test_charm_entitlement_still_requires_valid_price_and_currency(self):
        for field, value, message in (
            ("price", 0, "price or currency"),
            ("currency", "GOLD", "price or currency"),
            ("item", {"key": "unexpected"}, "entitlement cannot deliver an item"),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                catalogue = json.loads(runtime.OUT.read_text(encoding="utf-8"))
                offer = next(
                    row
                    for row in catalogue["offers"]
                    if row["key"] == "store:charm-expansion"
                )
                offer[field] = value
                path = Path(directory) / "catalogue.json"
                path.write_text(json.dumps(catalogue), encoding="utf-8")
                with patch.object(runtime, "OUT", path):
                    self.assertIn(f"store:charm-expansion: {message}", runtime.check())
