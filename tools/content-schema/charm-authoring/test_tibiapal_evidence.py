#!/usr/bin/env python3
"""Offline captured-source regressions; optional rerun against the pinned checkout."""

from __future__ import annotations

import copy
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[2]
SAMPLES = ROOT / "samples"
DESCRIPTION = SAMPLES / "tibiapal-description-calculator-evidence-2026-10-01.json"
EXECUTION = SAMPLES / "tibiapal-planner-execution-2026-10-01.json"
HARNESS = SAMPLES / "test-tibiapal-planner.cjs"
BROWSER = SAMPLES / "tibiapal-browser-verification-2026-10-01.json"
REVISION = "61ffa3e0502879ccec44e59ead859e92b6d88531"
SOURCE_HASHES = {
    "scripts/charm_planner.js": "18de343c48b73bbd37e21dc2aae3e7afb17b3c7b3b734f4f9768062307b13b8e",
    "scripts/charm_calculator.js": "04a94974bea35657987085fcc8eaa08060d5478b5a744a0993304a590c4b9811",
    "charm_calculator.html": "80037c53add5e89355257484687e4023c116b0c84dc53d032c13139e17b1a363",
}
FIXTURE_HASHES = {
    BROWSER: "1d31aec01601cada6b7264d4c9d2a5289e2888cfd4e6f9e0490b710503f7d5c7",
    DESCRIPTION: "8ce1fddc43dc621b17c081bb131468713b2405f8823b2613760a1be858de4fe4",
    EXECUTION: "1c23857ea18be077019cc21ae293a2c74c0c440251c520a9847c545f0c4340fb",
    HARNESS: "62dd042099168250eb230fb75b0673d8de40870715a74d573b86cd51e9bd9a9e",
}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def catalogue():
    return {
        r["definition"]["identity"]["key"]: r["definition"]
        for r in load(REPO / "content/charms/charms-00000-00024.json")["records"]
    }


def check_source(source):
    assert source["repository"] == "PawelKusnierek/TibiaPal"
    assert source["revision"] == REVISION
    assert source["sha256"] == SOURCE_HASHES[source["path"]]


def check_descriptions(audit):
    assert audit["schema"] == "OTERYN_TIBIAPAL_DESCRIPTION_AUDIT/v1"
    assert audit["live_site_test"] is False
    assert len(audit["sources"]) == 3
    for source in audit["sources"]:
        check_source(source)
    assert audit["description_count"] == 25
    assert audit["all_description_values_and_costs_match"] is True
    assert audit["description_mismatches"] == []
    rows = audit["rows"]
    definitions = catalogue()
    assert len(rows) == 25 and {r["key"] for r in rows} == set(definitions)
    for row in rows:
        definition = definitions[row["key"]]
        values = [s["value"] for s in definition["stages"]]
        assert row["catalogue_values"] == values
        assert row["catalogue_costs"] == [s["cost"] for s in definition["stages"]]
        asserted = row["asserted_semantics"]
        assert asserted.get("proc_values", asserted.get("bonus_values")) == values
        if "duration_ms" in asserted:
            assert asserted["duration_ms"] == definition["effect"]["duration_ms"]
        for field in ["numeric_match", "cost_match", "duration_match"]:
            assert row[field] is True
        assert 19 <= row["planner_source_line"] <= 43


def check_execution(report):
    assert report["schema"] == "OTERYN_TIBIAPAL_PLANNER_EXECUTION/v1"
    check_source(report["source"])
    assert report["runtime"]["real_browser"] is False
    results = report["results"]
    assert len(results) == 412
    ids = {r["id"]: r for r in results}
    assert len(ids) == 412
    assert report["summary"]["cases"] == report["summary"]["passed"] == 412
    assert report["summary"]["failed"] == 0
    for result in results:
        assert result["pass"] is True and result["actual"] == result["expected"]
    definitions = catalogue()
    for key, definition in definitions.items():
        name = key.split(".")[-1]
        assert ids[name + "/costs_vs_content"]["actual"] == [
            s["cost"] for s in definition["stages"]
        ]
        for stage in [1, 2, 3]:
            assert ids[f"{name}/advance{stage}"]["actual"]["stage"] == stage
            assert ids[f"{name}/revert{stage}"]["actual"]["stage"] == stage - 1
            if definition["category"] == "major":
                assert ids[f"{name}/exact_max_stage{stage}"]["actual"] == stage
                assert ids[f"{name}/insufficient_error_stage{stage}"]["actual"] is True
    assert report["summary"]["major_full_cycle_count"] == 14
    assert report["summary"]["minor_full_cycle_count"] == 11
    assert report["summary"]["major_exact_and_insufficient_stage_pairs"] == 42
    assert ids["all_major_echo_budget"]["actual"] == 5000
    assert ids["5000_echo_budget_exhausted"]["actual"] == [5000, 0]
    assert ids["all25_shortfall_message"]["actual"] is True
    assert ids["progression_unpromoted_initial_separate"]["actual"] == 0
    assert ids["reset_balances"]["actual"] == [0, 0, 100]


