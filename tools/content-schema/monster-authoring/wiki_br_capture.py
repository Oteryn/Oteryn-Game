"""Capture the infobox fields of named TibiaWiki BR pages at the programme target date (D33).

Evidence tooling only. TibiaWiki BR answers the build container with a Cloudflare bot check, so this runs on a
hosted CI runner (.github/workflows/monster-wiki-capture.yml). For each title it reads the newest revision at or
before the target cut and keeps the page id, the revision id, its timestamp, the SHA-256 of the revision text, and the
top-level fields of the first infobox (each cut to 300 characters) with the line each field starts on. Raw page text
never leaves the runner.

    python wiki_br_capture.py --out capture.json "Dark Merudri" "Count Vlarkorth"
    python wiki_br_capture.py --out population.json --titles-from samples/wiki-population-2026-09-27.json
    python wiki_br_capture.py self-test
"""
import argparse
import hashlib
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

API = 'https://www.tibiawiki.com.br/api.php'
TARGET_CUT = '2026-09-27'
CUT_TIMESTAMP = '2026-09-28T00:00:00Z'
USER_AGENT = 'OterynContentResearch/1.0 (+https://github.com/Oteryn/Oteryn-Game)'
FIELD_LIMIT = 300


def infobox(text):
    """Top-level `| key = value` fields of the first {{Infobox ...}}, honouring nested {{ }} and [[ ]]."""
    start = text.find('{{Infobox')
    if start < 0:
        return None, {}
    end_name = min(i for i in (text.find('|', start), text.find('}}', start), text.find('\n', start)) if i >= 0)
    name = text[start + 2:end_name].strip()
    depth, i, fields, current, buffer = 0, start, {}, None, []
    while i < len(text):
        pair = text[i:i + 2]
        if pair in ('{{', '[['):
            depth += 1
            if depth > 1:
                buffer.append(pair)
            i += 2
            continue
        if pair in ('}}', ']]'):
            depth -= 1
            if depth == 0:
                break
            buffer.append(pair)
            i += 2
            continue
        if text[i] == '|' and depth == 1:
            if current is not None:
                fields[current] = ''.join(buffer).strip()
            buffer, current = [], None
            i += 1
            continue
        if text[i] == '=' and depth == 1 and current is None:
            current = ''.join(buffer).strip()
            buffer = []
            i += 1
            continue
        buffer.append(text[i])
        i += 1
    if current is not None:
        fields[current] = ''.join(buffer).strip()
    return name, {key: value[:FIELD_LIMIT] for key, value in fields.items() if key and value}


def field_lines(text):
    """The 1-based line of the first `| key =` of each field name."""
    lines = {}
    for number, line in enumerate(text.splitlines(), 1):
        match = re.match(r'\s*\|\s*([^=|{}]+?)\s*=', line)
        if match:
            lines.setdefault(match.group(1).strip(), number)
    return lines


def revision_at_cut(title):
    query = {'action': 'query', 'format': 'json', 'formatversion': '2', 'prop': 'revisions', 'titles': title,
             'rvprop': 'ids|timestamp|content', 'rvslots': 'main', 'rvlimit': '1', 'rvstart': CUT_TIMESTAMP,
             'rvdir': 'older', 'redirects': '1'}
    request = urllib.request.Request(API + '?' + urllib.parse.urlencode(query), headers={'User-Agent': USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        data = json.loads(response.read().decode('utf-8'))
    page = data['query']['pages'][0]
    if page.get('missing') or not page.get('revisions'):
        return {'title': title, 'missing': True}
    rev = page['revisions'][0]
    content = rev['slots']['main']['content']
    name, fields = infobox(content)
    lines = field_lines(content)
    return {'title': title, 'page_title': page['title'], 'page_id': page['pageid'], 'revision_id': rev['revid'],
            'revision_timestamp': rev['timestamp'], 'content_sha256': hashlib.sha256(content.encode('utf-8')).hexdigest(),
            'infobox': name, 'fields': fields, 'field_lines': {key: lines[key] for key in fields if key in lines}}


def self_test():
    name, fields = infobox('x {{Infobox Criatura|nome = Dark Merudri\n| hp = 6500\n| notas = [[A|b]] {{c|d}}\n}} y')
    assert name == 'Infobox Criatura', name
    assert fields == {'nome': 'Dark Merudri', 'hp': '6500', 'notas': '[[A|b]] {{c|d}}'}, fields
    assert infobox('{{Infobox X|a=|b=1}}')[1] == {'b': '1'}
    assert infobox('no box') == (None, {})
    assert len(infobox('{{Infobox X|a=' + 'z' * 400 + '}}')[1]['a']) == FIELD_LIMIT
    assert field_lines('{{Infobox X\n| hp = 1\n| exp=2\n| hp = 3\n}}') == {'hp': 2, 'exp': 3}
    print('wiki_br_capture self-test: PASS')


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv == ['self-test']:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--titles-from', type=Path, help='a wiki population sample: every distinct wiki_title in it')
    parser.add_argument('titles', nargs='*')
    args = parser.parse_args(argv)
    titles = list(args.titles)
    if args.titles_from:
        sample = json.loads(args.titles_from.read_text(encoding='utf-8'))
        titles += sorted({m['wiki_title'] for m in sample['monsters']} - set(titles))
    if not titles:
        parser.error('no titles')
    pages = []
    for title in titles:
        for attempt in range(3):
            try:
                pages.append(revision_at_cut(title))
                break
            except (OSError, ValueError, KeyError) as error:
                if attempt == 2:
                    pages.append({'title': title, 'error': type(error).__name__})
                time.sleep(2 ** (attempt + 1))
        time.sleep(0.5)
    document = {'wiki': 'tibiawiki.com.br', 'api': API, 'target_cut': TARGET_CUT, 'cut_timestamp': CUT_TIMESTAMP,
                'field_limit': FIELD_LIMIT, 'pages': pages}
    args.out.write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    found = sum(1 for p in pages if 'revision_id' in p)
    errors = sum(1 for p in pages if 'error' in p)
    print(f'pages: {len(pages)}, found: {found}, missing: {len(pages) - found - errors}, errors: {errors}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
