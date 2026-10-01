"""Exercise the source access predicates using completed-event scenarios."""

import json
from pathlib import Path
import unittest


PACKET = Path(__file__).parent / "samples" / "imbuement-access.json"
ELIGIBILITY = PACKET.with_name("imbuement-eligibility.json")
METADATA = {"source_refs", "notes", "quest_line_source_locator"}


def unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate key: {key}")
        result[key] = value
    return result


class AccessEvidenceTest(unittest.TestCase):
    def test_basic_scroll_announcement_has_full_primary_capture_and_separate_live_release(self):
        teaser = self.packet["sources"]["official_echo_wardens"]
        live = self.packet["sources"]["official_echo_raids_live_release"]
        self.assertEqual(teaser["published_on"], "2026-06-11")
        self.assertEqual(teaser["access_status"], "FULL_BROWSER_TEXT_EXTRACTED")
        self.assertEqual(live["published_on"], "2026-07-13")
        self.assertIn("always drop one", teaser["selected_quote"])
        self.assertNotIn("always drop one", live["selected_quote"])
        self.assertIn("official_echo_raids_live_release",
                      self.packet["scroll_acquisition"]["loot"]["source_refs"])
        for family in self.families.values():
            self.assertIn("official_echo_raids_live_release", family["scroll_apply"]["source_refs"])
            self.assertEqual(family["scroll_apply"]["evidence_strength_by_tier"]["basic"],
                             "DERIVED_FROM_PRIMARY_AND_CLIENT")

    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(PACKET.read_text(), object_pairs_hook=unique_keys)
        cls.definitions = cls.packet["predicate_definitions"]
        cls.families = cls.packet["by_name"]
        cls.eligible_items = json.loads(ELIGIBILITY.read_text(), object_pairs_hook=unique_keys)["items"]

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

    def granted(self, expr, events, selected_family=None, existing_families=None,
                selected_tier=None, eligible_item=None):
        expr = self.resolve(expr)
        if "predicate_ref" in expr or "unlock_ref" in expr:
            return self.granted(expr, events, selected_family, existing_families, selected_tier, eligible_item)
        if "all_of" in expr:
            return all(self.granted(term, events, selected_family, existing_families,
                                    selected_tier, eligible_item) for term in expr["all_of"])
        if "any_of" in expr:
            return any(self.granted(term, events, selected_family, existing_families,
                                    selected_tier, eligible_item) for term in expr["any_of"])
        if eligible_item is not None and expr.get("type") == "item_accepts_imbuement_family":
            key = (selected_family or "").lower().replace(" ", "_")
            return key in eligible_item["allowed_types"]
        if expr.get("type") == "item_accepts_requested_imbuement_tier" and (
                eligible_item is not None or selected_tier is not None):
            if eligible_item is None or type(selected_tier) is not int:
                return False
            key = (selected_family or "").lower().replace(" ", "_")
            maximum = eligible_item["allowed_types"].get(key)
            return (type(maximum) is int and maximum in expr["requested_tier_domain"] and
                    selected_tier in expr["requested_tier_domain"] and selected_tier <= maximum)
        if expr.get("type") == "item_has_no_conflicting_elemental_conversion" and selected_family is not None:
            return (selected_family not in expr["selected_family_scope"] or
                    not set(existing_families or ()) & set(expr["conflicting_existing_families"]))
        if expr.get("type") == "item_has_no_duplicate_imbuement_family" and selected_family is not None:
            return selected_family not in (existing_families or ())
        return self.event(expr) in events

    def test_requested_tier_respects_selected_item_family_limit_for_shrine_and_scroll(self):
        compatible = self.definitions["compatible_item"]
        guards = [r for r in compatible["all_of"]
                  if r.get("type") == "item_accepts_requested_imbuement_tier"]
        self.assertEqual(len(guards), 1)
        guard = guards[0]
        self.assertEqual(guard["eligibility_profile"], "imbuement-eligibility.json#items")
        self.assertEqual(guard["item_match_field"], "item_ref.key")
        self.assertEqual(guard["max_tier_path"], "items[].allowed_types[requested_family_key]")
        self.assertEqual(guard["requested_tier_domain"], [1, 2, 3])
        source = self.packet["sources"][guard["source_refs"][0]]
        self.assertEqual(source["role"], "DERIVED_CLIENT_IDENTITY_AND_COMMUNITY_ELIGIBILITY")
        self.assertIn("source-qualified community", source["scope"])
        casque = next(r for r in self.eligible_items if r["client_id"] == 44636)
        self.assertEqual(casque["item_ref"]["key"], "oteryn:item.tibia.i44636")
        self.assertEqual(casque["allowed_types"]["epiphany"], 2)
        self.assertTrue(casque["selected_type_amendments"])
        routes = self.families["Epiphany"]
        for tier, numeric in guard["tier_name_to_number"].items():
            for route, expr in (("shrine", routes["direct_shrine"][tier]),
                                ("scroll", routes["scroll_apply"])):
                with self.subTest(route=route, tier=tier):
                    # All ownership, quest and slot events pass; the actual named
                    # item's Epiphany cap alone must stop Powerful on either route.
                    self.assertEqual(self.granted(expr, self.leaves(expr), "Epiphany", [], numeric, casque),
                                     numeric <= 2)
                    for invalid in (0, 4, -1, True, "2", 2.0, None):
                        self.assertFalse(self.granted(expr, self.leaves(expr), "Epiphany", [], invalid, casque))
        # The cap is per family rather than a single cap shared by the whole helmet.
        void = self.families["Void"]["scroll_apply"]
        self.assertTrue(self.granted(void, self.leaves(void), "Void", [], 3, casque))
        scorch = self.families["Scorch"]["scroll_apply"]
        self.assertFalse(self.granted(scorch, self.leaves(scorch), "Scorch", [], 1, casque))
        for invalid_limit in (0, 4, True, "2", None):
            invalid_item = dict(casque, allowed_types={"epiphany": invalid_limit})
            expr = routes["scroll_apply"]
            self.assertFalse(self.granted(expr, self.leaves(expr), "Epiphany", [], 1, invalid_item))
        shrine_count = scroll_count = 0
        for name, family in self.families.items():
            for tier, numeric in guard["tier_name_to_number"].items():
                for route, expr in (("shrine", family["direct_shrine"][tier]),
                                    ("scroll", family["scroll_apply"])):
                    self.assertIn({"predicate_ref": "compatible_item"}, expr["all_of"])
                    events = self.leaves(expr)
                    matching = next(r for r in self.eligible_items
                                    if r["allowed_types"].get(name.lower().replace(" ", "_"), 0) >= numeric)
                    self.assertTrue(self.granted(expr, events, name, [], numeric, matching))
                    self.assertFalse(self.granted(expr, events, name, [], 4, matching))
                    shrine_count += route == "shrine"
                    scroll_count += route == "scroll"
        self.assertEqual((shrine_count, scroll_count), (72, 72))

    def test_distinct_elemental_conversions_conflict_without_blocking_other_groups(self):
        compatible = self.definitions["compatible_item"]
        guards = [r for r in compatible["all_of"]
                  if r.get("type") == "item_has_no_conflicting_elemental_conversion"]
        self.assertEqual(len(guards), 1)
        guard = guards[0]
        expected = {"Scorch", "Venom", "Frost", "Electrify", "Reap"}
        self.assertEqual(set(guard["selected_family_scope"]), expected)
        self.assertEqual(set(guard["conflicting_existing_families"]), expected)
        self.assertEqual(guard["rule_profile"], "global-rules-evidence.json#shared_category_compatibility")
        source = self.packet["sources"][guard["source_refs"][0]]
        self.assertEqual(source["revision_id"], 1194750)
        # Venom and Scorch are different families: the ordinary duplicate-family
        # and empty-slot predicates both pass, while their shared conversion group fails.
        for name, family in self.families.items():
            for tier, expr in family["direct_shrine"].items():
                with self.subTest(route="shrine", name=name, tier=tier):
                    self.assertIn({"predicate_ref": "compatible_item"}, expr["all_of"])
                    events = self.leaves(expr)
                    self.assertEqual(self.granted(expr, events, name, ["Venom"]), name not in expected)
                    unrelated = "Void" if name != "Void" else "Vampirism"
                    self.assertTrue(self.granted(expr, events, name, [unrelated]))
                    self.assertFalse(self.granted(expr, events, name, [name]))
            expr = family["scroll_apply"]
            self.assertIn({"predicate_ref": "compatible_item"}, expr["all_of"])
            events = self.leaves(expr)
            self.assertEqual(self.granted(expr, events, name, ["Venom"]), name not in expected)
            unrelated = "Void" if name != "Void" else "Vampirism"
            self.assertTrue(self.granted(expr, events, name, [unrelated]))
            self.assertFalse(self.granted(expr, events, name, [name]))

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

    def test_blank_npc_purchase_requires_world_character_and_premium_gates(self):
        purchase = self.packet["scroll_acquisition"]["blank_npc_purchase"]
        events = self.leaves(purchase)
        self.assertEqual(len(events), 3)
        self.assertTrue(self.granted(purchase, events))
        for event in events:
            self.assertFalse(self.granted(purchase, events - {event}))
        self.assertEqual(purchase["npc_source_name"], "Albinius")
        self.assertEqual(purchase["gold"], 25000)

    def test_etcher_purchase_qualification_and_free_application_are_separate(self):
        purchase = self.packet["utility_acquisition"]["etcher_npc_purchase"]
        worthy = self.leaves(self.definitions["worthy_character"])
        self.assertEqual(worthy, self.leaves(self.definitions["shrine_access"]))
        self.assertTrue(self.granted(purchase, worthy))
        for event in worthy:
            self.assertFalse(self.granted(purchase, worthy - {event}))
        self.assertEqual(purchase["gold"], 30000)
        self.assertIsNone(purchase["premium_requirement"]["value"])
        self.assertFalse(worthy & self.leaves(self.definitions["premium"]))
        application = self.packet["utility_application"]["etcher"]
        self.assertFalse(application["premium_required"])
        self.assertEqual(application["item_ref"], purchase["item_ref"])

    def test_frozen_horror_report_opens_a_distinct_shrine_area(self):
        branch = self.definitions["forgotten_melting_frozen_horror"]
        kill, report, path_opened, shrine = branch["all_of"]
        self.assertEqual(kill["boss_source_name"], "Melting Frozen Horror")
        self.assertEqual(report["npc_source_name"], "A Dragon Mother")
        self.assertEqual(report["dialogue_keywords"], ["hi"])
        self.assertEqual(path_opened["type"], "quest_path_opened_after_npc_report")
        self.assertEqual(shrine["access_portal_source_position"], {"x": 32247, "y": 31030, "z": 12})
        self.assertNotIn("reward room", shrine["location_source_name"])
        without_report = self.leaves(kill) | self.leaves(path_opened) | self.leaves(shrine)
        for name in ("Blockade", "Frost", "Quara Scale"):
            self.assertFalse(self.granted(self.families[name]["powerful_unlock"], without_report))

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

    def test_completed_scroll_loot_has_individual_evidence_for_every_item(self):
        loot = self.packet["scroll_acquisition"]["loot"]
        self.assertTrue(loot["blank_scroll"])
        self.assertTrue(loot["basic_scroll"])
        for item in ("intricate_scroll", "powerful_scroll"):
            self.assertIs(loot[item], False)
            self.assertEqual(loot["evidence_status_by_item"][item], "COMMUNITY_EXPLICIT_ALL_24_ITEM_PAGES")
        self.assertNotIn("official_manual", loot["source_refs"])
        self.assertEqual(loot["scope"], "MONSTER_DROPS_IN_EXPLICIT_COMMUNITY_ITEM_RECORDS")
        rows = loot["completed_scroll_item_observations"]
        self.assertEqual(len(rows), 48)
        self.assertEqual({(r["family_name"], r["tier"]) for r in rows},
                         {(name, tier) for name in self.families for tier in ("intricate", "powerful")})
        for row in rows:
            self.assertEqual(row["named_monster_drop_sources"], [])
            self.assertEqual(row["evidence_status"], "COMMUNITY_EXPLICIT_ITEM_PAGE")
            self.assertEqual(row["item_source_name"], f'{row["tier"].title()} {row["family_name"]} Scroll')
            self.assertEqual(row["source_url"], "https://tibiopedia.pl/items/" + row["item_source_name"].replace(" ", "_"))
            self.assertTrue(row["free_account_application_explicit"])
            self.assertRegex(row["source_sha256"], r"^[a-f0-9]{64}$")
        # Empty monster-drop lists must not disable the independently documented crafting path.
        for family in self.families.values():
            for tier in ("intricate", "powerful"):
                expr = family["scroll_inscription"][tier]
                self.assertTrue(self.granted(expr, self.leaves(expr)))

    def test_blank_scroll_named_drops_have_a_captured_revision_not_a_hashless_snippet(self):
        record = self.packet["scroll_acquisition"]["loot"]["blank_named_monster_drop_sources"]
        source = self.packet["sources"]["br_blank_scroll"]
        self.assertEqual(record["source_refs"], ["br_blank_scroll"])
        self.assertEqual(record["capture_profile"], "sources.br_blank_scroll")
        self.assertEqual(source["access_status"], "FULL_ARTICLE_BROWSER_READ")
        self.assertEqual(source["revision_id"], record["source_revision_id"])
        self.assertEqual(source["revision_id"], 428617)
        self.assertEqual(source["revision_timestamp"], "2025-08-23T10:16:39Z")
        self.assertIn("revids=428617", source["revision_metadata_url"])
        self.assertRegex(source["source_sha256"], r"^[a-f0-9]{64}$")
        self.assertIn("Complete UTF-8", source["digest_scope"])
        self.assertIn("RENDERED_TEMPLATES_CURRENT", source["target_continuity"])
        self.assertIn("NOT_EXACT_TARGET_SERVER_CERTIFICATION", source["target_continuity"])
        heading, values = source["selected_quote"].split("\t", 1)
        self.assertEqual(heading, "Loot de:")
        names = values.removesuffix(".").split(", ")
        self.assertEqual(record["names"], names)
        self.assertEqual(len(names), 6)

    def test_yana_nine_material_bundles_are_cumulative_and_do_not_imbue_items(self):
        exchange = self.packet["material_acquisition"]["yana_gold_token_exchange"]
        rows = exchange["recipes"]
        self.assertEqual(len(rows), 9)
        self.assertEqual({(r["family_name"], r["tier"]) for r in rows},
                         {(name, tier) for name in ("Strike", "Vampirism", "Void")
                          for tier in ("basic", "intricate", "powerful")})
        self.assertFalse(exchange["exchange_imbues_item"])
        self.assertFalse(exchange["exchange_pays_shrine_fee"])
        indexed = {(r["family_name"], r["tier"]): r for r in rows}
        for name in ("Strike", "Vampirism", "Void"):
            previous = []
            for index, tier in enumerate(("basic", "intricate", "powerful"), 1):
                row = indexed[name, tier]
                self.assertEqual(row["inputs"], [{"source_name": "Gold Token", "count": index * 2,
                                                 "item_ref": {"family": "Item", "key": "oteryn:item.tibia.i22721",
                                                              "revision": "definition-r1"}}])
                self.assertEqual(len(row["outputs"]), index)
                self.assertEqual(row["outputs"][:index - 1], previous)
                self.assertTrue(all(r["count"] > 0 and r["item_ref"]["family"] == "Item"
                                    for r in row["outputs"]))
                self.assertEqual(row["dialogue_keywords"], ["hi", "trade", name.lower(), tier, "yes"])
                previous = row["outputs"]


if __name__ == "__main__":
    unittest.main()
