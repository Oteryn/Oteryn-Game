"""Capture official tibia.com manual sections (and, when available, the spell library).

Evidence tooling only. tibia.com answers the build container, and #1077 shows it also answers
GitHub-hosted runners, with a Cloudflare "Sorry, you have been blocked" challenge. This tool never
spoofs a user agent, solves the challenge or retries against it: `fetch` is meant to be run once by
the repository owner on an ordinary machine tibia.com serves directly, `verify` and `self-test` run
offline in CI. `fetch` never writes full page text to disk -- only a manifest of what was fetched
(url, UTC timestamp, HTTP status, SHA-256 of the raw body) and a deterministic list of short facts
(a section, a heading/anchor reference, a fact key and a value of at most 300 characters).

    python3 tibiacom_capture.py fetch --out imports/official/tibia-com/2026-09-28
    python3 tibiacom_capture.py verify imports/official/tibia-com/2026-09-28
    python3 tibiacom_capture.py self-test

Manual sections captured (docs.tibia.com/gameguides/?subtopic=manual&section=<name>):
controls, characters, combat, world, controls_trading, starting.

Spell library (S15, #1077): if `tools/content-schema/spell-authoring/tibiacom_spells.py` exists on
this checkout, `fetch` imports it and calls its `parse_spell_library(html_body)` (or `parse`)
entry point to turn the fetched spell-library page into facts -- this tool never copies that
module's own parsing logic. When the module does not exist yet, `manifest.json["spells"]` is set
to the string "PENDING_1077" and no spell-library request is made.
"""
import argparse
import hashlib
import html.parser
import importlib.util
import json
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

MANUAL_URL = 'https://www.tibia.com/gameguides/?subtopic=manual&section={section}'
MANUAL_SECTIONS = ('controls', 'characters', 'combat', 'world', 'controls_trading', 'starting')
SPELL_LIBRARY_URL = 'https://www.tibia.com/library/?subtopic=spells'
SPELL_MODULE_PATH = Path(__file__).resolve().parents[2] / 'tools/content-schema/spell-authoring/tibiacom_spells.py'
SPELL_PENDING = 'PENDING_1077'

USER_AGENT = 'OterynContentResearch/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
REQUEST_DELAY_SECONDS = 2
REQUEST_TIMEOUT_SECONDS = 30

MANIFEST_SCHEMA = 'OTERYN_TIBIACOM_CAPTURE_MANIFEST/v1'
FACTS_SCHEMA = 'OTERYN_TIBIACOM_CAPTURE_FACTS/v1'
MANIFEST_PAGE_KEYS = {'section', 'url', 'fetched_at', 'http_status', 'sha256'}
FACT_KEYS = {'section', 'anchor', 'key', 'value'}
FACT_VALUE_LIMIT = 300
TOTAL_FACT_VALUE_BYTES_LIMIT = 200_000
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')

CLOUDFLARE_MARKERS = (
    'Sorry, you have been blocked',
    'Attention Required! | Cloudflare',
    'cf-error-details',
    'cf-browser-verification',
    'Checking your browser before accessing',
)


def utc_now():
    return datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')


def normalize_space(text):
    return re.sub(r'\s+', ' ', text).strip()


def slugify(text):
    slug = re.sub(r'[^a-z0-9]+', '-', text.lower()).strip('-')
    return slug or 'section'


def is_cloudflare_challenge(status, body):
    if status in (403, 503):
        return True
    return any(marker in body for marker in CLOUDFLARE_MARKERS)


class ManualSectionParser(html.parser.HTMLParser):
    """Collects h1-h4 headings and the plain text of the p/li/td/th/dd blocks under each one.

    An anchor is the heading tag's `id` attribute when present, else a slug of its text. Blocks
    before the first heading are attached to the anchor 'root'. Kept deliberately simple and
    stdlib-only: no CSS-class guessing about which part of the page is "content".
    """
    HEADING_TAGS = {'h1', 'h2', 'h3', 'h4'}
    BLOCK_TAGS = {'p', 'li', 'td', 'th', 'dd'}

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.heading = None
        self.anchor = 'root'
        self._heading_attrs = {}
        self.in_heading = False
        self.in_block = False
        self.buffer = []
        self.blocks = []  # list of (heading, anchor, text)

    def handle_starttag(self, tag, attrs):
        if tag in self.HEADING_TAGS:
            self._flush_block()
            self.in_heading = True
            self.buffer = []
            self._heading_attrs = dict(attrs)
        elif tag in self.BLOCK_TAGS:
            self._flush_block()
            self.in_block = True
            self.buffer = []
        elif tag == 'br' and (self.in_heading or self.in_block):
            self.buffer.append(' ')

    def handle_endtag(self, tag):
        if tag in self.HEADING_TAGS and self.in_heading:
            text = normalize_space(''.join(self.buffer))
            self.in_heading = False
            self.buffer = []
            if text:
                self.heading = text
                self.anchor = self._heading_attrs.get('id') or slugify(text)
        elif tag in self.BLOCK_TAGS and self.in_block:
            self._flush_block()

    def handle_data(self, data):
        if self.in_heading or self.in_block:
            self.buffer.append(data)

    def _flush_block(self):
        if self.in_block:
            text = normalize_space(''.join(self.buffer))
            if text:
                self.blocks.append((self.heading, self.anchor, text))
        self.buffer = []
        self.in_block = False

    def close(self):
        self._flush_block()
        super().close()


