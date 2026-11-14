"""World-bundle runs must issue and activate the full, staged item index."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
RUN = (HERE / "run.sh").read_text()


def section(start, end):
    return RUN[RUN.index(start):RUN.index(end, RUN.index(start))]


class WorldBundleTests(unittest.TestCase):
    def test_bundle_readiness_declares_node_boot_capabilities(self):
        configure = re.search(r"(?ms)^write_node_config\(\) \{.*?^}", RUN).group(0)
        node_boot = (HERE.parent / "node_boot/run.sh").read_text()
        expected = re.search(r"NODE_REQUIRED_CAPABILITIES='(\[11,18,21,30\])'", node_boot).group(1)
        for mode in ("0", "1"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(dir=HERE) as directory:
                work = Path(directory)
                pin = work / "pin.json"
                pin.write_bytes((REPO / "content/world/pins/oteryn.json").read_bytes())
                script = r'''set -Eeuo pipefail
BASE=/staged
LL_HOST=127.0.0.1
GAME_PORT=17181
WORLD_ID=world
CHANNEL_ID=channel
PG_PORT=15433
ROUTE_REVISION=route-1
WP5_PORT=18463
DESCRIPTOR_INSTALLED_AT=1
PROJECTION_AUTHORITY=platform
WB_DIGEST=bundle-digest
sudo() { return 0; }
''' + configure + '\nwrite_node_config\n'
                result = subprocess.run(["bash", "-c", script], capture_output=True, text=True,
                                        env={**os.environ, "WORK": str(work), "PIN": str(pin),
                                             "LOGIN_LOCAL_WORLD_BUNDLE": mode}, timeout=10, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                config = tomllib.loads((work / "node.toml").read_text())
                self.assertEqual(config["readiness"]["required_gameplay_capabilities"],
                                 json.loads(expected) if mode == "1" else [])
                self.assertEqual(config["readiness"]["map_revision"],
                                 "bundle-digest" if mode == "1" else "map-ll-1")

    def test_activation_and_readiness_have_separate_node_boot_budgets(self):
        setup = section('GAME_SOURCE="$(git rev-parse --show-toplevel)"', 'PLATFORM_SOURCE=')
        setup = setup.replace('GAME_SOURCE="$(git rev-parse --show-toplevel)"', '')
        waits = section("grep -q 'report=ReportScopeAssignmentV1'",
                        'evidence "node assigned=operator')
        for mode, budget in (("0", 60), ("1", 180)):
            with self.subTest(mode=mode):
                script = r'''set -Eeuo pipefail
WORK=.
WB_DIGEST=d
grep() { return 0; }
evidence() { :; }
await_log() { printf '%s|%s\n' "$1" "$2"; }
''' + setup + waits
                result = subprocess.run(["bash", "-c", script], capture_output=True, text=True,
                                        env={**os.environ, "GAME_SOURCE": str(REPO),
                                             "LOGIN_LOCAL_WORLD_BUNDLE": mode}, timeout=10, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                bundle = [f"event=world_bundle .*state=booted map_revision=sha256:d|{budget}"] if mode == "1" else []
                self.assertEqual(result.stdout.splitlines(), [
                    f"event=content_activated .*activation_sequence=1 |{budget}",
                    *bundle,
                    "event=readiness .*ready=true|60",
                ])

    def test_world_bundle_stages_the_canonical_gameplay_inputs(self):
        block = section('if [[ "${LOGIN_LOCAL_WORLD_BUNDLE:-0}" == 1 ]]; then',
                        'sudo install -o root -m 0755 "$TARGET/')
        with tempfile.TemporaryDirectory(dir=HERE) as directory:
            root = Path(directory)
            work, base = root / "work", root / "base"
            work.mkdir()
            base.mkdir()
            # Run only the bundle block, with no compiler, privilege or service access.
            script = r'''set -Eeuo pipefail
cargo() { return 0; }
sudo() {
  local args=()
  while (( $# )); do
    case "$1" in -o|-g) shift 2 ;; *) args+=("$1"); shift ;; esac
  done
  # The compiler is stubbed; only its output installation is omitted.
  [[ "${args[*]}" != *".otwb "* ]] || return 0
  "${args[@]}"
}
LL_GAMEPLAY_ENV=()
''' + block + r'''
printf '%s\n' "${LL_GAMEPLAY_ENV[@]}"
'''
            result = subprocess.run(["bash", "-c", script], capture_output=True, text=True,
                                    env={**os.environ, "GAME_SOURCE": str(REPO), "WORK": str(work),
                                         "BASE": str(base), "LOGIN_LOCAL_WORLD_BUNDLE": "1"},
                                    timeout=60, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            manifest = base / "gameplay/content/spells.manifest.json"
            self.assertEqual(result.stdout.strip(), f"OTERYN_NATIVE_GAMEPLAY_MANIFEST={manifest}")
            self.assertTrue(manifest.is_file(), "bundle boot still uses the fixture item index")
            document = json.loads(manifest.read_bytes())
            self.assertIn("item_keys", document)
            self.assertIn("item_profiles", document)
            for pin in document.values():
                if isinstance(pin, dict) and "path" in pin and "sha256" in pin:
                    data = (manifest.parent / pin["path"]).read_bytes()
                    self.assertEqual(hashlib.sha256(data).hexdigest(), pin["sha256"])
            # Check the additional cosmetics closure as well as top-level pins.
            descriptor_path = manifest.parent / document["cosmetics"]["path"]
            descriptor = json.loads(descriptor_path.read_bytes())
            for name in ("authoring", "source_manifest"):
                pin = descriptor[name]
                data = (descriptor_path.parent / pin["path"]).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), pin["sha256"])
            source = descriptor_path.parent / descriptor["source_manifest"]["path"]
            for pin in json.loads(source.read_bytes())["files"]:
                data = (source.parent / pin["path"]).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), pin["sha256"])

    def test_operator_and_server_receive_the_same_manifest_in_both_rust_modes(self):
        ops = re.search(r"(?ms)^ops\(\) \{.*?^}", RUN).group(0)
        start = section('if [[ "$LL_RUST" == host ]]; then\n  sudo -u', 'NODE_PID=$!')
        start = start.replace('> /dev/null 2> "$node_log"', '')
        for mode in ("host", "container"):
            with self.subTest(mode=mode):
                script = r'''set -Eeuo pipefail
BASE=/staged
SERVICE_USER=test-node
SERVICE_UID=123
RUST_CONTAINER=test-container
WORK=.
node_log=/dev/null
LL_GAMEPLAY_ENV=(OTERYN_NATIVE_GAMEPLAY_MANIFEST=/staged/gameplay/content/spells.manifest.json)
sudo() { printf '%s\n' "$*"; }
docker() { printf '%s\n' "$*"; }
''' + ops + '\nops content activate\n' + start + '\nwait\n'
                result = subprocess.run(["bash", "-c", script], capture_output=True, text=True,
                                        env={**os.environ, "LL_RUST": mode}, timeout=10, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                commands = result.stdout.splitlines()
                self.assertEqual(len(commands), 2)
                for command in commands:
                    self.assertIn("OTERYN_NATIVE_GAMEPLAY_MANIFEST=/staged/gameplay/content/spells.manifest.json",
                                  command)


if __name__ == "__main__":
    unittest.main()
