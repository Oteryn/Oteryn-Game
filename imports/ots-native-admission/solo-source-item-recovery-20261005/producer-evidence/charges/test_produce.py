"""Small genuine parser and barrier fixtures; no whole Source pool opens."""

import unittest
from pathlib import Path

from produce import produce, replay_record, u32


def tree(*pairs):
    return {
        "content": [
            {
                "kind": "element",
                "node": {
                    "name": "other_tag",
                    "content": [{"kind": "element", "node": {}}],
                    "attributes": [
                        {"name": "key", "lexical_value": k},
                        {"name": "value", "lexical_value": v},
                    ],
                },
            }
            for k, v in pairs
        ]
    }


class OwnParser(unittest.TestCase):
    def test_full_unsigned_from_chars(self):
        for value, result in (
            ("4294967295", 4294967295),
            ("0" * 5000 + "7", 7),
            ("4294967296", 0),
            ("7junk", 0),
            ("-1", 0),
            ("+1", 0),
            (" 1", 0),
            ("", 0),
            ("１２", 0),
        ):
            with self.subTest(value=value[:20]):
                self.assertEqual(u32(value), result)

    def test_order_tags_nested_and_entities(self):
        value, assignments = replay_record(
            tree(("cHaRgEs", "12"), ("charges", "bad"), ("leveldoor", "&#x32;&#48;"))
        )
        self.assertEqual(value["charges_default_u32"], 0)
        self.assertEqual(value["charges_origin"], "EXPLICIT_ORDERED_XML")
        self.assertEqual(value["level_door_u32"], 20)
        self.assertEqual([a["attribute_ordinal"] for a in assignments], [0, 1, 2])
        self.assertEqual(assignments[2]["value_lexeme"], "&#x32;&#48;")

    def test_initializer_is_separate_from_explicit_zero(self):
        absent, a = replay_record(tree())
        explicit, b = replay_record(tree(("charges", "0")))
        self.assertEqual(absent["charges_default_u32"], explicit["charges_default_u32"])
        self.assertNotEqual(absent["charges_origin"], explicit["charges_origin"])
        self.assertEqual(a, [])
        self.assertEqual(len(b), 1)

    def test_missing_value_skips_and_duplicate_rejects(self):
        raw = tree(("charges", "7"))
        raw["content"][0]["node"]["attributes"].pop()
        self.assertEqual(replay_record(raw)[1], [])
        raw = tree(("charges", "7"))
        raw["content"][0]["node"]["attributes"].append(
            {"name": "value", "lexical_value": "8"}
        )
        with self.assertRaisesRegex(ValueError, "DUPLICATE"):
            replay_record(raw)

    def test_entity_invalid_is_held_not_fallback(self):
        with self.assertRaisesRegex(ValueError, "unknown entity"):
            replay_record(tree(("charges", "&unknown;")))

    def test_heavy_barrier_precedes_any_input_read(self):
        with self.assertRaisesRegex(ValueError, "RELEASE_REQUIRED"):
            produce(Path("/does-not-exist"), Path("/does-not-exist"))


if __name__ == "__main__":
    unittest.main()