def extract_facts(section, body):
    """Deterministic (section, anchor, key, value) facts: one per heading + one per block under it."""
    parser = ManualSectionParser()
    parser.feed(body)
    parser.close()
    facts = []
    counters = {}
    seen_headings = set()
    for heading, anchor, text in parser.blocks:
        if heading and anchor not in seen_headings:
            seen_headings.add(anchor)
            facts.append({'section': section, 'anchor': anchor, 'key': f'{section}.{anchor}.heading',
                          'value': heading[:FACT_VALUE_LIMIT]})
        counters[anchor] = counters.get(anchor, 0) + 1
        facts.append({'section': section, 'anchor': heading or anchor,
                      'key': f'{section}.{anchor}.{counters[anchor]}', 'value': text[:FACT_VALUE_LIMIT]})
    return facts


def fetch_url(url):
    request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
            return response.status, response.read().decode('utf-8', errors='replace')
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode('utf-8', errors='replace')


def load_spell_module():
    if not SPELL_MODULE_PATH.is_file():
        return None
    spec = importlib.util.spec_from_file_location('tibiacom_spells', SPELL_MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def cmd_fetch(out_dir):
    out_dir.mkdir(parents=True, exist_ok=True)
    pages = []
    facts = []
    for index, section in enumerate(MANUAL_SECTIONS):
        if index:
            time.sleep(REQUEST_DELAY_SECONDS)
        url = MANUAL_URL.format(section=section)
        status, body = fetch_url(url)
        if is_cloudflare_challenge(status, body):
            print(f'tibiacom_capture: blocked by a Cloudflare challenge fetching {url} '
                  f'(HTTP {status}). Run `fetch` from an ordinary machine tibia.com serves directly; '
                  f'this tool never solves or bypasses the challenge, and no output was written.',
                  file=sys.stderr)
            return 2
        pages.append({'section': section, 'url': url, 'fetched_at': utc_now(), 'http_status': status,
                      'sha256': hashlib.sha256(body.encode('utf-8')).hexdigest()})
        facts.extend(extract_facts(section, body))

    spell_module = load_spell_module()
    if spell_module is None:
        spells = SPELL_PENDING
    else:
        parse = getattr(spell_module, 'parse_spell_library', None) or getattr(spell_module, 'parse', None)
        if parse is None:
            print('tibiacom_capture: tools/content-schema/spell-authoring/tibiacom_spells.py has no '
                  'parse_spell_library()/parse() entry point; leaving spells as PENDING_1077.',
                  file=sys.stderr)
            spells = SPELL_PENDING
        else:
            time.sleep(REQUEST_DELAY_SECONDS)
            status, body = fetch_url(SPELL_LIBRARY_URL)
            if is_cloudflare_challenge(status, body):
                print(f'tibiacom_capture: blocked by a Cloudflare challenge fetching the spell '
                      f'library (HTTP {status}). No output was written.', file=sys.stderr)
                return 2
            pages.append({'section': 'spells', 'url': SPELL_LIBRARY_URL, 'fetched_at': utc_now(),
                          'http_status': status, 'sha256': hashlib.sha256(body.encode('utf-8')).hexdigest()})
            for item in parse(body):
                value = str(item.get('value', ''))[:FACT_VALUE_LIMIT]
                facts.append({'section': 'spells', 'anchor': str(item.get('anchor', 'root')),
                              'key': str(item.get('key', f'spells.{len(facts)}')), 'value': value})
            spells = 'captured'

    manifest = {'schema': MANIFEST_SCHEMA, 'captured_at': utc_now(), 'pages': pages, 'spells': spells}
    facts_doc = {'schema': FACTS_SCHEMA, 'captured_at': manifest['captured_at'], 'facts': facts}
    (out_dir / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=1) + '\n',
                                            encoding='utf-8', newline='\n')
    (out_dir / 'facts.json').write_text(json.dumps(facts_doc, ensure_ascii=False, indent=1) + '\n',
                                         encoding='utf-8', newline='\n')
    print(f'pages: {len(pages)}, facts: {len(facts)}, spells: {spells}')
    return 0


