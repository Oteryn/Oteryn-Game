"""Exercise the source access predicates using completed-event scenarios."""

import json
from pathlib import Path
import unittest


PACKET = Path(__file__).parent / "samples" / "imbuement-access.json"
METADATA = {"source_refs", "notes", "quest_line_source_locator"}


def unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate key: {key}")
        result[key] = value
    return result


class AccessEvidenceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(PACKET.read_text(), object_pairs_hook=unique_keys)
        cls.definitions = cls.packet["predicate_definitions"]
        cls.families = cls.packet["by_name"]

    def resolve(self, expr):
        if "predicate_ref" in expr:
            return self.definitions[expr["predicate_ref"]]
        if "unlock_ref" in expr:
            return self.families[expr["unlock_ref"]]["powerful_unlock"]
        return expr

    def event(self, expr):
        return json.dumps({k: v for k, v in expr.items() if k not in METADATA}, sort_keys=True)

    def leaves(self, expr, ancestors=()):
        marker = json.dumps(expr, sort_keys=True)
        self.assertNotIn(marker, ancestors, "Cyclic access expression")
        ancestors = (*ancestors, marker)
        resolved = self.resolve(expr)
        if resolved is not expr:
            return self.leaves(resolved, ancestors)
        ops = set(expr) & {"all_of", "any_of"}
        self.assertLessEqual(len(ops), 1, "Ambiguous access expression")
        if ops:
            op = next(iter(ops))
            self.assertTrue(expr[op], "An empty operator must not grant access")
            return set().union(*(self.leaves(term, ancestors) for term in expr[op]))
        self.assertIn("type", expr, "A predicate must have an explicit type")
        return {self.event(expr)}

    def granted(self, expr, events):
        expr = self.resolve(expr)
        if "predicate_ref" in expr or "unlock_ref" in expr:
            return self.granted(expr, events)
        if "all_of" in expr:
            return all(self.granted(term, events) for term in expr["all_of"])
        if "any_of" in expr:
            return any(self.granted(term, events) for term in expr["any_of"])
        return self.event(expr) in events

    def test_routes_are_total_and_unambiguous(self):
        self.assertEqual(len(self.families), 24)
        for name, family in self.families.items():
            with self.subTest(name=name):
                self.assertEqual(set(family["direct_shrine"]), {"basic", "intricate", "powerful"})
                for expr in family["direct_shrine"].values():
                    self.leaves(expr)
                self.leaves(family["scroll_apply"])
                for tier in ("intricate", "powerful"):
                    self.leaves(family["scroll_inscription"][tier])

    def test_free_character_can_shrine_basic_but_not_higher_tiers(self):
        events = self.leaves(self.definitions["shrine_access"]) | self.leaves(self.definitions["compatible_item"])
        for name, family in self.families.items():
            with self.subTest(name=name):
                self.assertTrue(self.granted(family["direct_shrine"]["basic"], events))
                self.assertFalse(self.granted(family["direct_shrine"]["intricate"], events))
                self.assertFalse(self.granted(family["direct_shrine"]["powerful"], events))

    def test_each_initial_shrine_event_is_required(self):
        events = self.leaves(self.definitions["shrine_access"])
        self.assertEqual(len(events), 2)
        for event in events:
            self.assertFalse(self.granted(self.definitions["shrine_access"], events - {event}))

    def test_premium_does_not_replace_powerful_unlock(self):
        events = self.leaves(self.definitions["shrine_access"]) | self.leaves(self.definitions["compatible_item"]) | self.leaves(self.definitions["premium"])
        for name, family in self.families.items():
            with self.subTest(name=name):
                self.assertTrue(self.granted(family["direct_shrine"]["intricate"], events))
                self.assertFalse(self.granted(family["direct_shrine"]["powerful"], events))

    def test_every_unlock_branch_is_sufficient_and_every_required_event_matters(self):
        for name, family in self.families.items():
            for branch in family["powerful_unlock"]["any_of"]:
                with self.subTest(name=name, branch=branch):
                    required = self.leaves(branch)
                    self.assertTrue(self.granted(family["powerful_unlock"], required))
                    # The Dream Courts reward NPCs are alternatives; tested separately.
                    if branch["predicate_ref"] == "dream_courts":
                        continue
                    for event in required:
                        self.assertFalse(self.granted(branch, required - {event}))

    def test_heart_of_destruction_is_an_alternative_for_exactly_eight_families(self):
        events = self.leaves(self.definitions["heart_of_destruction"])
        actual = {name for name, family in self.families.items() if self.granted(family["powerful_unlock"], events)}
        self.assertEqual(actual, {"Strike", "Epiphany", "Void", "Vampirism", "Lich Shroud", "Reap", "Dragon Hide", "Scorch"})

    def test_either_dream_courts_report_works_but_kill_alone_does_not(self):
        unlock = self.families["Vibrancy"]["powerful_unlock"]
        kill, reports = self.definitions["dream_courts"]["all_of"]
        self.assertFalse(self.granted(unlock, self.leaves(kill)))
        for report in reports["any_of"]:
            self.assertTrue(self.granted(unlock, self.leaves(kill) | self.leaves(report)))
            self.assertFalse(self.granted(unlock, self.leaves(report)))

    def test_free_unworthy_character_can_apply_all_scroll_tiers(self):
        for name, family in self.families.items():
            with self.subTest(name=name):
                apply = family["scroll_apply"]
                self.assertEqual(set(apply["tiers"]), {"basic", "intricate", "powerful"})
                self.assertEqual(set(apply["exempt_from"]), {"premium", "shrine_access", "powerful_unlock"})
                events = self.leaves(apply)
                self.assertTrue(self.granted(apply, events))
                self.assertFalse(events & self.leaves(self.definitions["premium"]))
                self.assertFalse(events & self.leaves(self.definitions["shrine_access"]))

    def test_scroll_inscription_does_not_inherit_application_exemptions(self):
        premium = self.leaves(self.definitions["premium"])
        for name, family in self.families.items():
            with self.subTest(name=name):
                for tier in ("intricate", "powerful"):
                    expr = family["scroll_inscription"][tier]
                    events = self.leaves(expr)
                    self.assertTrue(self.granted(expr, events))
                    self.assertFalse(self.granted(expr, events - premium))
                self.assertFalse(family["scroll_inscription"]["basic"]["allowed"])

    def test_blank_and_basic_loot_do_not_prove_completed_higher_tier_loot(self):
        loot = self.packet["scroll_acquisition"]["loot"]
        self.assertTrue(loot["blank_scroll"])
        self.assertTrue(loot["basic_scroll"])
        for item in ("intricate_scroll", "powerful_scroll"):
            self.assertIsNone(loot[item], "A completed-scroll drop requires its own named evidence")
            self.assertEqual(loot["evidence_status_by_item"][item], "UNKNOWN")
        self.assertNotIn("official_manual", loot["source_refs"])
        gap = loot["source_gaps"][0]
        self.assertEqual(gap["kind"], "COMPLETED_SCROLL_LOOT_UNCONFIRMED")
        self.assertEqual(set(gap["tiers"]), {"intricate", "powerful"})
        # Unknown loot must not disable the independently documented crafting path.
        for family in self.families.values():
            for tier in ("intricate", "powerful"):
                expr = family["scroll_inscription"][tier]
                self.assertTrue(self.granted(expr, self.leaves(expr)))


if __name__ == "__main__":
    unittest.main()
