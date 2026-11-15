"""Negative cases guard evidence scope, custody, known zeros and provenance."""
import copy
import json
import pathlib
import tempfile
import unittest
import validate as v

A, C, Q, H, M = v.PINS

class EvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.payloads = v.load()

    def rejects(self, member, mutate, pattern):
        data = copy.deepcopy(self.payloads)
        mutate(data[member])
        with self.assertRaisesRegex(ValueError, pattern):
            v.validate_payloads(data)

    def test_pinned_payloads_and_scope(self):
        v.validate_payloads(self.payloads)

    def test_member_omission(self):
        data = dict(self.payloads)
        del data[M]
        with self.assertRaisesRegex(ValueError, "exact five"):
            v.validate_payloads(data)

    def test_prepared_byte_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            for name in v.PINS:
                (root / name).write_bytes((v.ROOT / name).read_bytes())
            path = root / A
            path.write_bytes(path.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "pin drift"):
                v.load(root)

    def test_private_absolute_path(self):
        self.rejects(M, lambda x: x.update(extra="/home/person/private"), "private path")

    def test_capture_path(self):
        self.rejects(M, lambda x: x.update(extra="proof/cards/001.png"), "private path")

    def test_narrative_literal(self):
        self.rejects(Q, lambda x: x.update(Description="private narrative"), "forbidden literal")

    def test_house_name_counter_not_literal(self):
        self.rejects(H, lambda x: x["summary"]["field_counts"].update(source_name_exact="private name"), "house name literal")

    def test_achievement_false_equal(self):
        self.rejects(A, lambda x: x["record_arrays"][0][4].update(raw_sha256="0" * 64), "false equality")

    def test_achievement_semantic_promotion(self):
        self.rejects(A, lambda x: x["record_arrays"][0].__setitem__(6, "GRADE_1"), "grade inference")

    def test_quest_new_id_promotion(self):
        def mutate(x):
            next(r for r in x["rows"] if r[0] == 192)[0] = 999
        self.rejects(Q, mutate, "new quest IDs")

    def test_quest_old_hash_substitution(self):
        self.rejects(Q, lambda x: x["rows"][0].__setitem__(7, "0" * 64), "old quest equality")

    def test_quest_runtime_admission(self):
        self.rejects(Q, lambda x: x["record_contract"].update(profile_admitted=True), "runtime promotion")

    def test_house_source_id_duplicate(self):
        self.rejects(H, lambda x: x["rows"][1].__setitem__(0, x["rows"][0][0]), "house ID coverage")

    def test_house_zero_beds_is_not_absence(self):
        def mutate(x):
            next(r for r in x["rows"] if r[6] == 0)[6] = None
        self.rejects(H, mutate, "known numerics")

    def test_house_changed_layout_cannot_be_hidden(self):
        def mutate(x):
            r = next(r for r in x["rows"] if r[0] == 55015)
            r[14] = r[13]
        self.rejects(H, mutate, "layout status")

    def test_house_engine_entrance_forbidden(self):
        self.rejects(H, lambda x: x["boundaries"].update(engine_entrance={"x": 1}), "engine entrance exported")

    def test_house_hash_reference_out_of_range(self):
        self.rejects(H, lambda x: x["rows"][0].__setitem__(11, 999999), "hash references")

    def test_creature_runtime_admission(self):
        self.rejects(C, lambda x: x.update(runtime_admitted=True), "creature admission")

    def test_creature_unknown_semantics_preserved(self):
        self.rejects(C, lambda x: x["field_scope"].update(boss_f4="category"), "unqualified raw semantics")

    def test_map_provenance_population_not_collapsed(self):
        def mutate(x):
            c = x["counts"]["source_provenance_statuses"]
            c["EXACT_RAW_RECORD_PROVENANCE_MATCH"] += 2
            c["SAME_CLIENT_SOURCE_ID_OLD_RAW_RECORD_DIFFERENT"] = 0
        self.rejects(M, mutate, "map provenance states")

if __name__ == "__main__":
    unittest.main()
