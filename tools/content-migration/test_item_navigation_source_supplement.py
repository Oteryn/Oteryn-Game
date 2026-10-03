"""Source-frame, official identity and retained native/name precedence guards."""

import copy
import json
import unittest

from item_navigation_source_supplement import (
    ROOT,
    SEVEN_PATH,
    build_seven,
    derive_seven,
    load_alias_fallback,
    name_agrees,
)
from item_official_navigation import qualification_inputs
from item_taxonomy import CLIENT_PATH, build_identity_index, load_appearance_objects


class NavigationSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((ROOT / SEVEN_PATH).read_text())
        cls.entries = {e["appearance_id"]: e for e in cls.proof["records"]}
        cls.client = load_appearance_objects((ROOT / CLIENT_PATH).read_bytes())
        cls.members = qualification_inputs()[1]
        cls.snapshot = json.loads(
            (ROOT / "imports/tibiawiki/facts/items-stats.json").read_text()
        )
        cls.wiki = {
            r["item_id"]: r["observations"] for r in cls.snapshot["records"].values()
        }

    def args(self, iid=40522):
        entry = copy.deepcopy(self.entries[iid])
        return [
            entry,
            {
                "identity": entry["target"],
                "materializable": False,
                "stack_class": "Unknown",
            },
            copy.deepcopy(self.client[iid]),
            copy.deepcopy(self.wiki[iid]),
            copy.deepcopy(self.members[iid]),
            copy.deepcopy(entry["source_binding"]),
            self.proof["sources"]["qualification_cutoff"],
        ]

    def test_seven_true_sources_and_unbound_a12_role(self):
        for iid in self.entries:
            row = derive_seven(*self.args(iid))
            self.assertIsNotNone(row, iid)
            evidence = row["source_evidence"]
            if iid != 24415:
                self.assertIsNone(evidence["source_binding"])
                self.assertEqual(
                    evidence["identity_authority"], "A12-ITEM-IDENTITY-TIBIA-ID-V1"
                )
        rag = derive_seven(*self.args(24415))
        self.assertEqual(rag["family_profile"], "event_collectible")
        self.assertEqual(rag["source_evidence"]["selected_field"], "status")
        self.assertNotIn("status", self.wiki[24415][0]["fields"])

    def test_member_source_revision_and_native_identity_holds(self):
        for index, field, value in (
            (0, "appearance_id", 40523),
            (0, "family_profile", "tool"),
            (1, "materializable", True),
            (1, "stack_class", "Cumulative"),
            (2, "name", "other"),
        ):
            args = self.args()
            args[index][field] = value
            self.assertIsNone(derive_seven(*args))
        for coordinate in (
            "page_id",
            "revision_id",
            "revision_timestamp",
            "content_sha256",
        ):
            args = self.args()
            args[3][0][coordinate] = "drift"
            self.assertIsNone(derive_seven(*args))
        args = self.args()
        args[4][1] = "0" * 64
        self.assertIsNone(derive_seven(*args))
        args = self.args()
        args[3].append(copy.deepcopy(args[3][0]))
        self.assertIsNone(derive_seven(*args))

    def test_positive_item_domain_and_native_name_guards(self):
        for iid in (40522, 24415):
            for flag, value in (
                ("flags.take", False),
                ("flags.unmove", True),
                ("flags.corpse", True),
            ):
                args = self.args(iid)
                args[2]["flags"][flag] = value
                self.assertIsNone(derive_seven(*args))
            for state in ("BLOCKED", "CONFLICT", "NOT_APPLICABLE"):
                for presentation in (
                    {"state": state},
                    {"state": "KNOWN", "value": {"name": {"state": state}}},
                ):
                    args = self.args(iid)
                    args[1]["semantics"] = {"presentation": presentation}
                    self.assertIsNone(derive_seven(*args))
            args = self.args(iid)
            args[1]["semantics"] = {
                "presentation": {
                    "state": "KNOWN",
                    "value": {"name": {"state": "KNOWN", "value": "other"}},
                }
            }
            self.assertIsNone(derive_seven(*args))
            args[1]["semantics"]["presentation"] = {"state": "UNKNOWN"}
            self.assertIsNotNone(derive_seven(*args))

    def test_old_rag_cannot_generalize_primary_or_snapshot_replacement(self):
        for field, value in (("primarytype", "Food"), ("status", "event")):
            args = self.args(24415)
            args[3][0]["fields"][field] = value
            self.assertIsNone(derive_seven(*args))
        args = self.args(24415)
        args[5] = None
        self.assertIsNone(derive_seven(*args))
        args = self.args(24415)
        args[5]["disposition"] = "CANDIDATE"
        self.assertIsNone(derive_seven(*args))

    def test_alias26_existing_mapping_and_native_blocked_names(self):
        aliases = load_alias_fallback(ROOT, build_identity_index())
        self.assertEqual(len(aliases), 26)
        self.assertEqual(
            aliases["oteryn:item.tibia.i7519"]["match_basis"], "actualname"
        )
        self.assertEqual(aliases["oteryn:item.tibia.i7938"]["match_basis"], "title")
        self.assertEqual(
            aliases["oteryn:item.tibia.i9435"]["match_basis"], "actualname"
        )
        self.assertNotIn("oteryn:item.tibia.i6523", aliases)
        self.assertFalse(
            name_agrees({"semantics": {"presentation": {"state": "CONFLICT"}}}, "name")
        )

    def test_existing_owner_excludes_every_new_row(self):
        definitions = {
            e["target"]["key"]: self.args(iid)[1] for iid, e in self.entries.items()
        }
        rows = build_seven(definitions, self.snapshot, self.client, set())
        self.assertEqual(len(rows), 7)
        self.assertEqual(
            build_seven(definitions, self.snapshot, self.client, set(definitions)), []
        )


if __name__ == "__main__":
    unittest.main()
