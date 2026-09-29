"""Monsters of game version 15.30 that CrystalServer has and Canary does not, converted from a pinned CrystalServer revision.

Evidence tooling only: every output is OTS_HYPOTHESIS_ONLY source evidence, never Game truth. The game version is 15.30
(OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md) and no Canary branch has the Summer Update 2026 monsters,
so they come from the CrystalServer `summer-update` branch, pinned by commit (§2 of that decision). The files are in the
Canary monster format and are converted by the normal converter (canary_batch.py) with its Canary engine rules; the loot
names resolve through the 15.30 item tables of the same Crystal commit. The reference-date wiki values (D15) and the
Tibia.com library values (D47) are adopted over the Crystal values as for every Canary monster.

    python crystal_batch.py wiki --canary <Canary checkout> --crystal <Crystal checkout>   # refresh WIKI
    python crystal_batch.py self-test --canary <Canary checkout> --crystal <Crystal checkout>
"""
import argparse
import json
import os
import re
import subprocess
from pathlib import Path

import canary_batch as cb

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'zimbadev/crystalserver'
BRANCH = 'summer-update'
REVISION = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
READ = '2026-09-28'
MONSTER_DIR = 'data-global/monster/summer_update_2026'
WIKI = ROOT / 'samples' / 'wiki-population-crystal-00ce02a5-2026-09-27.json'
SHARED_SOURCES = ('data/items/items.xml', 'data/items/appearances.dat')
SOURCE_NOTE = (f'{REPOSITORY} branch {BRANCH} at {REVISION} (read {READ}): the only OTS source with the 15.30 Summer Update '
               f'monsters; Canary main and every active Canary branch lack them. Engine rules are those of the converter '
               f'(Canary {cb.REVISION[:8]}); loot names resolve through the 15.30 item tables of this commit.')


def require_revision(crystal):
    """Every output records REVISION, so the Crystal checkout must be at that commit."""
    head = subprocess.run(['git', '-C', str(crystal), 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    if head != REVISION:
        raise SystemExit(f'{crystal} is at {head or "no git commit"}, not the pinned CrystalServer revision {REVISION}')


def converter(canary, crystal):
    """A converter that reads the Crystal monster files and item tables and keeps the Canary engine rules."""
    require_revision(crystal)
    objects = cb.load_appearance_objects(crystal / 'data/items/appearances.dat')
    items = cb.load_items_xml(crystal / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    result = cb.Converter(canary, objects, items, names, index)
    result.monster_root, result.monster_dir = crystal, MONSTER_DIR
    result.source = {'repository': REPOSITORY, 'revision': REVISION}
    return result


def created_name(path):
    match = re.search(r'Game\.createMonsterType\("([^"]+)"', path.read_text(encoding='utf-8', errors='replace'))
    return match.group(1) if match else None


def files(canary, crystal):
    """Relative paths (no .lua) of the Crystal monsters whose name no Canary monster file creates."""
    require_revision(crystal)
    canary_slugs = {cb.slug(name) for path in (canary / cb.MONSTER_DIR).rglob('*.lua') if (name := created_name(path))}
    result = []
    for path in sorted((crystal / MONSTER_DIR).rglob('*.lua')):
        if cb.slug(created_name(path)) not in canary_slugs:
            result.append(str(path.relative_to(crystal / MONSTER_DIR))[:-4])
    return result


def shared_sources(crystal):
    return {p: cb.blob_id((crystal / p).read_bytes()) for p in SHARED_SOURCES}


def wiki(canary, crystal, cache):
    import wiki_compare as wc
    cb.CONVERTER = converter(canary, crystal)
    results, skipped = [], []
    for relative in files(canary, crystal):
        try:
            results.append(wc.compact(wc.compare(relative, canary, None, cache)))
        except Exception as exc:  # files the converter cannot convert (see population_census.py)
            skipped.append({'file': relative, 'error': f'{type(exc).__name__}: {str(exc).splitlines()[0][:100]}'})
    head = {'source': 'TibiaWiki (Fandom), CC BY-SA; only compared facts are recorded', 'api': wc.API,
            'target_cut': wc.TARGET_CUT, 'cut_rule': f'last revision at or before {wc.CUT_TIMESTAMP}',
            'classification': 'Wiki = player-observed reference evidence; Crystal = OTS_HYPOTHESIS_ONLY',
            'monster_source': {'repository': REPOSITORY, 'branch': BRANCH, 'revision': REVISION, 'monster_dir': MONSTER_DIR},
            'scope': 'The Crystal monsters of crystal_batch.py; the same compact rows as wiki-population-2026-09-27.json.',
            'not_converted': skipped, 'monsters': []}
    text = json.dumps(head, ensure_ascii=False, indent=2)[:-len('\n  "monsters": []\n}')]
    lines = ',\n'.join('    ' + json.dumps(r, ensure_ascii=False, separators=(',', ':')) for r in results)
    WIKI.write_text(text + '\n  "monsters": [\n' + lines + '\n  ]\n}\n', encoding='utf-8', newline='\n')
    print(json.dumps({'out': str(WIKI), 'monsters': len(results), 'not_converted': len(skipped)}))


def self_test(canary, crystal):
    selected = files(canary, crystal)
    conv = converter(canary, crystal)
    conv.pending_definitions = set()
    slug, _, _, _, manifest, source = conv.convert(selected[0])
    assert manifest['sources'][0] == {'repository': REPOSITORY, 'revision': REVISION}, manifest['sources'][0]
    assert source['file'] == f'{MONSTER_DIR}/{selected[0]}.lua' and not os.path.isabs(source['file'])
    assert all(e['source_file'] == source['file'] for e in manifest['entries'] if e['source_index'] == 0)
    print(json.dumps({'self_test': 'ok', 'monsters': len(selected), 'first': slug}))


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('command', choices=('wiki', 'self-test'))
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--cache', type=Path, default=Path('/tmp/oteryn-wiki-cache'))
    args = parser.parse_args()
    if args.command == 'wiki':
        args.cache.mkdir(parents=True, exist_ok=True)
        wiki(args.canary, args.crystal, args.cache)
    else:
        self_test(args.canary, args.crystal)


if __name__ == '__main__':
    main()
