"""Prepare missing-field observations; secondary pages never become admitted Creature data.

Read-only HTTP research uses public Tibiopedia presentation cookies, at most one request per
second. Records contain facts and citations, never article prose. Name lookup is explicitly
unverified identity evidence; current pages do not prove the 2026-09-27 reference cut.
"""
import argparse
import hashlib
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from fractions import Fraction
from html.parser import HTMLParser
from pathlib import Path


LABELS = {
    'Doświadczenie': 'experience', 'Życie': 'max_health', 'Pancerz': 'armor',
    'Prędkość': 'speed', 'Mitygacja': 'mitigation_percent', 'Mitigacja': 'mitigation_percent',
    'Przywołanie': 'summon_mana_cost', 'Zauroczenie': 'convince_mana_cost',
    'Punkty uroku': 'charm_points', 'Trudność': 'difficulty', 'Rzadkość': 'occurrence',
    'Klasa': 'class', 'Rola': 'role',
}
PENDING = ['UNVERIFIED_IDENTITY_BINDING', 'UNKNOWN_TARGET_CUT_CONTINUITY', 'PENDING_SOURCE_PRIORITY']


def normalized(text):
    return ' '.join(text.split())


class FactRows(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.rows, self.stack, self.title = [], [], []
        self.in_title = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'title':
            self.in_title = True
        elif tag == 'tr':
            self.stack.append({'line': self.getpos()[0], 'cells': [], 'current': None})
        elif tag in ('td', 'th') and self.stack:
            cell = {'line': self.getpos()[0], 'text': [], 'images': []}
            self.stack[-1]['cells'].append(cell)
            self.stack[-1]['current'] = cell
        elif tag == 'img' and self.stack and self.stack[-1]['current'] is not None:
            self.stack[-1]['current']['images'].append(
                {key: attrs[key] for key in ('src', 'alt', 'title') if key in attrs})

    def handle_endtag(self, tag):
        if tag == 'title':
            self.in_title = False
        elif tag == 'tr' and self.stack:
            self.rows.append(self.stack.pop())
        elif tag in ('td', 'th') and self.stack:
            self.stack[-1]['current'] = None

    def handle_data(self, data):
        if self.in_title:
            self.title.append(data)
        if self.stack and self.stack[-1]['current'] is not None:
            self.stack[-1]['current']['text'].append(data)


def numeric(raw):
    text = re.sub(r'\s+', '', raw).removesuffix('%').replace(',', '.')
    if not re.fullmatch(r'\d+(?:\.\d+)?', text):
        return None
    value = Fraction(text)
    return int(value) if value.denominator == 1 else {'numerator': value.numerator, 'denominator': value.denominator}


def extract_facts(html):
    parser = FactRows()
    parser.feed(html)
    title = normalized(''.join(parser.title))
    if not title.startswith('Potwory:') or 'monster_base_stats' not in html:
        return {'status': 'NOT_CREATURE_PAGE', 'title': title, 'fields': {}}
    fields = {}
    for row in parser.rows:
        cells = row['cells']
        if len(cells) < 2:
            continue
        label = normalized(''.join(cells[0]['text'])).rstrip(':')
        if label not in LABELS:
            continue
        cell = cells[-1]
        raw = normalized(''.join(cell['text']))
        images = cell['images']
        if not raw:
            raw = next((image.get('title') or image.get('alt') for image in images
                        if image.get('title') or image.get('alt')), '')
        field = LABELS[label]
        value = raw if field in ('class', 'role', 'difficulty', 'occurrence') else numeric(raw)
        fields[field] = {'status': 'OBSERVED' if value not in (None, '') else 'UNKNOWN',
                         'raw': raw, 'value': value, 'source_label': label,
                         'source_line': cells[0]['line'], 'images': images}
    return {'status': 'CURRENT_PAGE_OBSERVATIONS', 'title': title, 'fields': fields}


def missing_inventory(visible):
    cases = []
    for creature in visible['monsters']:
        fields = creature['fields']
        missing = []
        if fields['mitigation_percent']['status'] == 'SOURCE_UNSPECIFIED':
            missing.append('mitigation_percent')
        if fields['charm_points']['status'] == 'NOT_APPLICABLE_IN_SOURCE':
            missing.append('bestiary')
        if missing:
            cases.append({'identity': creature['identity'], 'name': creature['name'], 'missing_fields': missing})
    return cases


def prepare_packet(case, response):
    facts = extract_facts(response.get('html', '')) if response.get('status') == 200 else {
        'status': 'SOURCE_UNAVAILABLE', 'fields': {}}
    candidates = {field: fact for field, fact in facts['fields'].items() if fact['status'] == 'OBSERVED'}
    return {**case, 'schema': 'oteryn-missing-visible-source-evidence-v1',
            'admission_authorized': False, 'runtime_qualified': False,
            'lookup_basis': 'DISPLAY_NAME_ONLY_NOT_AN_IDENTITY_BINDING',
            'pending_issues': PENDING, 'target_cut': '2026-09-27',
            'source': {key: value for key, value in response.items() if key != 'html'},
            'observation': facts, 'typed_observed_candidates': candidates,
            'native_values_changed': 0,
            'missing_field_status': {field: 'UNKNOWN_PENDING_PRIMARY_EVIDENCE' for field in case['missing_fields']}}


def fetch(name):
    url = 'https://tibiopedia.pl/monsters/' + urllib.parse.quote(name.replace(' ', '_'), safe='')
    result = {'url': url, 'transport': 'normal_http',
              'retrieved_at': datetime.now(timezone.utc).isoformat()}
    request = urllib.request.Request(url, headers={
        'User-Agent': 'OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)',
        'Cookie': 'tp_lang=pl; tp_layout=library'})
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            body = response.read(2 * 1024 * 1024 + 1)
            if len(body) > 2 * 1024 * 1024:
                return {**result, 'status': 'RESPONSE_SIZE_LIMIT', 'bytes': len(body)}
            result.update(status=response.status, resolved_url=response.url, bytes=len(body),
                          body_sha256=hashlib.sha256(body).hexdigest(), html=body.decode('utf-8', 'replace'))
    except urllib.error.HTTPError as error:
        result.update(status=error.code, error=error.reason)
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        result.update(status='UNAVAILABLE', error=type(error).__name__)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--visible', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--max-pages', type=int)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    cases = missing_inventory(json.loads(args.visible.read_text(encoding='utf-8')))
    packets, halted, fetched, last = [], False, 0, 0.0
    for case in cases:
        slug = case['identity'].rsplit('.', 1)[-1]
        path = args.out / (slug + '.json')
        packet = None
        if path.exists():
            packet = json.loads(path.read_text(encoding='utf-8'))
            if (packet['identity'], packet['missing_fields']) != (case['identity'], case['missing_fields']):
                raise ValueError('cached packet input mismatch')
            if packet['source'].get('status') in (403, 429):
                halted = True
            if packet['source'].get('status') == 'NOT_REQUESTED':
                packet = None
        if packet is None:
            if halted or args.max_pages is not None and fetched >= args.max_pages:
                response = {'status': 'NOT_REQUESTED', 'transport': 'normal_http',
                            'reason': 'host access blocked' if halted else 'bounded page limit'}
            else:
                time.sleep(max(0, 1.0 - (time.monotonic() - last)))
                last = time.monotonic()
                response = fetch(case['name'])
                fetched += 1
                halted = response.get('status') in (403, 429)
            packet = prepare_packet(case, response)
            path.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        packets.append(packet)
        if len(packets) % 50 == 0:
            print(json.dumps({'packets': len(packets), 'requests_this_run': fetched}), flush=True)
    counts = {}
    for packet in packets:
        status = packet['observation']['status']
        counts[status] = counts.get(status, 0) + 1
    summary = {'schema': 'oteryn-missing-visible-source-evidence-summary-v1',
               'input_visible_sha256': hashlib.sha256(args.visible.read_bytes()).hexdigest(),
               'helper_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               'cases': len(packets), 'source_status_counts': counts,
               'requests_this_run': fetched, 'admission_authorized': False, 'native_values_changed': 0,
               'observed_mitigation_candidates': sum('mitigation_percent' in p['typed_observed_candidates'] for p in packets),
               'observed_bestiary_fragments': sum('charm_points' in p['typed_observed_candidates'] for p in packets),
               'pending_issues': PENDING,
               'packet_sha256': {p['identity']: hashlib.sha256((args.out / (p['identity'].rsplit('.', 1)[-1] + '.json')).read_bytes()).hexdigest()
                                 for p in packets}}
    (args.out / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({key: value for key, value in summary.items() if key != 'packet_sha256'}), flush=True)


if __name__ == '__main__':
    main()
