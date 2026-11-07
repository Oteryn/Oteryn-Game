"""Close the three familiar Ability dependencies with pinned core summon data.

Only authoring evidence is produced. Player spell admission/expiry is not implemented
by this producer; the preserved Creature familiar contract and report describe that
boundary explicitly. No damage Formula is invented for a non-damaging summon.
"""
import argparse
import copy
import hashlib
import json
import re
import subprocess
from pathlib import Path

import canary_batch as cb
import validate_monster as vm

CRYSTAL_REVISION = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
VOCATIONS = ('knight', 'monk', 'paladin')
UNREPRESENTED = [
    'player premium, level 200, vocation and no-other-summons cast admission (God bypass omitted)',
    'owner-selected familiar identity/look and owner speed synchronization',
    'timed familiar removal and 60/10-second notifications (duration retained on Creature)',
    'config/VIP/rate-dependent spell cooldown and support group cooldown',
    'mana payment and player summon persistence/death/login lifecycle',
]


def pinned(root, revision, relative):
    data = subprocess.check_output(['git', '-C', str(root), 'show', revision + ':' + relative])
    if (root / relative).read_bytes() != data:
        raise ValueError('source checkout differs from pinned object: ' + relative)
    return data.decode(), {'file': relative, 'revision': revision,
                          'sha256': hashlib.sha256(data).hexdigest(), 'git_blob': cb.blob_id(data)}


def call(text, method):
    matches = re.findall(r'^spell:' + method + r'\(([^\n]*)\)\s*(?:--[^\n]*)?$', text, re.M)
    if len(matches) != 1:
        raise ValueError('expected one spell:' + method)
    return matches[0]


def source_facts(root, revision, vocation):
    script, evidence = pinned(root, revision, f'data/scripts/spells/familiar/{vocation}_familiar.lua')
    player, player_evidence = pinned(root, revision, 'data/libs/functions/player.lua')
    config, config_evidence = pinned(root, revision, 'config.lua.dist')
    if 'return player:CreateFamiliarSpell(spellId)' not in script:
        raise ValueError('unexpected familiar callback')
    spell_id = re.findall(r'^local spellId = (\d+)$', script, re.M)
    familiar_time = re.findall(r'^familiarTime = (\d+)', config, re.M)
    if len(spell_id) != 1 or len(familiar_time) != 1:
        raise ValueError('nonliteral spell identity/config duration')
    for literal in ('#self:getSummons() >= 1',
                    '60 * configManager.getNumber(configKeys.FAMILIAR_TIME) / 2',
                    'Game.createMonster(familiarName, playerPosition, true, false, self)',
                    'addEvent(RemoveFamiliar, timeLeft * 1000'):
        if literal not in player:
            raise ValueError('unrecognized core familiar behavior: ' + literal)
    if call(script, 'register') != '' or call(script, 'id') != 'spellId':
        raise ValueError('unregistered player spell')
    vocations = re.findall(r'"([^";]+);true"', call(script, 'vocation'))
    if vocation not in vocations:
        raise ValueError('wrong player vocation')
    return {'spell_id': int(spell_id[0]), 'name': json.loads(call(script, 'name')),
            'words': json.loads(call(script, 'words')), 'mana_cost': int(call(script, 'mana')),
            'minimum_level': int(call(script, 'level')), 'vocations': vocations,
            'duration_ms': int(familiar_time[0]) * 30000,
            'group_cooldown_expression': call(script, 'groupCooldown'),
            'cast_sound_expression': call(script, 'castSound'),
            'evidence': [evidence, player_evidence, config_evidence]}


