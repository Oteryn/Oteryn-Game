"""Independent readability, own-ID scalar variants, and native evidence guards."""

import copy
import unittest

import lower_item_document_packet as doc


class DocumentSources(unittest.TestCase):
    def args(self, facts=None, ids="7"):
        target = {"family": "Item", "key": "item", "revision": "definition-r1"}
        binding = {"target": target, "external_id": "7"}
        raw = (
            "{{Infobox Object|itemid="
            + ids
            + "|name=Thing (Variant)|actualname=thing|writable=no|writechars=1023}}"
        )
        return [
            {
                "source_item_id": 7,
                "target": target,
                "binding": binding,
                "official_name": "thing",
                "facts": facts
                if facts is not None
                else {"readable": True, "writeable": False, "max_text_length": 1023},
                "wiki_sources": [
                    {
                        "raw_infobox": raw,
                        "raw_infobox_sha256": doc.sha(raw.encode()),
                        "revision_timestamp": "2026-09-01T00:00:00Z",
                    }
                ],
            },
            {"identity": target, "semantics": {}},
            binding,
            {
                "id": 7,
                "name": "thing",
                "flags": {"flags.take": True, "flags.write": True},
            },
            set(),
            "2026-09-27T23:59:59Z",
        ]

    def test_independent_readability_false_write_length_and_idempotence(self):
        args = self.args()
        rows = doc.qualify(*args)
        self.assertEqual(len(rows["facts"]), 3)
        args[1]["semantics"] = {
            "readable_writeable": {
                "state": "KNOWN",
                "value": {
                    member: {"state": "KNOWN", "value": value}
                    for member, value in rows["facts"].items()
                },
            }
        }
        self.assertEqual(doc.qualify(*args), rows)
        args = self.args({"readable": True}, "7, 8")
        self.assertEqual(doc.qualify(*args)["facts"], {"readable": True})
        args[0]["facts"]["writeable"] = False
        with self.assertRaisesRegex(ValueError, "identity/duplicate"):
            doc.qualify(*args)
        args = self.args({"writeable": False, "max_text_length": 1023})
        del args[3]["flags"]["flags.write"]
        self.assertEqual(len(doc.qualify(*args)["facts"]), 2)

    def test_native_full_identity_name_domain_and_every_field_state(self):
        cases = [
            lambda a: a[1]["identity"].update(revision="other"),
            lambda a: a[2].update(external_id="8"),
            lambda a: a[3].update(id=8),
            lambda a: a[4].add("item"),
            lambda a: a[3]["flags"].pop("flags.take"),
            lambda a: a[3]["flags"].update(**{"flags.unmove": True}),
            lambda a: a[3]["flags"].pop("flags.write"),
        ]
        for mutate in cases:
            args = copy.deepcopy(self.args())
            mutate(args)
            with self.assertRaises(ValueError):
                doc.qualify(*args)
        for member, value in (
            ("readable", False),
            ("writeable", True),
            ("max_text_length", 99),
        ):
            for state in ("CONFLICT", "NOT_APPLICABLE", "KNOWN"):
                args = self.args()
                args[1]["semantics"] = {
                    "readable_writeable": {
                        "state": "KNOWN",
                        "value": {member: {"state": state, "value": value}},
                    }
                }
                with self.assertRaisesRegex(ValueError, "native leaf"):
                    doc.qualify(*args)
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            args = self.args()
            args[1]["semantics"] = {"readable_writeable": {"state": state}}
            with self.assertRaisesRegex(ValueError, "native group"):
                doc.qualify(*args)
        args = self.args()
        args[1]["semantics"] = {
            "presentation": {
                "state": "KNOWN",
                "value": {"name": {"state": "KNOWN", "value": "different"}},
            }
        }
        with self.assertRaisesRegex(ValueError, "native name"):
            doc.qualify(*args)

    def test_literal_raw_source_cutoff_empty_units_duplicates_and_agreement(self):
        args = self.args()
        original = args[0]["wiki_sources"][0]
        for before, after in (
            ("1023", "0"),
            ("1023", "4294967296"),
            ("1023", "1.5"),
            ("1023", "1023 chars"),
            ("1023", ""),
            ("writable=no", "writable="),
            ("itemid=7", "itemid=7x"),
            ("itemid=7", "itemid=7|itemid=7"),
            ("writechars=1023", "writechars=1023|writechars=1023"),
        ):
            changed = copy.deepcopy(args)
            source = changed[0]["wiki_sources"][0]
            source["raw_infobox"] = source["raw_infobox"].replace(before, after)
            source["raw_infobox_sha256"] = doc.sha(source["raw_infobox"].encode())
            with self.subTest(after=after), self.assertRaises(ValueError):
                doc.qualify(*changed)
        for key, value in (
            ("raw_infobox_sha256", "0" * 64),
            ("revision_timestamp", "2026-09-28T00:00:00Z"),
        ):
            changed = copy.deepcopy(args)
            changed[0]["wiki_sources"][0][key] = value
            with self.assertRaises(ValueError):
                doc.qualify(*changed)
        other = original | {
            "raw_infobox": original["raw_infobox"].replace(
                "writable=no", "writable=yes"
            )
        }
        other["raw_infobox_sha256"] = doc.sha(other["raw_infobox"].encode())
        args[0]["wiki_sources"].append(other)
        with self.assertRaisesRegex(ValueError, "all-present"):
            doc.qualify(*args)


if __name__ == "__main__":
    unittest.main()
