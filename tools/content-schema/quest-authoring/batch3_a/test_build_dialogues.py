"""Exercise the authoring boundary used to emit the actual NPC documents."""

import copy
import unittest

from build_dialogues import validate_document


class DialogueBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.document = {
            "schema": 1,
            "quest": "oteryn:quest.a_father_s_burden_quest",
            "tracks": [],
            "rules": [{
                "key": "oteryn:quest-npc/a_father_s_burden_quest/tereban/test",
                "quest": "oteryn:quest.a_father_s_burden_quest",
                "npc": "oteryn:npc.tereban", "trigger": "say",
                "keywords": ["mission"], "guards": [], "reply": ["Hello"],
                "effects": [],
                "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/tereban_functions.lua:132"],
            }],
        }

    def test_rejects_unpinned_source_before_materialization(self):
        self.document["rules"][0]["sources"] = ["crystal:main:data-global/npc/tereban.lua:1"]
        with self.assertRaisesRegex(ValueError, "Unpinned"):
            validate_document(self.document)

    def test_requires_explicit_catch_all(self):
        self.document["rules"][0]["keywords"] = []
        with self.assertRaisesRegex(ValueError, "selector"):
            validate_document(self.document)
        self.document["rules"][0]["any_message"] = True
        validate_document(self.document)

    def test_preserves_eleven_writes_in_fathers_acceptance(self):
        rule = self.document["rules"][0]
        rule["effects"] = [{"kind": "set", "track": f"oteryn:quest-progress/test/{i}", "value": 1} for i in range(11)]
        before = copy.deepcopy(self.document)
        validate_document(self.document)
        self.assertEqual(before, self.document)

    def test_rejects_duplicate_rule(self):
        self.document["rules"].append(copy.deepcopy(self.document["rules"][0]))
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            validate_document(self.document)


if __name__ == "__main__":
    unittest.main()
