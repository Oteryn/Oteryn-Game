import copy
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import explain  # noqa: E402
import registry  # noqa: E402

GAME = registry.load_json(registry.ROOT / registry.GAME_PATH)
PROTOCOL = registry.load_json(registry.ROOT / registry.PROTOCOL_PATH)


def check(game=None, protocol=None, base_game=None, base_protocol=None):
    return registry.validate_files(
        game or copy.deepcopy(GAME),
        protocol or copy.deepcopy(PROTOCOL),
        base_game,
        base_protocol,
    )


class RegistryTests(unittest.TestCase):
    def test_committed_registries_are_valid(self):
        self.assertEqual(check(), [])

    def test_seed_codes_cover_the_required_blocks(self):
        numbers = {e["code"] for e in GAME["codes"]}
        for expected in (2001, 2013, 3001, 3009, 5001, 5007, 6001, 6006):
            self.assertIn(expected, numbers)
        self.assertEqual(
            [e["code"] for e in GAME["codes"] if 6000 <= e["code"] < 7000], list(range(6001, 6007))
        )

    def test_number_outside_game_block_is_refused(self):
        game = copy.deepcopy(GAME)
        game["codes"][0]["code"] = 1500
        self.assertTrue(any("Game-registry block" in e for e in check(game=game)))

    def test_duplicate_number_and_cross_registry_name_are_refused(self):
        game = copy.deepcopy(GAME)
        game["codes"].append(dict(game["codes"][0], name="OTHER_NAME"))
        self.assertTrue(any("duplicated" in e for e in check(game=game)))
        game = copy.deepcopy(GAME)
        game["codes"][0]["name"] = "MALFORMED_FRAME"
        self.assertTrue(any("already registered" in e for e in check(game=game)))

    def test_bad_category_progression_status_and_hint(self):
        for key, value in (
            ("category", "NOPE"),
            ("progression", "NOPE"),
            ("status", "GONE"),
            ("hint", "x" * 161),
        ):
            game = copy.deepcopy(GAME)
            game["codes"][0][key] = value
            self.assertTrue(check(game=game), key)

    def test_append_only_and_semantic_edits(self):
        base = copy.deepcopy(GAME)
        cur = copy.deepcopy(GAME)
        cur["codes"].append(
            dict(cur["codes"][0], code=2900, name="BRAND_NEW")
        )
        self.assertEqual(check(game=cur, base_game=base), [])
        removed = copy.deepcopy(GAME)
        del removed["codes"][0]
        self.assertTrue(any("removed" in e for e in check(game=removed, base_game=base)))
        renumbered = copy.deepcopy(GAME)
        renumbered["codes"][0]["code"] = 2950
        self.assertTrue(any("renumbered" in e for e in check(game=renumbered, base_game=base)))
        for key, value in (("name", "RENAMED"), ("category", "TIMEOUT"), ("progression", "RETRYABLE")):
            edited = copy.deepcopy(GAME)
            edited["codes"][0][key] = value
            if edited["codes"][0][key] == base["codes"][0][key]:
                continue
            self.assertTrue(
                any(f"{key} changed" in e for e in check(game=edited, base_game=base)), key
            )

    def test_free_edits_and_retirement(self):
        base = copy.deepcopy(GAME)
        cur = copy.deepcopy(GAME)
        cur["codes"][0].update(hint="new hint", owner="x::y", contract="DOC §1")
        cur["codes"][0]["public_class"] = "SESSION"
        cur["codes"][1]["status"] = "RETIRED"
        self.assertEqual(check(game=cur, base_game=base), [])

    def test_public_class_and_status_are_one_way(self):
        base = copy.deepcopy(GAME)
        base["codes"][0]["public_class"] = "SESSION"
        changed = copy.deepcopy(base)
        changed["codes"][0]["public_class"] = "OTHER"
        self.assertTrue(any("public_class" in e for e in check(game=changed, base_game=base)))
        dropped = copy.deepcopy(base)
        del dropped["codes"][0]["public_class"]
        self.assertTrue(any("public_class" in e for e in check(game=dropped, base_game=base)))
        retired = copy.deepcopy(GAME)
        retired["codes"][0]["status"] = "RETIRED"
        revived = copy.deepcopy(GAME)
        self.assertTrue(any("status" in e for e in check(game=revived, base_game=retired)))

    def test_protocol_semantic_edit_is_refused(self):
        base = copy.deepcopy(PROTOCOL)
        cur = copy.deepcopy(PROTOCOL)
        cur["error_codes"][0]["default_disposition"] = "RESYNC_REQUIRED"
        errors = check(protocol=cur, base_protocol=base)
        self.assertTrue(any("default_disposition changed" in e for e in errors))

    def test_protocol_disposition_outside_table_is_refused(self):
        protocol = copy.deepcopy(PROTOCOL)
        protocol["error_codes"][0]["default_disposition"] = "SOMETIMES_FATAL"
        self.assertTrue(any("outside the progression table" in e for e in check(protocol=protocol)))
        protocol["error_codes"][0]["progression"] = "TERMINAL"  # explicit is authoritative
        self.assertEqual(check(protocol=protocol), [])


class ExplainTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.entries = explain.load_entries()

    def test_every_protocol_code_resolves_with_derived_progression(self):
        for raw in PROTOCOL["error_codes"]:
            entry = explain.lookup(self.entries, f"E{raw['code']}")
            self.assertIsNotNone(entry, raw["code"])
            text = explain.format_entry(entry)
            self.assertIn(raw["name"], text)
            if "progression" in raw:
                continue
            progression, retry = registry.DISPOSITION_PROGRESSION[raw["default_disposition"]]
            self.assertIn(f"progression: {progression} (derived from {raw['default_disposition']})", text)
            self.assertIn(retry, text)
        self.assertEqual(
            sum(1 for r in PROTOCOL["error_codes"] if 1001 <= r["code"] <= 1050),
            len(PROTOCOL["error_codes"]),
        )

    def test_lookup_forms(self):
        by_number = explain.lookup(self.entries, "E2007")
        self.assertEqual(by_number, explain.lookup(self.entries, "2007"))
        self.assertEqual(by_number, explain.lookup(self.entries, "boot_bind_failed"))
        self.assertIsNone(explain.lookup(self.entries, "E9999"))

    def test_cli_and_scan(self):
        out = io.StringIO()
        with redirect_stdout(out):
            self.assertEqual(explain.main(["E3004"]), 0)
        self.assertIn("INBOX_ITEM_HAS_CONTENTS", out.getvalue())
        log = (
            "oteryn-game-server ts=1 level=error module=node event=x code=E2001 name=CONFIG_INVALID\n"
            "oteryn-game-server ts=2 level=error module=node event=x code=E2001 name=CONFIG_INVALID\n"
            "oteryn-game-server ts=3 level=error module=node event=x code=E4242\n"
        )
        lines = explain.scan(self.entries, log)
        self.assertTrue(lines[0].lstrip().startswith("2  E2001 CONFIG_INVALID"))
        self.assertIn("E4242 (unregistered)", lines[1])


if __name__ == "__main__":
    unittest.main()
