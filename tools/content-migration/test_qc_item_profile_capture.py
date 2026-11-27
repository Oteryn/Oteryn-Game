"""Published QC qualification evidence points to the current quest cohort."""

import hashlib
import json
import unittest

import qc_item_profiles as qc


class QcItemProfileCaptureTests(unittest.TestCase):
    def test_current_quest_inputs_and_qualified_records(self):
        packet = json.loads((qc.batch.ROOT / qc.PATH).read_bytes())
        descriptors = {row["path"]: row["sha256"] for row in packet["authoring_inputs"]}
        for path in qc.INPUTS:
            self.assertEqual(
                descriptors[path],
                hashlib.sha256((qc.batch.ROOT / path).read_bytes()).hexdigest(),
            )
        self.assertEqual(
            {
                row["production_definition"]["production_key"]
                for row in packet["records"]
            },
            qc.KEYS,
        )
