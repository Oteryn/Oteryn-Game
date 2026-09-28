"""Transcribe Canary and CrystalServer quest, key, level and dedicated-script doors into candidate door gates.

Usage: python ots_doors.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
                           [--chests samples/chests] [--coverage samples/quest-coverage-2026-09-27.json] [--out samples/doors]

Both servers list doors in startup tables (`door_quest.lua`, `door_key.lua`, `door_level.lua`); the map loader puts the
table key on the listed positions as the door's action id (or, for `QuestDoorUnique`, its unique id). Three shared
scripts decide (identical in both servers):
- quest door: passes a player whose storage named by the action id is set;
- level door: passes a player whose level is at least the action id minus 1000;
- key door: a key whose action id matches locks or unlocks the door for everyone.
`QuestDoorUnique` doors are each opened by their own dedicated script (identical source in both servers) instead of
one of the three shared ones:
- the four Dawnport vocation doors (`vocation_door.lua`) are not gates: the door never opens; a USE on it checks the
  vocation-choice track, removes mainland items and relocates the player, which is an interaction, so they stay
  `unresolved_semantics`;
- the Katana Quest door (`katana_quest_door.lua`/`katana_quest_lever.lua`): has no player-state condition at all,
  a nearby lever toggles it open and closed for everyone;
- the Secret Service door and CrystalServer's one restored-id door have no script that transforms them and stay
  `unresolved_semantics`.
Doors are joined by map position like the chests. Every output is OTS_HYPOTHESIS_ONLY evidence.
"""
import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables
from ots_chests import CONFLICT_DECISIONS, REVISION, ROOT, SOURCES, check_checkout, decided, git_blob, quest_key, ref, slug, unused_decisions, wiki_matcher

TABLES = {
    'quest': ('startup/tables/door_quest.lua', 'QuestDoorAction'),
    'quest_unique': ('startup/tables/door_quest.lua', 'QuestDoorUnique'),
    'key': ('startup/tables/door_key.lua', 'KeyDoorAction'),
    'level': ('startup/tables/door_level.lua', 'LevelDoorAction'),
}
SCRIPTS = ['data/scripts/actions/doors/quest_door.lua', 'data/scripts/actions/doors/key_door.lua',
           'data/scripts/actions/doors/level_door.lua']
# QuestDoorUnique doors, each opened by its own dedicated script instead of one of the three shared ones above;
# paths are datapack-relative like TABLES. The Secret Service door has no dedicated opening script (its script only
# reads the door as a mission-item target); it is kept here purely as evidence for the manifest.
DEDICATED_SCRIPTS = {
    'vocation': 'scripts/actions/dawnport/vocation_door.lua',
    'katana_door': 'scripts/actions/rookgaard/katana_quest/katana_quest_door.lua',
    'katana_lever': 'scripts/actions/rookgaard/katana_quest/katana_quest_lever.lua',
    'secret_service': 'scripts/quests/secret_service/actions_rust_bugs.lua',
}
# VOCATION.ID base vocations used by the Dawnport vocation doors (identical in both servers).
VOCATIONS = {'SORCERER': 'sorcerer', 'DRUID': 'druid', 'PALADIN': 'paladin', 'KNIGHT': 'knight'}


def read_vocation_map(repo, datapack):
    """uid -> base vocation name, parsed from the dedicated vocation_door.lua script."""
    text = (Path(repo) / datapack / DEDICATED_SCRIPTS['vocation']).read_text()
    return {int(uid): VOCATIONS[name] for uid, name in
            re.findall(r'\[(\d+)\]\s*=\s*\{\s*vocation\s*=\s*VOCATION\.ID\.(\w+)', text)}


def read_katana_lever(repo, datapack):
    """The unique id the dedicated katana_quest_door.lua script registers on, and the lever position it pairs
    the door with (the lever, not the door, is what the player uses to open or close it)."""
    text = (Path(repo) / datapack / DEDICATED_SCRIPTS['katana_door']).read_text()
    uid = int(re.search(r'\w+:uid\((\d+)\)', text).group(1))
    lever = re.search(r'leverPosition\s*=\s*Position\((\d+),\s*(\d+),\s*(\d+)\)', text)
    return uid, (int(lever.group(1)), int(lever.group(2)), int(lever.group(3)))


def read_secret_service_uid(repo, datapack):
    """The unique id the dedicated actions_rust_bugs.lua script reads as its target: it advances a mission
    storage and spawns nearby items, but never transforms the door itself."""
    text = (Path(repo) / datapack / DEDICATED_SCRIPTS['secret_service']).read_text()
    return int(re.search(r'target\.uid\s*~=\s*(\d+)', text).group(1))


