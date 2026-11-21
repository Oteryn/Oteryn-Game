import contextlib
import io
import json
import pathlib
import tempfile
import unittest

import extract

STYLE = '''import QtQuick
QtObject {
  readonly property int base: 4
  readonly property int gap: 2 * base
  readonly property color paper: "#D0D0D0" // synthetic fixture
  readonly property bool flag: true
  property string url: "https://example.invalid/a" // preserve slash pair
  property int unknown: controller.lookup()
  property var native: Text.NativeRendering
  property font sample
  sample { family: "Synthetic Font"; pixelSize: 9; bold: true; }
}
'''
BUTTON = '''Item {
  // source: "/images/ignored.png"
  /* source: "/images/also-ignored.png" */
  property string sourceUp: "/images/synthetic-up.png"
  property var textHorizontalAlignment: Text.AlignHCenter
  property int inset: pressed ? 3 : 0
}
'''


class ExtractTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.root = self.base / 'private-input'
        self.root.mkdir()
        self.inventory = self.base / 'inventory.json'
        self.write_sources({'Style.qml': STYLE, 'Button.qml': BUTTON})

    def write_sources(self, sources):
        files = []
        for name, value in sources.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            data = value.encode() if isinstance(value, str) else value
            path.write_bytes(data)
            files.append({'path': name, 'sha256': extract.sha256(data.replace(b'\r\n', b'\n'))})
        self.inventory.write_text(json.dumps({'source_version': 'synthetic-1', 'files': files}))

    def run_extract(self):
        return extract.extract(self.root, self.inventory, 'Style.qml')

    def test_literals_aliases_and_fonts(self):
        result = self.run_extract()
        self.assertEqual(result['style_tokens']['gap']['value'], 8)
        self.assertEqual(result['style_tokens']['paper']['value'], '#D0D0D0')
        self.assertTrue(result['style_tokens']['flag']['value'])
        self.assertEqual(result['style_tokens']['url']['value'], 'https://example.invalid/a')
        self.assertEqual(result['font_definitions']['sample']['attributes']['family']['value'], 'Synthetic Font')
        self.assertEqual(result['font_definitions']['sample']['attributes']['pixelSize']['value'], 9)

    def test_fractional_int_coercion_is_not_guessed(self):
        style = STYLE.replace('readonly property int gap: 2 * base', 'readonly property int gap: 5 / 2')
        self.write_sources({'Style.qml': style, 'Button.qml': BUTTON})
        token = self.run_extract()['style_tokens']['gap']
        self.assertEqual(token['status'], 'unresolved_expression')
        self.assertNotIn('value', token)
        self.assertEqual(token['reason'], 'integer coercion not inferred')

    def test_nested_style_properties_are_not_global_tokens(self):
        style = STYLE.replace('  property font sample', '  QtObject {\n    property int base: 99\n  }\n  property font sample')
        self.write_sources({'Style.qml': style, 'Button.qml': BUTTON})
        result = self.run_extract()
        self.assertEqual(result['style_tokens']['base']['value'], 4)
        self.assertTrue(any('property int base: 99' in r['expression'] for r in result['rule_evidence']))

    def test_controller_and_native_expressions_stay_unresolved(self):
        tokens = self.run_extract()['style_tokens']
        for name in ('unknown', 'native'):
            self.assertEqual(tokens[name]['status'], 'unresolved_expression')
            self.assertNotIn('value', tokens[name])

    def test_safe_evaluation_rejects_calls_attributes_pow_and_large_numbers(self):
        for expression in ['controller.lookup()', '__import__("os")', '(1).__class__', '2 ** 100000', '1e999', '9' * 100, 'true + 1', '"x" * 100000']:
            with self.subTest(expression=expression):
                with self.assertRaises(ValueError):
                    extract.constant(expression, {})

    def test_crlf_preserves_raw_hash_and_verifies_normalized_hash(self):
        raw = STYLE.replace('\n', '\r\n').encode()
        self.write_sources({'Style.qml': raw, 'Button.qml': BUTTON})
        entry = self.run_extract()['sources']['Style.qml']
        self.assertEqual(entry['raw_sha256'], extract.sha256(raw))
        self.assertEqual(entry['normalized_sha256'], extract.sha256(STYLE.encode()))
        self.assertNotEqual(entry['raw_sha256'], entry['normalized_sha256'])
        self.assertEqual(entry['normalization'], 'CRLF to LF only')

    def test_source_change_without_new_hash_fails(self):
        (self.root / 'Style.qml').write_text(STYLE.replace('base: 4', 'base: 7'))
        with self.assertRaisesRegex(extract.InputError, 'SHA256'):
            self.run_extract()

    def test_updated_source_and_inventory_have_fresh_facts_no_stale_contracts(self):
        self.write_sources({'Style.qml': STYLE.replace('base: 4', 'base: 7'),
                            'Button.qml': BUTTON.replace('AlignHCenter', 'AlignLeft')})
        result = self.run_extract()
        self.assertEqual(result['style_tokens']['gap']['value'], 14)
        self.assertNotIn('widget_contracts', result)
        evidence = '\n'.join(x['expression'] for x in result['rule_evidence'])
        self.assertIn('AlignLeft', evidence)
        self.assertNotIn('AlignHCenter', evidence)

    def test_missing_hash_rejected(self):
        self.inventory.write_text(json.dumps({'files': [{'path': 'Style.qml'}]}))
        with self.assertRaisesRegex(extract.InputError, 'sha256'):
            self.run_extract()

    def test_unknown_source_rejected(self):
        (self.root / 'Unknown.qml').write_text('Item {}')
        with self.assertRaisesRegex(extract.InputError, 'absent from inventory'):
            self.run_extract()

    def test_conflicting_duplicate_basename_rejected(self):
        self.write_sources({'Style.qml': STYLE, 'Button.qml': BUTTON, 'nested/Button.qml': 'Item { width: 99 }'})
        with self.assertRaisesRegex(extract.InputError, 'conflicting duplicate basename'):
            self.run_extract()

    def test_identical_duplicate_basename_keeps_relative_provenance(self):
        self.write_sources({'Style.qml': STYLE, 'Button.qml': BUTTON, 'nested/Button.qml': BUTTON})
        result = self.run_extract()
        self.assertIn('nested/Button.qml', result['sources'])
        self.assertTrue(any(x['source']['file'] == 'nested/Button.qml' for x in result['rule_evidence']))

    def test_rule_lines_and_asset_refs_exclude_comments(self):
        result = self.run_extract()
        urls = [x['url_or_fragment'] for x in result['asset_references']]
        self.assertEqual(urls, ['/images/synthetic-up.png'])
        for fact in result['rule_evidence']:
            location = fact['source']
            line = (self.root / location['file']).read_text().splitlines()[location['line'] - 1]
            self.assertIn(fact['expression'], line)
        self.assertTrue(any('textHorizontalAlignment:' in x['expression'] for x in result['rule_evidence']))

    def test_deterministic_output_has_no_machine_paths(self):
        first = json.dumps(self.run_extract(), sort_keys=True)
        self.assertEqual(first, json.dumps(self.run_extract(), sort_keys=True))
        self.assertNotIn(str(self.base), first)
        clone = self.base / 'other-location'
        import shutil
        shutil.copytree(self.root, clone)
        self.assertEqual(first, json.dumps(extract.extract(clone, self.inventory, 'Style.qml'), sort_keys=True))

    def test_inventory_paths_and_duplicates_rejected(self):
        for files in [[{'path': '../escape.qml', 'sha256': '0' * 64}],
                      [{'path': '/absolute.qml', 'sha256': '0' * 64}],
                      [{'path': 'C:/absolute.qml', 'sha256': '0' * 64}],
                      [{'path': 'Style.qml', 'sha256': '0' * 64}] * 2]:
            self.inventory.write_text(json.dumps({'files': files}))
            with self.assertRaises(extract.InputError):
                self.run_extract()

    def test_absent_inputs_never_report_success_or_overwrite_output(self):
        output = self.base / 'output.json'
        output.write_text('existing result')
        stderr, stdout = io.StringIO(), io.StringIO()
        with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(stdout):
            code = extract.main(['--sources', str(self.base / 'missing'), '--inventory', str(self.inventory),
                                 '--style', 'Style.qml', '--output', str(output)])
        self.assertEqual(code, 2)
        self.assertEqual(stdout.getvalue(), '')
        self.assertIn('not a directory', stderr.getvalue())
        self.assertEqual(output.read_text(), 'existing result')

    def test_external_symlink_is_rejected(self):
        outside = self.base / 'outside.qml'
        outside.write_text('Item {}')
        link = self.root / 'link.qml'
        try:
            link.symlink_to(outside)
        except OSError as exc:
            self.skipTest(f'OS cannot create symlink: {exc}')
        with self.assertRaisesRegex(extract.InputError, 'symlink escapes root'):
            self.run_extract()

    def test_output_cannot_overwrite_source_or_inventory(self):
        for output in [self.root / 'Style.qml', self.inventory]:
            with self.subTest(output=output.name):
                before = output.read_bytes()
                stderr, stdout = io.StringIO(), io.StringIO()
                with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(stdout):
                    code = extract.main(['--sources', str(self.root), '--inventory', str(self.inventory),
                                         '--style', 'Style.qml', '--output', str(output)])
                self.assertEqual(code, 2)
                self.assertEqual(stdout.getvalue(), '')
                self.assertIn('must not overwrite', stderr.getvalue())
                self.assertEqual(output.read_bytes(), before)

    def test_external_directory_symlink_is_rejected(self):
        outside = self.base / 'outside-dir'
        outside.mkdir()
        link = self.root / 'linked-directory'
        try:
            link.symlink_to(outside, target_is_directory=True)
        except OSError as exc:
            self.skipTest(f'OS cannot create symlink: {exc}')
        with self.assertRaisesRegex(extract.InputError, 'symlink escapes root'):
            self.run_extract()

    def test_missing_style_is_explicit(self):
        with self.assertRaisesRegex(extract.InputError, 'requested style source missing'):
            extract.extract(self.root, self.inventory, 'AbsentStyle.qml')


if __name__ == '__main__':
    unittest.main()
