"""Transcribe Canary and CrystalServer reward chests into candidate reward claims.

Usage: python ots_chests.py --canary <opentibiabr/canary at 47dfd51f> --crystal <zimbadev/crystalserver at ff7ede59>
                            [--coverage samples/quest-coverage-2026-09-27.json] [--out samples/chests]

Both servers keep reward chests as data (`startup/tables/chest.lua`, table `ChestUnique`) served by one
shared script (`scripts/actions/system/quest_reward_common.lua`). The two servers are one lineage but
number chests differently, so chests are joined by map position, not by unique id. Every output is
OTS_HYPOTHESIS_ONLY evidence; no source id, storage key or coordinate becomes an Oteryn identity.
"""
import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables

ROOT = Path(__file__).resolve().parent
SOURCES = {
    'canary': {'repository': 'opentibiabr/canary', 'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
               'datapack': 'data-otservbr-global', 'tag': 'canary-47dfd51f'},
    'crystalserver': {'repository': 'zimbadev/crystalserver', 'revision': 'ff7ede593c69d4c658b382c97443e8155926924a',
                      'datapack': 'data-global', 'tag': 'crystalserver-ff7ede59'},
}
CHESTS = 'startup/tables/chest.lua'
SCRIPT = 'scripts/actions/system/quest_reward_common.lua'
REVISION = 'canary-47dfd51f+crystalserver-ff7ede59'
# A storage expression that is not a storage key: Canary uid 6093 stores `keyAction` (an undefined global).
INVALID_MARKERS = {'keyAction'}
STOP = {'the', 'a', 'an', 'of', 'quest', 'quests', 's', 'and', 'in', 'to'}


def ref(family, key):
    return {'family': family, 'key': key, 'revision': REVISION}


def words(text):
    text = re.sub(r'([a-z])([A-Z])', r'\1 \2', text).replace('_', ' ').lower().replace("'", '')
    return [w for w in re.findall(r'[a-z0-9]+', text) if w not in STOP]


def slug(text):
    return re.sub(r'_+', '_', re.sub(r'[^a-z0-9]+', '_', re.sub(r'([a-z0-9])([A-Z])', r'\1_\2', text).lower())).strip('_')


def git_blob(repo, path):
    return subprocess.run(['git', '-C', str(repo), 'rev-parse', f'HEAD:{path}'], check=True,
                          capture_output=True, text=True).stdout.strip()


def check_checkout(name, repo):
    head = subprocess.run(['git', '-C', str(repo), 'rev-parse', 'HEAD'], check=True, capture_output=True, text=True).stdout.strip()
    if head != SOURCES[name]['revision']:
        sys.exit(f'{name}: checkout is at {head}, expected {SOURCES[name]["revision"]}')


def read_server(name, repo):
    """Chest entries per position plus the per-uid text and achievement tables of the shared script."""
    pack = Path(repo) / SOURCES[name]['datapack']
    chests = lua_tables.assignments((pack / CHESTS).read_text(), {'ChestUnique'})['ChestUnique']
    script = (pack / SCRIPT).read_text()
    extra = lua_tables.assignments(script, {'AttributeTable', 'achievementTable'})
    texts = {f['key']: lua_tables.as_python(f['value']) for f in extra['AttributeTable']['fields']}
    achievements = {f['key']: f['value'] for f in extra['achievementTable']['fields']}
    entries, section = [], None
    for field in chests['fields']:
        value = lua_tables.as_python(field['value'])
        positions = value.get('itemPos')
        positions = [positions] if isinstance(positions, dict) else positions or [None]
        labels = [c.lstrip('-').strip() for c in field['comments'] + field['value'].get('head', []) if c.lstrip('-').strip()]
        # a comment naming a quest opens a section; entries without their own quest comment inherit it
        section = next((label for label in reversed(labels)
                        if re.search(r'quest', label, re.I) and not re.search(r'path:|\.lua', label, re.I)), section)
        for position in positions:
            entries.append({'server': name, 'uid': field['key'], 'line': field['line'], 'value': value,
                            'position': (position['x'], position['y'], position['z']) if position else None,
                            'label': labels[-1] if labels else None, 'section': section,
                            'text': texts.get(field['key']), 'achievement': achievements.get(field['key'])})
    return entries


