"""Pinned WRITE content, boundaries, atomic conflict rejection and admission preservation."""
import copy
import hashlib
import json
import unittest

import register_spell_families as registration
import writable_item_profiles as writing
from quest_callback_profiles import EXPLORER_TRACING_SHEETS


class WritableProfilesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profiles = json.loads(writing.outputs()[writing.PATH])
        cls.native = json.loads((writing.ROOT / "content/items/definitions/spell-native-profiles.json").read_bytes())

    def generated(self):
        return {"profiles": json.dumps(self.native).encode(), "manifest": b'{"item_profiles":{}}'}

    def apply(self, generated, *, stage="complete"):
        writing.apply_native(writing.ROOT, generated, "profiles", "manifest",
                             registration.provider_order, writing.reading.encode, stage=stage)

    def test_pinned_sources_agree_and_resolve_carried_writes(self):
        rows = {r["item"]["key"]: r for r in self.profiles["records"]}
        self.assertEqual(len(rows), 115)
        self.assertEqual(sum(r["qualification"] == "PENDING_WRITE_ONCE_RULING" for r in rows.values()), 2)
        self.assertEqual(sum(r["qualification"] == "REWRITE_TEXT_SEMANTICS_ONLY" for r in rows.values()), 8)
        self.assertEqual(sum(r["qualification"] == "CRYSTAL_CARRIED_REWRITE" for r in rows.values()), 62)
        self.assertEqual(sum(r["qualification"] == "CRYSTAL_CARRIED_WRITE_ONCE" for r in rows.values()), 1)
        self.assertEqual(sum(r["qualification"] == "STATIONARY_TEXT_SEMANTICS_ONLY" for r in rows.values()), 3)
        self.assertEqual(sum(r["qualification"] == "WRITE_ONCE_TEXT_SEMANTICS_ONLY" for r in rows.values()), 15)
        self.assertEqual(sum(r["qualification"] == "FIXED_TEXT_SEMANTICS_ONLY" for r in rows.values()), 15)
        self.assertEqual(sum(r["qualification"] == "CRYSTAL_FIXED_TARGET" for r in rows.values()), 1)
        self.assertEqual(sum(r["qualification"] == "MISSING_WRITE_ONCE_TARGET" for r in rows.values()), 0)
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
        for source, target in [(2817, 2819), (2818, 2834), (2839, 2824), (8747, 8748)]:
            row = rows[f"oteryn:item.tibia.i{source}"]
            self.assertEqual(row["write_once_to"]["key"], f"oteryn:item.tibia.i{target}")
            qualification = "CRYSTAL_CARRIED_WRITE_ONCE" if source == 8747 else "WRITE_ONCE_TEXT_SEMANTICS_ONLY"
            self.assertEqual(row["qualification"], qualification)
            fixed = rows[f"oteryn:item.tibia.i{target}"]
            self.assertFalse(fixed["writable"])
            self.assertIsNone(fixed["write_once_to"])
        for number in [27932, 27933, 27934]:
            self.assertEqual(rows[f"oteryn:item.tibia.i{number}"]["max_characters"], 1500)
        sources = writing.reading.item_rows(writing.ROOT)
        for number in [*range(173, 180), 6533]:
            key = f"oteryn:item.tibia.i{number}"
            self.assertEqual(rows[key]["qualification"], "CRYSTAL_CARRIED_REWRITE")
            self.assertTrue(rows[key]["writable"])
            self.assertEqual(sources[key]["semantics"]["readable_writeable"]["value"]["writeable"],
                             writing.known(False))
        self.assertEqual(rows["oteryn:item.tibia.i2820"]["max_characters"], 0)
        self.assertEqual(self.profiles["parity"], "not verified against Global")
        self.assertEqual((writing.ROOT / writing.PATH).read_bytes(), writing.outputs()[writing.PATH])

    def test_native_round_trip_is_idempotent_and_limits_new_custody_to_carried_writers(self):
        generated = self.generated()
        self.apply(generated, stage="carried")
        self.assertEqual(json.loads(generated["profiles"]), self.native)

        # The full text pass runs before the other legacy owners. The two
        # written variants already lose the XML editor bound on main.
        expected = copy.deepcopy(self.native)
        for row in expected["records"]:
            if row["production_binding"]["external_id"] in {"637", "641"}:
                text = row["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["max_text_length"], {"state": "UNKNOWN"})
                text["max_text_length"] = writing.known(511)
        generated = self.generated()
        self.apply(generated)
        self.assertEqual(json.loads(generated["profiles"]), expected)
        rows = {r["production_definition"]["production_key"]: r for r in self.native["records"]}
        # An empty provider isolates the writable owner's admission decisions.
        isolated = {"profiles": b'{"records":[]}', "manifest": b'{"item_profiles":{}}'}
        self.apply(isolated, stage="legacy")
        legacy_profiles = json.loads(writing.outputs(crystal_carried=False)[writing.PATH])
        owned = {r["production_definition"]["production_key"]: r
                 for r in json.loads(isolated["profiles"])["records"]}
        self.assertEqual(owned.keys(), {
            profile["item"]["key"] for profile in legacy_profiles["records"]
            if profile["qualification"] in {
                "REWRITE_TEXT_SEMANTICS_ONLY", "WRITE_ONCE_TEXT_SEMANTICS_ONLY",
                "FIXED_TEXT_SEMANTICS_ONLY", "UNRESOLVED_WRITE_ONCE_TARGET",
            }
        })
        for profile in legacy_profiles["records"]:
            if profile["qualification"] == "UNRESOLVED_WRITE_ONCE_TARGET":
                text = owned[profile["item"]["key"]]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["writeable"], {"state": "UNKNOWN"})
                self.assertEqual(text["write_once_target"], {"state": "UNKNOWN"})
                self.assertEqual(text["max_text_length"], writing.known(profile["max_characters"]))
        sources = writing.reading.item_rows(writing.ROOT)
        independently_admitted = {
            "oteryn:item.tibia.i3505", "oteryn:item.tibia.i3507",
            "oteryn:item.tibia.i6387", "oteryn:item.tibia.i9204",
        } | EXPLORER_TRACING_SHEETS
        for profile in self.profiles["records"]:
            key = profile["item"]["key"]
            if key in owned:
                self.assertEqual(owned[key]["admission"]["materializable"], sources[key]["materializable"])
            qualification = profile["qualification"]
            carried = qualification in {"CRYSTAL_CARRIED_REWRITE", "CRYSTAL_CARRIED_WRITE_ONCE"}
            # Mail, cards, poems and the Explorer quest already admit these Items.
            if carried or key in independently_admitted:
                self.assertEqual(rows[key]["admission"], {
                    "materializable": True, "stack_class": "NonStackable",
                    "legal_destinations": ["CharacterInventory", "Ground"],
                })
            elif key in rows:
                self.assertEqual(rows[key]["admission"]["materializable"], sources[key]["materializable"])
            if qualification in {"REWRITE_TEXT_SEMANTICS_ONLY", "CRYSTAL_CARRIED_REWRITE"}:
                row = rows[key]
                self.assertEqual(row["semantics"]["readable_writeable"]["value"]["write_once_target"],
                                 {"state": "NOT_APPLICABLE"})
            elif qualification in {"WRITE_ONCE_TEXT_SEMANTICS_ONLY", "CRYSTAL_CARRIED_WRITE_ONCE"}:
                target = profile["write_once_to"]
                text = rows[key]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["write_once_target"], writing.known(
                    {field: target[field] for field in ("key", "revision")}))
                fixed = rows[target["key"]]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(fixed["writeable"], writing.known(False))
                self.assertEqual(fixed["write_once_target"], {"state": "NOT_APPLICABLE"})
            elif qualification in {"FIXED_TEXT_SEMANTICS_ONLY", "CRYSTAL_FIXED_TARGET"}:
                self.assertEqual(rows[key]["semantics"]["readable_writeable"]["value"]["writeable"],
                                 writing.known(False))
            elif profile["qualification"] == "STATIONARY_TEXT_SEMANTICS_ONLY":
                text = rows[key]["semantics"]["readable_writeable"]["value"]
                self.assertEqual(text["writeable"], {"state": "UNKNOWN"})
                self.assertEqual(text["write_once_target"], {"state": "UNKNOWN"})
                self.assertEqual(text["max_text_length"], writing.known(profile["max_characters"]))
            elif key == "oteryn:item.tibia.i2816":
                self.assertEqual(qualification, "PENDING_WRITE_ONCE_RULING")
                self.assertEqual(rows[key]["semantics"]["readable_writeable"], writing.known({
                    "readable": writing.known(True),
                    "writeable": writing.known(True),
                    "max_text_length": writing.known(1023),
                    "distance_read": {"state": "UNKNOWN"},
                    "write_once_target": {"state": "UNKNOWN"},
                }))
            elif key in {"oteryn:item.tibia.i2820", "oteryn:item.tibia.i2822",
                         "oteryn:item.tibia.i3506", "oteryn:item.tibia.i6387",
                         "oteryn:item.tibia.i8117", "oteryn:item.tibia.i9204"}:
                self.assertEqual(qualification, "READ_ONLY_OR_ZERO_LIMIT")
                field = rows[key]["semantics"].get("readable_writeable", {}).get("value", {})
                self.assertNotEqual(field.get("writeable"), writing.known(True))
                self.assertNotEqual(field.get("max_text_length", {}).get("state"), "KNOWN")
            else:
                self.assertNotIn(key, rows.keys())

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
        owner_conflict = "WRITABLE_OWNER_REPLACEMENT_CONFLICT"
        for number, field, value, error in [
            (27932, "max_text_length", writing.known(1501), owner_conflict),
            (2819, "writeable", {"state": "CONFLICT"}, owner_conflict),
            (173, "writeable", {"state": "CONFLICT"}, owner_conflict),
            (6386, "writeable", {"state": "CONFLICT"},
             "^WRITABLE_LEAF_CONFLICT:oteryn:item.tibia.i6386:writeable$"),
        ]:
            generated = self.generated()
            document = json.loads(generated["profiles"])
            row = next(r for r in document["records"]
                       if r["production_definition"]["production_key"] == f"oteryn:item.tibia.i{number}")
            row["semantics"]["readable_writeable"]["value"][field] = value
            generated["profiles"] = json.dumps(document).encode()
            before = copy.deepcopy(generated)
            with self.subTest(number=number), self.assertRaisesRegex(
                    ValueError, error):
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
