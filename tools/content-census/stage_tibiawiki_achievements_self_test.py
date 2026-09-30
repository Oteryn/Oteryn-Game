#!/usr/bin/env python3
"""Self-test for stage_tibiawiki_achievements.py on synthetic snapshots (no network)."""
from __future__ import annotations

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stage_tibiawiki_achievements as stage  # noqa: E402


def infobox(**fields: str) -> str:
    body = "".join(f"\n| {key} = {value}" for key, value in fields.items())
    return "{{Infobox Achievement|List={{{1|}}}|GetValue={{{GetValue|}}}" + body + "\n}}\n[[Category:Achievements]]"


BASE = {"grade": "1", "name": "Chorister", "description": "Lalalala.", "premium": "yes", "points": "1",
        "secret": "no", "achievementid": "2", "relatedpages": "[[Liberty Bay]], [[Hymn_Book|hymn]]",
        "spoiler": "Sing in [[Liberty Bay]]."}


def raw(*contents: str) -> dict:
    return {"category": stage.CATEGORY,
            "pages": [{"pageid": 10 + i, "title": f"Page {i}", "revid": 100 + i, "timestamp": "2026-09-29T00:00:00Z",
                       "content": content} for i, content in enumerate(contents)]}


class StageTibiawikiAchievementsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        static = self.root / stage.STATICDATA
        static.parent.mkdir(parents=True)
        static.write_text(json.dumps({"records": [
            {"source_id": 2, "name": "Chorister", "grade": 1, "description": "Lalalala."},
            {"source_id": 3, "name": "The Milkman", "grade": 1, "description": "Milk."}]}))

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_build(self, snapshot: dict) -> int:
        path = self.root / "raw.json"
        path.write_bytes(stage.canonical(snapshot))
        return stage.main(["build", str(path), "--root", str(self.root)])

    def load(self, name: str) -> dict:
        return json.loads((self.root / stage.OUTPUT_DIR / name).read_text())

    def test_build_join_and_check(self) -> None:
        secret = infobox(**{**BASE, "name": "Hidden", "achievementid": "7", "secret": "yes", "grade": "0",
                            "history": "Found via the bazaar.\n<gallery>\nA.png|caption\n</gallery>"})
        milkman = infobox(**{**BASE, "name": "The Milkman", "achievementid": "3", "description": "Milk!"})
        uncertain = infobox(**{**BASE, "name": "Achievement 9", "achievementid": "9?", "unknown": "yes"})
        snapshot = raw(infobox(**BASE), secret, milkman, uncertain, "{{DPL}} list page")
        self.assertEqual(self.run_build(snapshot), 0)
        facts, join = self.load(stage.FACTS_FILE), self.load(stage.JOIN_FILE)
        self.assertEqual(facts["counts"], {"achievements": 4, "skipped_pages": 1, "secret": 1})
        chorister = facts["pages"][0]["fields"]
        self.assertEqual(chorister["relatedpages_links"], ["Hymn Book", "Liberty Bay"])
        self.assertEqual(chorister["spoiler_links"], ["Liberty Bay"])
        self.assertNotIn("spoiler", chorister)
        self.assertNotIn("history", facts["pages"][2]["fields"])
        self.assertEqual(join["counts"]["joined"], 2)
        self.assertEqual(join["wiki_only"], [{"achievementid": 7, "title": "Page 1", "secret": "yes"}])
        self.assertEqual(join["uncertain_id"], [{"achievementid": "9?", "title": "Page 3"}])
        self.assertEqual(join["joined_with_differences"][0]["differences"],
                         [{"field": "description", "staticdata": "Milk.", "wiki": "Milk!"}])
        self.assertIn({"title": "Page 1", "field": "grade", "value": "0", "reason": "not a grade 1-4"},
                      join["anomalies"])
        self.assertEqual(stage.main(["check", "--root", str(self.root)]), 0)
        self.assertEqual(stage.main(["check", str(self.root / "raw.json"), "--root", str(self.root)]), 0)

    def test_check_rejects_tampering(self) -> None:
        self.assertEqual(self.run_build(raw(infobox(**BASE))), 0)
        join_path = self.root / stage.OUTPUT_DIR / stage.JOIN_FILE
        join = json.loads(join_path.read_text())
        join["counts"]["joined"] = 0
        join_path.write_bytes(stage.canonical(join))
        self.assertEqual(stage.main(["check", "--root", str(self.root)]), 1)
        self.run_build(raw(infobox(**BASE)))
        changed = raw(infobox(**{**BASE, "points": "3"}))
        (self.root / "changed.json").write_bytes(stage.canonical(changed))
        self.assertEqual(stage.main(["check", str(self.root / "changed.json"), "--root", str(self.root)]), 1)

    def test_rejects_unknown_parameter_and_duplicate_id(self) -> None:
        with self.assertRaisesRegex(ValueError, "unknown infobox parameter 'outfit'"):
            stage.build_facts(raw(infobox(**BASE, outfit="x")))
        with self.assertRaisesRegex(ValueError, "duplicate achievementid"):
            stage.build_facts(raw(infobox(**BASE), infobox(**BASE)))
        with self.assertRaisesRegex(ValueError, "invalid or duplicate achievementid"):
            stage.build_facts(raw(infobox(**{**BASE, "achievementid": "x"})))

    def test_rejects_unterminated_infobox(self) -> None:
        with self.assertRaisesRegex(ValueError, "unterminated"):
            stage.build_facts(raw("{{Infobox Achievement| name = A"))


if __name__ == "__main__":
    unittest.main()
