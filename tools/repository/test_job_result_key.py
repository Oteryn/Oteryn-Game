import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

import job_result_key as jrk


def run(cwd, *args):
    return subprocess.check_output(["git", "-c", "user.name=t", "-c", "user.email=t@t", *args], cwd=cwd, text=True).strip()


class KeyFixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name).resolve()
        run(self.root, "init", "-q")
        self.write(".github/workflows/merge-gate.yml", "wf")
        self.write("tools/repository/classify_pr_test_lanes.py", "cls")
        self.write("tools/repository/job_result_key.py", "key")
        self.write("Cargo.toml", "[workspace]")
        self.write("Cargo.lock", "lock")
        self.write("rust-toolchain.toml", "tc")
        self.write("crates/a/Cargo.toml", "a")
        self.write("crates/a/src/lib.rs", "pub fn a() {}")
        self.write("crates/b/Cargo.toml", "b")
        self.write("crates/b/src/lib.rs", "pub fn b() {}")
        self.write("crates/c/Cargo.toml", "c")
        self.write("crates/c/src/lib.rs", "pub fn c() {}")
        self.write("docs/note.md", "note")
        self.write("docs/agents/tasks/t.md", "task")
        self.write("content/data.json", "{}")
        self.metadata = self.root / "meta.json"
        self.packages = self.root / "pk.tsv"
        self.set_metadata({"a": ["b"], "b": [], "c": []})
        self.set_packages(["a"])

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        run(self.root, "add", "-A")

    def set_metadata(self, graph):
        packages = []
        for name, deps in graph.items():
            packages.append({
                "id": name, "name": name,
                "manifest_path": str(self.root / "crates" / name / "Cargo.toml"),
                "dependencies": [{"name": d, "path": str(self.root / "crates" / d), "kind": "dev"} for d in deps],
            })
        self.metadata.write_text(json.dumps({
            "workspace_root": str(self.root), "workspace_members": list(graph), "packages": packages,
        }))

    def set_packages(self, names):
        self.packages.write_text("".join(f"{n}\t--lib\n" for n in names))

    def key(self, job="rust_fast-clippy", mode="rust-fast", extras=()):
        return jrk.compute_key(job, mode, self.root, list(extras), self.metadata, self.packages)[0]


class RustFastKey(KeyFixture):
    def test_deterministic_and_prefixed(self):
        self.assertEqual(self.key(), self.key())
        self.assertTrue(self.key().startswith("jobresult-v1-rust_fast-clippy-"))

    def test_untracked_noise_and_unrelated_docs_do_not_change_key(self):
        before = self.key()
        self.write("docs/note.md", "edited")
        self.write("crates/c/src/lib.rs", "pub fn c2() {}")
        (self.root / "stray.txt").write_text("untracked")
        self.assertEqual(before, self.key())

    def test_changed_crate_and_dev_dependency_change_key(self):
        for path in ("crates/a/src/lib.rs", "crates/b/src/lib.rs"):
            before = self.key()
            self.write(path, "pub fn changed() {}")
            self.assertNotEqual(before, self.key(), path)

    def test_every_common_and_cargo_input_changes_key(self):
        for path in (".github/workflows/merge-gate.yml", "tools/repository/classify_pr_test_lanes.py",
                     "tools/repository/job_result_key.py", "Cargo.lock", "Cargo.toml",
                     "rust-toolchain.toml", "crates/c/Cargo.toml"):
            before = self.key()
            self.write(path, "changed " + path)
            self.assertNotEqual(before, self.key(), path)
        before = self.key()
        self.write(".cargo/config.toml", "[build]")
        self.assertNotEqual(before, self.key())

    def test_job_leg_package_list_and_extras_change_key(self):
        base = self.key()
        self.assertNotEqual(base, self.key(job="rust_fast-tests 1/3"))
        self.assertNotEqual(base, self.key(extras=["base=1"]))
        self.assertNotEqual(self.key(extras=["base=1"]), self.key(extras=["base=2"]))
        self.set_packages(["a", "c"])
        self.assertNotEqual(base, self.key())

    def test_job_label_is_sanitized(self):
        self.assertTrue(self.key(job="rust_fast-tests 1/3").startswith("jobresult-v1-rust_fast-tests-1-3-"))

    def test_escaping_crate_keys_whole_tree_minus_unread_prefixes(self):
        self.write("crates/a/src/lib.rs", 'const X: &str = include_str!("../../../content/data.json");')
        before = self.key()
        self.write("content/data.json", '{"x":1}')
        self.assertNotEqual(before, self.key())
        self.write("crates/c/src/lib.rs", "pub fn other() {}")
        self.assertNotEqual(self.key(), before)
        settled = self.key()
        self.write("docs/agents/tasks/t.md", "task edited")
        self.assertEqual(settled, self.key())
        self.write("docs/note.md", "doc edited")
        self.assertNotEqual(settled, self.key())

    def test_each_escape_marker_is_detected(self):
        for source in ('env!("CARGO_MANIFEST_DIR")', 'include_bytes!("x")', 'std::env::var("X")',
                       'Command::new("git")', 'Path::new("../x")', 'Path::new("..")', "p.parent()", "fs::read_dir(p)",
                       '#[path = "x.rs"] mod m;', 'concat!(env!("OUT_DIR"))'):
            with self.subTest(source=source):
                self.write("crates/a/src/lib.rs", f"fn f() {{ {source}; }}")
                before = self.key()
                self.write("content/data.json", source)
                self.assertNotEqual(before, self.key())
                self.write("content/data.json", "{}")

    def test_non_member_path_dependency_falls_back_to_whole_tree(self):
        self.set_metadata({"a": [], "b": [], "c": []})
        data = json.loads(self.metadata.read_text())
        data["packages"][0]["dependencies"].append({"name": "v", "path": str(self.root / "docs"), "kind": None})
        self.metadata.write_text(json.dumps(data))
        before = self.key()
        self.write("content/data.json", "{1}")
        self.assertNotEqual(before, self.key())

    def test_bad_inputs_raise_so_the_job_runs_uncached(self):
        self.set_packages(["missing"])
        with self.assertRaises(jrk.KeyError_):
            self.key()
        self.packages.write_text("")
        with self.assertRaises(jrk.KeyError_):
            self.key()
        self.set_packages(["a"])
        data = json.loads(self.metadata.read_text())
        data["workspace_root"] = str(self.root / "crates")
        self.metadata.write_text(json.dumps(data))
        with self.assertRaises(jrk.KeyError_):
            self.key()


