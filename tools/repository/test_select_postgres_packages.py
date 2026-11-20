#!/usr/bin/env python3
"""Regression coverage for PostgreSQL package selection on exact Git inputs."""

import contextlib
import json
from pathlib import Path
import runpy
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SELECTOR = Path(__file__).resolve().parents[1] / "ci" / "select_postgres_packages.py"
DISPATCH = "crates/gameplay/dispatch"
TRANSPORT = f"{DISPATCH}/src/gameplay_transport"
SCENARIOS = f"{TRANSPORT}/postgres_tests"
SUPPORT = f"{SCENARIOS}/support"
POTIONS = f"{SCENARIOS}/special_potions.rs"
TELEPORTS = f"{SCENARIOS}/srx3_seven_teleports.rs"
FIXTURE = f"{SUPPORT}/quest_catalogue_chests_fixtures.rs"
CONTAINER = f"{SUPPORT}/quest_container_chests_fixtures.rs"
FORGE = f"{SUPPORT}/forge_preview_fixtures.rs"
PKG = "oteryn-gameplay-dispatch"
SERVER = "oteryn-game-server"


class PostgresScopeTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.context = contextlib.chdir(self.root)
        self.context.__enter__()
        self.addCleanup(self.context.__exit__, None, None, None)
        self.git("init", "-q")
        self.selector = runpy.run_path(str(SELECTOR))["select"]
        self.real_command = self.selector.__globals__["command"]
        self.write(f"{DISPATCH}/build.rs", '''build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests/support",
        "postgres_test_support_mounts.rs",
        "",
    )?;
    build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests",
        "postgres_tests_mounts.rs",
        "",
    )?;
''')
        self.write(f"{TRANSPORT}/monk_save.rs", '''#[cfg(test)]
#[path = "monk_save_postgres_tests.rs"]
pub(super) mod postgres_tests;
''')
        self.write(f"{TRANSPORT}/monk_save_postgres_tests.rs",
                   'include!(concat!(env!("OUT_DIR"), "/postgres_test_support_mounts.rs"));\n'
                   'include!(concat!(env!("OUT_DIR"), "/postgres_tests_mounts.rs"));\n')
        for fixture in (FIXTURE, CONTAINER, FORGE):
            self.write(fixture, "fn fixture() {}\n")
        for scenario in (POTIONS, TELEPORTS):
            self.write(scenario, "#[test]\nfn scenario() {}\n")
        self.metadata = {"workspace_members": [PKG, SERVER], "packages": [
            {"id": PKG, "name": PKG, "manifest_path": str(self.root / DISPATCH / "Cargo.toml"),
             "dependencies": [], "targets": [{"src_path": str(self.root / DISPATCH / "src/lib.rs")}]},
            {"id": SERVER, "name": SERVER, "manifest_path": str(self.root / "apps/game-server/Cargo.toml"),
             "dependencies": [{"path": str(self.root / DISPATCH)}],
             "targets": [{"src_path": str(self.root / "apps/game-server/src/lib.rs")}]},
        ]}

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid", *args],
            text=True,
        ).strip()

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def select(self, base):
        def command(*args):
            if args[0] == "cargo":
                return json.dumps(self.metadata)
            return self.real_command(*args)
        with patch.dict(self.selector.__globals__, command=command):
            return self.selector(base, "HEAD")[0]

    def test_existing_cfg_test_fixtures_select_only_dispatch(self):
        base = self.commit()
        self.write(FIXTURE, "fn fixture() { test_chest(); }\n")
        self.write(CONTAINER, "fn fixture() { test_container(); }\n")
        self.commit()
        self.assertEqual(self.select(base), [PKG])
        base = self.git("rev-parse", "HEAD")
        self.write(FORGE, "fn fixture() { test_forge(); }\n")
        self.commit()
        self.assertEqual(self.select(base), [PKG])

    def test_production_and_build_changes_keep_existing_scope(self):
        base = self.commit()
        self.write(FIXTURE, "fn fixture() { test_chest(); }\n")
        self.write(f"{DISPATCH}/src/lib.rs", "pub fn changed() {}\n")
        self.commit()
        self.assertEqual(self.select(base), [SERVER, PKG])
        self.write("Cargo.lock", "new lockfile\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_existing_cfg_test_scenarios_and_fixtures_select_only_dispatch(self):
        self.write("tools/ci/pg_test_durations.json", '{"special_potions::scenario": 42}\n')
        base = self.commit()
        self.write(POTIONS, "#[test]\nfn scenario() { assert!(ready()); }\n")
        self.write(TELEPORTS, "#[test]\nfn scenario() { assert!(arrived()); }\n")
        self.commit()
        self.assertEqual(self.select(base), [PKG])
        base = self.git("rev-parse", "HEAD")
        self.write(POTIONS, "#[test]\nfn scenario() { assert!(replayed()); }\n")
        self.write(FORGE, "fn fixture() { test_forge(); }\n")
        self.commit()
        self.assertEqual(self.select(base), [PKG])

    def test_scenario_requires_generated_mount(self):
        self.write(f"{TRANSPORT}/monk_save_postgres_tests.rs",
                   'include!(concat!(env!("OUT_DIR"), "/postgres_test_support_mounts.rs"));\n')
        base = self.commit()
        self.write(POTIONS, "#[test]\nfn scenario() { assert!(ready()); }\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_scenario_path_consumer_cannot_be_dropped(self):
        self.write("apps/game-server/src/lib.rs", f'include!("../../../{TELEPORTS}");\n')
        base = self.commit()
        self.write(TELEPORTS, "#[test]\nfn scenario() { assert!(arrived()); }\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_other_consumer_cannot_be_dropped(self):
        self.write("apps/game-server/src/lib.rs", f'include!("../../../{FIXTURE}");\n')
        base = self.commit()
        self.write(FIXTURE, "fn fixture() { test_chest(); }\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_directory_consumer_cannot_be_dropped(self):
        self.write("tools/collect.py", f'path = Path("{SUPPORT}")\n')
        base = self.commit()
        self.write(FORGE, "fn fixture() { test_forge(); }\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_loss_of_cfg_test_mount_proof_selects_full(self):
        self.write(f"{TRANSPORT}/monk_save.rs", '''#[path = "monk_save_postgres_tests.rs"]
pub(super) mod postgres_tests;
''')
        base = self.commit()
        self.write(FORGE, "fn fixture() { test_forge(); }\n")
        self.commit()
        self.assertIsNone(self.select(base))

    def test_additions_and_symlinks_select_full(self):
        base = self.commit()
        self.write(f"{SUPPORT}/new_fixtures.rs", "fn new_fixture() {}\n")
        self.commit()
        self.assertIsNone(self.select(base))
        base = self.git("rev-parse", "HEAD")
        fixture = self.root / FIXTURE
        fixture.unlink()
        fixture.symlink_to(Path(FORGE).name)
        self.commit()
        self.assertIsNone(self.select(base))

    def test_explicit_cargo_target_consumer_selects_full(self):
        base = self.commit()
        self.write(FORGE, "fn fixture() { test_forge(); }\n")
        self.commit()
        self.metadata["packages"][1]["targets"].append({"src_path": str(self.root / FORGE)})
        self.assertIsNone(self.select(base))


if __name__ == "__main__":
    unittest.main()