def verify_snapshot(directory):
    errors = []
    manifest_path = directory / 'manifest.json'
    facts_path = directory / 'facts.json'
    if not manifest_path.is_file():
        return [f'{directory}: missing manifest.json']
    if not facts_path.is_file():
        return [f'{directory}: missing facts.json']

    try:
        manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as error:
        return [f'{directory}: manifest.json is not valid JSON ({error})']
    try:
        facts_doc = json.loads(facts_path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as error:
        return [f'{directory}: facts.json is not valid JSON ({error})']

    if manifest.get('schema') != MANIFEST_SCHEMA:
        errors.append(f'{directory}: manifest.json schema must be {MANIFEST_SCHEMA!r}')
    if 'spells' not in manifest:
        errors.append(f'{directory}: manifest.json is missing "spells"')

    pages = manifest.get('pages')
    sections_in_manifest = set()
    if not isinstance(pages, list) or not pages:
        errors.append(f'{directory}: manifest.json "pages" must be a non-empty list')
        pages = []
    for page in pages:
        if not isinstance(page, dict):
            errors.append(f'{directory}: manifest page entry is not an object: {page!r}')
            continue
        missing = MANIFEST_PAGE_KEYS - page.keys()
        if missing:
            errors.append(f'{directory}: manifest page {page.get("section", "?")} missing keys {sorted(missing)}')
            continue
        if not isinstance(page['sha256'], str) or not SHA256_RE.match(page['sha256']):
            errors.append(f'{directory}: manifest page {page["section"]} sha256 is not a 64-hex-digit string')
        if not isinstance(page['http_status'], int):
            errors.append(f'{directory}: manifest page {page["section"]} http_status must be an int')
        sections_in_manifest.add(page['section'])

    if facts_doc.get('schema') != FACTS_SCHEMA:
        errors.append(f'{directory}: facts.json schema must be {FACTS_SCHEMA!r}')
    facts = facts_doc.get('facts')
    if not isinstance(facts, list):
        errors.append(f'{directory}: facts.json "facts" must be a list')
        facts = []

    total_value_bytes = 0
    seen_keys = set()
    for fact in facts:
        if not isinstance(fact, dict):
            errors.append(f'{directory}: fact entry is not an object: {fact!r}')
            continue
        missing = FACT_KEYS - fact.keys()
        if missing:
            errors.append(f'{directory}: fact missing keys {sorted(missing)}: {fact}')
            continue
        value = fact['value']
        if not isinstance(value, str):
            errors.append(f'{directory}: fact {fact["key"]} value must be a string')
            continue
        if len(value) > FACT_VALUE_LIMIT:
            errors.append(f'{directory}: fact {fact["key"]} value is {len(value)} chars, '
                          f'over the {FACT_VALUE_LIMIT}-char cap (no full page text)')
        total_value_bytes += len(value.encode('utf-8'))
        if fact['key'] in seen_keys:
            errors.append(f'{directory}: duplicate fact key {fact["key"]!r}')
        seen_keys.add(fact['key'])
        if fact['section'] not in sections_in_manifest:
            errors.append(f'{directory}: fact {fact["key"]!r} references section {fact["section"]!r}, '
                          f'which is not a manifest.json page')

    if total_value_bytes > TOTAL_FACT_VALUE_BYTES_LIMIT:
        errors.append(f'{directory}: total fact value bytes {total_value_bytes} exceeds the '
                      f'{TOTAL_FACT_VALUE_BYTES_LIMIT}-byte cap (looks like full page text, not facts)')

    return errors


def cmd_verify(directories):
    errors = []
    for directory in directories:
        errors.extend(verify_snapshot(directory))
    if errors:
        for error in errors:
            print(f'FAIL: {error}', file=sys.stderr)
        return 1
    print(f'verify: {len(directories)} snapshot dir(s), OK')
    return 0


FIXTURE_HTML = """
<html><body>
<div class="BoxContent">
<h2 id="moving">Moving Around</h2>
<p>Use the arrow keys or click on the ground to move your character.</p>
<p>Running requires double-clicking the destination tile.</p>
<h2>Combat Basics</h2>
<ul>
<li>Attack value determines damage per hit.</li>
<li>Defense reduces incoming damage, up to a cap of 100%.</li>
</ul>
</div>
</body></html>
"""


def self_test():
    facts = extract_facts('controls', FIXTURE_HTML)
    by_key = {f['key']: f for f in facts}
    assert by_key['controls.moving.heading']['value'] == 'Moving Around', by_key
    assert by_key['controls.moving.1']['value'] == (
        'Use the arrow keys or click on the ground to move your character.'), by_key
    assert by_key['controls.moving.2']['value'] == 'Running requires double-clicking the destination tile.', by_key
    assert by_key['controls.combat-basics.heading']['value'] == 'Combat Basics'
    assert by_key['controls.combat-basics.1']['value'] == 'Attack value determines damage per hit.'
    assert by_key['controls.combat-basics.2']['value'] == 'Defense reduces incoming damage, up to a cap of 100%.'
    assert all(f['section'] == 'controls' for f in facts)
    assert all(len(f['value']) <= FACT_VALUE_LIMIT for f in facts)

    long_html = '<h2>X</h2><p>' + ('z' * (FACT_VALUE_LIMIT + 50)) + '</p>'
    truncated = extract_facts('world', long_html)
    assert len(truncated[-1]['value']) == FACT_VALUE_LIMIT

    assert extract_facts('world', '<p>orphan block, no heading yet</p>')[0]['anchor'] == 'root'
    assert slugify('Combat Basics!') == 'combat-basics'
    assert not is_cloudflare_challenge(200, '<html>ok</html>')
    assert is_cloudflare_challenge(403, '<html>anything</html>')
    assert is_cloudflare_challenge(200, 'Sorry, you have been blocked')

    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        manifest = {'schema': MANIFEST_SCHEMA, 'captured_at': utc_now(), 'spells': SPELL_PENDING,
                    'pages': [{'section': 'controls', 'url': MANUAL_URL.format(section='controls'),
                              'fetched_at': utc_now(), 'http_status': 200, 'sha256': 'a' * 64}]}
        facts_doc = {'schema': FACTS_SCHEMA, 'captured_at': manifest['captured_at'],
                    'facts': [{'section': 'controls', 'anchor': 'moving', 'key': 'controls.moving.1',
                              'value': 'Use the arrow keys to move.'}]}
        (directory / 'manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
        (directory / 'facts.json').write_text(json.dumps(facts_doc), encoding='utf-8')
        assert verify_snapshot(directory) == [], verify_snapshot(directory)

        facts_doc['facts'][0]['value'] = 'z' * (FACT_VALUE_LIMIT + 1)
        (directory / 'facts.json').write_text(json.dumps(facts_doc), encoding='utf-8')
        errors = verify_snapshot(directory)
        assert any('over the' in e for e in errors), errors

        facts_doc['facts'][0]['value'] = 'ok'
        facts_doc['facts'][0]['section'] = 'nowhere'
        (directory / 'facts.json').write_text(json.dumps(facts_doc), encoding='utf-8')
        errors = verify_snapshot(directory)
        assert any('not a manifest.json page' in e for e in errors), errors

        manifest['pages'][0]['sha256'] = 'not-hex'
        facts_doc['facts'][0]['section'] = 'controls'
        (directory / 'manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
        (directory / 'facts.json').write_text(json.dumps(facts_doc), encoding='utf-8')
        errors = verify_snapshot(directory)
        assert any('64-hex-digit' in e for e in errors), errors

        missing = Path(tmp) / 'missing'
        assert verify_snapshot(missing) == [f'{missing}: missing manifest.json']

    print('tibiacom_capture self-test: PASS')


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv == ['self-test']:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    subparsers = parser.add_subparsers(dest='command', required=True)

    fetch_parser = subparsers.add_parser('fetch', help='fetch the manual sections (and spell library, if S15 code exists) once, from an owner machine')
    fetch_parser.add_argument('--out', type=Path, required=True)

    verify_parser = subparsers.add_parser('verify', help='offline: check one or more snapshot directories')
    verify_parser.add_argument('directories', nargs='+', type=Path)

    subparsers.add_parser('self-test', help='offline: run the embedded-fixture self-test')

    args = parser.parse_args(argv)
    if args.command == 'fetch':
        return cmd_fetch(args.out)
    if args.command == 'verify':
        return cmd_verify(args.directories)
    if args.command == 'self-test':
        self_test()
        return 0
    parser.error('unknown command')
    return 2


if __name__ == '__main__':
    sys.exit(main())
