"""Bounded in-memory mutation of explicit unowned-item preservation contracts."""
import hashlib
import importlib
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

repo = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2])
sys.path.insert(0, str(repo / "tools/content-schema/native-gameplay"))
results = []
for name in ("test_encounter_corpse_items", "test_native_goal_corpse_items"):
    module = importlib.import_module(name)
    producer = module.encounter if name == "test_encounter_corpse_items" else module.native
    original = producer.apply_admissions
    for mutant in (False, True):
        dropped = []
        def apply(definitions, *args, **kwargs):
            result = original(definitions, *args, **kwargs)
            removed = [row["identity"]["key"] for row in definitions
                       if row["identity"]["key"] == "oteryn:item.tibia.i39530"]
            if removed:
                definitions[:] = [row for row in definitions
                                  if row["identity"]["key"] != "oteryn:item.tibia.i39530"]
                dropped.extend(removed)
            return result
        stream = io.StringIO()
        with patch.object(producer, "apply_admissions", apply if mutant else original):
            suite = unittest.defaultTestLoader.loadTestsFromModule(module)
            result = unittest.TextTestRunner(stream=stream, verbosity=2).run(suite)
        results.append({"module": name, "mutant": mutant,
                        "source_sha256": hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest(),
                        "producer_sha256": hashlib.sha256(Path(producer.__file__).read_bytes()).hexdigest(),
                        "testsRun": result.testsRun, "errors": len(result.errors),
                        "failures": len(result.failures), "skipped": len(result.skipped),
                        "successful": result.wasSuccessful(), "removed_unowned_ids": dropped,
                        "output": stream.getvalue()})
        print(name, "MUTANT" if mutant else "BASELINE", result.testsRun,
              result.wasSuccessful(), "removed", len(dropped), flush=True)
output.write_text(json.dumps({"limits": "In-memory Python test-oracle probe, no product/source edit, no hosted CI claim.",
                              "mutation": "After successful admission, remove explicit unowned fixture identity39530 from input list.",
                              "results": results}, indent=2) + "\n")
