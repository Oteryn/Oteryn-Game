"""Measured census refreshes retain preservation tests and reject stale ownership reports."""
from pathlib import Path
import importlib.util
import json
import re
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("current_world_regen", Path(__file__).with_name("regenerate_content.py"))
regen = importlib.util.module_from_spec(spec)
spec.loader.exec_module(regen)

class CurrentWorldCensusRefresh(unittest.TestCase):
    def test_measured_source_and_npc_censuses_preserve_strict_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = ["apps/game-server/tests/content_world_project_repository.rs",
                     "apps/game-server/tests/content_npc_service_model.rs",
                     "apps/game-server/tests/integration_rest/content_item_fx_audio_raw.rs",
                     "apps/game-server/examples/support/item_fx_audio_raw_import.rs",
                     "content/world/provenance/imports.json"]
            for name in files:
                target=root/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ROOT/name,target)
            original = regen.ROOT, regen.PIN_TEST
            try:
                regen.ROOT=root;regen.PIN_TEST=root/files[0]
                batches=json.loads((ROOT/"content/world/provenance/imports.json").read_bytes())["batches"]
                sources=json.loads((ROOT/"content/world/provenance/sources.json").read_bytes())["sources"]
                service=(root/files[1]).read_text()
                def count(expression):
                    return int(re.search(r"assert_eq!\("+expression+r",\s*([\d_]+)\);",service)[1].replace("_",""))
                model={key:count(expression) for key,expression in {
                    "held_npcs":r"model\.held_npcs\(\)\.len\(\)","services":r"model\.services\(\)\.len\(\)",
                    "offers":r"model\.admitted_offer_count\(\)","routes":r"model\.admitted_route_count\(\)",
                    "generated_replies":r"model\.generated_reply_npc_count\(\)"}.items()}
                model["npcs"]=1283
                model["held_offers"]={key:int(value.replace("_","")) for key,value in re.findall(r'\("([A-Za-z]+)",\s*([\d_]+)\),',service)}
                census={"source_count":len(sources),"import_candidate_count":sum(len(b["candidates"]) for b in batches),
                        "reimport_count":sum(len(b["reimport_states"]) for b in batches),"npc_service_model":model,
                        "imports":[{"batch_id":b["batch_id"],"reimport_states":len(b["reimport_states"])} for b in batches]}
                raw_before=(root/files[2]).read_text()
                regen.repin_current_source_censuses(census)
                first={name:(root/name).read_bytes() for name in files[:3]}
                regen.repin_current_source_censuses(census)
                self.assertEqual(first,{name:(root/name).read_bytes() for name in files[:3]},"refresh is idempotent")
                package=(root/files[0]).read_text();self.assertIn(f"max_import_records: {len(sources)},",package)
                self.assertIn("assert_eq!(model.npcs().len(), 1283);",(root/files[1]).read_text())
                raw_after=(root/files[2]).read_text()
                for clause in ["assert_eq!(raw::append(&mut batches), Ok(296));", "assert_eq!(batches, once);", "assert_eq!(batches, opposed);"]:
                    self.assertIn(clause,raw_before);self.assertIn(clause,raw_after)
                bad=dict(census,imports=[])
                with self.assertRaisesRegex(SystemExit,"raw FX batch"):
                    regen.repin_current_source_censuses(bad)
            finally:
                regen.ROOT,regen.PIN_TEST=original

if __name__=="__main__":unittest.main()
