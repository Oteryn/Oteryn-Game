#!/usr/bin/env python3
"""Exercise the pinned TibiaPal site in Chromium; capture observations, not battle parity."""

from __future__ import annotations

import argparse
import functools
import hashlib
import http.server
import math
import re
import subprocess
import threading
from fractions import Fraction
from pathlib import Path

import charm_mechanics as cm

REVISION = "61ffa3e0502879ccec44e59ead859e92b6d88531"
SCRIPT_HASHES = {
    "charm_planner.js": "18de343c48b73bbd37e21dc2aae3e7afb17b3c7b3b734f4f9768062307b13b8e",
    "charm_calculator.js": "04a94974bea35657987085fcc8eaa08060d5478b5a744a0993304a590c4b9811",
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class SiteHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path.split("?")[0] in {"/charm_planner", "/charm_calculator"}:
            self.path += ".html"
        super().do_GET()

    def log_message(self, *_args):
        pass


def browser_verify(checkout: Path, screenshots: Path, chromium: str) -> dict:
    from playwright.sync_api import sync_playwright

    actual_revision = subprocess.check_output(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True
    ).strip()
    assert actual_revision == REVISION, "TibiaPal checkout drift"
    assert not subprocess.check_output(
        ["git", "-C", str(checkout), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ).strip(), "TibiaPal tracked files must match the pinned commit"
    inputs = {}
    for filename, expected in SCRIPT_HASHES.items():
        assert sha(checkout / "scripts" / filename) == expected, filename
        assert sha(checkout / "_site/scripts" / filename) == expected, filename
        inputs["scripts/" + filename] = expected
    for filename in [
        "charm_planner.html",
        "charm_calculator.html",
        "scripts/onload.js",
    ]:
        inputs["_site/" + filename] = sha(checkout / "_site" / filename)

    catalogue = cm.load(cm.CATALOGUE)
    definitions = [r["definition"] for r in catalogue["records"]]
    rows, transitions, boundaries, calculations, errors = [], [], [], [], []
    screenshots.mkdir(parents=True, exist_ok=True)
    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", 0),
        functools.partial(SiteHandler, directory=str(checkout / "_site")),
    )
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    origin = f"http://127.0.0.1:{server.server_port}"
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                executable_path=chromium, headless=True, args=["--no-sandbox"]
            )
            page = browser.new_page(viewport={"width": 1440, "height": 1000})
            page.route(
                "**/*",
                lambda route: (
                    route.continue_()
                    if route.request.url.startswith(origin + "/")
                    else route.abort()
                ),
            )
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.goto(origin + "/charm_planner", wait_until="networkidle")

            def tab(category):
                page.get_by_role(
                    "button",
                    name="Major Charms" if category == "major" else "Minor Charms",
                ).click()

            def state(key):
                return page.evaluate(
                    "key => ({stage: major_charm_state_dict[key] ?? minor_charm_state_dict[key],"
                    "major: total_major_charm_points, minor: total_minor_charm_points,"
                    "echoes: total_minor_charm_points_available})",
                    key,
                )

            def reset():
                page.locator("#reset_charms").click()
                page.locator("#max_charm_points").fill("")

            for definition in definitions:
                key = definition["identity"]["key"].split(".")[-1]
                category = definition["category"]
                reset()
                if category == "minor":
                    tab("major")
                    for seed in ["wound", "poison"]:
                        for _ in range(3):
                            page.locator("#" + seed).click()
                tab(category)
                card = page.locator("#" + key)
                card.hover()
                description = page.locator("#charm_description").inner_text()
                triple = [
                    float(n)
                    for n in re.search(
                        r"(\d+(?:\.\d+)?)/(\d+(?:\.\d+)?)/(\d+(?:\.\d+)?)%",
                        description,
                    ).groups()
                ]
                assert triple == [s["value"] for s in definition["stages"]], key
                image = card.locator("img")
                assert image.evaluate("img => img.complete && img.naturalWidth > 0"), (
                    key
                )
                expected_costs = [s["cost"] for s in definition["stages"]]
                displayed_costs = [
                    int(n)
                    for n in re.findall(r"\d+", card.locator("span").inner_text())
                ]
                assert displayed_costs == expected_costs, key
                base = state(key)
                for stage in [1, 2, 3]:
                    card.click()
                    observed = state(key)
                    assert observed["stage"] == stage, key
                    assert (
                        page.locator("#" + key + "_stage").inner_text()
                        == f"Stage: {stage}"
                    )
                    cost = sum(expected_costs[:stage])
                    if category == "major":
                        assert observed["major"] == cost
                        assert observed["echoes"] == 100 + sum([50, 100, 200][:stage])
                    else:
                        assert observed["minor"] == cost
                        assert observed["echoes"] == base["echoes"] - cost
                    transitions.append({"key": key, "direction": "buy", **observed})
                previous = state(key)
                card.click()
                assert state(key) == previous, "fourth stage must not be purchased"
                for stage in [2, 1, 0]:
                    card.click(button="right")
                    observed = state(key)
                    assert observed["stage"] == stage
                    cost = sum(expected_costs[:stage])
                    if category == "major":
                        assert observed["major"] == cost
                        assert observed["echoes"] == 100 + sum([50, 100, 200][:stage])
                    else:
                        assert observed["minor"] == cost
                        assert observed["echoes"] == base["echoes"] - cost
                    transitions.append({"key": key, "direction": "refund", **observed})
                previous = state(key)
                card.click(button="right")
                assert state(key) == previous, "stage cannot become negative"
                rows.append(
                    {
                        "key": definition["identity"]["key"],
                        "category": category,
                        "costs": displayed_costs,
                        "values": triple,
                        "description_sha256": hashlib.sha256(
                            description.encode()
                        ).hexdigest(),
                        "icon_loaded": True,
                        "stage_bounds_verified": True,
                    }
                )

            for definition in [d for d in definitions if d["category"] == "major"]:
                key = definition["identity"]["key"].split(".")[-1]
                for stage in [1, 2, 3]:
                    reset()
                    tab("major")
                    card = page.locator("#" + key)
                    budget = sum(s["cost"] for s in definition["stages"][:stage])
                    page.locator("#max_charm_points").fill(str(budget - 1))
                    for _ in range(stage - 1):
                        card.click()
                    card.click()
                    assert state(key)["stage"] == stage - 1
                    assert (
                        "Not enough Major charm points"
                        in page.locator("#error_message").inner_text()
                    )
                    page.locator("#max_charm_points").fill(str(budget))
                    card.click()
                    assert state(key)["stage"] == stage
                    boundaries.append(
                        {
                            "key": key,
                            "stage": stage,
                            "budget": budget,
                            "one_short_rejected": True,
                            "exact_accepted": True,
                        }
                    )

            reset()
            tab("minor")
            page.locator("#bless").click()
            page.locator("#bless").click()
            assert state("bless") == {"stage": 1, "major": 0, "minor": 100, "echoes": 0}
            assert (
                "Not enough Minor charm points"
                in page.locator("#error_message").inner_text()
            )
            reset()
            tab("major")
            for _ in range(3):
                page.locator("#wound").click()
            tab("minor")
            page.locator("#bless").click()
            page.locator("#bless").click()
            page.locator("#numb").click()
            tab("major")
            previous = state("wound")
            page.locator("#wound").click(button="right")
            assert state("wound") == previous
            assert (
                "Unassign a Minor charm first"
                in page.locator("#error_message").inner_text()
            )
            reset()
            assert state("wound") == {"stage": 0, "major": 0, "minor": 0, "echoes": 100}
            for category in ["major", "minor"]:
                tab(category)
                page.screenshot(
                    path=str(screenshots / f"planner-{category}.png"), full_page=True
                )

            page.goto(origin + "/charm_calculator", wait_until="networkidle")
            cases = [
                (6813, 12600, 100),
                (6821, 12613, 100),
                (11, 19, 50),
                (20, 20, 100),
                (200000, 1000, 110),
                (6800, 12600, 100),
            ]
            cases += [
                (r, h, s)
                for r in [1, 39, 10000]
                for h in [1, 19, 12613]
                for s in [0, 100, 150]
            ]
            for kind in ["Overpower", "Overflux"]:
                page.locator("#charms").select_option(kind)
                label = page.locator("#health_mana_value_label").inner_text()
                assert ("Health" if kind == "Overpower" else "Mana") in label
                for resource, health, sensitivity in cases:
                    page.locator("#health_mana_value").fill(str(resource))
                    page.locator("#monster_health_value").fill(str(health))
                    page.locator("#monster_resistance").fill(f"{sensitivity}%")
                    page.locator("#charms_submit_button").click()
                    rendered = page.locator("#charm_results").inner_text()
                    raw = min(
                        Fraction(resource, 20 if kind == "Overpower" else 40),
                        Fraction(health * 8, 100),
                    )
                    elemental = Fraction(health * 5 * sensitivity, 10000)
                    expected = [
                        math.floor(raw + Fraction(1, 2)),
                        math.floor(elemental + Fraction(1, 2)),
                    ]
                    actual = [int(n) for n in re.findall(r"damage: (\d+)", rendered)]
                    assert actual == expected, (
                        kind,
                        resource,
                        health,
                        sensitivity,
                        rendered,
                    )
                    calculations.append(
                        {
                            "charm": kind,
                            "resource": resource,
                            "creature_max_health": health,
                            "sensitivity_percent": sensitivity,
                            "browser_damage": actual[0],
                            "browser_elemental_damage": actual[1],
                            "floor_resource_damage": math.floor(raw),
                            "floor_elemental_after_scaling": math.floor(elemental),
                        }
                    )
            page.screenshot(path=str(screenshots / "calculator.png"), full_page=True)
            browser_version = browser.version
            browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    assert not errors, errors
    return {
        "schema": "OTERYN_TIBIAPAL_BROWSER_VERIFICATION/v1",
        "source_revision": REVISION,
        "source_hashes": inputs,
        "catalogue_sha256": sha(cm.CATALOGUE),
        "engine": "Chromium",
        "browser_version": browser_version,
        "scope": "PINNED_LOCAL_SITE",
        "live_domain_verified": False,
        "live_domain_access": "ERR_TUNNEL_CONNECTION_FAILED through session proxy",
        "external_assets": "External fonts, ads and stream requests blocked; local charm assets tested",
        "planner_charms": rows,
        "planner_transitions": transitions,
        "major_budget_boundaries": boundaries,
        "minor_budget_and_refund_guard": True,
        "reset_verified": True,
        "calculator_cases": calculations,
        "page_errors": errors,
        "limitations": [
            "Planner descriptions are not executable combat mechanics",
            "Calculator only implements Overpower/Overflux and an elemental comparison",
            "Calculator has no level cap input and computes pre-mitigation",
            "Math.round differs from Oteryn floor arithmetic; official rounding unverified",
            "Exact status/leech/critical/loot/skinning battle behavior is not simulated",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--screenshots", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--chromium", default="/usr/bin/chromium")
    args = parser.parse_args()
    result = browser_verify(args.checkout, args.screenshots, args.chromium)
    args.output.write_text(cm.dumps(result), encoding="utf-8")
    print(
        f"PASS Chromium: {len(result['planner_charms'])} charms; "
        f"{len(result['planner_transitions'])} transitions; "
        f"{len(result['major_budget_boundaries'])} budget boundaries; "
        f"{len(result['calculator_cases'])} calculator cases"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
