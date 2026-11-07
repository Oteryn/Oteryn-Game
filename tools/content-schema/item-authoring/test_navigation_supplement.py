"""Navigation qualification retains source/identity holds and gameplay UNKNOWN."""

import json
import tempfile
import unittest
from pathlib import Path

import engine_items as engine


class NavigationSupplementTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.key = "oteryn:item.navigation.alpha"
        self.owner = Path(self.temp.name) / "owner.json"
        self.owner.write_text(
            json.dumps(
                {
                    "schema": engine.OWNER_FAMILY_DECISIONS_SCHEMA,
                    "decisions": {
                        self.key: {
                            "name": "tested item",
                            "profile": "quest_item",
                            "reason": "Retained explicit per-ID owner review.",
                            "source": {
                                "wiki_url": None,
                                "facts": "Reviewed tested item.",
                            },
                        }
                    },
                }
            )
        )
        self.definition = {
            "identity": {"key": self.key},
            "semantics": {
                "presentation": {
                    "state": "KNOWN",
                    "value": {"name": {"state": "KNOWN", "value": "tested item"}},
                }
            },
        }

    def qualify(self, fields=None, flags=None, definition=None, **kwargs):
        snapshot = {
            "snapshot_sha256": "0" * 64,
            "records": {
                "source-record": {
                    "item_id": 17,
                    "observations": [{"fields": fields or {}}],
                }
            },
        }
        return engine.qualified_navigation_supplement(
            {self.key: definition or self.definition},
            snapshot,
            {17: {"name": "tested item", "flags": flags or {}}},
            identity_index=kwargs.pop(
                "identity_index", {17: (self.key, "crystal_exact_binding")}
            ),
            owner_decisions_path=self.owner,
            **kwargs,
        )

    def test_owner_guard_uses_explicit_binding_and_preserves_higher_holds(self):
        row = self.qualify()[self.key]
        self.assertEqual(row["profile"], "quest_item")
        self.assertEqual(
            row["source_evidence"]["identity_bridge"]["source_item_id"], 17
        )
        self.assertEqual(row["source_evidence"]["scope"], "NAVIGATION_ONLY")
        self.assertEqual(
            row["source_evidence"]["source_inputs"]["role"], "TEST_FIXTURE"
        )
        self.assertEqual(
            len(
                row["source_evidence"]["source_inputs"]["canonical_definitions_sha256"]
            ),
            64,
        )
        self.assertEqual(self.qualify({"primarytype": "Others"}), {})
        self.assertEqual(self.qualify(routed_keys={self.key}), {})
        duplicate = {
            17: (self.key, "crystal_exact_binding"),
            18: (self.key, "crystal_exact_binding"),
        }
        self.assertEqual(self.qualify(identity_index=duplicate), {})
        conflict = {
            "semantics": {
                "presentation": {
                    "value": {"name": {"state": "KNOWN", "value": "different item"}}
                }
            }
        }
        self.assertEqual(self.qualify(definition=conflict), {})

    def clear_owner(self):
        self.owner.write_text(
            json.dumps(
                {"schema": engine.OWNER_FAMILY_DECISIONS_SCHEMA, "decisions": {}}
            )
        )

    def test_explicit_presentation_holds_never_fallback_to_client_name(self):
        for state in ("BLOCKED", "CONFLICT"):
            group = {"semantics": {"presentation": {"state": state}}}
            leaf = {
                "semantics": {
                    "presentation": {
                        "state": "KNOWN",
                        "value": {"name": {"state": state}},
                    }
                }
            }
            with self.subTest(state=state):
                self.assertEqual(self.qualify(definition=group), {})
                self.assertEqual(self.qualify(definition=leaf), {})

    def test_specific_market_requires_positive_capability(self):
        self.clear_owner()
        fist = {"flags.take": True, "market.category": 27, "flags.proficiency": True}
        self.assertEqual(self.qualify(flags=fist)[self.key]["profile"], "weapon_melee")
        self.assertEqual(self.qualify(flags=fist | {"flags.proficiency": False}), {})
        self.assertEqual(
            self.qualify(flags={"flags.take": True, "market.category": 27}), {}
        )
        product = {"flags.take": True, "market.category": 24, "flags.cumulative": True}
        self.assertEqual(
            self.qualify(flags=product)[self.key]["profile"], "material_valuable"
        )
        self.assertEqual(self.qualify(flags=product | {"flags.take": False}), {})

    def test_wiki_slot_requires_known_corresponding_client_slot(self):
        self.clear_owner()
        flags = {"flags.take": True, "clothes.slot": 1}
        fields = {"slot": "Head", "armor": "9"}
        self.assertEqual(
            self.qualify(fields, flags)[self.key]["profile"], "equipment_armor"
        )
        self.assertEqual(self.qualify(fields, flags | {"clothes.slot": 4}), {})
        self.assertEqual(self.qualify({"slot": "Head"}, flags), {})
        self.assertEqual(
            self.qualify({"slot": "Shield Hand", "armor": "9"}, {"flags.take": True}),
            {},
        )


if __name__ == "__main__":
    unittest.main()
