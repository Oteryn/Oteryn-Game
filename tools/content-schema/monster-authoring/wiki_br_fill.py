"""Pick the TibiaWiki BR health and experience values that fill gaps left by the Fandom wiki (owner decision D43).

Evidence tooling only. The owner's source order puts TibiaWiki (Fandom) before TibiaWiki BR, so a BR value is used
only where the Fandom page at the target cut has no certain value: the page is missing, or its field is empty or
marked uncertain (`?`, `~`) or unparsed. Only `hp` and `exp` are filled (the owner declined BR element modifiers, whose
"100%" is often the template default, and BR speed, which is rarely given). A BR value is used only when it is a plain
number.

    python wiki_br_fill.py --capture wiki-br-population.json --cache /tmp/oteryn-wiki-cache
    python wiki_br_fill.py self-test
"""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import wiki_compare as wc

ROOT = Path(__file__).resolve().parent
FANDOM_SAMPLE = ROOT / 'samples' / 'wiki-population-2026-09-27.json'
OUT = ROOT / 'samples' / 'wiki-br-fill-2026-09-27.json'
FIELDS = {'hp': 'max_health', 'exp': 'experience'}
NUMBER = re.compile(r'\d{1,3}(?:\.\d{3})+|\d+')


def plain_number(raw):
    """A plain non-negative integer ("7900" or "7.900"), otherwise None."""
    raw = (raw or '').strip()
    return int(raw.replace('.', '')) if NUMBER.fullmatch(raw) else None


def fandom_certain(raw):
    """The D15 rule of wiki_compare.py: an exact number, not uncertain or approximate."""
    return wc.number(raw) is not None


def fill(capture, fandom_sample, fandom_infobox):
    """The fill records: one per monster whose BR page gives a plain hp or exp where the Fandom value is not certain."""
    br = {page['title']: page for page in capture['pages'] if 'revision_id' in page}
    records = []
    for monster in fandom_sample['monsters']:
        page = br.get(monster['wiki_title'])
        if not page:
            continue
        fandom = fandom_infobox(monster) if monster['status'] == 'COMPARED' else {}
        fields = {}
        for label, field in FIELDS.items():
            value = plain_number(page['fields'].get(label))
            if value is None or fandom_certain(fandom.get(label)):
                continue
            fields[field] = {'br_raw': page['fields'][label], 'br': value, 'br_line': page['field_lines'].get(label, 1),
                             'fandom_raw': fandom.get(label, '') if monster['status'] == 'COMPARED' else None}
        if fields:
            records.append({'monster': monster['monster'], 'br_title': page['page_title'], 'page_id': page['page_id'],
                            'revision_id': page['revision_id'], 'revision_timestamp': page['revision_timestamp'],
                            'content_sha256': page['content_sha256'], 'fandom_status': monster['status'], 'fields': fields})
    return records


def self_test():
    assert plain_number('7900') == 7900 and plain_number('7.900') == 7900 and plain_number('0') == 0
    assert plain_number('7900?') is None and plain_number('~20000') is None and plain_number('') is None
    assert plain_number('1-2') is None and plain_number('1.50') is None
    page = {'title': 'A', 'page_title': 'A', 'page_id': 1, 'revision_id': 2, 'revision_timestamp': 't', 'content_sha256': 'h',
            'fields': {'hp': '7900', 'exp': '0'}, 'field_lines': {'hp': 5, 'exp': 6}}
    capture = {'pages': [page, {'title': 'B', 'missing': True}]}
    sample = {'monsters': [{'monster': 'a', 'wiki_title': 'A', 'status': 'COMPARED'},
                           {'monster': 'b', 'wiki_title': 'B', 'status': 'COMPARED'}]}
    records = fill(capture, sample, lambda m: {'hp': '?', 'exp': '0'})
    assert [r['monster'] for r in records] == ['a'] and list(records[0]['fields']) == ['max_health'], records
    assert records[0]['fields']['max_health'] == {'br_raw': '7900', 'br': 7900, 'br_line': 5, 'fandom_raw': '?'}
    missing = fill(capture, {'monsters': [{'monster': 'a', 'wiki_title': 'A', 'status': 'WIKI_PAGE_MISSING'}]}, None)
    assert list(missing[0]['fields']) == ['max_health', 'experience'] and missing[0]['fields']['experience']['fandom_raw'] is None
    print('wiki_br_fill self-test: PASS')


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv == ['self-test']:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--capture', type=Path, required=True, help='wiki-br-population.json from monster-wiki-capture.yml')
    parser.add_argument('--cache', type=Path, required=True, help='the wiki_compare.py page cache of the Fandom sample')
    parser.add_argument('--out', type=Path, default=OUT)
    args = parser.parse_args(argv)
    capture_bytes = args.capture.read_bytes()
    capture = json.loads(capture_bytes)
    sample = json.loads(FANDOM_SAMPLE.read_text(encoding='utf-8'))

    def fandom_infobox(monster):
        return wc.infobox(wc.fetch(monster['wiki_title'], args.cache)['cut']['content'])

    records = fill(capture, sample, fandom_infobox)
    document = {
        'source': capture['api'], 'target_cut': capture['target_cut'], 'cut_timestamp': capture['cut_timestamp'],
        'rule': 'Owner decision D43: a TibiaWiki BR hp or exp fills the value only where the Fandom page at the target cut '
                'is missing or its value is empty, uncertain or unparsed; the BR value must be a plain number.',
        'capture': {'workflow': '.github/workflows/monster-wiki-capture.yml', 'sha256': hashlib.sha256(capture_bytes).hexdigest(),
                    'pages': len(capture['pages']), 'found': sum(1 for p in capture['pages'] if 'revision_id' in p)},
        'fandom_sample': FANDOM_SAMPLE.name,
        'counts': {field: sum(1 for r in records if field in r['fields']) for field in FIELDS.values()},
        'monsters': records}
    args.out.write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'monsters': len(records), **document['counts']}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