def marker(value):
    """The per-character claim marker of a chest: its storage key, or its KV quest name."""
    if value.get('useKV') and value.get('questName'):
        return f'kv/{slug(value["questName"])}'
    storage = value.get('storage')
    if isinstance(storage, dict) and storage['expr'] not in INVALID_MARKERS:
        return '/'.join(slug(part) for part in storage['expr'].split('.')[1:])
    return None


def items(pairs, server):
    namespace = 'canary' if server == 'canary' else 'crystalserver'
    return [{'item': ref('Item', f'{namespace}:item/{int(i)}'), 'count': int(n)} for i, n in pairs or [] if i is not None]


def placement(entry):
    """The Oteryn view of one chest; `weight` is not kept because the runtime derives it from item definitions."""
    value, server = entry['value'], entry['server']
    namespace = 'canary' if server == 'canary' else 'crystalserver'
    reward = {'items': items(value.get('reward'), server)}
    if value.get('container'):
        reward['container'] = ref('Item', f'{namespace}:item/{value["container"]}')
    if value.get('randomReward'):
        reward['random_one_of'] = items(value['randomReward'], server)
    if value.get('isKey') or value.get('keyAction'):
        key_storage = value.get('keyAction') if isinstance(value.get('keyAction'), dict) else value.get('storage')
        key_id = re.search(r'ID(\d+)$', key_storage['expr']) if isinstance(key_storage, dict) else None
        reward['key_binding'] = f'{namespace}:door-key/{key_id.group(1)}' if key_id else None
    if entry['text']:
        text = entry['text']
        reward['written_text'] = {'item': ref('Item', f'{namespace}:item/{text["itemId"]}') if text.get('itemId') else None,
                                  'text': text['text'].strip('\n')}
    result = {'position': dict(zip('xyz', entry['position'])),
              'appearance': ref('Item', f'{namespace}:item/{value["itemId"]}') if value.get('itemId') else None,
              'reward': reward}
    if entry['achievement']:
        result['achievement'] = ref('Achievement', f'{namespace}:achievement/{slug(entry["achievement"])}')
    return result


def comparable(entry):
    """Fields that must agree between the servers for one chest; the unique id and label may differ."""
    value = {k: v for k, v in entry['value'].items() if k not in ('itemPos', 'weight')}
    storage = value.get('storage')
    if isinstance(storage, dict) and storage['expr'] in INVALID_MARKERS:
        value.pop('storage')
    return value


def repeat(entries):
    hours = {e['value'].get('time') for e in entries if e['value'].get('time')}
    if not hours:
        return {'kind': 'once'}
    return {'kind': 'cooldown', 'hours': max(hours)}


def joined(text):
    return ''.join(w for w in words(text) if not w.isdigit())


def wiki_matcher(coverage):
    """Match a source name to a wiki quest: equal letters first (`griffinshield1` = Griffin Shield Quest),
    then the longest wiki title contained word by word (`Key 3008 (Draconia Quest)` = Draconia Quest)."""
    if not coverage:
        return lambda text: None
    titles = [(words(re.sub(r'\s*\(.*\)$', '', row['title'])), row) for row in coverage['quests']]
    by_letters = {joined(re.sub(r'\s*\(.*\)$', '', row['title'])): row for row in coverage['quests']}

    def match(text):
        if not text:
            return None
        letters = joined(re.sub(r'\d+$', '', text))
        if letters in by_letters:
            return by_letters[letters]
        text_words = words(re.sub(r'\(.*?\)', ' ', text))
        best = None
        for key, row in titles:
            n = len(key)
            if n and any(text_words[i:i + n] == key for i in range(len(text_words) - n + 1)):
                if best is None or n > len(best[0]):
                    best = (key, row)
        return best[1] if best else None
    return match


