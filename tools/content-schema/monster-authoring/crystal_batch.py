"""Monsters of game version 15.30 that CrystalServer has and Canary does not, converted from a pinned CrystalServer revision.

Evidence tooling only: every output is OTS_HYPOTHESIS_ONLY source evidence, never Game truth. The game version is 15.30
(OTERYN_GAME_VERSION_1530_AND_OTS_BRANCHES_DECISION_20260928.md) and no Canary branch has the Summer Update 2026 monsters,
so they come from the CrystalServer `summer-update` branch, pinned by commit (§2 of that decision). The files are in the
Canary monster format and are converted by the normal converter (canary_batch.py) with its Canary engine rules; the loot
names resolve through the 15.30 item tables of the same Crystal commit. The reference-date wiki values (D15) and the
Tibia.com library values (D47) are adopted over the Crystal values as for every Canary monster. EXTRA_MONSTERS adds an
explicit allow-list of ordinary monsters that CrystalServer keeps in other directories.

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
import spell_scripts

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'zimbadev/crystalserver'
BRANCH = 'summer-update'
REVISION = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
READ = '2026-09-28'
MONSTER_ROOT = 'data-global/monster'
MONSTER_DIR = MONSTER_ROOT + '/summer_update_2026'
# Ordinary client 15.30 monsters that Canary lacks and CrystalServer keeps outside MONSTER_DIR (paths under MONSTER_ROOT). An
# explicit allow-list, not a directory widening: the 28 `real monster` rows with a `crystal_other_dir` of
# samples/wiki-only-candidates-2026-09-30.json (test_crystal_batch.py compares them). Quest, event, raid and summon-like
# creatures of the same directories stay out until they are decided one by one.
EXTRA_MONSTERS = (
    'inkborn/bluebeak', 'inkborn/bramble_wyrmling', 'inkborn/cinder_wyrmling', 'inkborn/crusader', 'inkborn/hawk_hopper',
    'inkborn/headwalker', 'inkborn/ink_splash', 'inkborn/lion_hydra', 'inkborn/shell_drake',
    'humanoids/gloom_maw', 'humanoids/norcferatu_heartless', 'humanoids/norcferatu_nightweaver', 'humanoids/varg',
    'undeads/dworc_shadowstalker', 'undeads/orclops_bloodbreaker',
    'winter_update_2025/creepy_crawler', 'winter_update_2025/crypt_construct', 'winter_update_2025/crypt_fiend',
    'winter_update_2025/crypt_mage', 'winter_update_2025/cyclursus', 'winter_update_2025/haunted_hunter',
    'winter_update_2025/night_harpy', 'winter_update_2025/raubritter_chastener', 'winter_update_2025/raubritter_marksman',
    'winter_update_2025/raubritter_skirmisher', 'winter_update_2025/roaming_dread', 'winter_update_2025/stag',
    'winter_update_2025/walking_dread',
)
WIKI = ROOT / 'samples' / 'wiki-population-crystal-00ce02a5-2026-09-27.json'
SHARED_SOURCES = ('data/items/items.xml', 'data/items/appearances.dat')
SOURCE_NOTE = (f'{REPOSITORY} branch {BRANCH} at {REVISION} (read {READ}): the only OTS source with the 15.30 Summer Update '
               f'monsters; Canary main and every active Canary branch lack them. Engine rules are those of the converter '
               f'(Canary {cb.REVISION[:8]}); loot names resolve through the 15.30 item tables of this commit.')


def git(root, *args):
    return subprocess.run(['git', '-C', str(root), *args], capture_output=True, text=True).stdout.strip()


# The Canary files the converter reads: its monster names (files()) and the engine rules and spell scripts.
CANARY_READ = (cb.MONSTER_DIR, cb.EFFECT_CONSTANTS, spell_scripts.SPELL_LIB, spell_scripts.ENGINE_DEFINITIONS,
               *spell_scripts.SCRIPT_DIRS)


def require_pinned(root, name, revision, paths, extra):
    """Every output records the pinned revisions, so the files read must be those of the commits: HEAD is the revision,
    the tracked files read are clean and, where `extra` is set (paths read by globbing the checkout), no untracked or
    ignored file lies in them."""
    head = git(root, 'rev-parse', 'HEAD')
    if head != revision:
        raise SystemExit(f'{root} is at {head or "no git commit"}, not the pinned {name} revision {revision}')
    flags = ['--untracked-files=all', '--ignored'] if extra else ['--untracked-files=no']
    dirty = git(root, 'status', '--porcelain', *flags, '--', *paths)
    if dirty:
        raise SystemExit(f'{root} has local changes to {name} files read at {revision}:\n{dirty}')


def require_revision(canary, crystal):
    """The Crystal monsters are listed from the REVISION tree (files()), so only its tracked files need to be clean; the
    Canary engine rules and spell scripts are read by globbing, so their paths also admit no untracked or ignored file."""
    extra = tuple(f'{MONSTER_ROOT}/{relative}.lua' for relative in EXTRA_MONSTERS)
    require_pinned(crystal, 'CrystalServer', REVISION, (MONSTER_DIR, *extra, *SHARED_SOURCES), extra=False)
    require_pinned(crystal, 'CrystalServer', REVISION, spell_scripts.EXTRA_SCRIPT_DIRS, extra=True)
    require_pinned(canary, 'Canary', cb.REVISION, CANARY_READ, extra=True)


def converter(canary, crystal):
    """A converter that reads the Crystal monster files and item tables and keeps the Canary engine rules."""
    require_revision(canary, crystal)
    objects = cb.load_appearance_objects(crystal / 'data/items/appearances.dat')
    items = cb.load_items_xml(crystal / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    result = cb.Converter(canary, objects, items, names, index)
    result.monster_root, result.monster_dir = crystal, MONSTER_ROOT
    # Monster spells that only CrystalServer registers (globbed, so require_revision admits no untracked file there).
    result.spell_scripts = spell_scripts.SpellScripts(canary, extra_roots=(crystal,))
    result.source = {'repository': REPOSITORY, 'revision': REVISION}
    return result


def created_name(path):
    match = re.search(r'Game\.createMonsterType\("([^"]+)"', path.read_text(encoding='utf-8', errors='replace'))
    return match.group(1) if match else None


def files(canary, crystal):
    """Paths under MONSTER_ROOT (no .lua) of the Crystal monsters whose name no Canary monster file creates: those of
    MONSTER_DIR, then the EXTRA_MONSTERS, which must be tracked at REVISION and not created by any Canary file."""
    require_revision(canary, crystal)
    canary_files = git(canary, 'ls-tree', '-r', '--name-only', cb.REVISION, '--', cb.MONSTER_DIR).splitlines()
    canary_slugs = {cb.slug(name) for path in canary_files if path.endswith('.lua') and (name := created_name(canary / path))}
    tracked = set(git(crystal, 'ls-tree', '-r', '--name-only', REVISION, '--', MONSTER_ROOT).splitlines())
    result = []
    for path in sorted(crystal / name for name in tracked if name.startswith(MONSTER_DIR + '/') and name.endswith('.lua')):
        if cb.slug(created_name(path)) not in canary_slugs:
            result.append(str(path.relative_to(crystal / MONSTER_ROOT))[:-4])
    for relative in EXTRA_MONSTERS:
        if f'{MONSTER_ROOT}/{relative}.lua' not in tracked:
            raise SystemExit(f'allow-listed {relative} is not a tracked CrystalServer file at {REVISION}')
        name = created_name(crystal / MONSTER_ROOT / (relative + '.lua'))
        if not name or cb.slug(name) in canary_slugs:
            raise SystemExit(f'allow-listed {relative} is not a monster that Canary lacks')
        result.append(relative)
    if len({cb.slug(created_name(crystal / MONSTER_ROOT / (r + '.lua'))) for r in result}) != len(result):
        raise SystemExit('two selected Crystal files create one monster')
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
    assert source['file'] == f'{MONSTER_ROOT}/{selected[0]}.lua' and not os.path.isabs(source['file'])
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