class OtherModes(KeyFixture):
    def test_tree_mode_changes_with_any_file(self):
        before = self.key("governance", "tree")
        self.write("docs/agents/tasks/t.md", "task edited")
        self.assertNotEqual(before, self.key("governance", "tree"))

    def test_rust_workspace_mode_tracks_rust_inputs_not_docs(self):
        before = self.key("rust_policy", "rust-workspace")
        self.write("docs/note.md", "edited")
        self.write("content/data.json", "{1}")
        self.assertEqual(before, self.key("rust_policy", "rust-workspace"))
        for path in ("crates/c/src/lib.rs", "crates/zz/Cargo.toml", "docs/migration/m.json", "vendor/x/Cargo.toml", "rustfmt.toml"):
            before = self.key("rust_policy", "rust-workspace")
            self.write(path, "changed " + path)
            self.assertNotEqual(before, self.key("rust_policy", "rust-workspace"), path)

    def test_unknown_mode_is_rejected(self):
        with self.assertRaises(jrk.KeyError_):
            self.key(mode="nope")


class CliAndMarker(KeyFixture):
    def invoke(self, *args, env=None):
        output = self.root / "gh-output"
        merged = {**os.environ, "GITHUB_OUTPUT": str(output), "GITHUB_RUN_ID": "42", "GITHUB_REPOSITORY": "o/r",
                  "GITHUB_SERVER_URL": "https://github.com", "GITHUB_SHA": "abc", **(env or {})}
        subprocess.check_call(["python3", "-I", str(Path(jrk.__file__)), *args], cwd=self.root, env=merged, stdout=subprocess.DEVNULL)
        return output.read_text() if output.exists() else ""

    def test_marker_round_trip_and_wrong_key_misses(self):
        key = "jobresult-v1-x-1"
        directory = self.root / "marker"
        self.invoke("marker", "--key", key, "--dir", str(directory))
        self.assertIn("hit=true", self.invoke("verify", "--key", key, "--dir", str(directory)))
        (self.root / "gh-output").unlink()
        self.assertIn("hit=false", self.invoke("verify", "--key", "jobresult-v1-x-2", "--dir", str(directory)))

    def test_missing_corrupt_or_empty_marker_misses(self):
        directory = self.root / "m2"
        self.assertIn("hit=false", self.invoke("verify", "--key", "k", "--dir", str(directory)))
        directory.mkdir()
        (directory / "marker.json").write_text("not json")
        (self.root / "gh-output").unlink()
        self.assertIn("hit=false", self.invoke("verify", "--key", "k", "--dir", str(directory)))
        (self.root / "gh-output").unlink()
        self.assertIn("hit=false", self.invoke("verify", "--key", "", "--dir", str(directory)))

    def test_key_command_writes_output_and_failure_yields_empty_key(self):
        text = self.invoke("key", "--job", "governance", "--mode", "tree", "--root", str(self.root))
        self.assertRegex(text, r"key=jobresult-v1-governance-[0-9a-f]{64}\n")
        (self.root / "gh-output").unlink()
        text = self.invoke("key", "--job", "x", "--mode", "rust-fast", "--root", str(self.root))
        self.assertEqual(text, "key=\n")


class RepositoryGuards(unittest.TestCase):
    """The unread prefixes must stay unread by Rust, or the relaxed fallback is unsound."""

    def test_no_rust_source_reads_an_unread_prefix(self):
        root = Path(__file__).resolve().parents[2]
        try:
            listing = subprocess.check_output(["git", "-C", str(root), "ls-files", "-z", "--", "*.rs"], stderr=subprocess.DEVNULL)
        except (OSError, subprocess.SubprocessError):
            self.skipTest("not a git checkout")
        offenders = []
        for name in listing.decode().split("\0"):
            if not name:
                continue
            path = root / name
            if not path.is_file():
                continue
            for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if line.lstrip().startswith("//"):
                    continue
                for prefix in jrk.UNREAD_PREFIXES:
                    if prefix.rstrip("/") in line:
                        offenders.append(f"{name}:{number}: {prefix}")
        self.assertEqual(offenders, [])

    def test_common_files_exist_in_repository(self):
        root = Path(__file__).resolve().parents[2]
        for path in jrk.COMMON_FILES:
            self.assertTrue((root / path).is_file(), path)


if __name__ == "__main__":
    unittest.main()
