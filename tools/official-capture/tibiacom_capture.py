"""Capture official tibia.com manual sections (and, when available, the spell library).

Evidence tooling only. tibia.com answers the build container, and #1077 shows it also answers
GitHub-hosted runners, with a Cloudflare "Sorry, you have been blocked" challenge. This tool never
spoofs a user agent, solves the challenge or retries against it: `fetch` is meant to be run once by
the repository owner on an ordinary machine tibia.com serves directly, `verify` and `self-test` run
offline in CI. `fetch` never writes full page text to disk, and never copies a page's blocks
verbatim -- only a manifest of what was fetched (url, UTC timestamp, HTTP status, SHA-256 of the
raw body, and the page's total visible-text length) and a bounded, deterministic list of short
facts (a section, a heading/anchor reference, a fact key and a value of at most 300 characters,
kept only when it carries a factual signal -- a number, a key/value pattern or a named control
key -- and capped per section).

    python3 tibiacom_capture.py fetch --out imports/official/tibia-com/2026-09-28
    python3 tibiacom_capture.py verify imports/official/tibia-com/2026-09-28
    python3 tibiacom_capture.py self-test

Manual sections captured (docs.tibia.com/gameguides/?subtopic=manual&section=<name>):
controls, characters, combat, world, controls_trading, starting. Only an HTTP 200 response is
accepted for any page; anything else aborts the whole `fetch` run before writing output.

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
MANIFEST_PAGE_KEYS = {'section', 'url', 'fetched_at', 'http_status', 'sha256', 'visible_text_chars'}
FACT_KEYS = {'section', 'anchor', 'key', 'value'}
FACT_VALUE_LIMIT = 300
FACTS_PER_SECTION_CAP = 40
FACT_TO_VISIBLE_TEXT_RATIO_LIMIT = 0.25
TOTAL_FACT_VALUE_BYTES_LIMIT = 200_000
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
ACCEPTED_HTTP_STATUS = 200

CLOUDFLARE_MARKERS = (
    'Sorry, you have been blocked',
    'Attention Required! | Cloudflare',
    'cf-error-details',
    'cf-browser-verification',
    'Checking your browser before accessing',
)

# A fact fragment is kept only when it carries a factual signal: a digit, an explicit
# "Key: value"/"Key = value" pattern, or the name of a control/input key. This is what keeps
# `fetch` from serializing every ordinary paragraph on the page (P1 r4120578784).
NUMBER_RE = re.compile(r'\d')
KEY_VALUE_RE = re.compile(r'^[A-Za-z][A-Za-z0-9 /()-]{0,40}[:=]\s*\S')
CONTROL_KEY_RE = re.compile(
    r'\b(Ctrl|Alt|Shift|Tab|Esc(?:ape)?|Enter|Space(?:bar)?|F[1-9][0-2]?|Arrow Keys?|'
    r'Left Mouse Button|Right Mouse Button|Mouse Wheel|Page Up|Page Down|Backspace|Delete|'
    r'Insert|Home|End)\b', re.IGNORECASE)
SENTENCE_SPLIT_RE = re.compile(r'(?<=[.!?])\s+(?=[A-Z0-9])')


def utc_now():
    return datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')


def normalize_space(text):
    return re.sub(r'\s+', ' ', text).strip()


def slugify(text):
    slug = re.sub(r'[^a-z0-9]+', '-', text.lower()).strip('-')
    return slug or 'section'


def is_cloudflare_challenge(body):
    return any(marker in body for marker in CLOUDFLARE_MARKERS)


def has_factual_signal(text):
    """A fragment is worth keeping as a fact: it has a number, a key/value pattern, or a named key."""
    return bool(NUMBER_RE.search(text) or KEY_VALUE_RE.match(text) or CONTROL_KEY_RE.search(text))


def split_sentences(text):
    parts = [part.strip() for part in SENTENCE_SPLIT_RE.split(text) if part.strip()]
    return parts or ([text] if text else [])


class VisibleTextParser(html.parser.HTMLParser):
    """Counts the normalized length of every text node outside <script>/<style>."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self._skip_depth = 0
        self.chars = 0

    def handle_starttag(self, tag, attrs):
        if tag in ('script', 'style'):
            self._skip_depth += 1

    def handle_endtag(self, tag):
        if tag in ('script', 'style') and self._skip_depth:
            self._skip_depth -= 1

    def handle_data(self, data):
        if not self._skip_depth:
            self.chars += len(normalize_space(data))