def js_positive_round(value):
    return math.floor(value + Fraction(1, 2))


def check_browser(report):
    assert report["schema"] == "OTERYN_TIBIAPAL_BROWSER_VERIFICATION/v1"
    assert report["scope"] == "PINNED_LOCAL_SITE" and report["engine"] == "Chromium"
    assert report["live_domain_verified"] is False
    assert report["source_revision"] == REVISION
    assert (
        report["source_hashes"]["scripts/charm_planner.js"]
        == SOURCE_HASHES["scripts/charm_planner.js"]
    )
    assert (
        report["source_hashes"]["scripts/charm_calculator.js"]
        == SOURCE_HASHES["scripts/charm_calculator.js"]
    )
    for digest in report["source_hashes"].values():
        assert re.fullmatch(r"[a-f0-9]{64}", digest)
    assert report["page_errors"] == [] and report["reset_verified"] is True
    assert report["minor_budget_and_refund_guard"] is True
    definitions = catalogue()
    assert len(report["planner_charms"]) == 25
    assert {r["key"] for r in report["planner_charms"]} == set(definitions)
    for row in report["planner_charms"]:
        definition = definitions[row["key"]]
        assert row["category"] == definition["category"]
        assert row["costs"] == [s["cost"] for s in definition["stages"]]
        assert row["values"] == [s["value"] for s in definition["stages"]]
        assert row["icon_loaded"] is True and row["stage_bounds_verified"] is True
        assert re.fullmatch(r"[a-f0-9]{64}", row["description_sha256"])
    transitions = report["planner_transitions"]
    assert len(transitions) == 150
    for key, definition in definitions.items():
        rows = [r for r in transitions if r["key"] == key.split(".")[-1]]
        assert [r["stage"] for r in rows] == [1, 2, 3, 2, 1, 0]
        assert [r["direction"] for r in rows] == ["buy"] * 3 + ["refund"] * 3
        costs = [s["cost"] for s in definition["stages"]]
        for row in rows:
            spent = sum(costs[: row["stage"]])
            if definition["category"] == "major":
                assert row["major"] == spent and row["minor"] == 0
                assert row["echoes"] == 100 + sum([50, 100, 200][: row["stage"]])
            else:
                assert row["minor"] == spent and row["major"] == 3600
                assert row["echoes"] == 800 - spent
    boundaries = report["major_budget_boundaries"]
    assert len(boundaries) == 42
    assert len({(r["key"], r["stage"]) for r in boundaries}) == 42
    for row in boundaries:
        definition = definitions["oteryn:charm." + row["key"]]
        assert definition["category"] == "major"
        assert row["budget"] == sum(
            s["cost"] for s in definition["stages"][: row["stage"]]
        )
        assert row["exact_accepted"] is True and row["one_short_rejected"] is True
    cases = report["calculator_cases"]
    assert len(cases) == 66
    for row in cases:
        multiplier = (
            Fraction(5, 100) if row["charm"] == "Overpower" else Fraction(25, 1000)
        )
        raw = row["resource"] * multiplier
        cap = Fraction(row["creature_max_health"]) * Fraction(8, 100)
        elemental = (
            Fraction(row["creature_max_health"])
            * Fraction(5, 100)
            * Fraction(row["sensitivity_percent"], 100)
        )
        assert row["browser_damage"] == min(
            js_positive_round(raw), js_positive_round(cap)
        )
        assert row["browser_elemental_damage"] == js_positive_round(elemental)
        assert row["floor_resource_damage"] == min(math.floor(raw), math.floor(cap))
        assert row["floor_elemental_after_scaling"] == math.floor(elemental)


