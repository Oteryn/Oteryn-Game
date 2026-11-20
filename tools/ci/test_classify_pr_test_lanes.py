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

    def test_consumed_docs_require_matching_trusted_and_candidate_closures(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=repo, text=True).strip()

            git('init', '-q')
            git('config', 'user.name', 'CI')
            git('config', 'user.email', 'ci@example.invalid')
            note = 'docs/contracts/guide.md'
            paths = {
                note: 'guide',
                'apps/client/src/lib.rs': f'const GUIDE: &str = include_str!("../../../{note}");',
            }
            for name, content in paths.items():
                path = repo / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            git('add', '.')
            git('commit', '-qm', 'input')
            head = git('rev-parse', 'HEAD')
            metadata = repo / 'metadata.json'
            metadata.write_text(json.dumps(fixture()))
            owners = ['oteryn-client', 'oteryn-synthetic-client-harness']
            docs = [dict(filename=note, status='modified')]

            def run(records, proof, **env):
                output = subprocess.check_output(
                    ['python3', '-I', selection.__file__, str(metadata)], cwd=repo, text=True,
                    env=os.environ | dict(
                        ENUMERATION_COMPLETE='true', CHANGED_FILE_COUNT=str(len(records)),
                        CHANGED_FILE_RECORDS=json.dumps(records), RUST_REQUIRED='true',
                        EXPECTED_HEAD=head, TRUSTED_DOC_PACKAGES=proof,
                    ) | env,
                )
                return json.loads(output)

            self.assertEqual(run(docs, json.dumps(owners)), dict(mode='packages', packages=owners))
            for proof in ['', '[]', '{', '{}', '[null]', json.dumps(owners[:1]),
                          json.dumps(owners + ['unknown']), json.dumps(owners[::-1])]:
                with self.subTest(proof=proof):
                    self.assertEqual(run(docs, proof)['mode'], 'workspace')
            for records in [[dict(filename=note, status='removed')],
                            [dict(filename='docs/unused.md', status='modified')],
                            [dict(filename=note, status='renamed', previous_filename='docs/old.md')]]:
                self.assertEqual(run(records, json.dumps(owners))['mode'], 'workspace')
            self.assertEqual(run(docs, json.dumps(owners), EXPECTED_HEAD='missing')['mode'], 'workspace')
            self.assertEqual(run(docs, json.dumps(owners), CHANGED_FILE_COUNT='2')['mode'], 'workspace')
            self.assertEqual(run(docs, json.dumps(owners), ENUMERATION_COMPLETE='false',
                                 EXPECTED_BASE=head)['mode'], 'workspace')
            (repo / 'unverified-link').symlink_to(note)
            git('add', 'unverified-link')
            git('commit', '-qm', 'special candidate mode')
            self.assertEqual(run(docs, json.dumps(owners), EXPECTED_HEAD=git('rev-parse', 'HEAD'))['mode'],
                             'workspace')
            git('rm', '-q', 'unverified-link')
            control = repo / '.github/workflows/merge-gate.yml'
            control.parent.mkdir(parents=True)
            control.write_text(f'run: read {note}\n')
            git('add', '.github')
            git('commit', '-qm', 'additional control consumer')
            self.assertEqual(run(docs, json.dumps(owners), EXPECTED_HEAD=git('rev-parse', 'HEAD'))['mode'],
                             'workspace')

    def test_consumed_docs_proof_wiring(self):
        workflow = Path('.github/workflows/merge-gate.yml').read_text()
        self.assertIn('doc_packages: ${{ steps.classify.outputs.doc_packages }}', workflow)
        self.assertIn('TRUSTED_DOC_PACKAGES: ${{ needs.lanes.outputs.doc_packages }}', workflow)

    def test_workflow_commands_with_subset_workspace_and_empty_selection(self):
        workflow = Path('.github/workflows/merge-gate.yml').read_text()
        steps = []
        for block in workflow.split('      - name: ')[1:]:
            match = re.search(r'^        run: \|\n((?:          .*\n|\n)+)', block, re.M)
            if match:
                ident = re.search(r'^        id: (.*)$', block, re.M)
                steps.append(dict(name=block.splitlines()[0], id=ident[1] if ident else '',
                                  run=textwrap.dedent(match[1])))
        route = next(s['run'] for s in steps if s['name'] == 'Select changed workspace crates')
        route = route[route.index('python - "'):]
        plan = next(s['run'] for s in steps if s['name'] == 'Plan selected workspace crates')
        clippy = next(s['run'] for s in steps if s.get('id') == 'clippy')
        nextest = next(s['run'] for s in steps if s.get('id') == 'nextest')
        hit = next(s['run'] for s in steps if s['name'] == 'Select crates without a passed result')
        data = fixture()
        for package in data['packages']:
            package['targets'] = [{'kind': ['lib']}]
        for mode, names in [('packages', ['oteryn-client']), ('workspace', []), ('none', [])]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'metadata.json').write_text(json.dumps(data))
                (root / 'rust-fast-metadata.json').write_text(json.dumps(data))
                (root / 'rust-selection.json').write_text(json.dumps(dict(mode=mode, packages=names)))
                archive = root / 'rust-fast-archive/all'
                archive.mkdir(parents=True)
                (archive / 'cargo-nextest').touch()
                cargo = root / 'cargo'
                cargo.write_text('#!/bin/bash\nif [[ "$*" == *metadata* ]]; then cat "$MOCK_METADATA"; else printf "%s\\n" "$*" >> "$MOCK_LOG"; fi\n')
                cargo.chmod(0o755)
                env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}", RUNNER_TEMP=directory,
                           MOCK_METADATA=str(root / 'metadata.json'), MOCK_LOG=str(root / 'commands'),
                           RUST_SELECTION=json.dumps(dict(mode=mode, packages=names)),
                           GITHUB_OUTPUT=str(root / 'output'), SHARD='1/5', KIND='tests',
                           BYPASS_LEDGER='true', ARCHIVE_ROLES='["all"]', CARGO_TARGET_DIR=str(root / 'target'))
                def run(script):
                    return subprocess.run(['bash', '-e', '-c', script], env=env, check=True,
                                          capture_output=True, text=True)
                run(route)
                outputs = dict(row.split('=', 1) for row in (root / 'output').read_text().splitlines())
                self.assertEqual(json.loads(outputs['archives']), ['all'])
                env['RUST_SELECTION'] = outputs['selection']
                run(plan)
                run(hit)
                run(clippy)
                if mode == 'none':
                    self.assertFalse((root / 'commands').exists())
                    self.assertIn('hit=true', (root / 'output').read_text())
                    self.assertFalse((root / 'rust-fast-run.tsv').read_text())
                else:
                    run(nextest)
                    clippy_cmd, nextest_cmd = (root / 'commands').read_text().splitlines()
                    expected = '--workspace' if mode == 'workspace' else '-p oteryn-client'
                    self.assertIn(expected, clippy_cmd)
                    selected = json.loads(outputs['selection'])['packages']
                    self.assertIn('-E ' + '|'.join(f'package(={name})' for name in selected), nextest_cmd)
                    self.assertIn('--archive-file ', nextest_cmd)
                    self.assertIn('--partition hash:1/5 --no-tests=pass', nextest_cmd)

    def test_workflow_keeps_shards_and_empty_selection_guard(self):
        workflow = Path(".github/workflows/merge-gate.yml").read_text()
        self.assertIn('"${package_args[@]}" --all-targets', workflow)
        self.assertIn('all) target_args=(--lib --bins)', workflow)
        self.assertIn('--partition "hash:${SHARD}" --no-tests=pass', workflow)
        self.assertIn('if [[ "${#filters[@]}" -eq 0 ]]', workflow)
        self.assertIn('if [[ ! -s "$RUNNER_TEMP/rust-fast-packages.tsv" ]]', workflow)


if __name__ == "__main__":
    unittest.main()