def marker_of(key):
    """The storage a quest door reads, as the claim-marker path used for chests; a bare number stays a number."""
    if isinstance(key, dict):
        return '/'.join(slug(part) for part in key['expr'].split('.')[1:])
    return f'storage/{key}'


def read_server(name, repo):
    pack = Path(repo) / SOURCES[name]['datapack']
    rows = []
    for kind, (path, table) in TABLES.items():
        found = lua_tables.assignments((pack / path).read_text(), {table})
        for field in found[table]['fields']:
            value = lua_tables.as_python(field['value'])
            labels = [c.lstrip('-').strip() for c in field['comments'] + field['value'].get('head', []) if c.lstrip('-').strip()]
            for part in value if isinstance(value, list) else [value]:
                positions = part.get('itemPos')
                for position in positions if isinstance(positions, list) else [positions] if positions else []:
                    rows.append({'server': name, 'kind': kind, 'key': field['key'], 'line': field['line'],
                                 'position': (position['x'], position['y'], position['z']),
                                 'appearance': part.get('itemId') or None, 'label': labels[-1] if labels else None})
    return rows


def key_text(key):
    return key['expr'] if isinstance(key, dict) else key


def letters(text):
    return re.sub(r'[^a-z0-9/]', '', text.lower())


def condition_of(row, namespace, claims_by_marker, claims_by_key):
    if row['kind'] == 'level':
        return {'kind': 'min_level', 'level': int(row['key']) - 1000}
    if row['kind'] == 'key':
        number = re.search(r'ID(\d+)$', row['key']['expr']).group(1) if isinstance(row['key'], dict) else str(row['key'])
        binding = f'{namespace}:door-key/{number}'
        return {'kind': 'door_key', 'key_binding': binding,
                'key_from_claims': sorted(claims_by_key.get(number, []), key=lambda r: r['key'])}
    marker = marker_of(row['key'])
    return {'kind': 'quest_progress', 'progress': f'{namespace}:quest-progress/{marker}',
            'claim': claims_by_marker.get(marker)}


def gate_key(condition):
    if condition['kind'] == 'min_level':
        return f'level/{condition["level"]}'
    if condition['kind'] == 'door_key':
        return 'key/' + condition['key_binding'].rsplit('/', 1)[1]
    if condition['kind'] == 'vocation':
        return f'vocation/{condition["vocation"]}'
    if condition['kind'] == 'lever':
        p = condition['lever_position']
        return f'lever/{p["x"]}_{p["y"]}_{p["z"]}'
    return 'progress/' + condition['progress'].split('/', 1)[1]


