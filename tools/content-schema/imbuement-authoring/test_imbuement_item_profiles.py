import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "content-migration"))
import imbuement_item_profiles as profiles


class CrystalCaptureTests(unittest.TestCase):
    def test_stage_crystal_checks_historical_facts_with_expanded_native_catalogue(self):
        crystal = b"""
        <items>
          <item id="2854">
            <attribute key="imbuementslot" value="1">
              <attribute key="increase capacity" value="3"/>
            </attribute>
          </item>
          <item id="13997"><attribute key="imbuementslot" value="1"/></item>
        </items>
        """
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            canary = root / profiles.CANARY
            canary.parent.mkdir(parents=True)
            canary.write_text("""
                <items>
                  <item id="13997">
                    <attribute key="imbuementslot" value="1">
                      <attribute key="paralysis deflection" value="3"/>
                    </attribute>
                  </item>
                  <item id="50251">
                    <attribute key="imbuementslot" value="1"/>
                    <attribute key="imbuementslot" value="2"/>
                  </item>
                </items>
            """)
            native = root / "content/items/definitions/spell-native-profiles.json"
            native.parent.mkdir(parents=True)
            native.write_text(
                json.dumps(
                    {
                        "records": [
                            {"production_binding": {"external_id": str(i)}}
                            for i in (2854, 13997, 50251)
                        ]
                    }
                )
            )
            source = root / "crystal.xml"
            source.write_bytes(crystal)
            facts = root / profiles.CRYSTAL_FACTS
            facts.parent.mkdir(parents=True)

            with (
                patch.object(profiles, "__file__", str(root / "tools/capture/main.py")),
                patch.object(
                    profiles,
                    "CRYSTAL_SHA256",
                    hashlib.sha256(crystal).hexdigest(),
                ),
                patch("sys.argv", ["capture", "--stage-crystal", str(source)]) as argv,
            ):
                self.assertEqual(profiles.main(), 0)
                self.assertEqual(
                    json.loads(facts.read_bytes())["records"],
                    {
                        "13997": {"slot_count": 1, "families": {}},
                        "2854": {
                            "slot_count": 1,
                            "families": {"INCREASE_CAPACITY": "3"},
                        },
                    },
                )
                argv.append("--check")
                self.assertEqual(profiles.main(), 0)
