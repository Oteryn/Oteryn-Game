"""Pinned WRITE content, boundaries, atomic conflict rejection and admission preservation."""
import copy
import hashlib
import json
import unittest

import register_spell_families as registration
import writable_item_profiles as writing


class WritableProfilesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profiles = json.loads(writing.outputs()[writing.PATH])
        cls.native = json.loads((writing.ROOT / "content/items/definitions/spell-native-profiles.json").read_bytes())

    def generated(self):
        return {"profiles": json.dumps(self.native).encode(), "manifest": b'{"item_profiles":{}}'}

    def apply(self, generated):
        writing.apply_native(writing.ROOT, generated, "profiles", "manifest",
                             registration.provider_order, writing.reading.encode)

    def test_pinned_sources_agree_and_preserve_unresolved_limits_and_targets(self):
        rows = {r["item"]["key"]: r for r in self.profiles["records"]}
        self.assertEqual(len(rows), 112)
        self.assertEqual(sum(r["qualification"] == "PENDING_WRITE_ONCE_RULING" for r in rows.values()), 2)
        self.assertEqual(sum(r["qualification"] == "REWRITE_TEXT_SEMANTICS_ONLY" for r in rows.values()), 8)
        self.assertEqual(sum(r["qualification"] == "UNRESOLVED_WRITE_ONCE_TARGET" for r in rows.values()), 56)
        self.assertEqual(sum(r["qualification"] == "WRITE_ONCE_TEXT_SEMANTICS_ONLY" for r in rows.values()), 15)
        self.assertEqual(sum(r["qualification"] == "FIXED_TEXT_SEMANTICS_ONLY" for r in rows.values()), 15)
        self.assertEqual(sum(r["qualification"] == "PRESERVED_NONWRITABLE" for r in rows.values()), 8)
        self.assertEqual(sum(r["qualification"] == "MISSING_WRITE_ONCE_TARGET" for r in rows.values()), 1)
        for number, maximum in [(2813, 511), (2815, 511), (2598, 1023), (3505, 1999), (3507, 79)]:
            row = rows[f"oteryn:item.tibia.i{number}"]
            self.assertEqual((row["writable"], row["write_policy"], row["max_characters"]),
                             (True, "rewrite", maximum))
        self.assertEqual(rows["oteryn:item.tibia.i2813"]["source_write_once_id"], 2820)
        self.assertIsNone(rows["oteryn:item.tibia.i2813"]["write_once_to"])
        for source, target in [(2816, 2837), (2837, 2816)]:
            row = rows[f"oteryn:item.tibia.i{source}"]
            self.assertEqual(row["write_once_to"]["key"], f"oteryn:item.tibia.i{target}")
            self.assertEqual(row["qualification"], "PENDING_WRITE_ONCE_RULING")
        for source, target in [(2817, 2819), (2818, 2834), (2839, 2824)]:
            row = rows[f"oteryn:item.tibia.i{source}"]
            self.assertEqual(row["write_once_to"]["key"], f"oteryn:item.tibia.i{target}")
            self.assertEqual(row["qualification"], "WRITE_ONCE_TEXT_SEMANTICS_ONLY")
            fixed = rows[f"oteryn:item.tibia.i{target}"]
            self.assertFalse(fixed["writable"])
            self.assertIsNone(fixed["write_once_to"])
        for number in [27932, 27933, 27934]:
            self.assertEqual(rows[f"oteryn:item.tibia.i{number}"]["max_characters"], 1500)
        sources = writing.reading.item_rows(writing.ROOT)
        for number in [*range(173, 180), 6533]:
            key = f"oteryn:item.tibia.i{number}"
            self.assertEqual(rows[key]["qualification"], "PRESERVED_NONWRITABLE")
            self.assertEqual(sources[key]["semantics"]["readable_writeable"]["value"]["writeable"],
                             writing.known(False))
        self.assertEqual(rows["oteryn:item.tibia.i2820"]["max_characters"], 0)
        self.assertEqual(self.profiles["parity"], "not verified against Global")
        self.assertEqual((writing.ROOT / writing.PATH).read_bytes(), writing.outputs()[writing.PATH])

    def test_native_round_trip_is_idempotent_and_does_not_grant_mint_authority(self):
        generated = self.generated()
        self.apply(generated)
        self.assertEqual(json.loads(generated["profiles"]), self.native)
        rows = {r["production_definition"]["production_key"]: r for r in self.native["records"]}
        sources = writing.reading.item_rows(writing.ROOT)
        for profile in self.profiles["records"]:
            key = profile["item"]["key"]
            if key in rows:
                self.assertEqual(rows[key]["admission"]["materializable"], sources[key]["materializable"])
            if profile["qualification"] == "REWRITE_TEXT_SEMANTICS_ONLY":
                row = rows[key]
                self.assertEqual(row["admission"]["materializable"], sources[key]["materializable"])
                self.assertEqual(row["semantics"]["readable_writeable"]["value"]["write_once_target"],
                                 {"state": "NOT_APPLICABLE"})
            elif profile["qualification"] == "WRITE_ONCE_TEXT_SEMANTICS_ONLY":
                target = profile["write_once_to"]
                text = rows[key]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["write_once_target"], writing.known(
                    {field: target[field] for field in ("key", "revision")}))
                fixed = rows[target["key"]]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(fixed["writeable"], writing.known(False))
                self.assertEqual(fixed["write_once_target"], {"state": "NOT_APPLICABLE"})
            elif profile["qualification"] == "FIXED_TEXT_SEMANTICS_ONLY":
                self.assertEqual(rows[key]["semantics"]["readable_writeable"]["value"]["writeable"],
                                 writing.known(False))
            elif profile["qualification"] == "UNRESOLVED_WRITE_ONCE_TARGET":
                text = rows[key]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["writeable"], {"state": "UNKNOWN"})
                self.assertEqual(text["write_once_target"], {"state": "UNKNOWN"})
                self.assertEqual(text["max_text_length"], writing.known(profile["max_characters"]))
            else:
                self.assertNotIn(key, rows)

    def test_late_leaf_conflicts_or_stackable_text_fail_atomically(self):
        for failure in ("length", "stack_class", "stack_semantics", "group"):
            generated = self.generated()
            document = json.loads(generated["profiles"])
            row = next(r for r in document["records"]
                       if r["production_definition"]["production_key"] == "oteryn:item.tibia.i31268")
            if failure == "length":
                row["semantics"]["readable_writeable"]["value"]["max_text_length"]["value"] = 4001
            elif failure == "stack_class":
                row["admission"]["stack_class"] = "StackCapable"
            elif failure == "stack_semantics":
                row["semantics"]["stack"] = writing.known({"stackable": writing.known(True)})
            else:
                row["semantics"]["readable_writeable"] = {"state": "CONFLICT"}
            generated["profiles"] = json.dumps(document).encode()
            before = copy.deepcopy(generated)
            with self.subTest(failure=failure), self.assertRaises(ValueError):
                self.apply(generated)
            self.assertEqual(generated, before)

        # Owner-authorized replacements must not hide unrelated corruption.
        for number, field, value in [(27932, "max_text_length", writing.known(1501)),
                                     (2819, "writeable", {"state": "CONFLICT"}),
                                     (6386, "writeable", {"state": "CONFLICT"})]:
            generated = self.generated()
            document = json.loads(generated["profiles"])
            row = next(r for r in document["records"]
                       if r["production_definition"]["production_key"] == f"oteryn:item.tibia.i{number}")
            row["semantics"]["readable_writeable"]["value"][field] = value
            generated["profiles"] = json.dumps(document).encode()
            before = copy.deepcopy(generated)
            with self.subTest(number=number), self.assertRaisesRegex(
                    ValueError, "WRITABLE_OWNER_REPLACEMENT_CONFLICT"):
                self.apply(generated)
            self.assertEqual(generated, before)

    def test_max_and_max_plus_one_duplicate_and_source_digest_rejection(self):
        def parse(xml):
            raw = xml.encode()
            return writing.source_facts(raw, hashlib.sha256(raw).hexdigest())
        def xml(maximum):
            return (f'<items><item id="1" name="book"><attribute key="writeable" value="1"/>'
                    f'<attribute key="maxtextlen" value="{maximum}"/></item></items>')
        self.assertEqual(parse(xml(4000))[1]["source_facts"]["maxtextlen"], 4000)
        for maximum in (-1, 4001):
            with self.assertRaisesRegex(ValueError, "WRITABLE_MAX_CHARACTERS"):
                parse(xml(maximum))
        with self.assertRaisesRegex(ValueError, "WRITABLE_DUPLICATE_ATTRIBUTE"):
            parse(xml(1023).replace('</item>', '<attribute key="writeable" value="1"/></item>'))
        with self.assertRaisesRegex(ValueError, "WRITABLE_DUPLICATE_ITEM"):
            parse(xml(1023).replace('</items>', xml(1023)[7:]))
        with self.assertRaisesRegex(ValueError, "WRITABLE_BOOLEAN"):
            parse(xml(1023).replace('value="1"', 'value="2"'))
        with self.assertRaisesRegex(ValueError, "WRITABLE_SOURCE_DIGEST"):
            writing.source_facts(b'<items/>', 'wrong')
        before = parse(xml(1023).replace('</item>', '<attribute key="readable" value="0"/></item>'))
        self.assertFalse(before[1]["effective_readable"])


if __name__ == "__main__":
    unittest.main()