def build(repos, coverage):
    servers = {name: read_server(name, repo) for name, repo in repos.items()}
    by_position = defaultdict(dict)
    manifest_entries = []
    for name, entries in servers.items():
        for entry in entries:
            if entry['position'] is None:
                manifest_entries.append({'source': name, 'uid': entry['uid'], 'source_lines': [entry['line']],
                                         'status': 'unresolved_semantics', 'resolution': 'no map position'})
                continue
            if entry['position'] in by_position and name in by_position[entry['position']]:
                first = by_position[entry['position']][name]
                if comparable(entry) == comparable(first):
                    status, resolution = 'approved_omission', f'exact duplicate of uid {first["uid"]}'
                elif not entry['value'].get('reward'):
                    status, resolution = 'approved_omission', f'unique id marker without a reward on the chest of uid {first["uid"]}'
                else:
                    status, resolution = 'unresolved_semantics', f'second reward chest at the position of uid {first["uid"]}'
                manifest_entries.append({'source': name, 'uid': entry['uid'], 'source_lines': [entry['line']],
                                         'status': status, 'resolution': resolution})
                continue
            by_position[entry['position']][name] = entry

    match_quest = wiki_matcher(coverage)
    groups = defaultdict(list)          # claim marker -> [(position, chosen entry, pair, sources, status, resolution)]
    empty = []
    for position, pair in sorted(by_position.items()):
        primary = pair.get('canary') or pair['crystalserver']
        status, resolution = 'mapped', 'present in both servers and identical'
        if len(pair) == 1:
            resolution = f'present only in {next(iter(pair))}'
        elif comparable(pair['canary']) != comparable(pair['crystalserver']):
            a, b = comparable(pair['canary']), comparable(pair['crystalserver'])
            fields = sorted(k for k in set(a) | set(b) if a.get(k) != b.get(k))
            if marker(pair['canary']['value']) is None and marker(pair['crystalserver']['value']):
                primary, resolution = pair['crystalserver'], 'crystalserver repairs the invalid Canary claim marker'
                fields = [f for f in fields if f not in ('storage', 'keyAction', 'isKey')]
            if fields:
                status = 'conflict'
                resolution = 'servers disagree on ' + ', '.join(fields) + '; ' + '; '.join(
                    f'{n}: ' + json.dumps({f: comparable(pair[n]).get(f) for f in fields}) for n in pair)
        if len(pair) == 2 and primary is pair['canary'] and (pair['crystalserver']['text'] or {}).get('itemId'):
            # CrystalServer names the reward item that carries a written text; Canary stamps every item
            primary = dict(primary, text=pair['crystalserver']['text'])
        sources = [{'source': n, 'uid': e['uid'], 'source_lines': [e['line']]} for n, e in pair.items()]
        value = primary['value']
        if not value.get('reward') and not value.get('randomReward'):
            empty.append({'position': dict(zip('xyz', position)), 'sources': sources})
            manifest_entries.append({'position': list(position), 'sources': sources, 'status': 'approved_omission',
                                     'resolution': 'empty container without a reward: a world object, not a claim'})
            continue
        claim_marker = marker(value)
        if claim_marker is None:
            manifest_entries.append({'position': list(position), 'sources': sources, 'status': 'unresolved_semantics',
                                     'resolution': 'no claim marker (storage or KV quest name): the source cannot remember the claim'})
            continue
        groups[claim_marker].append((position, primary, pair, sources, status, resolution))

    claims, quests = [], {}
    for claim_marker, members in sorted(groups.items()):
        label = next((m[1]['label'] for m in members if m[1]['label']), None)
        section = next((m[1]['section'] for m in members if m[1]['section']), None)
        in_canary = any('canary' in m[2] for m in members)
        namespace = 'canary' if in_canary else 'crystalserver'
        key = f'{namespace}:reward-claim/{claim_marker}'
        storage_quest = re.match(r'quest/u[0-9_]+/([a-z0-9_]+)', claim_marker)
        kv_name = claim_marker[3:] if claim_marker.startswith('kv/') else None
        basis, wiki = next(((b, w) for b, w in ((b, match_quest(t)) for b, t in (
            ('kv_quest_name', kv_name), ('storage_key', storage_quest.group(1) if storage_quest else None),
            ('label', label), ('section', section))) if w), (None, None))
        quest, candidate = None, None
        if wiki and basis == 'section':
            # the file's section headers do not always cover the entries below them (outlaw camp keys sit
            # under the Katana Quest header), so a section-only match is kept for review, not as a link
            candidate = ref('Quest', f'{namespace}:quest/{slug(wiki["title"])}')
        elif wiki:
            quest_key = f'{namespace}:quest/{slug(wiki["title"])}'
            quest = ref('Quest', quest_key)
            entry = quests.setdefault(quest_key, {'identity': {'key': quest_key, 'revision': REVISION},
                                                  'display_name': wiki['title'], 'kind': 'reward_only',
                                                  'shown_in_quest_log': wiki['in_quest_log'],
                                                  'wiki': {k: wiki[k] for k in ('title', 'pageid', 'revid')},
                                                  'claims': []})
            for field in ('premium', 'lvl'):
                if wiki.get(field):
                    entry.setdefault('requirements_from_wiki', {})[field] = wiki[field]
            entry['claims'].append(ref('RewardClaim', key))
        timed = [m[1] for m in members if m[1]['value'].get('time')]
        claim = {'identity': {'key': key, 'revision': REVISION}, 'label': label, 'section': section, 'quest': quest,
                 'quest_link_basis': basis if quest else None, 'quest_candidate_from_section': candidate,
                 'claim': {'per': 'character', 'repeat': repeat([m[1] for m in members])},
                 'placements': [placement(m[1]) for m in members]}
        if timed:
            claim['source_divergence'] = ('The source data sets `time` in hours but the shared script only honours '
                                          '`timerStorage`, so both servers hand this reward out once. The intent is kept.')
        claims.append(claim)
        for position, primary, pair, sources, status, resolution in members:
            manifest_entries.append({'position': list(position), 'sources': sources, 'status': status,
                                     'resolution': resolution, 'destination': key})

    manifest = {
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'join': 'map position (unique ids differ between the servers)',
        'sources': [{'kind': 'git', 'repository': SOURCES[n]['repository'], 'revision': SOURCES[n]['revision'],
                     'path': f'{SOURCES[n]["datapack"]}/{p}', 'blob_sha1': git_blob(repos[n], f'{SOURCES[n]["datapack"]}/{p}')}
                    for n in repos for p in (CHESTS, SCRIPT)],
        'counts': {},
        'entries': sorted(manifest_entries, key=lambda e: json.dumps(e, sort_keys=True)),
    }
    counts = defaultdict(int)
    for e in manifest['entries']:
        counts[e['status']] += 1
    manifest['counts'] = {
        'source_entries': {n: len(v) for n, v in servers.items()},
        'positions': len(by_position),
        'positions_in_both': sum(1 for p in by_position.values() if len(p) == 2),
        'positions_only_canary': sum(1 for p in by_position.values() if set(p) == {'canary'}),
        'positions_only_crystalserver': sum(1 for p in by_position.values() if set(p) == {'crystalserver'}),
        'claims': len(claims), 'placements': sum(len(c['placements']) for c in claims),
        'claims_linked_to_wiki_quest': dict(sorted(Counter(c['quest_link_basis'] or 'none' for c in claims).items())),
        'claims_with_section_candidate_only': sum(1 for c in claims if c['quest_candidate_from_section']),
        'quests': len(quests),
        'by_status': dict(sorted(counts.items())),
    }
    definitions = sorted({json.dumps(r, sort_keys=True) for c in claims for r in definition_refs(c)} |
                         {json.dumps(ref('Quest', q), sort_keys=True) for q in quests})
    catalog = {'definitions': [json.loads(d) for d in definitions]}
    return {'claims.json': {'claims': claims}, 'quests.json': {'quests': sorted(quests.values(), key=lambda q: q['identity']['key'])},
            'catalog.json': catalog, 'manifest.json': manifest, 'empty_containers.json': {'empty_containers': empty}}


def definition_refs(value):
    """Item and Achievement references of a claim (quest links are listed separately)."""
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            if value['family'] != 'Quest':
                yield value
            return
        for child in value.values():
            yield from definition_refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from definition_refs(child)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--coverage', type=Path, default=ROOT / 'samples/quest-coverage-2026-09-27.json')
    parser.add_argument('--out', type=Path, default=ROOT / 'samples/chests')
    args = parser.parse_args()
    repos = {'canary': args.canary, 'crystalserver': args.crystal}
    for name, repo in repos.items():
        check_checkout(name, repo)
    coverage = json.loads(args.coverage.read_text()) if args.coverage and args.coverage.exists() else None
    outputs = build(repos, coverage)
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in outputs.items():
        (args.out / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(outputs['manifest.json']['counts'], indent=2))
    digest = hashlib.sha256(json.dumps(outputs, sort_keys=True).encode()).hexdigest()
    print(f'output sha256 {digest}')


if __name__ == '__main__':
    main()