def visible_text_length(body):
    parser = VisibleTextParser()
    parser.feed(body)
    parser.close()
    return parser.chars


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
        self._block_tag = None
        self.buffer = []
        self.blocks = []  # list of (heading, anchor, tag, text)

    def handle_starttag(self, tag, attrs):
        if tag in self.HEADING_TAGS:
            self._flush_block()
            self.in_heading = True
            self.buffer = []
            self._heading_attrs = dict(attrs)
        elif tag in self.BLOCK_TAGS:
            self._flush_block()
            self.in_block = True
            self._block_tag = tag
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
                self.blocks.append((self.heading, self.anchor, self._block_tag, text))
        self.buffer = []
        self.in_block = False
        self._block_tag = None

    def close(self):
        self._flush_block()
        super().close()


def extract_facts(section, body):
    """Bounded, deterministic (section, anchor, key, value) facts.

    Keeps every heading, plus the sentences (for `<p>`) or whole cell/item text (for `<li>`/`<td>`/
    `<th>`/`<dd>`) under it that carry a factual signal (a number, a key/value pattern, or a named
    control key) -- not every block verbatim. Stops at `FACTS_PER_SECTION_CAP` facts for the
    section so one page cannot flood `facts.json` (P1 r4120578784).
    """
    parser = ManualSectionParser()
    parser.feed(body)
    parser.close()
    facts = []
    counters = {}
    seen_headings = set()
    for heading, anchor, tag, text in parser.blocks:
        if len(facts) >= FACTS_PER_SECTION_CAP:
            break
        if heading and anchor not in seen_headings:
            seen_headings.add(anchor)
            facts.append({'section': section, 'anchor': anchor, 'key': f'{section}.{anchor}.heading',
                          'value': heading[:FACT_VALUE_LIMIT]})
            if len(facts) >= FACTS_PER_SECTION_CAP:
                break
        fragments = split_sentences(text) if tag == 'p' else [text]
        for fragment in fragments:
            if len(facts) >= FACTS_PER_SECTION_CAP:
                break
            if not has_factual_signal(fragment):
                continue
            counters[anchor] = counters.get(anchor, 0) + 1
            facts.append({'section': section, 'anchor': heading or anchor,
                          'key': f'{section}.{anchor}.{counters[anchor]}', 'value': fragment[:FACT_VALUE_LIMIT]})
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


def fetch_page_or_abort(url, what):
    """Fetch `url`; return (status, body) only on an accepted HTTP 200, non-challenge response.

    On anything else -- a non-200 status (P2 r4120578795) or a Cloudflare challenge served with a
    200 -- prints a clear message and returns None so the caller aborts without writing output.
    """
    status, body = fetch_url(url)
    if status != ACCEPTED_HTTP_STATUS:
        print(f'tibiacom_capture: fetching {what} ({url}) returned HTTP {status}; only HTTP '
              f'{ACCEPTED_HTTP_STATUS} is accepted. Aborting without writing output.', file=sys.stderr)
        return None
    if is_cloudflare_challenge(body):
        print(f'tibiacom_capture: blocked by a Cloudflare challenge fetching {what} ({url}). Run '
              f'`fetch` from an ordinary machine tibia.com serves directly; this tool never solves '
              f'or bypasses the challenge. Aborting without writing output.', file=sys.stderr)
        return None
    return status, body


