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
    python3 tibiacom_capture.py check-immutability --base <sha> --head <sha>
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
import os
import re
import stat
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

SNAPSHOT_PREFIX = 'imports/official/tibia-com/'
DATED_DIR_RE = re.compile(re.escape(SNAPSHOT_PREFIX) + r'([^/]+)/')

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
MANIFEST_KEYS = {'schema', 'captured_at', 'pages', 'spells'}
MANIFEST_PAGE_KEYS = {'section', 'url', 'fetched_at', 'http_status', 'sha256', 'visible_text_chars'}
FACTS_DOC_KEYS = {'schema', 'captured_at', 'facts'}
FACT_KEYS = {'section', 'anchor', 'key', 'value'}
# A snapshot directory holds exactly these two files -- no raw pages, scratch files or
# subdirectories (P2 r4121003204, page-copy class).
SNAPSHOT_FILENAMES = {'manifest.json', 'facts.json'}
FACT_VALUE_LIMIT = 300
# Every serialized text field is bounded, not only `value` (P2 r4120883667): a `key`/`anchor` this
# long is not an identifier any more, so both fetch and verify cap them the same way.
FIELD_ID_LIMIT = 120
FACTS_PER_SECTION_CAP = 40
# The spell library is structured spell rows delegated to #1077's own parser, not manual-page
# prose, so it gets its own compatible bound instead of the 40-fact manual-page cap
# (P2 r4120758016). Each row's field values still obey FACT_VALUE_LIMIT.
SPELL_RECORDS_CAP = 600
FACT_TO_VISIBLE_TEXT_RATIO_LIMIT = 0.25
TOTAL_FACT_VALUE_BYTES_LIMIT = 200_000
# Closes the whole "smuggled text" class in one generic check, regardless of encoding trick: the
# RAW bytes of facts.json on disk (not the parsed field lengths) must fit the same 25% ratio, plus
# a fixed per-fact JSON-structure overhead allowance.
PER_FACT_JSON_OVERHEAD_BYTES = 64
SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
ACCEPTED_HTTP_STATUS = 200
# Strict match on what utc_now() produces (P2 r4120883679): an ISO-8601 UTC timestamp ending in Z.
TIMESTAMP_RE = re.compile(r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$')
MAX_FETCH_RUN_SECONDS = 3600
SYMLINK_MODE = '120000'

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


def bound_field(text, limit=FIELD_ID_LIMIT):
    """Cap an identifier-like field (a fact `key` or `anchor`) at `limit` chars (P2 r4120883667)."""
    return str(text)[:limit]


def bound_key(base, suffix, limit=FIELD_ID_LIMIT):
    """A bounded fact `key` that stays unique even when `base` (section+anchor) is long enough to
    force truncation (P2 r4121003212): a long anchor must not make the `.heading`/`.N` suffix that
    disambiguates facts under it collide away. `base` is truncated first, reserving room for the
    suffix; when that truncation actually happens, an 8-hex-char hash of the full, untruncated key
    is spliced in ahead of the suffix so two different long bases (or two different suffixes on the
    same long base) cannot produce the same bounded key.
    """
    full = f'{base}{suffix}'
    if len(full) <= limit:
        return full
    digest = hashlib.sha256(full.encode('utf-8')).hexdigest()[:8]
    tail = f'-{digest}{suffix}'
    if len(tail) >= limit:
        return tail[:limit]
    return f'{base[:limit - len(tail)]}{tail}'


def parse_utc_timestamp(value):
    """Parse a strict `utc_now()`-shaped timestamp, or None if it isn't one (P2 r4120883679)."""
    if not isinstance(value, str) or not TIMESTAMP_RE.match(value):
        return None
    try:
        return datetime.strptime(value, '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def _reject_duplicate_keys(pairs):
    """`object_pairs_hook` for `json.loads`: raise instead of silently keeping only the last of a
    duplicate key (P2 r4121095556) -- applies at every nesting level, so a duplicate `value` inside
    one fact object is caught exactly like a duplicate top-level key.
    """
    seen = {}
    for key, value in pairs:
        if key in seen:
            raise ValueError(f'duplicate key {key!r}')
        seen[key] = value
    return seen


def load_json_no_duplicates(text):
    """`json.loads` with duplicate-key rejection; raises ValueError (JSONDecodeError included)."""
    return json.loads(text, object_pairs_hook=_reject_duplicate_keys)


def normalize_space(text):
    return re.sub(r'\s+', ' ', text).strip()


def slugify(text):
    slug = re.sub(r'[^a-z0-9]+', '-', text.lower()).strip('-')
    return slug or 'section'


def is_cloudflare_challenge(body):
    return any(marker in body for marker in CLOUDFLARE_MARKERS)


def facts_cap_for_section(section):
    """The spell library (structured rows) gets SPELL_RECORDS_CAP; every manual section gets
    FACTS_PER_SECTION_CAP (P2 r4120758016)."""
    return SPELL_RECORDS_CAP if section == 'spells' else FACTS_PER_SECTION_CAP


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
                self.anchor = bound_field(self._heading_attrs.get('id') or slugify(text))
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
            facts.append({'section': section, 'anchor': anchor,
                          'key': bound_key(f'{section}.{anchor}', '.heading'),
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
            # Always the parser's normalized anchor (id/slug), never the raw heading text
            # (P2 r4120883657), so `anchor` stays a consistent HTML-id/slug reference. The key is
            # built with bound_key, not a plain truncation, so a long anchor can't make the
            # `.heading`/`.N` suffix collide away (P2 r4121003212).
            facts.append({'section': section, 'anchor': anchor,
                          'key': bound_key(f'{section}.{anchor}', f'.{counters[anchor]}'),
                          'value': fragment[:FACT_VALUE_LIMIT]})
    return facts


def fetch_url(url):
    """Return (status, raw_bytes) -- the exact response body, undecoded (P2 r4120758054)."""
    request = urllib.request.Request(url, headers={'User-Agent': USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def load_spell_module():
    if not SPELL_MODULE_PATH.is_file():
        return None
    spec = importlib.util.spec_from_file_location('tibiacom_spells', SPELL_MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def fetch_page_or_abort(url, what):
    """Fetch `url`; return (status, raw_bytes, body) only on an accepted HTTP 200, non-challenge
    response. `raw_bytes` is the exact, undecoded response body (hash that); `body` is a separate
    decoded copy for parsing only (P2 r4120758054).

    On anything else -- a non-200 status (P2 r4120578795) or a Cloudflare challenge served with a
    200 -- prints a clear message and returns None so the caller aborts without writing output.
    """
    status, raw = fetch_url(url)
    if status != ACCEPTED_HTTP_STATUS:
        print(f'tibiacom_capture: fetching {what} ({url}) returned HTTP {status}; only HTTP '
              f'{ACCEPTED_HTTP_STATUS} is accepted. Aborting without writing output.', file=sys.stderr)
        return None
    body = raw.decode('utf-8', errors='replace')
    if is_cloudflare_challenge(body):
        print(f'tibiacom_capture: blocked by a Cloudflare challenge fetching {what} ({url}). Run '
              f'`fetch` from an ordinary machine tibia.com serves directly; this tool never solves '
              f'or bypasses the challenge. Aborting without writing output.', file=sys.stderr)
        return None
    return status, raw, body


def cmd_fetch(out_dir):
    # Stamped once, before any request, so every page's fetched_at is >= captured_at
    # (P2 r4120883679) -- this run takes seconds, nowhere near MAX_FETCH_RUN_SECONDS.
    captured_at = utc_now()
    pages = []
    facts = []
    for index, section in enumerate(MANUAL_SECTIONS):
        if index:
            time.sleep(REQUEST_DELAY_SECONDS)
        url = MANUAL_URL.format(section=section)
        result = fetch_page_or_abort(url, f'manual section {section!r}')
        if result is None:
            return 2
        status, raw, body = result
        pages.append({'section': section, 'url': url, 'fetched_at': utc_now(), 'http_status': status,
                      'sha256': hashlib.sha256(raw).hexdigest(),
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
            status, raw, body = result
            pages.append({'section': 'spells', 'url': SPELL_LIBRARY_URL, 'fetched_at': utc_now(),
                          'http_status': status, 'sha256': hashlib.sha256(raw).hexdigest(),
                          'visible_text_chars': visible_text_length(body)})
            spell_facts = 0
            for item in parse(body):
                if spell_facts >= SPELL_RECORDS_CAP:
                    break
                # Bound every delegated field the same way verify does (P2 r4120883667): a
                # delegated parser is not exempt from the key/anchor/value size caps.
                value = str(item.get('value', ''))[:FACT_VALUE_LIMIT]
                anchor = bound_field(item.get('anchor', 'root'))
                key = bound_field(item.get('key', f'spells.{len(facts)}'))
                facts.append({'section': 'spells', 'anchor': anchor, 'key': key, 'value': value})
                spell_facts += 1
            spells = 'captured'

    out_dir.mkdir(parents=True, exist_ok=True)

    manifest = {'schema': MANIFEST_SCHEMA, 'captured_at': captured_at, 'pages': pages, 'spells': spells}
    facts_doc = {'schema': FACTS_SCHEMA, 'captured_at': captured_at, 'facts': facts}
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

    # Both documents must be regular files, not symlinks (or anything else) -- Path.is_file()
    # above follows a symlink to a regular file, so a committed symlink would otherwise pass
    # (P2 r4121095564). os.lstat inspects the path itself, without following a final symlink.
    for path in (manifest_path, facts_path):
        try:
            mode = os.lstat(path).st_mode
        except OSError as error:
            return errors + [f'{directory}: cannot lstat {path.name} ({error})']
        if not stat.S_ISREG(mode):
            return errors + [f'{directory}: {path.name} must be a regular file, not a symlink '
                             f'or other special file']

    # Exactly manifest.json and facts.json -- no raw pages, scratch files or subdirectories
    # (P2 r4121003204, page-copy class): both required files exist, so directory is safe to list.
    extra_files = sorted(p.name for p in directory.iterdir() if p.name not in SNAPSHOT_FILENAMES)
    if extra_files:
        errors.append(f'{directory}: snapshot directory must contain exactly {sorted(SNAPSHOT_FILENAMES)}, '
                      f'found extra: {extra_files}')

    # The directory name is the snapshot's provenance date: a real calendar date, equal to
    # captured_at's date part (P2 r4121095570). Checked before JSON parsing so it applies even to
    # an otherwise-malformed snapshot.
    dir_date = None
    try:
        dir_date = date.fromisoformat(directory.name)
    except ValueError:
        errors.append(f'{directory}: directory name {directory.name!r} must be a real YYYY-MM-DD '
                      f'calendar date')

    try:
        manifest = load_json_no_duplicates(manifest_path.read_text(encoding='utf-8'))
    except ValueError as error:
        return errors + [f'{directory}: manifest.json is not valid JSON ({error})']
    try:
        facts_doc = load_json_no_duplicates(facts_path.read_text(encoding='utf-8'))
    except ValueError as error:
        return errors + [f'{directory}: facts.json is not valid JSON ({error})']

    # Closed schema (P2 r4121003195): both documents are objects with exactly their allowed
    # top-level keys -- an unrecognized field (e.g. a raw_html dump) must not slip past every
    # size/ratio check just because nothing reads it by name.
    if not isinstance(manifest, dict):
        return errors + [f'{directory}: manifest.json must be a JSON object']
    if not isinstance(facts_doc, dict):
        return errors + [f'{directory}: facts.json must be a JSON object']
    extra_manifest_keys = manifest.keys() - MANIFEST_KEYS
    if extra_manifest_keys:
        errors.append(f'{directory}: manifest.json has unexpected top-level key(s) {sorted(extra_manifest_keys)}')
    extra_facts_doc_keys = facts_doc.keys() - FACTS_DOC_KEYS
    if extra_facts_doc_keys:
        errors.append(f'{directory}: facts.json has unexpected top-level key(s) {sorted(extra_facts_doc_keys)}')

    if manifest.get('schema') != MANIFEST_SCHEMA:
        errors.append(f'{directory}: manifest.json schema must be {MANIFEST_SCHEMA!r}')
    spells = manifest.get('spells')
    if spells not in (SPELL_PENDING, 'captured'):
        errors.append(f'{directory}: manifest.json "spells" must be {SPELL_PENDING!r} or "captured", got {spells!r}')

    # Capture timestamps (P2 r4120883679): both top-level captured_at must be present, ISO-8601 UTC
    # 'Z' timestamps, and equal to each other; every page's fetched_at is checked against this below.
    manifest_captured_at = manifest.get('captured_at')
    facts_captured_at = facts_doc.get('captured_at')
    captured_dt = parse_utc_timestamp(manifest_captured_at)
    if captured_dt is None:
        errors.append(f'{directory}: manifest.json "captured_at" must be an ISO-8601 UTC timestamp ending in Z, '
                      f'got {manifest_captured_at!r}')
    if parse_utc_timestamp(facts_captured_at) is None:
        errors.append(f'{directory}: facts.json "captured_at" must be an ISO-8601 UTC timestamp ending in Z, '
                      f'got {facts_captured_at!r}')
    elif captured_dt is not None and manifest_captured_at != facts_captured_at:
        errors.append(f'{directory}: manifest.json and facts.json "captured_at" must be equal '
                      f'({manifest_captured_at!r} != {facts_captured_at!r})')
    if dir_date is not None and captured_dt is not None and dir_date.isoformat() != manifest_captured_at[:10]:
        errors.append(f'{directory}: directory name {directory.name!r} must equal the date part '
                      f'of captured_at {manifest_captured_at!r}')

    pages = manifest.get('pages')
    pages_by_section = {}
    seen_urls = set()
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
        extra_page_keys = page.keys() - MANIFEST_PAGE_KEYS
        if extra_page_keys:
            errors.append(f'{directory}: manifest page {page.get("section", "?")} has unexpected '
                          f'key(s) {sorted(extra_page_keys)}')
            continue
        if not isinstance(page['sha256'], str) or not SHA256_RE.match(page['sha256']):
            errors.append(f'{directory}: manifest page {page["section"]} sha256 is not a 64-hex-digit string')
        if page['http_status'] != ACCEPTED_HTTP_STATUS:
            errors.append(f'{directory}: manifest page {page["section"]} http_status is {page["http_status"]!r}, '
                          f'must be {ACCEPTED_HTTP_STATUS} (a failed/error fetch cannot become committed evidence)')
        if not isinstance(page['visible_text_chars'], int) or page['visible_text_chars'] < 0:
            errors.append(f'{directory}: manifest page {page["section"]} visible_text_chars must be a non-negative int')
        fetched_dt = parse_utc_timestamp(page.get('fetched_at'))
        if fetched_dt is None:
            errors.append(f'{directory}: manifest page {page["section"]} fetched_at must be an ISO-8601 UTC '
                          f'timestamp ending in Z, got {page.get("fetched_at")!r}')
        elif captured_dt is not None:
            delta = (fetched_dt - captured_dt).total_seconds()
            if delta < 0 or delta > MAX_FETCH_RUN_SECONDS:
                errors.append(f'{directory}: manifest page {page["section"]} fetched_at {page["fetched_at"]!r} '
                              f'must be >= captured_at {manifest_captured_at!r} and within '
                              f'{MAX_FETCH_RUN_SECONDS} seconds of it')
        # Duplicate sections/URLs (P2 r4120758056): a later duplicate must not silently overwrite
        # an earlier page entry in pages_by_section.
        if page['section'] in pages_by_section:
            errors.append(f'{directory}: duplicate manifest page section {page["section"]!r}')
        else:
            pages_by_section[page['section']] = page
        if page.get('url') in seen_urls:
            errors.append(f'{directory}: duplicate manifest page url {page.get("url")!r}')
        seen_urls.add(page.get('url'))

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
        extra_fact_keys = fact.keys() - FACT_KEYS
        if extra_fact_keys:
            errors.append(f'{directory}: fact has unexpected key(s) {sorted(extra_fact_keys)}: {fact}')
            continue
        value = fact['value']
        key = fact['key']
        anchor = fact['anchor']
        if not isinstance(value, str):
            errors.append(f'{directory}: fact {key!r} value must be a string')
            continue
        if not isinstance(key, str):
            errors.append(f'{directory}: fact key {key!r} must be a string')
            continue
        if not isinstance(anchor, str):
            errors.append(f'{directory}: fact {key!r} anchor {anchor!r} must be a string')
            continue
        if not value.strip():
            # An empty (or whitespace-only) value must not count toward a section's completeness
            # (P2 r4121218009): a hand-authored or truncated snapshot could otherwise satisfy the
            # "at least one fact per section" requirement with no actual factual content.
            errors.append(f'{directory}: fact {key!r} value must be non-empty after stripping whitespace')
            continue
        if len(value) > FACT_VALUE_LIMIT:
            errors.append(f'{directory}: fact {key!r} value is {len(value)} chars, '
                          f'over the {FACT_VALUE_LIMIT}-char cap (no full page text)')
        # key/anchor are bounded too, not only value (P2 r4120883667).
        if len(key) > FIELD_ID_LIMIT:
            errors.append(f'{directory}: fact key {key!r} is {len(key)} chars, over the '
                          f'{FIELD_ID_LIMIT}-char cap')
        if len(anchor) > FIELD_ID_LIMIT:
            errors.append(f'{directory}: fact {key!r} anchor is {len(anchor)} chars, over the '
                          f'{FIELD_ID_LIMIT}-char cap')
        # The ratio/total-byte caps below count key + anchor + value together (P2 r4120883667): a
        # page cannot be smuggled out by padding key/anchor instead of value.
        field_bytes = len(value.encode('utf-8')) + len(key.encode('utf-8')) + len(anchor.encode('utf-8'))
        field_chars = len(value) + len(key) + len(anchor)
        total_value_bytes += field_bytes
        if key in seen_keys:
            errors.append(f'{directory}: duplicate fact key {key!r}')
        seen_keys.add(key)
        section = fact['section']
        if section not in pages_by_section:
            errors.append(f'{directory}: fact {key!r} references section {section!r}, '
                          f'which is not a manifest.json page')
        fact_count_by_section[section] = fact_count_by_section.get(section, 0) + 1
        value_chars_by_section[section] = value_chars_by_section.get(section, 0) + field_chars

    # Every required manual section (and a "captured" spell library) needs at least one fact
    # (P2 r4120578800).
    for section in MANUAL_SECTIONS:
        if section in pages_by_section and fact_count_by_section.get(section, 0) < 1:
            errors.append(f'{directory}: manual section {section!r} has zero facts')
    if spells == 'captured' and fact_count_by_section.get('spells', 0) < 1:
        errors.append(f'{directory}: "spells" is "captured" but has zero facts')

    # Per-section bounds (P1 r4120578784): a fact-count cap (the spell library gets its own,
    # compatible bound for structured rows -- P2 r4120758016), and total fact chars for a page must
    # stay at or below a fraction of that page's own visible-text length -- a page copied in
    # bounded-size chunks still fails this even though each chunk individually passes the per-value
    # cap.
    for section, count in fact_count_by_section.items():
        cap = facts_cap_for_section(section)
        if count > cap:
            errors.append(f'{directory}: section {section!r} has {count} facts, over the '
                          f'{cap}-fact-per-section cap')
        page = pages_by_section.get(section)
        if page is None or not isinstance(page.get('visible_text_chars'), int):
            continue
        visible_chars = page['visible_text_chars']
        # A page that has facts must report a positive visible-text length: a zero would silently
        # skip the ratio check below while still carrying arbitrary capped facts (P2 r4120758045).
        if count >= 1 and visible_chars <= 0:
            errors.append(f'{directory}: manifest page {section!r} has {count} fact(s) but '
                          f'visible_text_chars is {visible_chars!r}; it must be a positive int for '
                          f'any page that has facts')
            continue
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

    # Generic no-smuggled-text guard (closes the whole class in one place): the RAW bytes of
    # facts.json on disk, not any parsed field, must fit the page-ratio budget plus a fixed
    # per-fact JSON-structure allowance. Whitespace padding, an oversized field a field-level check
    # missed, or any other encoding trick that inflates the file without inflating a parsed value
    # still fails this.
    total_visible_chars = sum(page['visible_text_chars'] for page in pages_by_section.values()
                              if isinstance(page.get('visible_text_chars'), int))
    allowed_raw_bytes = total_visible_chars * FACT_TO_VISIBLE_TEXT_RATIO_LIMIT + PER_FACT_JSON_OVERHEAD_BYTES * len(facts)
    raw_facts_bytes = facts_path.stat().st_size
    if raw_facts_bytes > allowed_raw_bytes + 1e-9:
        errors.append(f'{directory}: facts.json is {raw_facts_bytes} bytes on disk, over the allowed '
                      f'{allowed_raw_bytes:.0f} bytes ({FACT_TO_VISIBLE_TEXT_RATIO_LIMIT:.0%} of total '
                      f'visible-text chars + {PER_FACT_JSON_OVERHEAD_BYTES} B/fact overhead) -- looks '
                      f'like hidden text regardless of encoding')

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


def dated_dir_for_path(path):
    match = DATED_DIR_RE.match(path)
    return f'{SNAPSHOT_PREFIX}{match.group(1)}' if match else None


def parse_raw_diff_line(line):
    """Parse one `git diff --raw` line into (old_mode, new_mode, status, paths), or None if `line`
    isn't a raw diff entry. Format: `:oldmode newmode oldsha newsha status<TAB>path[<TAB>path2]`
    (a second path only for a rename/copy status)."""
    line = line.rstrip('\n')
    if not line.startswith(':'):
        return None
    meta, sep, rest = line.partition('\t')
    if not sep:
        return None
    fields = meta[1:].split()
    if len(fields) < 5:
        return None
    old_mode, new_mode, status = fields[0], fields[1], fields[4]
    return old_mode, new_mode, status, rest.split('\t')


def find_immutability_violations(diff_lines, exists_at_base):
    """Dated snapshot directories are immutable once committed, hold exactly two files, and never
    a symlink (P2 r4120578808/r4120758029/r4121003204/r4121095564).

    `diff_lines` are `git diff --raw -M <base> <head> -- <SNAPSHOT_PREFIX>` lines (any iterable of
    strings); `exists_at_base(dated_dir)` reports whether that dated directory already existed at
    the base commit. Returns one violation string per offending line: a symlink mode (120000) on
    either side of the change; any `A`, `M`, `D`, `R` or `C` whose affected path's dated directory
    already existed at base -- including an `A` that only adds a new file inside an
    already-committed directory; or whose filename is not in `SNAPSHOT_FILENAMES`, even inside a
    brand-new dated directory. Only `manifest.json`/`facts.json`, as regular files, inside a
    brand-new dated directory are allowed.
    """
    violations = []
    for line in diff_lines:
        parsed = parse_raw_diff_line(line)
        if parsed is None:
            continue
        old_mode, new_mode, status, paths = parsed
        if not status or status[0] not in ('A', 'M', 'D', 'R', 'C'):
            continue
        if SYMLINK_MODE in (old_mode, new_mode):
            violations.append(f'{status}\t{"->".join(paths)} (symlink mode {SYMLINK_MODE} not '
                              f'allowed in a snapshot directory)')
            continue
        for path in paths:
            dated_dir = dated_dir_for_path(path)
            if not dated_dir:
                continue
            if exists_at_base(dated_dir):
                violations.append(f'{status}\t{path} (dated directory {dated_dir}/ exists at the PR base)')
                break  # one violation per offending diff line, even if several of its paths match
            if Path(path).name not in SNAPSHOT_FILENAMES:
                violations.append(f'{status}\t{path} (a dated directory may contain only '
                                  f'{sorted(SNAPSHOT_FILENAMES)})')
                break
    return violations


def cmd_check_immutability(base, head):
    root = subprocess.run(['git', 'rev-parse', '--show-toplevel'], capture_output=True, text=True,
                          check=True).stdout.strip()
    diff = subprocess.run(['git', 'diff', '--raw', '-M', base, head, '--', SNAPSHOT_PREFIX],
                          capture_output=True, text=True, check=True, cwd=root).stdout

    def exists_at_base(dated_dir):
        return subprocess.run(['git', 'cat-file', '-e', f'{base}:{dated_dir}'],
                              capture_output=True, cwd=root).returncode == 0

    violations = find_immutability_violations(diff.splitlines(), exists_at_base)
    if violations:
        print('tibiacom_capture: dated snapshot directories are immutable once committed -- only '
              'new dated directories may be added.', file=sys.stderr)
        for violation in violations:
            print(f'  - {violation}', file=sys.stderr)
        return 1
    print('check-immutability: OK (no edits to already-committed dated directories)')
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
    # One shared timestamp for captured_at and every page's fetched_at (P2 r4120883679): fetched_at
    # must be >= captured_at, exactly as a real `fetch` run stamps captured_at before any request.
    captured_at = utc_now()
    for section in MANUAL_SECTIONS:
        pages.append({'section': section, 'url': MANUAL_URL.format(section=section), 'fetched_at': captured_at,
                      'http_status': 200, 'sha256': 'a' * 64, 'visible_text_chars': 1000})
        facts.append({'section': section, 'anchor': 'root', 'key': f'{section}.root.1',
                      'value': f'The {section} section has a value of 7 here.'})
    manifest = {'schema': MANIFEST_SCHEMA, 'captured_at': captured_at, 'spells': SPELL_PENDING, 'pages': pages}
    facts_doc = {'schema': FACTS_SCHEMA, 'captured_at': captured_at, 'facts': facts}
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
    # Every fact under a heading uses the normalized anchor (id/slug), never the raw heading text
    # (P2 r4120883657) -- 'Moving Around' has an explicit id, 'Combat Basics' is slugified.
    assert by_key['controls.moving.1']['anchor'] == 'moving', by_key['controls.moving.1']
    assert by_key['controls.combat-basics.1']['anchor'] == 'combat-basics', by_key['controls.combat-basics.1']
    assert by_key['controls.combat-basics.3']['anchor'] == 'combat-basics', by_key['controls.combat-basics.3']
    assert all(f['anchor'] in ('moving', 'combat-basics') for f in facts), facts

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

    # Spell library gets its own compatible cap, not the manual 40-fact one (P2 r4120758016).
    assert facts_cap_for_section('spells') == SPELL_RECORDS_CAP
    assert facts_cap_for_section('controls') == FACTS_PER_SECTION_CAP

    # Every serialized text field is bounded (P2 r4120883667): key/anchor as well as value.
    assert bound_field('x' * 300) == 'x' * FIELD_ID_LIMIT
    assert bound_field('short') == 'short'
    long_id_html = ('<h2 id="' + ('x' * 200) + '">Heading</h2>'
                    '<p>Value is 5 here.</p><p>Another value 6 here.</p>')
    long_id_facts = extract_facts('world', long_id_html)
    assert all(len(f['anchor']) == FIELD_ID_LIMIT for f in long_id_facts), long_id_facts
    assert all(len(f['key']) <= FIELD_ID_LIMIT for f in long_id_facts), long_id_facts
    # A 200-char heading id forces key truncation on every fact under it; bound_key must still
    # keep them unique -- the heading fact and both content facts share the same long base
    # (P2 r4121003212).
    long_id_keys = [f['key'] for f in long_id_facts]
    assert len(long_id_keys) == 3, long_id_facts  # heading + 2 content facts
    assert len(long_id_keys) == len(set(long_id_keys)), long_id_keys

    assert bound_key('short', '.heading') == 'short.heading'
    long_base = 'y' * 200
    bounded_heading_key = bound_key(long_base, '.heading')
    bounded_content_key = bound_key(long_base, '.1')
    assert len(bounded_heading_key) <= FIELD_ID_LIMIT
    assert bounded_heading_key.endswith('.heading')
    assert bounded_content_key.endswith('.1')
    assert bounded_heading_key != bounded_content_key  # same long base, different suffix

    # Timestamps must be strict ISO-8601 UTC 'Z' (P2 r4120883679).
    assert parse_utc_timestamp('2026-09-28T12:00:00Z') is not None
    assert parse_utc_timestamp('2026-09-28T12:00:00') is None  # no trailing Z
    assert parse_utc_timestamp('2026-09-28 12:00:00Z') is None  # no 'T'
    assert parse_utc_timestamp(None) is None
    assert parse_utc_timestamp('not a timestamp') is None

    # Hash the raw response bytes, decode a separate copy for parsing (P2 r4120758054): a response
    # with bytes that are not valid UTF-8 must still be hashed exactly as received.
    global fetch_url, load_spell_module
    _original_fetch_url = fetch_url
    raw_sample = b'<html><body><p>Deals 42 damage.</p>broken utf8: \xff\xfe end</body></html>'
    fetch_url = lambda url: (200, raw_sample)
    try:
        result = fetch_page_or_abort('http://example.test/raw-bytes', 'a raw-bytes test page')
    finally:
        fetch_url = _original_fetch_url
    assert result is not None
    _, raw, body = result
    assert raw == raw_sample, raw
    assert hashlib.sha256(raw).hexdigest() == hashlib.sha256(raw_sample).hexdigest()
    assert hashlib.sha256(raw).hexdigest() != hashlib.sha256(body.encode('utf-8')).hexdigest()

    # Immutability (P2 r4120758029): only paths inside a brand-new dated directory are allowed --
    # including an `A` that lands inside an already-committed one. Lines are `git diff --raw`
    # shaped (P2 r4121095564 moved the check off `--name-status` to see file modes).
    def _raw(status, *paths, old_mode='100644', new_mode='100644'):
        sha = 'a' * 40
        return f':{old_mode} {new_mode} {sha} {sha} {status}\t' + '\t'.join(paths)

    existing_at_base = {SNAPSHOT_PREFIX + '2026-09-28'}
    exists_at_base = lambda dated_dir: dated_dir in existing_at_base
    assert find_immutability_violations(
        [_raw('A', f'{SNAPSHOT_PREFIX}2026-10-05/manifest.json', old_mode='000000')], exists_at_base) == []
    assert len(find_immutability_violations(
        [_raw('A', f'{SNAPSHOT_PREFIX}2026-09-28/extra.json', old_mode='000000')], exists_at_base)) == 1
    assert len(find_immutability_violations(
        [_raw('M', f'{SNAPSHOT_PREFIX}2026-09-28/manifest.json')], exists_at_base)) == 1
    assert len(find_immutability_violations(
        [_raw('D', f'{SNAPSHOT_PREFIX}2026-09-28/facts.json', new_mode='000000')], exists_at_base)) == 1
    assert len(find_immutability_violations(
        [_raw('R100', f'{SNAPSHOT_PREFIX}2026-09-28/facts.json', f'{SNAPSHOT_PREFIX}2026-09-28/renamed.json')],
        exists_at_base)) == 1
    assert find_immutability_violations(
        [_raw('R100', f'{SNAPSHOT_PREFIX}2026-10-05/manifest.json', f'{SNAPSHOT_PREFIX}2026-10-05/facts.json')],
        exists_at_base) == []
    assert find_immutability_violations([_raw('M', 'README.md')], exists_at_base) == []
    # A brand-new dated directory may still hold only manifest.json/facts.json (P2 r4121003204):
    # a raw page dump added alongside them is rejected even though the directory itself is new.
    assert len(find_immutability_violations(
        [_raw('A', f'{SNAPSHOT_PREFIX}2026-10-05/raw.html', old_mode='000000')], exists_at_base)) == 1
    # A symlink is rejected outright, mode 120000 on either side, new dir or not (P2 r4121095564).
    assert len(find_immutability_violations(
        [_raw('A', f'{SNAPSHOT_PREFIX}2026-10-05/manifest.json', old_mode='000000', new_mode='120000')],
        exists_at_base)) == 1
    assert len(find_immutability_violations(
        [_raw('M', f'{SNAPSHOT_PREFIX}2026-09-28/manifest.json', old_mode='120000')],
        exists_at_base)) == 1
    assert parse_raw_diff_line('not a raw diff line') is None
    assert parse_raw_diff_line('') is None

    import copy
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        base_manifest, base_facts = _valid_snapshot_documents()
        # The directory name is now part of the schema (P2 r4121095570): it must be the real
        # calendar date matching captured_at, so the fixture directory is named accordingly
        # instead of using tmp's own (non-date-shaped) name.
        directory = Path(tmp) / base_manifest['captured_at'][:10]
        directory.mkdir()
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

        # An empty (or whitespace-only) value is rejected outright and does not count toward a
        # section's completeness (P2 r4121218009).
        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['value'] = '   '
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('value must be non-empty' in e for e in errors), errors
        assert any('zero facts' in e for e in errors), errors  # the section now has no valid facts

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

        # A page with facts must report a positive visible_text_chars -- zero would silently skip
        # the ratio check above (P2 r4120758045).
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['visible_text_chars'] = 0
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('positive int for any page that has facts' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['spells'] = 'weird'
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('"spells" must be' in e for e in errors), errors

        # The spell library is capped at SPELL_RECORDS_CAP, not the 40-fact manual-page cap
        # (P2 r4120758016): 41 spell facts must pass; over SPELL_RECORDS_CAP must fail.
        manifest = copy.deepcopy(base_manifest)
        manifest['spells'] = 'captured'
        manifest['pages'].append({'section': 'spells', 'url': SPELL_LIBRARY_URL,
                                  'fetched_at': manifest['captured_at'],
                                  'http_status': 200, 'sha256': 'b' * 64, 'visible_text_chars': 1_000_000})
        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'] += [{'section': 'spells', 'anchor': 'root', 'key': f'spells.root.{i}',
                                'value': f'Spell {i} costs {i} mana.'} for i in range(41)]
        _write_snapshot(directory, manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert not any('fact-per-section cap' in e for e in errors), errors

        facts_doc['facts'] += [{'section': 'spells', 'anchor': 'root', 'key': f'spells.root.extra{i}',
                                'value': f'Spell extra {i} costs {i} mana.'} for i in range(41, SPELL_RECORDS_CAP + 1)]
        _write_snapshot(directory, manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any(f'{SPELL_RECORDS_CAP}-fact-per-section cap' in e for e in errors), errors

        # Duplicate manifest sections/URLs must not silently overwrite (P2 r4120758056).
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'].append(dict(manifest['pages'][0]))
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('duplicate manifest page section' in e for e in errors), errors
        assert any('duplicate manifest page url' in e for e in errors), errors

        # key/anchor are bounded too, not only value (P2 r4120883667).
        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['anchor'] = 'x' * (FIELD_ID_LIMIT + 1)
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('anchor is' in e and 'char' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['key'] = 'x' * (FIELD_ID_LIMIT + 1)
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('fact key' in e and 'char' in e for e in errors), errors

        # The 25% ratio and total-byte caps count key + anchor + value together (P2 r4120883667):
        # a short value with an oversized (but individually within-cap) key/anchor must still fail.
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['visible_text_chars'] = 100
        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['anchor'] = 'a' * FIELD_ID_LIMIT
        facts_doc['facts'][0]['key'] = 'k' * FIELD_ID_LIMIT
        facts_doc['facts'][0]['value'] = 'short'
        _write_snapshot(directory, manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('visible-text chars, over the' in e for e in errors), errors

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

        # Timestamps (P2 r4120883679): fetched_at/captured_at must be ISO-8601 UTC 'Z', equal
        # between manifest.json and facts.json, and every fetched_at within
        # [captured_at, captured_at + MAX_FETCH_RUN_SECONDS].
        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['fetched_at'] = None
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('fetched_at must be an ISO-8601' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['captured_at'] = None
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('manifest.json "captured_at" must be an ISO-8601' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['captured_at'] = None
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('facts.json "captured_at" must be an ISO-8601' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['captured_at'] = '2020-01-01T00:00:00Z'
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('must be equal' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['fetched_at'] = '2000-01-01T00:00:00Z'  # long before captured_at
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('within' in e and 'seconds of it' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        captured_dt = parse_utc_timestamp(manifest['captured_at'])
        too_late = captured_dt + timedelta(seconds=MAX_FETCH_RUN_SECONDS + 60)
        manifest['pages'][0]['fetched_at'] = too_late.strftime('%Y-%m-%dT%H:%M:%SZ')
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('within' in e and 'seconds of it' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['fetched_at'] = '2026-09-28 12:00:00'  # not ISO-8601 'Z'
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('fetched_at must be an ISO-8601' in e for e in errors), errors

        # fetch enforces the same key/anchor/value bounds on delegated spell-parser output
        # (P2 r4120883667), and stamps captured_at once, before any request (P2 r4120883679).
        class _FakeSpellModule:
            @staticmethod
            def parse_spell_library(body):
                return [{'anchor': 'a' * 300, 'key': 'k' * 300, 'value': 'x' * 300 + ' costs 5 mana.'}]

        # Enough ordinary (non-factual, so excluded) filler text that the one real fact stays
        # under the 25% ratio cap even counting its key+anchor overhead -- a tiny fixture would
        # fail that cap on overhead alone and prove nothing about this test's actual subject.
        filler = 'This is ordinary descriptive text about the interface. ' * 60
        fetch_body = f'<h2 id="x">Heading</h2><p>{filler}Value is 5 here.</p>'.encode()

        _original_fetch_url, _original_load_spell_module = fetch_url, load_spell_module
        fetch_url = lambda url: (200, fetch_body)
        load_spell_module = lambda: _FakeSpellModule()
        _original_sleep = time.sleep
        time.sleep = lambda seconds: None
        try:
            # A separate temp dir, not nested under `directory` -- `directory` is reused below as
            # the exactly-two-files fixture, and a subdirectory would pollute that check.
            with tempfile.TemporaryDirectory() as fetch_tmp:
                # A valid-date directory name (P2 r4121095570): cmd_fetch stamps captured_at with
                # today's date, so naming the output directory that way keeps verify_snapshot happy.
                fetch_out = Path(fetch_tmp) / utc_now()[:10]
                assert cmd_fetch(fetch_out) == 0
                fetched_manifest = json.loads((fetch_out / 'manifest.json').read_text(encoding='utf-8'))
                fetched_facts = json.loads((fetch_out / 'facts.json').read_text(encoding='utf-8'))
                fetched_verify_errors = verify_snapshot(fetch_out)
        finally:
            fetch_url, load_spell_module = _original_fetch_url, _original_load_spell_module
            time.sleep = _original_sleep
        spell_facts = [f for f in fetched_facts['facts'] if f['section'] == 'spells']
        assert spell_facts, fetched_facts
        assert all(len(f['anchor']) <= FIELD_ID_LIMIT for f in spell_facts), spell_facts
        assert all(len(f['key']) <= FIELD_ID_LIMIT for f in spell_facts), spell_facts
        assert all(len(f['value']) <= FACT_VALUE_LIMIT for f in spell_facts), spell_facts
        assert fetched_manifest['captured_at'] == fetched_facts['captured_at']
        assert fetched_verify_errors == [], fetched_verify_errors

        # A snapshot directory holds exactly manifest.json and facts.json (P2 r4121003204):
        # a raw page dump alongside them must fail even though both required files are valid.
        _write_snapshot(directory, base_manifest, base_facts)
        (directory / 'raw.html').write_text('<html>full page text</html>', encoding='utf-8')
        errors = verify_snapshot(directory)
        (directory / 'raw.html').unlink()
        assert any('found extra' in e and 'raw.html' in e for e in errors), errors
        assert verify_snapshot(directory) == [], verify_snapshot(directory)  # clean again

        # Closed schema (P2 r4121003195): unrecognized top-level/page/fact keys are rejected, not
        # silently ignored by every size/ratio check that only reads fields it knows by name.
        manifest = copy.deepcopy(base_manifest)
        manifest['raw_pages'] = {'controls': 'z' * 100_000}
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('manifest.json has unexpected top-level key' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['raw_pages'] = {'controls': 'z' * 100_000}
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('facts.json has unexpected top-level key' in e for e in errors), errors

        manifest = copy.deepcopy(base_manifest)
        manifest['pages'][0]['raw_html'] = 'z' * 100_000
        _write_snapshot(directory, manifest, base_facts)
        errors = verify_snapshot(directory)
        assert any('has unexpected key' in e for e in errors), errors

        facts_doc = copy.deepcopy(base_facts)
        facts_doc['facts'][0]['raw_html'] = 'z' * 100_000
        _write_snapshot(directory, base_manifest, facts_doc)
        errors = verify_snapshot(directory)
        assert any('fact has unexpected key' in e for e in errors), errors

        # Reject duplicate JSON keys before validation (P2 r4121095556): json.loads would
        # otherwise silently keep only the last of a duplicate 'value', hiding full-page text.
        assert load_json_no_duplicates('{"a": 1, "b": 2}') == {'a': 1, 'b': 2}
        try:
            load_json_no_duplicates('{"a": 1, "a": 2}')
            assert False, 'expected duplicate-key rejection'
        except ValueError as error:
            assert 'duplicate key' in str(error), error
        dup_key_facts_json = (
            '{"schema": "' + FACTS_SCHEMA + '", "captured_at": "' + base_manifest['captured_at'] + '", '
            '"facts": [{"section": "controls", "anchor": "root", "key": "controls.root.1", '
            '"value": "short", "value": "z"}]}'
        )
        (directory / 'facts.json').write_text(dup_key_facts_json, encoding='utf-8')
        errors = verify_snapshot(directory)
        assert any('facts.json is not valid JSON' in e and 'duplicate key' in e for e in errors), errors
        _write_snapshot(directory, base_manifest, base_facts)

        # Both documents must be regular files, not symlinks (P2 r4121095564).
        symlinked = directory / 'manifest.json'
        saved_manifest_text = symlinked.read_text(encoding='utf-8')
        symlinked.unlink()
        symlink_target = Path(tmp) / 'symlink-target.json'
        symlink_target.write_text(saved_manifest_text, encoding='utf-8')
        os.symlink(symlink_target, symlinked)
        errors = verify_snapshot(directory)
        symlinked.unlink()
        symlink_target.unlink()
        assert any('must be a regular file' in e for e in errors), errors
        _write_snapshot(directory, base_manifest, base_facts)
        assert verify_snapshot(directory) == [], verify_snapshot(directory)

        # The directory name must be a real calendar date matching captured_at (P2 r4121095570).
        not_a_date_dir = Path(tmp) / 'not-a-date'
        not_a_date_dir.mkdir()
        _write_snapshot(not_a_date_dir, base_manifest, base_facts)
        errors = verify_snapshot(not_a_date_dir)
        assert any('calendar date' in e for e in errors), errors

        not_a_real_date_dir = Path(tmp) / '2026-02-30'  # February has no 30th
        not_a_real_date_dir.mkdir()
        _write_snapshot(not_a_real_date_dir, base_manifest, base_facts)
        errors = verify_snapshot(not_a_real_date_dir)
        assert any('calendar date' in e for e in errors), errors

        mismatched_date_dir = Path(tmp) / '2020-01-01'
        mismatched_date_dir.mkdir()
        _write_snapshot(mismatched_date_dir, base_manifest, base_facts)  # captured_at is today
        errors = verify_snapshot(mismatched_date_dir)
        assert any('must equal the date part of captured_at' in e for e in errors), errors

        # Generic no-smuggled-text guard: verify checks facts.json's RAW byte size on disk, not
        # the parsed form -- padding that appears in no field must still fail.
        facts_file = directory / 'facts.json'
        padded = facts_file.read_text(encoding='utf-8') + (' ' * 50_000)
        facts_file.write_text(padded, encoding='utf-8')
        errors = verify_snapshot(directory)
        assert any('bytes on disk' in e for e in errors), errors
        _write_snapshot(directory, base_manifest, base_facts)
        assert verify_snapshot(directory) == [], verify_snapshot(directory)

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

    immutability_parser = subparsers.add_parser(
        'check-immutability', help='offline (needs a local git checkout): reject edits to already-committed '
                                    'dated snapshot directories between two commits')
    immutability_parser.add_argument('--base', required=True)
    immutability_parser.add_argument('--head', required=True)

    subparsers.add_parser('self-test', help='offline: run the embedded-fixture self-test')

    args = parser.parse_args(argv)
    if args.command == 'fetch':
        return cmd_fetch(args.out)
    if args.command == 'verify':
        return cmd_verify(args.directories)
    if args.command == 'check-immutability':
        return cmd_check_immutability(args.base, args.head)
    if args.command == 'self-test':
        self_test()
        return 0
    parser.error('unknown command')
    return 2


if __name__ == '__main__':
    sys.exit(main())
