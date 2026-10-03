"""Seven real sources, existing identity admission and closed primary exception."""

import copy
import hashlib
import json
import re
import unittest
from unittest.mock import patch

import item_bounded7_navigation as nav


class BoundedSevenTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = nav.source_inputs()
        cls.entries = {e["appearance_id"]: e for e in cls.inputs[0]["records"]}
        pin = cls.inputs[0]["sources"]["current_artifact"]["appearances_sha256"]
        cls.client = nav.engine.load_appearance_objects(
            (nav.ROOT / f"content/assets/files/appearances-{pin}.dat").read_bytes()
        )
        cls.definitions = {}
        for path in (nav.ROOT / "content/items/definitions").glob("items-*.json"):
            for row in json.loads(path.read_bytes())["records"]:
                definition = row["definition"]
                cls.definitions[definition["identity"]["key"]] = definition

    def args(self, iid=43778):
        e = self.entries[iid]
        return [
            copy.deepcopy(e),
            copy.deepcopy(self.definitions[e["target"]["key"]]),
            copy.deepcopy(self.client[iid]),
            copy.deepcopy(self.inputs),
            1,
            set(),
        ]

    def reseal_case(self, args):
        # Exercise a changed inner predicate after the outer fixed-entry guard.
        args[3][0]["records"] = [args[0]]
        args[0]["decoded_record_sha256"] = nav.decoded_digest(args[2])

    def test_real_seven_no_native_mutation_and_honest_primary(self):
        for iid in nav.IDS:
            args = self.args(iid)
            before = copy.deepcopy(args[1])
            row = nav.derive_row(*args)
            self.assertEqual(args[1], before)
            self.assertEqual(row["source_evidence"]["scope"], "NAVIGATION_ONLY")
            if iid == 4290:
                self.assertEqual(row["source_taxonomy"]["primary"], "Corpses")
                self.assertEqual(
                    row["source_evidence"]["source_primarytype"]["state"], "KNOWN"
                )
                self.assertEqual(
                    len(
                        row["source_evidence"]["family_basis"][
                            "scalar_disagreements_held"
                        ]
                    ),
                    3,
                )
                self.assertNotIn("capacity", row)
            else:
                self.assertEqual(row["family_profile"], "quest_item")
                self.assertEqual(
                    row["source_taxonomy"]["primary"], "Source primarytype absent"
                )
                self.assertEqual(
                    row["source_evidence"]["source_primarytype"],
                    {"state": "UNKNOWN", "reason": "ABSENT_OR_EMPTY"},
                )
            if iid in nav.A12_IDS:
                self.assertIsNone(row["source_evidence"]["source_binding"])
                self.assertEqual(
                    before.get("semantics", {}).get("presentation", {}), {}
                )
        self.assertEqual(
            nav.build_bounded7(
                self.definitions,
                self.client,
                {e["target"]["key"] for e in self.entries.values()},
            ),
            [],
        )

    def test_scope_native_identity_binding_and_precedence_failures(self):
        cases = [
            lambda a: a[0].update(appearance_id=1),
            lambda a: a[5].add(a[0]["target"]["key"]),
            lambda a: a[1].update(materializable=True),
            lambda a: a[1].update(stack_class="Cumulative"),
            lambda a: a[1]["identity"].update(key="oteryn:item.tibia.i1"),
            lambda a: a[3][1]["client-15.30"][a[0]["appearance_id"]].__setitem__(
                1, "0" * 64
            ),
            lambda a: a[3][2][a[0]["target"]["key"]].append({"disposition": "EXACT"}),
        ]
        for mutate in cases:
            args = self.args()
            mutate(args)
            with self.assertRaises(ValueError):
                nav.derive_row(*args)
        for iid in (4290, 39571):
            args = self.args(iid)
            args[3][3][str(iid)].append(copy.deepcopy(args[0]["source_binding"]))
            with self.assertRaisesRegex(ValueError, "EXACT_BINDING"):
                nav.derive_row(*args)

    def test_admission_and_source_seals_are_required(self):
        spec = self.inputs[0]["sources"]["native_admission"]
        text = (nav.ROOT / spec["path"]).read_text()
        with self.assertRaises(ValueError):
            nav.admission_ids(text.replace("43778,", "43777,"), spec)
        nav.source_inputs.cache_clear()
        try:
            with patch.object(nav, "SHA", "0" * 64), self.assertRaises(ValueError):
                nav.source_inputs()
        finally:
            nav.source_inputs.cache_clear()

    def test_name_domain_and_native_state_guards(self):
        for flag in (
            nav.engine.CORPSE_FLAGS
            + nav.engine.GROUND_OR_BORDER_FLAGS
            + ("flags.unmove",)
        ):
            args = self.args()
            args[2]["flags"][flag] = True
            self.reseal_case(args)
            with self.assertRaisesRegex(ValueError, "WORLD_ROUTE"):
                nav.derive_row(*args)
        for mutation in (
            lambda a: a[2]["flags"].pop("flags.take"),
            lambda a: a[2].update(name="different name"),
            lambda a: a.__setitem__(4, 2),
        ):
            args = self.args()
            mutation(args)
            self.reseal_case(args)
            with self.assertRaises(ValueError):
                nav.derive_row(*args)
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            args = self.args()
            p = {"state": state}
            args[1]["semantics"] = {"presentation": p}
            args[0]["native_presentation_projection"] = p
            self.reseal_case(args)
            with self.assertRaises(ValueError):
                nav.derive_row(*args)

    def test_exact_own_wiki_and_dom_not_other_categories(self):
        for field, value in (
            ("primarytype", "Others"),
            ("objectclass", "Decoration"),
            ("secondarytype", "Food"),
            ("status", "Event"),
            ("itemid", "43778,43779"),
            ("pickupable", "no"),
            ("immobile", "yes"),
            ("notes", "Other quest"),
        ):
            args = self.args()
            s = args[0]["wiki_source"]
            raw = s["raw_own_infobox"]
            pattern = rf"(?mi)^\|\s*{field}\s*=.*$"
            if re.search(pattern, raw):
                raw = re.sub(pattern, f"| {field}={value}", raw)
            else:
                raw = raw[:-2] + f"| {field}={value}\n" + "}}"
            s["raw_own_infobox"] = raw
            s["raw_own_infobox_sha256"] = hashlib.sha256(raw.encode()).hexdigest()
            s["parameter_values"][field] = [value]
            self.reseal_case(args)
            with self.assertRaises(ValueError):
                nav.derive_row(*args)
        for part in (
            "document_title",
            "own_caption_prefix",
            "own_category_header",
            "own_category_values",
        ):
            args = self.args()
            excerpt = args[3][5]["43778"]["excerpts"][part]
            raw = (
                excerpt["raw"]
                .replace("Organic Acid", "Other Item")
                .replace("<strong>Class</strong>", "<strong>Type</strong>")
                .replace("#quest", "#taming")
                .replace(">Quest<", ">Taming<")
            )
            excerpt.update(raw=raw, sha256=hashlib.sha256(raw.encode()).hexdigest())
            with self.assertRaises(ValueError):
                nav.derive_row(*args)
        args = self.args()
        args[0]["wiki_source"]["coordinates"]["revision_timestamp"] = (
            "2026-10-02T00:00:00Z"
        )
        self.reseal_case(args)
        with self.assertRaises(ValueError):
            nav.derive_row(*args)

    def test_container_exception_is_only4290_and_no_scalar_promotion(self):
        for mutation in (
            lambda a: a[2]["flags"].pop("flags.container"),
            lambda a: a[0]["own_bound_crystal_record"]["attrs"].update(
                containersize="11"
            ),
            lambda a: a[0].update(scalar_holds=[]),
            lambda a: a[3][4][4290][0]["fields"].update(primarytype="Others"),
        ):
            args = self.args(4290)
            mutation(args)
            self.reseal_case(args)
            with self.assertRaisesRegex(ValueError, "ID4290"):
                nav.derive_row(*args)
        args = self.args(4290)
        args[0]["appearance_id"] = 4094
        self.reseal_case(args)
        with self.assertRaises(ValueError):
            nav.derive_row(*args)


if __name__ == "__main__":
    unittest.main()
