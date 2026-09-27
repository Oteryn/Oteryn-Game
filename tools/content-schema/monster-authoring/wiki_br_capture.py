"""Capture the infobox fields of named TibiaWiki BR pages at the programme target date (D33).

Evidence tooling only. TibiaWiki BR answers the build container with a Cloudflare bot check, so this runs on a
hosted CI runner (.github/workflows/monster-wiki-capture.yml). For each title it reads the newest revision at or
before the target cut and keeps the revision id, its timestamp and the top-level fields of the first infobox, each
cut to 300 characters. Raw page text never leaves the runner.

    python wiki_br_capture.py --out capture.json "Dark Merudri" "Count Vlarkorth"
    python wiki_br_capture.py self-test
"""
import argparse
import json
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
    return name, {key: value[:FIELD_LIMIT] for key, value in fields.items() if key}


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
    name, fields = infobox(rev['slots']['main']['content'])
    return {'title': title, 'page_title': page['title'], 'revision_id': rev['revid'], 'revision_timestamp': rev['timestamp'],
            'infobox': name, 'fields': fields}


def self_test():
    name, fields = infobox('x {{Infobox Criatura|nome = Dark Merudri\n| hp = 6500\n| notas = [[A|b]] {{c|d}}\n}} y')
    assert name == 'Infobox Criatura', name
    assert fields == {'nome': 'Dark Merudri', 'hp': '6500', 'notas': '[[A|b]] {{c|d}}'}, fields
    assert infobox('no box') == (None, {})
    assert len(infobox('{{Infobox X|a=' + 'z' * 400 + '}}')[1]['a']) == FIELD_LIMIT
    print('wiki_br_capture self-test: PASS')


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv == ['self-test']:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('titles', nargs='+')
    args = parser.parse_args(argv)
    pages = []
    for title in args.titles:
        pages.append(revision_at_cut(title))
        time.sleep(0.5)
    document = {'wiki': 'tibiawiki.com.br', 'api': API, 'target_cut': TARGET_CUT, 'cut_timestamp': CUT_TIMESTAMP,
                'field_limit': FIELD_LIMIT, 'pages': pages}
    args.out.write_text(json.dumps(document, ensure_ascii=False, indent=1) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({p['title']: p.get('revision_id', 'missing') for p in pages}, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    sys.exit(main())
