"""Closed public-category join, own-ID priority and source/native negative guards."""

import copy
import json
import re
import unittest

from item_external_family_refinement import (
    IDS,
    PATH,
    ROOT,
    build_external,
    derive_external,
    digest,
)
from item_taxonomy import CLIENT_PATH, load_appearance_objects


class ExternalFamilyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((ROOT / PATH).read_text())
        cls.entries = {e["appearance_id"]: e for e in cls.proof["records"]}
        cls.client = load_appearance_objects((ROOT / CLIENT_PATH).read_bytes())
        cls.snapshot = json.loads(
            (ROOT / "imports/tibiawiki/facts/items-stats.json").read_text()
        )
        cls.wiki = {
            r["item_id"]: r["observations"] for r in cls.snapshot["records"].values()
        }
        cls.dom = json.loads(
            (ROOT / cls.proof["sources"]["dom_witnesses"]["path"]).read_text()
        )["records"]
        cls.definitions = {
            r["definition"]["identity"]["key"]: r["definition"]
            for p in (ROOT / "content/items/definitions").glob("items-*.json")
            for r in json.loads(p.read_text())["records"]
        }

    def args(self, iid=52819):
        e = copy.deepcopy(self.entries[iid])
        return [
            e,
            copy.deepcopy(self.definitions[e["target"]["key"]]),
            copy.deepcopy(self.client[iid]),
            copy.deepcopy(self.wiki[iid]),
            copy.deepcopy(e["source_binding"]),
            [iid],
            copy.deepcopy(self.dom.get(str(iid))),
            self.proof["sources"]["qualification_cutoff"],
        ]

    def reseal_appearance(self, args):
        args[0]["decoded_record_sha256"] = digest(
            json.dumps(
                args[2], sort_keys=True, ensure_ascii=False, separators=(",", ":")
            ).encode()
        )

    def change_raw(self, args, field, value):
        s = args[0]["wiki_sources"][0]
        old = s["parameter_values"][field][0]
        s["raw_own_infobox"] = re.sub(
            r"(?m)^(\|[ \t]*"
            + re.escape(field)
            + r"[ \t]*=[ \t]*)"
            + re.escape(old)
            + r"(?=[ \t]*$)",
            lambda m: m[1] + value,
            s["raw_own_infobox"],
            count=1,
        )
        s["raw_own_infobox_sha256"] = digest(s["raw_own_infobox"].encode())
        s["parameter_values"][field] = [value]

    def test_closed18_and_existing_owner_rows_preserved(self):
        rows = build_external(self.definitions, self.snapshot, self.client, set())
        self.assertEqual({r["source_evidence"]["appearance_id"] for r in rows}, IDS)
        self.assertEqual(sum(r["family_profile"] == "quest_item" for r in rows), 16)
        for row in rows:
            self.assertEqual(row["source_taxonomy"], {"primary": "Others"})
            self.assertEqual(row["source_evidence"]["classification"], "DERIVED")
        keys = {r["target"]["key"] for r in rows}
        self.assertEqual(
            build_external(self.definitions, self.snapshot, self.client, keys), []
        )
        self.assertNotIn(52789, IDS)

    def test_exact_binding_unique_name_and_native_state_guards(self):
        mutations = [
            (0, "appearance_id", 52789),
            (0, "mode", "GENERIC_OTHERS"),
            (0, "family_profile", "tool"),
            (1, "materializable", True),
            (1, "stack_class", "Cumulative"),
            (4, "external_id", "52820"),
            (4, "source_revision", "drift"),
            (4, "identity_namespace", "client"),
            (4, "disposition", "CANDIDATE"),
        ]
        for i, field, value in mutations:
            args = self.args()
            args[i][field] = value
            self.assertIsNone(derive_external(*args), (i, field))
        args = self.args()
        args[5] = [52819, 52820]
        self.assertIsNone(derive_external(*args))
        for group, leaf in [
            ("BLOCKED", "KNOWN"),
            ("KNOWN", "CONFLICT"),
            ("NOT_APPLICABLE", "KNOWN"),
            ("KNOWN", "NOT_APPLICABLE"),
        ]:
            args = self.args()
            args[1]["semantics"]["presentation"]["state"] = group
            args[1]["semantics"]["presentation"]["value"]["name"]["state"] = leaf
            self.assertIsNone(derive_external(*args))
        args = self.args()
        args[1]["semantics"]["presentation"]["value"]["name"]["value"] = "Other"
        self.assertIsNone(derive_external(*args))

    def test_portable_domain_and_official_record_guard(self):
        for flag, value in [
            ("flags.take", False),
            ("flags.unmove", True),
            ("flags.bank", True),
        ]:
            args = self.args()
            args[2]["flags"][flag] = value
            self.reseal_appearance(args)
            self.assertIsNone(derive_external(*args), flag)
        args = self.args()
        args[2]["name"] = "wrong"
        self.reseal_appearance(args)
        self.assertIsNone(derive_external(*args))
        args = self.args()
        args[0]["decoded_record_sha256"] = "0" * 64
        self.assertIsNone(derive_external(*args))

    def test_all_own_id_sources_and_current_coordinates(self):
        for field, value in [
            ("itemid", "52819,52820"),
            ("primarytype", "Tools"),
            ("pickupable", "no"),
            ("objectclass", "Doors"),
            ("actualname", "another item"),
        ]:
            args = self.args()
            self.change_raw(args, field, value)
            self.assertIsNone(derive_external(*args), field)
        for coordinate in [
            "page_id",
            "revision_id",
            "revision_timestamp",
            "content_sha256",
            "wiki_title",
        ]:
            args = self.args()
            args[3][0][coordinate] = "drift"
            self.assertIsNone(derive_external(*args), coordinate)
        args = self.args()
        args[3][0]["fields"]["primarytype"] = "Doors"
        self.assertIsNone(derive_external(*args))
        args = self.args()
        args[0]["wiki_sources"].append(copy.deepcopy(args[0]["wiki_sources"][0]))
        self.assertIsNone(derive_external(*args))
        args = self.args()
        s = args[0]["wiki_sources"][0]
        s["raw_own_infobox"] = s["raw_own_infobox"][:-2] + "|itemid=52819}}"
        s["raw_own_infobox_sha256"] = digest(s["raw_own_infobox"].encode())
        self.assertIsNone(derive_external(*args))

    def test_actual_dom_category_caption_revision_and_hash(self):
        for excerpt, old, new in [
            ("own_category_values", "Quest", "Others"),
            ("own_category_values", "#quest", "#taming"),
            ("own_caption_prefix", "Adlerauge", "Clavius"),
            ("document_title", "Adlerauge", "Clavius"),
            ("own_category_header", "Subclass", "Other"),
        ]:
            args = self.args()
            e = args[6]["excerpts"][excerpt]
            e["raw"] = e["raw"].replace(old, new)
            e["sha256"] = digest(e["raw"].encode())
            self.assertIsNone(derive_external(*args), excerpt)
        args = self.args()
        args[6]["article_revision"] = 123
        self.assertIsNone(derive_external(*args))
        args = self.args()
        args[6]["excerpts"]["document_title"]["sha256"] = "0" * 64
        self.assertIsNone(derive_external(*args))

    def test_etcher_function_literal_and_no_generic_others_tool(self):
        args = self.args(51443)
        row = derive_external(*args)
        self.assertEqual(row["family_profile"], "tool")
        self.assertEqual(row["source_evidence"]["refinement"]["field"], "notes")
        self.change_raw(args, "notes", "Can be consumed to drink.")
        self.assertIsNone(derive_external(*args))
        args = self.args(51443)
        args[0]["appearance_id"] = 52789
        self.assertIsNone(derive_external(*args))


if __name__ == "__main__":
    unittest.main()