def supplement(monster, dependencies, catalog, manifest, vocation, facts):
    """Return a supplemented copy; all pre-existing payloads remain intact."""
    monster, dependencies, catalog, manifest = map(copy.deepcopy, (monster, dependencies, catalog, manifest))
    familiar = monster['creature']['summoning']['familiar']
    reference = familiar['summon_ability']
    expected = cb.ref('Ability', f'canary:ability/spell/summon_{vocation}_familiar')
    if reference != expected or familiar['vocation'] != vocation:
        raise ValueError('familiar contract does not reference the pinned player spell')
    if familiar['mana_cost'] != facts['mana_cost'] or familiar['duration_ms'] != facts['duration_ms']:
        raise ValueError('existing familiar cost/duration differs from pinned source')
    if any(a['identity']['key'] == reference['key'] for a in dependencies['abilities']):
        raise ValueError('summon Ability already locally defined')
    if catalog['definitions'].count(reference) != 1:
        raise ValueError('expected exactly one missing catalog Ability')
    effect_key = reference['key'] + '/effect'
    effect = {'identity': {'key': effect_key, 'revision': reference['revision']},
              'operation': 'summon_creature', 'summon': {
                  'creatures': [cb.ref('Creature', monster['creature']['identity']['key'])],
                  'count_mode': 'fixed', 'count': 1, 'only_below_summons': 1,
                  'owned': True, 'max_offset_tiles': 0}}
    ability = {'identity': {'key': reference['key'], 'revision': reference['revision']},
               'kind': 'spell', 'range_tiles': 0, 'needs_target': False,
               'needs_direction': False, 'effects': [cb.ref('Effect', effect_key)]}
    ability_index = len(dependencies['abilities'])
    dependencies['abilities'].append(ability)
    dependencies['effects'].append(effect)
    catalog['definitions'].remove(reference)
    source_index = next((i for i, s in enumerate(manifest['sources'])
                         if s.get('repository') == cb.REPOSITORY and s.get('revision') == cb.REVISION), None)
    if source_index is None:
        raise ValueError('bundle has no pinned Canary source')
    manifest['entries'].append({
        'source_index': source_index,
        'source_file': f'data/scripts/spells/familiar/{vocation}_familiar.lua',
        'source_line': 5, 'source_field': 'spell.onCastSpell/CreateFamiliarSpell/core-owned-summon',
        'kind': 'field', 'status': 'mapped', 'destination': f'/dependencies/abilities/{ability_index}',
        'resolution': 'Pinned core: one owned familiar at caster position, only with no existing summons. '
                      'Creature duration/mana preserved; player gating, expiry, dynamic owner appearance/speed '
                      'and cooldown/payment are not represented by this Ability (flagged in completion report).'})
    errors = vm.validate(monster, dependencies, catalog, manifest)
    if errors:
        raise ValueError('supplemented bundle invalid: ' + '\n'.join(errors))
    return monster, dependencies, catalog, manifest


def prepare(canary, crystal, bundles, output):
    output = Path(output)
    if output.exists() and any(output.iterdir()):
        raise ValueError('output must be empty/new')
    report = {'format_version': 1, 'classification': 'SOURCE_BACKED_CORE_SUMMON_WITH_RUNTIME_GAPS',
              'live_gameplay_verified': False, 'unrepresented_player_semantics': UNREPRESENTED,
              'canary_revision': cb.REVISION, 'crystal_revision': CRYSTAL_REVISION, 'creatures': []}
    prepared = []
    for vocation in VOCATIONS:
        slug = vocation + '_familiar'
        donor = source_facts(Path(canary), cb.REVISION, vocation)
        comparison = source_facts(Path(crystal), CRYSTAL_REVISION, vocation)
        for field in ('mana_cost', 'duration_ms', 'minimum_level', 'vocations', 'spell_id', 'words'):
            if donor[field] != comparison[field]:
                raise ValueError('Canary/Crystal core mismatch: ' + field)
        directory = Path(bundles) / slug
        filenames = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
        originals = [json.loads((directory / name).read_text()) for name in filenames]
        values = supplement(*originals, vocation, donor)
        assert values[0] == originals[0]
        report['creatures'].append({'slug': slug, 'ability': values[0]['creature']['summoning']['familiar']['summon_ability'],
            'flags': ['PLAYER_FAMILIAR_RUNTIME_INTEGRATION_PENDING', 'SOURCE_BACKED_CORE_SUMMON'],
            'canary': donor, 'crystal': comparison,
            'differences': {field: {'canary': donor[field], 'crystal': comparison[field]}
                            for field in ('name', 'cast_sound_expression') if donor[field] != comparison[field]},
            'baseline_sha256': {name: hashlib.sha256((directory / name).read_bytes()).hexdigest() for name in filenames}})
        prepared.append((slug, filenames, values))
    output.mkdir(parents=True, exist_ok=True)
    for slug, filenames, values in prepared:
        target = output / slug
        target.mkdir()
        for name, value in zip(filenames, values):
            (target / name).write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
    (output / 'completion-report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('canary', 'crystal', 'bundles', 'output'):
        parser.add_argument('--' + arg, type=Path, required=True)
    args = parser.parse_args()
    prepare(args.canary, args.crystal, args.bundles, args.output)
