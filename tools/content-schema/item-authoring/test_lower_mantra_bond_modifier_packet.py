"""Whole-vector parsing and closed current-identity/source guards for Mantra/Bond57."""

import copy
import json
import unittest

import lower_mantra_bond_modifier_packet as packet


class MantraBondPacketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((packet.ROOT / packet.PROOF).read_bytes())
        cls.order = packet.shared.native_order()
        cls.definitions, cls.routed = packet.stats.physical_field_inputs()
        cls.bindings = packet.base.exact_bindings(
            json.loads((packet.ROOT / packet.base.BINDINGS).read_bytes())["bindings"],
            set(packet.SOURCE_LABELS),
        )
        _, admitted = packet.membership.load_admitted()
        cls.members = {
            label: {r[0]: r for r in m["entries"]} for label, m in admitted.items()
        }
        cls.objects = {}
        for tag, raw in packet.shared.protobuf_fields(
            (packet.ROOT / packet.base.CLIENT).read_bytes()
        ):
            if tag == 1:
                obj = packet.shared.decode_appearance_object(raw)
                cls.objects[obj["id"]] = obj | {"object_sha256": packet.base.sha(raw)}
        cls.wiki = {
            r["item_id"]: r["observations"]
            for r in json.loads((packet.ROOT / packet.base.WIKI).read_bytes())[
                "records"
            ].values()
        }

    def qualify(
        self, entry, definition=None, binding=None, obj=None, routed=None, members=None
    ):
        return packet.qualify(
            entry,
            definition
            if definition is not None
            else self.definitions[entry["item_key"]],
            binding if binding is not None else self.bindings[entry["item_key"]],
            obj if obj is not None else self.objects[entry["item_id"]],
            self.wiki.get(entry["item_id"], []),
            self.routed if routed is None else routed,
            self.order,
            self.proof["source_cutoff"],
            self.members if members is None else members,
        )

    def test_complete70_preserves57_vectors130_atoms13_holds(self):
        rows = []
        for entry in self.proof["records"]:
            row = self.qualify(entry)
            self.assertEqual(
                row is not None, entry["item_id"] in packet.SOURCE_IDS, entry["item_id"]
            )
            if row:
                self.assertEqual(
                    row["typed_value"]["value"], entry["source_expected_vector"]
                )
                rows.append(row)
        self.assertEqual(len(rows), 57)
        self.assertEqual(sum(len(r["typed_value"]["value"]) for r in rows), 130)
        self.assertFalse(packet.IDS & packet.shared.IDS)

    def test_cutoff_source_is_distinct_from_postcutoff_retained_stats(self):
        entry = next(e for e in self.proof["records"] if e["item_id"] == 50239)
        self.assertEqual(entry["retained_observations"][0]["revision_id"], 1177239)
        self.assertEqual(
            entry["retained_stats_observations"][0]["revision_id"], 1207393
        )
        self.assertIsNotNone(self.qualify(entry))
        wrong = copy.deepcopy(entry)
        wrong["retained_observations"] = wrong["retained_stats_observations"]
        self.assertIsNone(self.qualify(wrong))
        self.assertEqual(
            self.proof["historical_source_separation"][
                "current_public_complete_vector_continuity"
            ],
            {"state": "UNKNOWN"},
        )

    def test_native49_excludes_all8_physical_whole_vectors(self):
        data = json.loads(packet.OUTPUT.read_bytes())
        expected_keys = {
            e["item_key"] for e in self.proof["records"] if e["item_id"] in packet.IDS
        }
        self.assertEqual({r["item_key"] for r in data["promotions"]}, expected_keys)
        self.assertEqual(len(data["promotions"]), 49)
        self.assertEqual(
            sum(len(r["typed_value"]["value"]) for r in data["promotions"]), 114
        )
        carriers = {
            h["item_id"]: h
            for h in data["holds"]
            if h.get("reason") == "CARRIER_MISSING"
        }
        self.assertEqual(set(carriers), packet.CARRIER_MISSING)
        for entry in self.proof["records"]:
            if entry["item_id"] in carriers:
                self.assertEqual(
                    carriers[entry["item_id"]]["whole_source_vector_preserved"][
                        "value"
                    ],
                    entry["source_expected_vector"],
                )
        self.assertEqual(len(data["holds"]), 21)
        self.assertEqual(data["counts"]["source_qualified_vectors"], 57)
        self.assertEqual(data["counts"]["source_qualified_atoms"], 130)

    def test_signed_mantra_unit_bounds_and_three_bond_elements(self):
        for value in ("-32768", "0", "+32767"):
            typed = packet.whole_vector({"mantra": [value]}, self.order)
            self.assertEqual(
                typed["value"][0]["parameter"]["value"],
                {"kind": "SIGNED_POINTS", "value": int(value)},
            )
        for value in ("32768", "-32769", "2%", "2.0", "two", "2 seconds"):
            self.assertIsNone(packet.whole_vector({"mantra": [value]}, self.order))
        for value in ("Physical", "ENERGY", "earth"):
            typed = packet.whole_vector({"elementalbond": [value]}, self.order)
            self.assertEqual(
                typed["value"][0]["parameter"]["value"],
                {"kind": "ELEMENT", "value": value.upper()},
            )
        for value in ("Fire", "None", "Physical/Earth", "energy 1"):
            self.assertIsNone(
                packet.whole_vector({"elementalbond": [value]}, self.order)
            )

    def test_other_fields_empty_duplicate_or_unsupported_hold_whole_vector(self):
        for field in packet.shared.GROUP:
            for invalid in ([""], ["1", "1"]):
                self.assertIsNone(
                    packet.whole_vector({"mantra": ["2"], field: invalid}, self.order),
                    field,
                )
        for attrib in (
            "unknown skill +2",
            "magic level +2, magic level +2",
            "magic level +2147483648",
        ):
            self.assertIsNone(
                packet.whole_vector({"mantra": ["2"], "attrib": [attrib]}, self.order)
            )

    def test_complete_baseline_and_contexts_preserved_no_alias_mutation(self):
        aliases = copy.deepcopy(packet.shared.ALIASES)
        fields = {
            "mantra": ["2"],
            "elementalbond": ["Energy"],
            "attrib": ["magic level +2, earth magic level +1"],
            "crit_chance": ["10"],
        }
        # Use the actual accepted percentage vocabulary instead of inventing a source key.
        percent = next(iter(packet.stats.MODIFIER_PERCENTS))
        fields.pop("crit_chance")
        fields[percent] = ["10%"]
        typed = packet.whole_vector(fields, self.order)
        self.assertEqual(len(typed["value"]), 5)
        self.assertEqual(
            [self.order[e["kind"]] for e in typed["value"]],
            sorted(self.order[e["kind"]] for e in typed["value"]),
        )
        for entry in typed["value"]:
            for context in ("target_domain", "evaluation_phase", "priority"):
                self.assertEqual(entry[context], {"state": "UNKNOWN"})
        self.assertEqual(aliases, packet.shared.ALIASES)
        self.assertIsNone(packet.shared.whole_vector(fields, self.order))

    def test_current_membership_projection_source_and_full_native_identity(self):
        entry = next(e for e in self.proof["records"] if e["item_id"] in packet.IDS)
        for field in ("current_member", "old_member"):
            wrong = copy.deepcopy(entry)
            wrong[field][1] = "wrong projection"
            self.assertIsNone(self.qualify(wrong))
        wrong = copy.deepcopy(entry)
        wrong["binding_member_label"] = "canary-47dfd51f"
        self.assertIsNone(self.qualify(wrong))
        wrong_binding = copy.deepcopy(entry["binding"])
        wrong_binding["source_revision"] = "invented"
        self.assertIsNone(self.qualify(entry, binding=wrong_binding))
        for field in ("family", "revision", "key"):
            definition = copy.deepcopy(self.definitions[entry["item_key"]])
            definition["identity"][field] = "wrong"
            self.assertIsNone(self.qualify(entry, definition=definition))
        self.assertIsNone(self.qualify(entry, routed={entry["item_key"]}))

    def test_raw_group_source_tampering_and_identity_guard_preserved(self):
        entry = next(e for e in self.proof["records"] if e["item_id"] in packet.IDS)
        wrong = copy.deepcopy(entry)
        wrong["own_boxes"][0]["raw"] += "tampered"
        self.assertIsNone(self.qualify(wrong))
        wrong = copy.deepcopy(entry)
        box = wrong["own_boxes"][0]
        box["raw"] = box["raw"][:-2] + "| mantra =\n}}"
        box["raw_sha256"] = packet.base.sha(box["raw"].encode())
        box["parameter_values"] = packet.base.raw_parameters(box["raw"])
        self.assertIsNone(self.qualify(wrong))
        wrong = copy.deepcopy(entry)
        wrong["own_boxes"][0]["revision_timestamp"] = "2026-10-02T00:00:00Z"
        self.assertIsNone(self.qualify(wrong))
        obj = copy.deepcopy(self.objects[entry["item_id"]])
        obj["flags"]["flags.unmove"] = True
        self.assertIsNone(self.qualify(entry, obj=obj))

    def test_known_same_is_idempotent_different_or_blocked_native_held(self):
        entry = next(e for e in self.proof["records"] if e["item_id"] in packet.IDS)
        definition = copy.deepcopy(self.definitions[entry["item_key"]])
        row = self.qualify(entry)
        definition["semantics"]["skill_modifiers"] = {
            "state": "KNOWN",
            "value": {
                "modifiers": {"state": "KNOWN", "value": row["typed_value"]["value"]}
            },
        }
        self.assertEqual(self.qualify(entry, definition=definition), row)
        definition["semantics"]["skill_modifiers"]["value"]["modifiers"]["value"][0][
            "priority"
        ] = {"state": "KNOWN", "value": 1}
        self.assertIsNone(self.qualify(entry, definition=definition))
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            definition["semantics"]["skill_modifiers"] = {"state": state}
            self.assertIsNone(self.qualify(entry, definition=definition))


if __name__ == "__main__":
    unittest.main()