class TibiaPalEvidenceTests(unittest.TestCase):
    def test_fixture_provenance_and_25_description_triples(self):
        for path, expected in FIXTURE_HASHES.items():
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), expected)
        check_descriptions(load(DESCRIPTION))

    def test_412_executed_planner_checks_and_catalogue_cycles(self):
        check_execution(load(EXECUTION))

    def test_calculator_rounding_caps_and_runtime_difference(self):
        audit = load(DESCRIPTION)
        self.assertEqual(len(audit["actual_node_vm_cases"]), 7)
        for case in audit["actual_node_vm_cases"]:
            values = case["input"]
            multiplier = (
                Fraction(5, 100)
                if values["type"] == "Overpower"
                else Fraction(25, 1000)
            )
            resource = Fraction(values["resource"]) * multiplier
            cap = Fraction(values["hp"]) * Fraction(8, 100)
            sensitivity = Fraction(values["resistance"].rstrip("%")) / 100
            elemental = Fraction(values["hp"]) * Fraction(5, 100) * sensitivity
            observed = case["observed_round"]
            self.assertEqual(
                observed,
                {
                    "resource": js_positive_round(resource),
                    "cap": js_positive_round(cap),
                    "elemental": js_positive_round(elemental),
                },
            )
            self.assertEqual(
                case["rendered_damage"],
                {
                    "resource": min(observed["resource"], observed["cap"]),
                    "elemental": observed["elemental"],
                },
            )
            self.assertEqual(
                case["current_runtime_unmitigated_floor"]["resource_damage"],
                min(math.floor(resource), math.floor(cap)),
            )
        first = audit["actual_node_vm_cases"][0]
        self.assertEqual(first["rendered_damage"]["resource"], 341)
        self.assertEqual(
            first["current_runtime_unmitigated_floor"]["resource_damage"], 340
        )
        self.assertEqual(
            first["elemental_at_example_level_100"],
            {"tibiapal": 631, "oteryn_level_capped_base": 200},
        )
        semantics = audit["calculator_semantics"]
        for field in [
            "no_player_level_input",
            "no_elemental_level_cap",
            "no_armor_or_monster_mitigation_input",
            "no_trigger_chance_or_expected_dps_calculation",
        ]:
            self.assertTrue(semantics[field])
        self.assertEqual(len(semantics["unsupported_effects"]), 16)
        self.assertTrue(
            any(
                f.get("official_rounding", "").startswith("UNKNOWN")
                for f in audit["runtime_comparison_findings"]
            )
        )

    def test_fixture_cannot_promote_live_site_or_wrong_source(self):
        audit = load(DESCRIPTION)
        audit["live_site_test"] = True
        with self.assertRaises(AssertionError):
            check_descriptions(audit)
        audit = load(DESCRIPTION)
        audit["sources"][0]["revision"] = "0" * 40
        with self.assertRaises(AssertionError):
            check_descriptions(audit)

    def test_missing_charm_wrong_triple_and_forged_pass_rejected(self):
        audit = load(DESCRIPTION)
        for mutation in ["missing", "triple"]:
            altered = copy.deepcopy(audit)
            if mutation == "missing":
                altered["rows"].pop()
            else:
                altered["rows"][0]["catalogue_values"][0] += 1
            with self.assertRaises(AssertionError):
                check_descriptions(altered)
        report = load(EXECUTION)
        report["results"][0]["actual"] = "forged result despite pass=true"
        with self.assertRaises(AssertionError):
            check_execution(report)

    def test_captured_chromium_25_icons_150_transitions_42_boundaries_66_calculations(
        self,
    ):
        check_browser(load(BROWSER))

    def test_browser_missing_transition_and_false_live_promotion_rejected(self):
        report = load(BROWSER)
        report["planner_transitions"].pop()
        with self.assertRaises(AssertionError):
            check_browser(report)
        report = load(BROWSER)
        report["live_domain_verified"] = True
        with self.assertRaises(AssertionError):
            check_browser(report)

    def test_browser_wrong_rounding_and_missing_levelcap_not_official_truth(self):
        report = load(BROWSER)
        report["calculator_cases"][0]["browser_damage"] = 340
        with self.assertRaises(AssertionError):
            check_browser(report)
        self.assertTrue(
            any("official rounding unverified" in v for v in report["limitations"])
        )
        self.assertTrue(any("no level cap" in v for v in report["limitations"]))

    def test_optional_actual_source_harness(self):
        checkout = os.environ.get("OTERYN_TIBIAPAL_CHECKOUT")
        if not checkout:
            return  # Offline CI checks captured evidence; source checkout is opt-in.
        self.assertTrue(shutil.which("node"))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "execution.json"
            result = subprocess.run(
                ["node", str(HARNESS), checkout, str(REPO), str(output)],
                text=True,
                capture_output=True,
                timeout=60,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            check_execution(load(output))
            self.assertRegex(
                load(output)["runtime"]["test_harness_sha256"],
                re.compile(r"^[0-9a-f]{64}$"),
            )


def run_suite():
    result = unittest.TextTestRunner(verbosity=1).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(TibiaPalEvidenceTests)
    )
    if not result.wasSuccessful():
        raise AssertionError("TibiaPal captured-source evidence regressions failed")
    return result.testsRun


if __name__ == "__main__":
    run_suite()
