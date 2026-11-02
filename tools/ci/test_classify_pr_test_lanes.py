"""Focused package-selection and Rust-job wiring regressions."""
import importlib.util
from pathlib import Path
import unittest
import json
import os
import subprocess
import tempfile
import textwrap
import re

SPEC = importlib.util.spec_from_file_location("selection", Path(__file__).with_name("classify_pr_test_lanes.py"))
selection = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(selection)


def fixture():
    roots = dict(selection.lanes.REQUIRED)
    roots.update({"protocol-oteryn": "crates/protocol-oteryn", "client-ui": "crates/client-ui"})
    edges = {
        "oteryn-client": ["client-ui", "protocol-oteryn"],
        "oteryn-synthetic-client-harness": ["oteryn-client"],
        "oteryn-game-server": ["protocol-oteryn"],
    }
    return {
        "workspace_root": "/repo", "workspace_members": list(roots),
        "packages": [dict(name=name, id=name, manifest_path=f"/repo/{root}/Cargo.toml",
                          dependencies=[dict(name=dep, kind=None, path=f"/repo/{roots[dep]}")
                                        for dep in edges.get(name, [])])
                     for name, root in roots.items()],
    }


class SelectionTests(unittest.TestCase):
    def select(self, *paths):
        return selection.select([dict(filename=p, status="modified") for p in paths], len(paths), fixture())

    def test_client_reverse_closure(self):
        self.assertEqual(self.select("crates/client-ui/src/lib.rs"), dict(mode="packages", packages=[
            "client-ui", "oteryn-client", "oteryn-synthetic-client-harness"]))

    def test_protocol_reverse_closure(self):
        self.assertEqual(self.select("crates/protocol-oteryn/src/lib.rs")["packages"], [
            "oteryn-client", "oteryn-game-server", "oteryn-synthetic-client-harness", "protocol-oteryn"])

    def test_unowned_and_shared_inputs_select_workspace(self):
        for path in ["Cargo.lock", "Cargo.toml", "rust-toolchain", ".cargo/config.toml",
                     "vendor/foo/lib.rs", ".github/workflows/merge-gate.yml", "build.rs",
                     "tools/shared-build.py", "unknown/input"]:
            with self.subTest(path=path):
                self.assertEqual(self.select(path)["mode"], "workspace")

    def test_docs_only_selects_none(self):
        self.assertEqual(self.select("docs/architecture/example.md"), dict(mode="none", packages=[]))

    def test_docs_do_not_hide_unowned_input(self):
        self.assertEqual(self.select("docs/example.md", "unknown/input")["mode"], "workspace")

    def test_rename_includes_previous_owner(self):
        result = selection.select([dict(filename="docs/example.md", status="renamed",
                                        previous_filename="crates/client-ui/src/lib.rs")], 1, fixture())
        self.assertIn("oteryn-client", result["packages"])

    def test_incomplete_or_malformed_fails_safe(self):
        for files, count, complete in [([], 1, True), ([{}], 1, True), ([], 0, False),
                                       ([dict(filename="../x", status="modified")], 1, True)]:
            self.assertEqual(selection.select(files, count, fixture(), complete)["mode"], "workspace")

    def test_whole_closure_uses_workspace(self):
        data = fixture()
        paths = [dict(filename=p["manifest_path"].removeprefix("/repo/"), status="modified")
                 for p in data["packages"]]
        self.assertEqual(selection.select(paths, len(paths), data)["mode"], "workspace")

    def test_cli_docs_and_consumed_docs(self):
        with tempfile.TemporaryDirectory() as directory:
            metadata = Path(directory) / 'metadata.json'
            metadata.write_text(json.dumps(fixture()))
            env = dict(os.environ, ENUMERATION_COMPLETE='true', CHANGED_FILE_COUNT='1',
                       CHANGED_FILE_RECORDS=json.dumps([dict(filename='docs/example.md', status='modified')]))
            for required, mode in [('false', 'none'), ('true', 'workspace')]:
                result = subprocess.check_output(['python3', '-I', str(Path(selection.__file__)), str(metadata)],
                                                 env=dict(env, RUST_REQUIRED=required), text=True)
                self.assertEqual(json.loads(result)['mode'], mode)

    def test_workflow_commands_with_subset_workspace_and_empty_selection(self):
        workflow = Path('.github/workflows/merge-gate.yml').read_text()
        steps = []
        for block in workflow.split('      - name: ')[1:]:
            match = re.search(r'^        run: \|\n((?:          .*\n|\n)+)', block, re.M)
            if match:
                ident = re.search(r'^        id: (.*)$', block, re.M)
                steps.append(dict(name=block.splitlines()[0], id=ident[1] if ident else '',
                                  run=textwrap.dedent(match[1])))
        plan = next(s['run'] for s in steps if s['name'] == 'Select changed workspace crates')
        plan = plan[plan.index('python - "'):]
        clippy = next(s['run'] for s in steps if s.get('id') == 'clippy')
        nextest = next(s['run'] for s in steps if s.get('id') == 'nextest')
        hit = next(s['run'] for s in steps if s.get('id') == 'jr_hit')
        data = fixture()
        for package in data['packages']:
            package['targets'] = [{'kind': ['lib']}]
        for mode, names in [('packages', ['oteryn-client']), ('workspace', []), ('none', [])]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'metadata.json').write_text(json.dumps(data))
                (root / 'rust-fast-metadata.json').write_text(json.dumps(data))
                cargo = root / 'cargo'
                cargo.write_text('#!/bin/bash\nif [[ "$*" == *metadata* ]]; then cat "$MOCK_METADATA"; else printf "%s\\n" "$*" >> "$MOCK_LOG"; fi\n')
                cargo.chmod(0o755)
                env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}", RUNNER_TEMP=directory,
                           MOCK_METADATA=str(root / 'metadata.json'), MOCK_LOG=str(root / 'commands'),
                           RUST_SELECTION=json.dumps(dict(mode=mode, packages=names)),
                           GITHUB_OUTPUT=str(root / 'output'), SHARD='1/3')
                def run(script):
                    return subprocess.run(['bash', '-e', '-c', script], env=env, check=True,
                                          capture_output=True, text=True)
                run(plan)
                (root / 'rust-fast-run.tsv').write_text((root / 'rust-fast-packages.tsv').read_text())
                run(clippy)
                run(nextest)
                if mode == 'none':
                    run(hit)
                    self.assertFalse((root / 'commands').exists())
                    self.assertIn('hit=true', (root / 'output').read_text())
                    self.assertIn('code=0', (root / 'output').read_text())
                else:
                    commands = (root / 'commands').read_text().splitlines()
                    self.assertEqual(len(commands), 2)
                    expected = '--workspace' if mode == 'workspace' else '-p oteryn-client'
                    for command in commands:
                        self.assertIn(expected, command)
                    self.assertIn('--partition hash:1/3 --no-tests=pass', commands[1])

    def test_workflow_keeps_shards_and_empty_selection_guard(self):
        workflow = Path(".github/workflows/merge-gate.yml").read_text()
        self.assertIn('"${package_args[@]}" --all-targets', workflow)
        self.assertIn('"${package_args[@]}" --lib --bins', workflow)
        self.assertIn('--partition "hash:${SHARD}" --no-tests=pass', workflow)
        self.assertIn('if [[ "${#filters[@]}" -eq 0 ]]', workflow)
        self.assertIn('if [[ ! -s "$RUNNER_TEMP/rust-fast-packages.tsv" ]]', workflow)


if __name__ == "__main__":
    unittest.main()