def build(repos, chests_dir, coverage):
    claims = json.loads((chests_dir / 'claims.json').read_text())['claims']
    claims_by_marker, claims_by_key = {}, defaultdict(list)
    for claim in claims:
        marker = claim['identity']['key'].split('/', 1)[1]
        claims_by_marker[marker] = ref('RewardClaim', claim['identity']['key'])
        for placement in claim['placements']:
            binding = placement['reward'].get('key_binding')
            if binding:
                claims_by_key[binding.rsplit('/', 1)[1]].append(ref('RewardClaim', claim['identity']['key']))

    by_position, scripted, manifest_entries = defaultdict(dict), defaultdict(dict), []
    for name, repo in repos.items():
        for row in read_server(name, repo):
            if row['kind'] == 'quest_unique':
                scripted[row['position']][name] = row
                continue
            if name in by_position[row['position']]:
                first = by_position[row['position']][name]
                same = (first['kind'], key_text(first['key'])) == (row['kind'], key_text(row['key']))
                manifest_entries.append({'source': name, 'position': list(row['position']), 'source_lines': [row['line']],
                                         'status': 'approved_omission' if same else 'unresolved_semantics',
                                         'resolution': 'exact duplicate' if same else 'second door rule at the same position'})
                continue
            by_position[row['position']][name] = row

    # QuestDoorUnique doors: each is opened by its own dedicated script instead of one of the three shared ones.
    # The dedicated scripts are read once (identical source is asserted, not assumed) and matched onto the
    # positions collected above by their unique id.
    vocation_maps, katana, secret_service_uids = {}, {}, {}
    for name, repo in repos.items():
        datapack = SOURCES[name]['datapack']
        vocation_maps[name] = read_vocation_map(repo, datapack)
        katana[name] = read_katana_lever(repo, datapack)
        secret_service_uids[name] = read_secret_service_uid(repo, datapack)
    if vocation_maps['canary'] != vocation_maps['crystalserver']:
        raise SystemExit(f'vocation door script disagrees between servers: {vocation_maps}')
    if len(set(katana.values())) > 1:
        raise SystemExit(f'katana door script disagrees between servers: {katana}')
    if len(set(secret_service_uids.values())) > 1:
        raise SystemExit(f'secret service script disagrees between servers: {secret_service_uids}')
    vocation_map = vocation_maps['canary']
    katana_uid, katana_lever = katana['canary']
    secret_service_uid = secret_service_uids['canary']

    gates = {}
    for position, pair in sorted(scripted.items()):
        label = next(r['label'] for r in pair.values())
        uid = next(r['key'] for r in pair.values())
        primary = pair.get('canary') or pair['crystalserver']
        agreement = 'present in both servers and identical' if len(pair) == 2 else f'present only in {primary["server"]}'
        sources = [{'source': n, 'uid': r['key'], 'source_lines': [r['line']]} for n, r in pair.items()]
        if uid == katana_uid:
            condition = {'kind': 'lever', 'lever_position': dict(zip('xyz', katana_lever))}
            state = 'shared_lock'
            note = f'dedicated script: lever-controlled door, no player-state condition ({label})'
            namespace = 'canary' if 'canary' in pair else 'crystalserver'
            key = f'{namespace}:door-gate/{gate_key(condition)}'
            gate = gates.setdefault(key, {'identity': {'key': key, 'revision': REVISION}, 'label': label, 'quest': None,
                                          'quest_link_basis': None, 'condition': condition, 'state': state, 'placements': []})
            gate['placements'].append({'position': dict(zip('xyz', position)),
                                       'appearance': ref('Item', f'{primary["server"]}:item/{primary["appearance"]}') if primary['appearance'] else None})
            manifest_entries.append({'position': list(position), 'status': 'mapped', 'destination': key,
                                     'resolution': f'{note}; {agreement}', 'sources': sources})
            continue
        if uid in vocation_map:
            manifest_entries.append({'position': list(position), 'status': 'unresolved_semantics', 'sources': sources,
                                     'resolution': f'not a gate: the dedicated script never opens the door; a USE checks the '
                                                   f'vocation-choice track for {vocation_map[uid]}, removes mainland items and '
                                                   f'relocates the player, which is an interaction ({label}); {agreement}'})
            continue
        resolution = (f'dedicated script reads this door only as a mission-item target and never transforms it ({label}); {agreement}'
                     if uid == secret_service_uid else
                     f'uid declared to restore an id lost when world.otbm stopped storing it, the same defect as a '
                     f'bare-number quest door; no script sets or reads it ({label or "no label"}); {agreement}')
        manifest_entries.append({'position': list(position), 'status': 'unresolved_semantics', 'resolution': resolution, 'sources': sources})

    match_quest = wiki_matcher(coverage)
    # a gate keeps one identity across servers: CrystalServer-only doors join a rule Canary also has
    canary_rules = {gate_key(condition_of(pair['canary'], 'canary', claims_by_marker, claims_by_key))
                    for pair in by_position.values()
                    if 'canary' in pair and not (pair['canary']['kind'] == 'quest' and not isinstance(pair['canary']['key'], dict))}
    used_decisions = set()
    for position, pair in sorted(by_position.items()):
        primary = pair.get('canary') or pair['crystalserver']
        if primary['kind'] == 'quest' and not isinstance(primary['key'], dict):
            # a bare number is no named quest progress: CrystalServer restores such ids because world.otbm stopped
            # storing them; they seal a door unless a script sets that storage or handles the id itself
            manifest_entries.append({'position': list(position), 'status': 'unresolved_semantics',
                                     'resolution': f'quest door reading bare storage {primary["key"]} ({primary["label"] or "no label"})',
                                     'sources': [{'source': n, 'action_key': key_text(r['key']), 'source_lines': [r['line']]}
                                                 for n, r in pair.items()]})
            continue
        rule = gate_key(condition_of(primary, 'canary', claims_by_marker, claims_by_key))
        namespace = 'canary' if 'canary' in pair or rule in canary_rules else 'crystalserver'
        condition = condition_of(primary, namespace, claims_by_marker, claims_by_key)
        status = 'mapped'
        resolution = 'present in both servers and identical' if len(pair) == 2 else f'present only in {primary["server"]}'
        if len(pair) == 2:
            other = condition_of(pair['crystalserver'], namespace, claims_by_marker, claims_by_key)
            if letters(gate_key(other)) != letters(gate_key(condition)):
                status = 'conflict'
                resolution = f'servers disagree: canary {gate_key(condition)}; crystalserver {gate_key(other)}'
                decision = CONFLICT_DECISIONS['doors'].get(','.join(map(str, position)))
                if decision:
                    used_decisions.add(','.join(map(str, position)))
                    if decision['decision'] != 'canary':
                        raise SystemExit(f'{position}: only Canary door decisions are wired')
                    status, resolution = 'mapped', resolution + '; ' + decided(decision)
            elif gate_key(other) != gate_key(condition):
                resolution = 'present in both servers; the storage name differs only in letter case'
        key = f'{namespace}:door-gate/{gate_key(condition)}'
        gate = gates.setdefault(key, {'identity': {'key': key, 'revision': REVISION}, 'label': None, 'quest': None,
                                      'quest_link_basis': None, 'condition': condition,
                                      'state': 'shared_lock' if condition['kind'] == 'door_key' else 'per_character_pass',
                                      'placements': []})
        gate['label'] = gate['label'] or primary['label']
        gate['placements'].append({'position': dict(zip('xyz', position)),
                                   'appearance': ref('Item', f'{primary["server"]}:item/{primary["appearance"]}') if primary['appearance'] else None})
        manifest_entries.append({'position': list(position), 'status': status, 'resolution': resolution, 'destination': key,
                                 'sources': [{'source': n, 'action_key': key_text(r['key']),
                                              'source_lines': [r['line']]} for n, r in pair.items()]})

    for gate in gates.values():
        if gate['condition']['kind'] != 'quest_progress':
            continue
        storage_quest = re.match(r'[a-z]+:quest-progress/quest/u[0-9_]+/([a-z0-9_]+)', gate['condition']['progress'])
        basis, wiki = next(((b, w) for b, w in ((b, match_quest(t)) for b, t in (
            ('storage_key', storage_quest.group(1) if storage_quest else None), ('label', gate['label']))) if w), (None, None))
        if wiki:
            gate['quest'], gate['quest_link_basis'] = ref('Quest', quest_key(wiki)), basis

    gate_list = sorted(gates.values(), key=lambda g: g['identity']['key'])
    counts = Counter(e['status'] for e in manifest_entries)
    unused_decisions('doors', used_decisions)
    manifest = {
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'join': 'map position',
        'sources': [{'kind': 'git', 'repository': SOURCES[n]['repository'], 'revision': SOURCES[n]['revision'], 'path': p,
                     'blob_sha1': git_blob(repos[n], p)}
                    for n in repos for p in sorted({f'{SOURCES[n]["datapack"]}/{t[0]}' for t in TABLES.values()} |
                                                    {f'{SOURCES[n]["datapack"]}/{s}' for s in DEDICATED_SCRIPTS.values()}) + SCRIPTS],
        'counts': {
            'positions': len(by_position),
            'positions_in_both': sum(1 for p in by_position.values() if len(p) == 2),
            'positions_only_canary': sum(1 for p in by_position.values() if set(p) == {'canary'}),
            'positions_only_crystalserver': sum(1 for p in by_position.values() if set(p) == {'crystalserver'}),
            'gates': dict(sorted(Counter(g['condition']['kind'] for g in gate_list).items())),
            'progress_gates_reading_a_claim': sum(1 for g in gate_list if g['condition'].get('claim')),
            'key_gates_with_a_key_from_a_chest': sum(1 for g in gate_list if g['condition'].get('key_from_claims')),
            'progress_gates_linked_to_wiki_quest': sum(1 for g in gate_list if g['quest']),
            'by_status': dict(sorted(counts.items())),
        },
        'entries': sorted(manifest_entries, key=lambda e: json.dumps(e, sort_keys=True)),
    }
    return {'gates.json': {'gates': gate_list}, 'manifest.json': manifest}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--chests', type=Path, default=ROOT / 'samples/chests')
    parser.add_argument('--coverage', type=Path, default=ROOT / 'samples/quest-coverage-2026-09-27.json')
    parser.add_argument('--out', type=Path, default=ROOT / 'samples/doors')
    args = parser.parse_args()
    repos = {'canary': args.canary, 'crystalserver': args.crystal}
    for name, repo in repos.items():
        check_checkout(name, repo)
    outputs = build(repos, args.chests, json.loads(args.coverage.read_text()))
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in outputs.items():
        (args.out / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(outputs['manifest.json']['counts'], indent=2))
    print('output sha256 ' + hashlib.sha256(json.dumps(outputs, sort_keys=True).encode()).hexdigest())


if __name__ == '__main__':
    main()
