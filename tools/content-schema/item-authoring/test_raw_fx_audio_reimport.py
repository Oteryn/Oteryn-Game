import copy
import hashlib
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = (
    Path(__file__).resolve().parents[3] / "imports/ots-source-evidence/item-fx-audio295"
)


PRODUCER = ROOT / "producer/raw_fx_audio_reimport.py"
PRODUCER_SHA = "7f3c82e62d37c3bd9b3cba3bf8215a1f7ef3c2ec6df01c623aa75671a4ced031"
if hashlib.sha256(PRODUCER.read_bytes()).hexdigest() != PRODUCER_SHA:
    raise ValueError("sealed producer code digest drift")
spec = importlib.util.spec_from_file_location("sealed_raw_fx_audio_producer", PRODUCER)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class RawEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (ROOT / "raw-source-staging.json").read_bytes()
        cls.bundle = json.loads((ROOT / "retained-source-files.json").read_bytes())
        cls.batch = m.build_batch(cls.raw, cls.bundle)

    def test_exact_lossless_capture_whole295_plus_context(self):
        self.assertEqual(m.reconstruct(self.batch), self.raw)
        self.assertEqual(len(self.batch["reimport_states"]), 296)
        self.assertEqual(self.batch["candidates"], [])
        self.assertTrue(
            all(
                r["baseline"] is None and r["local"] is None
                for r in self.batch["reimport_states"]
            )
        )

    def test_actual_parent_all_seventeen_batches_are_preserved(self):
        document = json.loads(
            (ROOT.parents[2] / "content/world/provenance/imports.json").read_bytes()
        )
        document["batches"] = [
            b for b in document["batches"] if b["batch_id"] != m.BATCH_ID
        ]
        self.assertEqual(len(document["batches"]), 17)
        self.assertEqual(
            sum(len(b["reimport_states"]) for b in document["batches"]), 108
        )
        old = m.canonical(document)
        merged = m.append_batch(document, self.batch)
        self.assertEqual(m.canonical(document), old)
        self.assertEqual(len(merged["batches"]), 18)
        self.assertEqual(sum(len(b["reimport_states"]) for b in merged["batches"]), 404)
        retained = {b["batch_id"]: b for b in merged["batches"]}
        self.assertTrue(
            all(
                m.canonical(b) == m.canonical(retained[b["batch_id"]])
                for b in document["batches"]
            )
        )

    def test_current_reimport_meaning_all_five_decisions(self):
        a, b, c = ({"type": "Text", "value": t} for t in ("a", "b", "c"))
        self.assertEqual(m.decide(None, a, None), "AdoptUpstream")
        self.assertEqual(m.decide(a, a, a), "Unchanged")
        self.assertEqual(m.decide(a, a, b), "RetainLocal")
        self.assertEqual(m.decide(a, b, b), "Converged")
        self.assertEqual(m.decide(a, b, c), "Conflict")
        self.assertEqual(m.decide(True, 1, 0), "Conflict")

    def test_full_raw_source_opposition_duplicate_missing_rejected(self):
        for mutate in ("opposed", "duplicate", "missing"):
            bundle = copy.deepcopy(self.bundle)
            if mutate == "opposed":
                bundle["source_files"][-1]["raw_utf8"] += "\nchanged"
            elif mutate == "duplicate":
                bundle["source_files"].append(bundle["source_files"][-1])
            else:
                bundle["source_files"].pop()
            with self.assertRaises(ValueError):
                m.build_batch(self.raw, bundle)

    def test_wrong_capture_rejected(self):
        with self.assertRaises(ValueError):
            m.build_batch(self.raw + b" ", self.bundle)

    def test_namespace_duplicate_late_decision_header_rejected(self):
        for mutate in ("field", "duplicate", "decision", "header"):
            batch = copy.deepcopy(self.batch)
            if mutate == "field":
                batch["reimport_states"][-1]["field_path"] = "binding.native-item"
            elif mutate == "duplicate":
                batch["reimport_states"][-1] = batch["reimport_states"][-2]
            elif mutate == "decision":
                batch["reimport_states"][-1]["decision"] = "Unchanged"
            else:
                batch["mapper_sha256"] = "0" * 64
            with self.assertRaises(ValueError):
                m.reconstruct(batch)

    def test_atomic_append_idempotent_preserves_old(self):
        original = {
            "schema": "OTERYN_WORLD_PROJECT_IMPORT_CANDIDATES/v1",
            "batches": [{"batch_id": "old-batch", "reimport_states": []}],
        }
        frozen = m.canonical(original)
        result = m.append_batch(original, self.batch)
        self.assertEqual(m.canonical(original), frozen)
        self.assertEqual(
            m.canonical(m.append_batch(result, self.batch)), m.canonical(result)
        )
        opposed = copy.deepcopy(result)
        next(b for b in opposed["batches"] if b["batch_id"] == m.BATCH_ID)[
            "mapper_revision"
        ] = "opposed"
        saved = m.canonical(opposed)
        with self.assertRaises(ValueError):
            m.append_batch(opposed, self.batch)
        self.assertEqual(m.canonical(opposed), saved)


if __name__ == "__main__":
    unittest.main()
