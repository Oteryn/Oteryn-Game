"""Keep third-party captures in every imbuement packet to bounded excerpts."""
import hashlib
import json
from pathlib import Path
import re
import unittest

import behavior_answers
import capture_bounds
import combat_evidence

SAMPLES = Path(__file__).parent / "samples"
# Any field name that looks quote-bearing must be bounded, or be a digest,
# scope, reference or flag companion of one.
QUOTE_LIKE = re.compile(r"quot|verbatim|excerpt|snippet|transcri|captured_text|source_lines|context")
COMPANION = re.compile(r"_(?:sha256|scope|refs|read)$")


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

    def combat_claims(self, packet, source_id):
        return next(s for s in packet["sources"] if s["id"] == source_id)["selected_claims"]

    def test_full_page_in_plural_quotes_is_rejected(self):
        packet = load("imbuement-combat.json")
        self.combat_claims(packet, "tibiaqa_two_leech_pairs_2021")["quotes"] = "x" * 20000
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            combat_evidence.validate(packet)

    def test_quotes_array_cannot_exceed_source_bound(self):
        packet = load("imbuement-combat.json")
        self.combat_claims(packet, "tibiaqa_two_leech_pairs_2021")["quotes"] = ["y" * 400] * 3
        with self.assertRaisesRegex(ValueError, "exceeds its source bound"):
            combat_evidence.validate(packet)

    def test_overlong_verbatim_anchor_is_rejected(self):
        packet = load("imbuement-combat.json")
        claims = self.combat_claims(packet, "fandom_formulae_1205374")
        claims["native_percentage_reduction_example"]["verbatim_anchor"] = "z" * 451
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            combat_evidence.validate(packet)

    def test_other_quote_bearing_fields_are_bounded(self):
        cases = (
            ("global-rules-evidence.json",
             lambda p: p["sources"]["official_cm_scroll_anywhere_2025"].update(question_context="q" * 451)),
            ("global-rules-evidence.json",
             lambda p: next(c for c in p["conflicts"] if "shorter_tier_snippets" in c).update(
                 shorter_tier_snippets="s" * 451)),
            ("imbuement-eligibility.json",
             lambda p: next(i for i in p["items"] if i.get("independent_field_corroborations"))[
                 "independent_field_corroborations"][0].update(quote_context="c" * 451)),
            ("imbuement-access.json",
             lambda p: p["sources"]["br_blank_scroll"].update(selected_quote="a" * 451)),
        )
        for name, mutate in cases:
            with self.subTest(packet=name):
                packet = load(name)
                mutate(packet)
                with self.assertRaisesRegex(ValueError, "passage exceeds"):
                    capture_bounds.validate(packet, name)

    def test_every_quote_like_field_is_bounded(self):
        def walk(node, path):
            if isinstance(node, list):
                for item in node:
                    walk(item, path)
            elif isinstance(node, dict):
                for key, value in node.items():
                    if QUOTE_LIKE.search(key) and not isinstance(value, dict) and not COMPANION.search(key):
                        self.assertTrue(capture_bounds.is_text_field(key, value), f"{path}.{key} is not bounded")
                    walk(value, f"{path}.{key}")
        for path in sorted(SAMPLES.glob("*.json")):
            walk(json.loads(path.read_text(encoding="utf-8")), path.name)


if __name__ == "__main__":
    unittest.main()