def cmd_fetch(out_dir):
    pages = []
    facts = []
    for index, section in enumerate(MANUAL_SECTIONS):
        if index:
            time.sleep(REQUEST_DELAY_SECONDS)
        url = MANUAL_URL.format(section=section)
        result = fetch_page_or_abort(url, f'manual section {section!r}')
        if result is None:
            return 2
        status, body = result
        pages.append({'section': section, 'url': url, 'fetched_at': utc_now(), 'http_status': status,
                      'sha256': hashlib.sha256(body.encode('utf-8')).hexdigest(),
                      'visible_text_chars': visible_text_length(body)})
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
            result = fetch_page_or_abort(SPELL_LIBRARY_URL, 'the spell library')
            if result is None:
                return 2
            status, body = result
            pages.append({'section': 'spells', 'url': SPELL_LIBRARY_URL, 'fetched_at': utc_now(),
                          'http_status': status, 'sha256': hashlib.sha256(body.encode('utf-8')).hexdigest(),
                          'visible_text_chars': visible_text_length(body)})
            for item in parse(body):
                value = str(item.get('value', ''))[:FACT_VALUE_LIMIT]
                facts.append({'section': 'spells', 'anchor': str(item.get('anchor', 'root')),
                              'key': str(item.get('key', f'spells.{len(facts)}')), 'value': value})
            spells = 'captured'

    out_dir.mkdir(parents=True, exist_ok=True)

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
    spells = manifest.get('spells')
    if spells not in (SPELL_PENDING, 'captured'):
        errors.append(f'{directory}: manifest.json "spells" must be {SPELL_PENDING!r} or "captured", got {spells!r}')

    pages = manifest.get('pages')
    pages_by_section = {}
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
        if page['http_status'] != ACCEPTED_HTTP_STATUS:
            errors.append(f'{directory}: manifest page {page["section"]} http_status is {page["http_status"]!r}, '
                          f'must be {ACCEPTED_HTTP_STATUS} (a failed/error fetch cannot become committed evidence)')
        if not isinstance(page['visible_text_chars'], int) or page['visible_text_chars'] < 0:
            errors.append(f'{directory}: manifest page {page["section"]} visible_text_chars must be a non-negative int')
        pages_by_section[page['section']] = page

    # Completeness (P2 r4120578800): every required manual section, at its exact URL, HTTP 200.
    for section in MANUAL_SECTIONS:
        page = pages_by_section.get(section)
        if page is None:
            errors.append(f'{directory}: manifest.json is missing required manual section {section!r}')
            continue
        expected_url = MANUAL_URL.format(section=section)
        if page.get('url') != expected_url:
            errors.append(f'{directory}: manifest page {section!r} url is {page.get("url")!r}, expected {expected_url!r}')
    extra_manual_like = set(pages_by_section) - set(MANUAL_SECTIONS) - {'spells'}
    if extra_manual_like:
        errors.append(f'{directory}: manifest.json has unexpected page section(s) {sorted(extra_manual_like)}')
    if spells == 'captured':
        spell_page = pages_by_section.get('spells')
        if spell_page is None:
            errors.append(f'{directory}: manifest.json "spells" is "captured" but no "spells" page is present')
        elif spell_page.get('url') != SPELL_LIBRARY_URL:
            errors.append(f'{directory}: manifest page "spells" url is {spell_page.get("url")!r}, '
                          f'expected {SPELL_LIBRARY_URL!r}')
    elif 'spells' in pages_by_section:
        errors.append(f'{directory}: manifest.json has a "spells" page but "spells" is {spells!r}, not "captured"')

    if facts_doc.get('schema') != FACTS_SCHEMA:
        errors.append(f'{directory}: facts.json schema must be {FACTS_SCHEMA!r}')
    facts = facts_doc.get('facts')
    if not isinstance(facts, list):
        errors.append(f'{directory}: facts.json "facts" must be a list')
        facts = []

    total_value_bytes = 0
    seen_keys = set()
    value_chars_by_section = {}
    fact_count_by_section = {}
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
        section = fact['section']
        if section not in pages_by_section:
            errors.append(f'{directory}: fact {fact["key"]!r} references section {section!r}, '
                          f'which is not a manifest.json page')
        fact_count_by_section[section] = fact_count_by_section.get(section, 0) + 1
        value_chars_by_section[section] = value_chars_by_section.get(section, 0) + len(value)

    # Every required manual section (and a "captured" spell library) needs at least one fact
    # (P2 r4120578800).
    for section in MANUAL_SECTIONS:
        if section in pages_by_section and fact_count_by_section.get(section, 0) < 1:
            errors.append(f'{directory}: manual section {section!r} has zero facts')
    if spells == 'captured' and fact_count_by_section.get('spells', 0) < 1:
        errors.append(f'{directory}: "spells" is "captured" but has zero facts')

    # Per-section bounds (P1 r4120578784): a fixed fact-count cap, and total fact chars for a page
    # must stay at or below a fraction of that page's own visible-text length -- a page copied in
    # bounded-size chunks still fails this even though each chunk individually passes the per-value
    # cap.
    for section, count in fact_count_by_section.items():
        if count > FACTS_PER_SECTION_CAP:
            errors.append(f'{directory}: section {section!r} has {count} facts, over the '
                          f'{FACTS_PER_SECTION_CAP}-fact-per-section cap')
        page = pages_by_section.get(section)
        if page is None or not isinstance(page.get('visible_text_chars'), int):
            continue
        visible_chars = page['visible_text_chars']
        value_chars = value_chars_by_section.get(section, 0)
        if visible_chars > 0 and value_chars > visible_chars * FACT_TO_VISIBLE_TEXT_RATIO_LIMIT + 1e-9:
            ratio = value_chars / visible_chars
            errors.append(f'{directory}: section {section!r} facts total {value_chars} chars, '
                          f'{ratio:.0%} of the page\'s {visible_chars} visible-text chars, over the '
                          f'{FACT_TO_VISIBLE_TEXT_RATIO_LIMIT:.0%} cap (looks like the page copied in '
                          f'chunks, not extracted facts)')

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
<p>Use the Arrow Keys or the Left Mouse Button to move your character.</p>
<p>This paragraph is simply flavor text about the misty streets of Thais, with no numbers
or key names in it at all, and must not be copied into a fact.</p>
<h2>Combat Basics</h2>
<ul>
<li>Attack value determines damage per hit, typically between 10 and 300.</li>
<li>Defense reduces incoming damage, up to a cap of 100%.</li>
</ul>
<table><tr><td>Hit Points: 185</td></tr></table>
</div>
</body></html>
"""


def _valid_snapshot_documents():
    """A manifest/facts pair with all six manual sections, each with one qualifying fact."""
    pages = []
    facts = []
    for section in MANUAL_SECTIONS:
        pages.append({'section': section, 'url': MANUAL_URL.format(section=section), 'fetched_at': utc_now(),
                      'http_status': 200, 'sha256': 'a' * 64, 'visible_text_chars': 1000})
        facts.append({'section': section, 'anchor': 'root', 'key': f'{section}.root.1',
                      'value': f'The {section} section has a value of 7 here.'})
    manifest = {'schema': MANIFEST_SCHEMA, 'captured_at': utc_now(), 'spells': SPELL_PENDING, 'pages': pages}
    facts_doc = {'schema': FACTS_SCHEMA, 'captured_at': manifest['captured_at'], 'facts': facts}
    return manifest, facts_doc


def _write_snapshot(directory, manifest, facts_doc):
    (directory / 'manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
    (directory / 'facts.json').write_text(json.dumps(facts_doc), encoding='utf-8')


def self_test():
    facts = extract_facts('controls', FIXTURE_HTML)
    by_key = {f['key']: f for f in facts}
    assert set(by_key) == {
        'controls.moving.heading', 'controls.moving.1',
        'controls.combat-basics.heading', 'controls.combat-basics.1',
        'controls.combat-basics.2', 'controls.combat-basics.3',
    }, by_key  # the ordinary "flavor text" paragraph must not be copied wholesale (P1 r4120578784)
    assert by_key['controls.moving.heading']['value'] == 'Moving Around', by_key
    assert by_key['controls.moving.1']['value'] == (
        'Use the Arrow Keys or the Left Mouse Button to move your character.'), by_key
    assert by_key['controls.combat-basics.heading']['value'] == 'Combat Basics'
    assert by_key['controls.combat-basics.1']['value'] == (
        'Attack value determines damage per hit, typically between 10 and 300.')
    assert by_key['controls.combat-basics.2']['value'] == 'Defense reduces incoming damage, up to a cap of 100%.'
    assert by_key['controls.combat-basics.3']['value'] == 'Hit Points: 185'
    assert all(f['section'] == 'controls' for f in facts)
    assert all(len(f['value']) <= FACT_VALUE_LIMIT for f in facts)

    long_html = '<h2>X</h2><p>Value: ' + ('9' * (FACT_VALUE_LIMIT + 50)) + '</p>'
    truncated = extract_facts('world', long_html)
    assert len(truncated) == 2, truncated  # heading + the one over-long fragment
    assert len(truncated[-1]['value']) == FACT_VALUE_LIMIT

    assert extract_facts('world', '<p>orphan block mentions Ctrl to confirm.</p>')[0]['anchor'] == 'root'
    assert extract_facts('world', '<p>ordinary sentence with no signal at all</p>') == []
    assert slugify('Combat Basics!') == 'combat-basics'

    # Per-section fact cap (P1 r4120578784): even when every block qualifies, extraction stops.
    many_items = '<h2>Cap</h2>' + ''.join(f'<li>Item {i} has value {i}.</li>' for i in range(50))
    capped = extract_facts('cap-test', many_items)
    assert len(capped) == FACTS_PER_SECTION_CAP, len(capped)

    assert not is_cloudflare_challenge('<html>ok</html>')
    assert is_cloudflare_challenge('Sorry, you have been blocked')

    assert has_factual_signal('Hit Points: 185')
    assert has_factual_signal('Press Ctrl to run.')
    assert has_factual_signal('Damage is 42.')
    assert not has_factual_signal('This is ordinary flavor text with no data in it.')

    assert split_sentences('First sentence. Second sentence!') == ['First sentence.', 'Second sentence!']
    assert split_sentences('Only one.') == ['Only one.']
    assert split_sentences('') == []

    assert visible_text_length(
        '<html><body><script>var x=1;</script><p>Hello world</p>'
        '<style>.a{color:red}</style></body></html>') == len('Hello world')

    import copy
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        base_manifest, base_facts = _valid_snapshot_documents()
        _write_snapshot(directory, base_manifest, base_facts)
        assert verify_snapshot(directory) == [], verify_snapshot(directory)

        # HTTP status must be exactly 200 (P2 r4120578795).
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['http_status'] = 500
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('http_status' in e and '200' in e for e in errors), errors

        # Completeness: all six sections required, at their exact URL (P2 r4120578800).
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'].pop()
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('missing required manual section' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['url'] = 'https://example.com/wrong'
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('expected' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'] = [f for f in facts_doc['facts'] if f['section'] != MANUAL_SECTIONS[0]]
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('zero facts' in e for e in errors), errors

        # Per-section fact-count cap, enforced independently by verify too.
        facts_doc = copy.deepcopy(base_facts)
        section = MANUAL_SECTIONS[0]
        facts_doc['facts'] += [{'section': section, 'anchor': 'root', 'key': f'{section}.root.extra{i}',
                                'value': f'extra fact number {i}'} for i in range(FACTS_PER_SECTION_CAP)]
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('fact-per-section cap' in e for e in errors), errors

        # No-page-copy ratio: total fact chars for a page must stay <=25% of its visible text
        # (P1 r4120578784) -- a page "copied" via many small, individually-compliant fragments
        # must still fail even though nothing here is a single over-length value.
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['visible_text_chars'] = 10
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('visible-text chars, over the' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['spells'] = 'weird'
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('"spells" must be' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['value'] = 'z' * (FACT_VALUE_LIMIT + 1)
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('over the' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['section'] = 'nowhere'
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('not a manifest.json page' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['sha256'] = 'not-hex'
        _write_snapshot(directory, manifest, base_facts)
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
