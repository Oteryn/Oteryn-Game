"""Qualify evidence content against the existing native Quest loader/evaluator.

Creates a small temporary Cargo project outside the repository. No runtime, served
content, catalogue pin or QuestState is modified. Requires cached pinned Rust dependencies.
"""
import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]


def qualify(output, adversarial):
    # Cargo accepts TOML 1.1 multiline inline tables; Python 3.12 tomllib does not.
    # Read only these three exact dependency declarations; fail on layout drift.
    manifest_text = (REPO / "Cargo.toml").read_text(encoding="utf-8")
    def version(name, inline=False):
        pattern = rf'^{name}\s*=\s*' + (r'\{\s*version\s*=\s*' if inline else '') + r'"(=[^"]+)"'
        matches = re.findall(pattern, manifest_text, flags=re.MULTILINE)
        if len(matches) != 1:
            raise SystemExit("Unrecognized dependency declaration: " + name)
        return matches[0]
    serde, serde_json, sha2 = version("serde"), version("serde_json"), version("sha2", True)
    (output / "src").mkdir(parents=True)
    manifest = f'''[package]
name = "shards-q1-offline-qualification"
version = "0.1.0"
edition = "2024"
[dependencies]
serde = {{ version = "{serde}", features = ["derive"] }}
serde_json = "{serde_json}"
sha2 = {{ version = "{sha2}", default-features = false }}
'''
    (output / "Cargo.toml").write_text(manifest, encoding="utf-8")
    quoted = lambda p: json.dumps(p.as_posix())

    def source(fixture):
        return f'''#![allow(dead_code)]
#[path = {quoted(REPO / 'apps/game-server/src/quest/mod.rs')}]
mod quest;
const Q1_FIXTURE: &str = include_str!({quoted(fixture)});
include!({quoted(ROOT / 'q1_catalogue_qualification.rs')});
'''

    fixture = ROOT / "q1-native-catalogue-candidate.json"
    rust = output / "src/lib.rs"
    rust.write_text(source(fixture), encoding="utf-8")
    command = ["cargo", "test", "--offline", "--manifest-path", str(output / "Cargo.toml")]
    subprocess.run(command, check=True)
    if not adversarial:
        return
    mutant = json.loads(fixture.read_text(encoding="utf-8"))
    join = next(t for t in mutant["quests"][0]["transitions"] if t["key"].endswith("/three_clues_join"))
    join["effects"] = [e for e in join["effects"] if not e["track"].endswith("/plants_done")]
    mutant_file = output / "missing-plants-guard.json"
    mutant_file.write_text(json.dumps(mutant), encoding="utf-8")
    rust.write_text(source(mutant_file), encoding="utf-8")
    try:
        result = subprocess.run(command, capture_output=True, text=True)
        (output / "missing-plants-guard.log").write_text(result.stdout + result.stderr, encoding="utf-8")
        required = ["every_incomplete_subset_refuses_join_without_state_changes",
                    "stale_positive_session_copy_does_not_bypass_locked_join"]
        if result.returncode == 0 or not all(name + " ... FAILED" in result.stdout for name in required):
            raise SystemExit("Adversarial check did not reject the missing prerequisite guard")
        print("RED witness: missing plants_done guard is detected by incomplete-subset and stale-input cases.")
    finally:
        rust.write_text(source(fixture), encoding="utf-8")
    subprocess.run(command, check=True)
    print("Restored native candidate: GREEN. Pure catalogue qualification only; no durable/runtime E2E.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adversarial", action="store_true")
    args = parser.parse_args()
    subprocess.run([sys.executable, str(ROOT / "build_q1_candidate.py"), "--check"], check=True)
    with tempfile.TemporaryDirectory(prefix="shards-q1-") as scratch:
        qualify(Path(scratch), args.adversarial)


if __name__ == "__main__":
    main()
