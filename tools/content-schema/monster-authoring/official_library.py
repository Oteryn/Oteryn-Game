"""Tibia.com creature library facts for the Canary monsters (owner decision D47, 2026-09-28).

Evidence tooling only. Reads the TibiaData v4 creature records captured on CAPTURED (the Tibia.com library has no
revision history, and tibia.com itself answers with a Cloudflare challenge) and writes SAMPLE: per Canary monster
whose library entry it identifies, the library health and experience. The converter adopts them over Canary and the
reference-date wiki (canary_batch.Converter.adopt_official). D47 accepts the capture as the reference-date state
because no Tibia.com news item dated after the reference date and up to the capture changes a creature
(NEWS_CHECKED).

Usage: python official_library.py --canary <Canary checkout> --captures <dir of <race>.json> [--out FILE]
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
SAMPLE = ROOT / 'samples' / 'official-library-2026-09-28.json'
CAPTURED = '2026-09-28'
API = 'https://api.tibiadata.com/v4/creature/'
LIBRARY = 'https://www.tibia.com/library/?subtopic=creatures&race='
# Tibia.com news after the reference date (2026-09-27) up to the capture; none changes a creature.
NEWS_CHECKED = [
    {'id': 8947, 'date': '2026-09-28', 'title': 'Exaltation Overload'},
    {'id': 8979, 'date': '2026-09-28', 'title': 'server save ticker'},
]


def norm(text):
    return re.sub(r'[^a-z]', '', text.lower())


def plural(name):
    name = name.lower()
    for singular, many in (('man', 'men'), ('mouse', 'mice'), ('wolf', 'wolves'), ('tooth', 'teeth')):
        if name.endswith(singular):
            return name[:-len(singular)] + many
    if re.search(r'[^aeiou]y$', name):
        return name[:-1] + 'ies'
    if re.search(r'(s|x|z|ch|sh)$', name):
        return name + 'es'
    return name + 's'


def facts_digest(facts):
    return hashlib.sha256(json.dumps(facts, sort_keys=True, separators=(',', ':')).encode('utf-8')).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--captures', required=True, type=Path)
    parser.add_argument('--out', type=Path, default=SAMPLE)
    args = parser.parse_args()
    library = [json.loads(path.read_text(encoding='utf-8')) for path in sorted(args.captures.glob('*.json'))]
    by_key = {}
    for entry in library:
        by_key.setdefault(norm(entry['race']), []).append(entry)
        by_key.setdefault(norm(entry['name']), []).append(entry)
    monsters = []
    for path in sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua')):
        match = re.search(r'Game\.createMonsterType\("([^"]+)"', path.read_text(encoding='utf-8', errors='replace'))
        if not match:
            continue
        name = match.group(1)
        found = {e['race']: e for key in (norm(name), norm(plural(name))) for e in by_key.get(key, [])}
        if len(found) != 1:
            continue
        entry = next(iter(found.values()))
        facts = {'race': entry['race'], 'name': entry['name'], 'hitpoints': entry['hitpoints'],
                 'experience_points': entry['experience_points']}
        fields = {}
        if isinstance(entry['hitpoints'], int) and entry['hitpoints'] > 0:
            fields['max_health'] = entry['hitpoints']
        if isinstance(entry['experience_points'], int) and entry['experience_points'] >= 0:
            fields['experience'] = entry['experience_points']
        if not fields:
            continue
        monsters.append({'monster': cb.slug(name),
                         'canary_name': name, 'title': entry['name'], 'url': LIBRARY + entry['race'],
                         'captured': CAPTURED, 'content_sha256': facts_digest(facts), 'facts': facts, 'fields': fields})
    monsters.sort(key=lambda m: m['monster'])
    if len({m['monster'] for m in monsters}) != len(monsters):
        raise SystemExit('two library entries map to one Canary monster')
    sample = {'schema': 'OTERYN_OFFICIAL_LIBRARY_FACTS/v1', 'decision': 'D47', 'captured': CAPTURED, 'api': API,
              'library_entries': len(library), 'news_checked': NEWS_CHECKED, 'monsters': monsters}
    args.out.write_text(json.dumps(sample, indent=1, ensure_ascii=False) + '\n', encoding='utf-8')
    print(json.dumps({'library_entries': len(library), 'monsters': len(monsters)}))


if __name__ == '__main__':
    main()
