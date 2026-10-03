"""Keep third-party captures in every imbuement packet to bounded excerpts."""
import hashlib
import json
from pathlib import Path
import unittest

import behavior_answers
import capture_bounds
import combat_evidence

SAMPLES = Path(__file__).parent / "samples"


def load(name):
    return json.loads((SAMPLES / name).read_text(encoding="utf-8"))


def first_code_anchor(packet):
    for question in packet["questions"]:
        for answer in question.get("engine_answers", {}).values():
            for anchor in answer.get("code_anchors", []):
                if "quote" in anchor:
                    return anchor
    raise AssertionError("no quoted code anchor")


class CaptureBoundsTests(unittest.TestCase):
    def test_every_packet_is_bounded(self):
        for path in sorted(SAMPLES.glob("*.json")):
            with self.subTest(packet=path.name):
                capture_bounds.validate(json.loads(path.read_text(encoding="utf-8")), path.name)

    def test_full_combat_article_is_rejected(self):
        packet = load("imbuement-combat.json")
        source = next(s for s in packet["sources"] if s["id"] == "official_vocation_release_8833")
        source["captured_text"] = capture_bounds.EXCERPT_SEPARATOR.join(["Latest News " * 30] * 4)
        source["captured_text_sha256"] = hashlib.sha256(source["captured_text"].encode()).hexdigest()
        with self.assertRaisesRegex(ValueError, "exceeds its source bound"):
            combat_evidence.validate(packet)

    def test_unscoped_combat_capture_is_rejected(self):
        packet = load("imbuement-combat.json")
        next(s for s in packet["sources"] if s["id"] == "official_vocation_release_8849").pop("captured_text_scope")
        with self.assertRaisesRegex(ValueError, "bounded quoted excerpt"):
            combat_evidence.validate(packet)

    def test_overlong_combat_verbatim_is_rejected(self):
        packet = load("imbuement-combat.json")
        claims = next(s for s in packet["sources"] if s["id"] == "fandom_formulae_1205374")["selected_claims"]
        claims["verbatim"] += " " + "x" * 450
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            combat_evidence.validate(packet)

    def test_full_code_function_is_rejected(self):
        packet = load("current-behavior-answers.json")
        anchor = first_code_anchor(packet)
        anchor["quote"] = "\n".join(["\treturn;"] * 200)
        anchor["excerpt_sha256"] = hashlib.sha256(anchor["quote"].encode()).hexdigest()
        self.assertTrue(any("passage exceeds" in e for e in behavior_answers.validate(packet)))

    def test_code_excerpt_digest_mismatch_is_rejected(self):
        packet = load("current-behavior-answers.json")
        first_code_anchor(packet)["quote"] += "\n}"
        self.assertTrue(any("code anchor excerpt digest" in e for e in behavior_answers.validate(packet)))

    def test_full_public_quote_is_rejected(self):
        packet = load("global-rules-evidence.json")
        packet["sources"]["fandom_life_current_full"]["selected_quotes"] = ["Life Leech " * 50]
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            capture_bounds.validate(packet)

    def test_full_table_excerpt_is_rejected(self):
        packet = load("missing-item-source-facts.json")
        claim = next(iter(packet["records"][0]["field_evidence"].values()))
        claim["literal_excerpt"] = "| row |" * 100
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            capture_bounds.validate(packet)


if __name__ == "__main__":
    unittest.main()
