"""Prepare the accepted SW-3/4/5 successor without authorizing E4 admission.

Reads Git objects at the pinned Canary revision, including sparse-checkout files.
The undecided SW-6 pool policy stays an open manifest row and has no covers claim.
The caller may transcribe the returned accepted artifacts into the real sample;
SW-6 and E4 readiness remain blocked. No runtime or source-policy decisions are changed.
"""
import argparse
import copy
import hashlib
import json
import re
import subprocess
import xml.etree.ElementTree as ET
from fractions import Fraction
from jsonschema import Draft202012Validator
from pathlib import Path

import canary_encounters as ce
import validate_encounter as ve

ROOT = Path(__file__).resolve().parent
MONSTERS = ce.MONSTER_DIR + '/quests/soul_war/'
TELEPORTERS = {
    'bony_sea_devil': 'normal_monsters/bony_sea_devil.lua',
    'brachiodemon': 'normal_monsters/brachiodemon.lua',
    'branchy_crawler': 'normal_monsters/branchy_crawler.lua',
    'cloak_of_terror': 'normal_monsters/furious_crater/cloak_of_terror.lua',
    'many_faces': 'normal_monsters/many_faces.lua',
    'spiteful_spitter': 'spiteful_spitter.lua',
    'dreadful_harvester': 'dreadful_harvester.lua',
}
VOCATIONS = ('druid', 'knight', 'paladin', 'sorcerer', 'monk')
POSITION = r'Position\(\s*(\d+),\s*(\d+),\s*(\d+)\s*\)'
CORNER = r'\{\s*x\s*=\s*(\d+),\s*y\s*=\s*(\d+),\s*z\s*=\s*(\d+)\s*\}'


def pinned_bytes(canary, path):
    return subprocess.run(['git', '-C', str(canary), 'show', ce.REVISION + ':' + path],
                          check=True, capture_output=True).stdout


def zone_location(text):
    """Read exactly the five add/subtract areas and six boss lever specPos boxes."""
    boxes, minus = [], []
    zones = []
    pattern = r'SoulWarQuest\.areaZones\.(\w+):(addArea|subtractArea)\(' + CORNER + r',\s*' + CORNER + r'\)'
    for match in re.finditer(pattern, text):
        zone, operation, *values = match.groups()
        ax, ay, az, bx, by, bz = map(int, values)
        if operation == 'addArea':
            zones.append(zone)
        for floor in range(min(az, bz), max(az, bz) + 1):
            (boxes if operation == 'addArea' else minus).append(ce.box((ax, bx), (ay, by), floor))
    rooms = re.findall(r'specPos\s*=\s*\{\s*from\s*=\s*' + POSITION + r',\s*to\s*=\s*' + POSITION, text)
    if len(zones) != 5 or len(set(zones)) != 5 or len(minus) != 5 or len(rooms) != 6:
        raise ValueError('pinned Soul War zone/room census no longer matches SW-5')
    for values in rooms:
        ax, ay, az, bx, by, bz = map(int, values)
        if az != bz:
            raise ValueError('boss room spans floors unexpectedly')
        boxes.append(ce.box((ax, bx), (ay, by), az))
    if len(boxes) != 23:
        raise ValueError('SW-5 needs exactly 23 boxes')
    return {'boxes': boxes, 'minus': minus}


def teleport_rule(role, text):
    return {'key': role + '_taint_teleport',
            'trigger': {'kind': 'timer_elapsed', 'timer': 'taint_check', 'each': role},
            'conditions': [
                {'kind': 'creature_present', 'players': True, 'near': {'triggering': True, 'radius': 30},
                 'where': [{'kind': 'killer_progress', 'subject': {'candidate': True},
                            'progress': 'canary:quest-progress/soul_war_taint_1', 'op': '==', 'value': True},
                           {'kind': 'in_anchor', 'subject': {'candidate': True}, 'anchor': 'soul_war_taint_zones'}],
                 'pick': 'farthest', 'present': True}, {'kind': 'chance_percent', 'value': 10}],
            'actions': [{'kind': 'teleport', 'who': {'triggering': True}, 'to': {'picked_position': True},
                         'after_ms': 2000, 'picked_cooldown_ms': 10000, 'say': text,
                         'warning_effect': 'canary.appearance:effect/mortarea',
                         'arrival_effect': 'canary.appearance:effect/teleport'}]}


def mirror_rules():
    rules = []
    for vocation in VOCATIONS:
        branches = [{'weight': 28 if other == vocation else 3,
                     'actions': [{'kind': 'transform', 'role': 'mirror_image', 'health': 'full',
                                  'into': ce.creature(other + "'s Apparition")}]} for other in VOCATIONS]
        rules.append({'key': 'mirror_image_turns_for_' + vocation,
                      'trigger': {'kind': 'damage_taken', 'role': 'mirror_image', 'source': 'player',
                                  'base_vocation': vocation}, 'conditions': [],
                      'actions': [{'kind': 'one_of', 'branches': branches}]})
    rules.append({'key': 'mirror_image_floor', 'trigger': {'kind': 'lethal_damage', 'role': 'mirror_image'},
                  'conditions': [{'kind': 'killer_is_player', 'value': False}],
                  'actions': [{'kind': 'prevent_death', 'role': 'mirror_image'}]})
    return rules


def pool_preparation(canary, library, mechanics):
    """SW-6 facts are complete independently of the unresolved pool creation policy."""
    match = re.search(r'theBloodOfCloakTerrorIds\s*=\s*\{([^}]+)\}', library)
    ids = [int(value) for value in re.findall(r'\d+', match[1])] if match else []
    if ids != [33854, 34006, 34007]:
        raise ValueError('SW-6 source pool item identity changed')
    percentages = re.search(r'poolDamagePercentages\s*=\s*\{([^}]+)\}', library)
    shares = {int(item): Fraction(value) for item, value in
              re.findall(r'\[(\d+)\]\s*=\s*([\d.]+)', percentages[1] if percentages else '')}
    if set(shares) != set(ids):
        raise ValueError('SW-6 pool shares are incomplete')
    heal = re.search(r'local healAmount\s*=\s*math\.random\((\d+),\s*(\d+)\)', mechanics)
    required = ('player:addHealth(-damage, COMBAT_ENERGYDAMAGE)',
                'local damage = maxHealth * damagePercentage', 'monster:getName() == "Cloak of Terror"',
                'monster:addHealth(healAmount)', 'item:remove()')
    if not heal or not all(value in mechanics for value in required):
        raise ValueError('SW-6 source step-in semantics changed')
    content = pinned_bytes(canary, 'data/items/items.xml')
    xml = ET.fromstring(content)
    items = {}
    for node in xml.findall('item'):
        if node.get('id') and int(node.get('id')) in ids:
            item_id = int(node.get('id'))
            attributes = {child.get('key').lower(): child.get('value') for child in node.findall('attribute')}
            items[item_id] = {'item': ce.ref('Item', 'canary:item/' + str(item_id)),
                             'name': node.get('name'), 'source_attributes': attributes,
                             'duration_ms': int(attributes['duration']) * 1000,
                             'decay_target_source_id': int(attributes['decayto']),
                             'player_max_health_share': {'numerator': shares[item_id].numerator,
                                                       'denominator': shares[item_id].denominator}}
    if set(items) != set(ids):
        raise ValueError('SW-6 pool decay definitions missing')
    if [items[item]['decay_target_source_id'] for item in ids] != [34006, 34007, 0]:
        raise ValueError('SW-6 decay chain changed')
    health_path = 'src/lua/functions/creatures/creature_functions.cpp'
    number_path = 'src/lua/functions/lua_functions_loader.hpp'
    health = pinned_bytes(canary, health_path).decode('utf-8')
    numbers = pinned_bytes(canary, number_path).decode('utf-8')
    if 'damage.primary.value = Lua::getNumber<int32_t>(L, 2);' not in health or 'return static_cast<T>(number);' not in numbers:
        raise ValueError('SW-6 source numeric conversion requires a fresh read')
    return {'classification': 'OTS_HYPOTHESIS_ONLY', 'source_repository': ce.REPOSITORY,
            'source_revision': ce.REVISION, 'admission_authorized': False, 'runtime_qualified': False,
            'interaction_key': 'canary:interaction/blood_of_cloak_of_terror', 'source_items': [items[i] for i in ids],
            'player_step_in': {'source_damage_type': 'COMBAT_ENERGYDAMAGE', 'basis': 'player_max_health',
                               'source_expression': 'maxHealth * damagePercentage',
                               'source_numeric_conversion': 'Lua numeric product -> signed int32 truncation toward zero for in-range values',
                               'native_numeric_parity': 'UNKNOWN_NOT_QUALIFIED'},
            'cloak_step_in': {'source_exact_name': 'Cloak of Terror',
                              'heal_minimum': int(heal[1]), 'heal_maximum': int(heal[2])},
            'every_creature_step_in': {'remove_item': True},
            'creation_policy': {'status': 'UNKNOWN', 'stack_or_enlarge': 'UNKNOWN', 'unless_present': 'UNDECIDED'},
            'source_create_behavior': 'One largest pool only when item 33854 is absent; the source sign test responds to heals.',
            'accepted_deviation': 'Q6a: a player hit creates blood; no source bug adopted as final behavior.',
            'sources': [{'path': ce.SOUL_WAR_LIB, 'lines': [14, 20, 21, 22, 23, 24], 'blob_sha1': ce.blob_id(library.encode())},
                        {'path': ce.SOUL_WAR_MECHANICS, 'lines': list(range(785, 836)), 'blob_sha1': ce.blob_id(mechanics.encode())},
                        {'path': 'data/items/items.xml', 'blob_sha1': ce.blob_id(content)},
                        {'path': health_path, 'lines': list(range(583, 602)), 'blob_sha1': ce.blob_id(health.encode())},
                        {'path': number_path, 'lines': list(range(214, 233)), 'blob_sha1': ce.blob_id(numbers.encode())}],
            'dependency_blockers': ['Canonical Item semantics/decay admission for all three Item identities',
                                    'Interaction step-in owner/runtime and integer conversion semantics',
                                    'Pool stacking/enlargement evidence before SW-6 create policy',
                                    'E4 entire Soul War participant cohort closure']}


def prepare(canary):
    # Regenerate the plain source baseline, not the accepted sample we return.
    # The original transcriber supports this narrow encounter without any writes;
    # sparse source files are read by immutable Git object below, never synthesized.
    class PinnedBaseline(ce.Encounters):
        def __init__(self, canary):
            self.canary, self.items, self.monster_files = canary, {}, {}
            paths = subprocess.run(['git', '-C', str(canary), 'ls-tree', '-r', '--name-only',
                                    ce.REVISION, '--', MONSTERS], check=True,
                                   capture_output=True, text=True).stdout.splitlines()
            for path in sorted(p for p in paths if p.endswith('.lua')):
                content = pinned_bytes(canary, path)
                local = canary / path
                # Check before event selection: a removed registration must not
                # silently remove its monster from the evidence or covers roster.
                if local.exists() and local.read_bytes() != content:
                    raise ValueError('source working bytes differ from the pinned object: ' + path)
                text = content.decode('utf-8')
                match = re.search(r'Game\.createMonsterType\("([^"]+)"', text)
                if match:
                    key = match[1].lower()
                    if key in self.monster_files:
                        raise ValueError('duplicate pinned Soul War registration: ' + key)
                    self.monster_files[key] = (match[1], path, text)

        def source(self, item, path):
            sources = item['manifest']['sources']
            for index, existing in enumerate(sources):
                if existing['path'] == path:
                    return index
            content = pinned_bytes(canary, path)
            local = canary / path
            if local.exists() and local.read_bytes() != content:
                raise ValueError('source working bytes differ from the pinned object: ' + path)
            sources.append({'kind': 'git', 'repository': ce.REPOSITORY, 'revision': ce.REVISION,
                            'path': path, 'blob_sha1': ce.blob_id(content)})
            return len(sources) - 1
    build = PinnedBaseline(canary)
    ce.soul_war_taint_zones(build)
    baseline = build.items['soul_war_taint_zones']
    encounter = baseline['encounter']
    manifest = baseline['manifest']
    catalog = {'definitions': baseline['definitions']}
    source_text = {}
    for source in manifest['sources']:
        content = pinned_bytes(canary, source['path'])
        if ce.blob_id(content) != source['blob_sha1']:
            raise ValueError('baseline source is not the pinned Git blob: ' + source['path'])
        source_text[source['path']] = content.decode('utf-8')

    def source(path):
        for index, item in enumerate(manifest['sources']):
            if item['path'] == path:
                return index
        content = pinned_bytes(canary, path)
        local = canary / path
        if local.exists() and local.read_bytes() != content:
            raise ValueError('source working bytes differ from the pinned object: ' + path)
        source_text[path] = content.decode('utf-8')
        manifest['sources'].append({'kind': 'git', 'repository': ce.REPOSITORY, 'revision': ce.REVISION,
                                    'path': path, 'blob_sha1': ce.blob_id(content)})
        return len(manifest['sources']) - 1

    def entry(path, lines, resolution, destination=None, status='mapped'):
        value = {'source_index': source(path), 'source_lines': lines, 'status': status, 'resolution': resolution}
        if destination:
            value['destination'] = destination
        manifest['entries'].append(value)

    # SW-4: the last event assignment replaces the first, exactly as the Lua registrar does.
    mirror_path = MONSTERS + 'mirror_image.lua'
    mirror = source_text[mirror_path]
    lists = re.findall(r'monster\.events\s*=\s*\{([^}]*)\}', mirror, re.S)
    if not lists or re.findall(r'"([^"]+)"', lists[-1]) != ['MirrorImageTransform']:
        raise ValueError('Mirror Image effective event registration changed')
    mirror_key = ce.creature('Mirror Image')['key']
    hunting = encounter['participants'][0]['creatures']
    hunting[:] = [creature for creature in hunting if creature['key'] != mirror_key]
    manifest['covers']['FourthTaintBossesPrepareDeath'].remove(mirror_key)
    manifest['entries'] = [row for row in manifest['entries'] if not (
        row['source_index'] == 0 and row.get('destination') == '/encounter/participants/0/creatures')]
    entry(mirror_path, [16, 17, 109, 110, 111], 'The last monster.events assignment replaces the first; '
          'Mirror Image leaves hunting_monster and has only MirrorImageTransform.', status='approved_omission')
    encounter['participants'].append({'role': 'mirror_image', 'creatures': [ce.creature('Mirror Image')]})

    # The repeating check and all seven role actions are the explicit accepted Q1a data.
    encounter['state']['timers'].append({'name': 'taint_check', 'duration_ms': 2000, 'repeat': True})
    encounter['rules'].append({'key': 'taint_check_runs', 'trigger': {'kind': 'encounter_started'}, 'conditions': [],
                               'actions': [{'kind': 'timer', 'timer': 'taint_check', 'operation': 'start'}]})
    for role, relative in TELEPORTERS.items():
        path = MONSTERS + relative
        source(path)
        text = source_text[path]
        if role == 'dreadful_harvester':
            saying = 'You have been chosen for a harvest!'
            if saying not in text:
                raise ValueError('Dreadful Harvester accepted voice-line evidence missing')
            lines = [next(i for i, line in enumerate(text.splitlines(), 1) if saying in line)]
            resolution = 'Accepted Q1a adds Dreadful Harvester from the cited wiki evidence; text is DERIVED '
            resolution += 'from this voice line, not an existing Canary teleport callback. (§13.1/13.6)'
        else:
            match = re.search(r'mType\.onThink\s*=\s*function\([^)]*\)\s*'
                              r'\w+:tryTeleportToPlayer\("([^"]+)"\)\s*end', text)
            if not match:
                raise ValueError('teleport callback no longer matches the full accepted template: ' + path)
            saying = match[1]
            lines = [text[:match.start()].count('\n') + 1, text[:match.end()].count('\n') + 1]
            resolution = 'The entire onThink is tryTeleportToPlayer; relocated under SW-3 and Q1a.'
            manifest['covers'].setdefault('mType.onThink', []).append(ce.creature(role)['key'])
        encounter['participants'].append({'role': role, 'creatures': [ce.creature(role)]})
        encounter['rules'].append(teleport_rule(role, saying))
        entry(path, lines, resolution, '/encounter/rules/' + str(len(encounter['rules']) - 1))
        catalog['definitions'].append(ce.creature(role))

    lib = ce.SOUL_WAR_LIB
    encounter['anchors'][0]['location'] = zone_location(source_text[lib])
    encounter['anchors'][0]['description'] = 'Accepted SW-5/Q2a: five hunting grounds and six boss rooms minus safe areas.'
    entry(lib, list(range(862, 882)), 'SW-5 reads addArea/subtractArea and six lever specPos; Q2a normalizes '
          'Rotten Wasteland reversed x corners to the stated full rectangle.', '/encounter/anchors/0/location')
    entry(lib, list(range(1241, 1299)), 'SW-3/Q1a: first-taint holders only; 2-second repeating checks; farthest '
          'candidate; 10% chance, 2-second warning, 10-second cooldown after a successful move. The skipped taint '
          'test, 1-second check and logout-stuck cooldown are accepted source deviations; no path requirement.',
          '/encounter/rules')

    encounter['rules'].extend(mirror_rules())
    manifest['covers']['mType.onPlayerAttack'] = [mirror_key]
    manifest['covers']['MirrorImageTransform'] = [mirror_key]
    entry(mirror_path, list(range(113, 144)), 'SW-4/Q5b: own player damage transforms by base vocation; weights '
          '28:3:3:3:3 produce 70%/7.5%; full health. Other damage cannot kill it. No-vocation removal is omitted '
          'as an accepted deviation.', '/encounter/rules')
    transform = 'data-otservbr-global/scripts/creaturescripts/monster/mirror_image_transform.lua'
    entry(transform, list(range(1, 23)), 'The separate player DoT path is superseded by accepted Q5b: all own '
          'player damage follows the same weighted full-health transformation.', status='approved_omission')
    for vocation in VOCATIONS:
        catalog['definitions'].append(ce.creature(vocation + "'s Apparition"))
    # Do not claim or produce the provisional SW-6 action before the pool policy is known.
    entry(ce.SOUL_WAR_MECHANICS, list(range(785, 836)), 'SW-6/Q6a pool creation remains blocked: whether repeated '
          'hits stack pools or enlarge one pool is unverified. No unless_present choice or covers claim is made.',
          status='unresolved_semantics')
    catalog['definitions'] = list({(r['family'], r['key'], r['revision']): r
                                  for r in catalog['definitions']}.values())
    encounter['display_name'] = 'Soul War taints and Mirror Image: accepted SW-3/4/5 draft'
    errors = ve.validate(encounter, catalog, manifest)
    if errors:
        raise ValueError('accepted draft validation failed: ' + '; '.join(errors))
    cohort = sorted({ref['key'] for participant in encounter['participants'] for ref in participant['creatures']})
    receipt = {'source_revision': ce.REVISION, 'schema_valid': True, 'semantic_valid': True,
               'manifest_resolved': False, 'admission_authorized': False, 'native_ready': False,
               'runtime_qualified': False, 'accepted_extensions': ['SW-3', 'SW-4', 'SW-5'],
               'remaining': ['SW-6 pool stacking/enlargement evidence', 'E4 entire participant cohort closure'],
               'counts': {'rules': len(encounter['rules']), 'teleport_roles': len(TELEPORTERS),
               'area_boxes': 23, 'safe_area_boxes': 5}, 'covers': copy.deepcopy(manifest['covers']),
               'e4_cohort': {'creatures': cohort, 'creature_count': len(cohort), 'admission_ready': False,
                             'policy': 'All referenced participant definitions and complete Encounter together; no partial admission.'}}
    pools = pool_preparation(canary, source_text[lib], source_text[ce.SOUL_WAR_MECHANICS])
    evidence_schema = json.loads((ROOT.parent / 'monster-authoring/custom-pattern-preparation.schema.json').read_text())
    Draft202012Validator(evidence_schema['$defs']['poolSourcePreparation']).validate(pools)
    return {'encounter.json': encounter, 'manifest.json': manifest, 'catalog.json': catalog,
            'receipt.json': receipt, 'pool-source-preparation.json': pools}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    repo = ROOT.parents[2]
    if out == repo or repo in out.parents:
        raise ValueError('candidate output must be external; E4 admission is not authorized')
    artifacts = prepare(args.canary)
    out.mkdir(parents=True, exist_ok=True)
    hashes = {}
    for name, value in artifacts.items():
        content = (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
        (out / name).write_bytes(content)
        hashes[name] = hashlib.sha256(content).hexdigest()
    (out / 'hashes.json').write_text(json.dumps(hashes, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(artifacts['receipt.json']))


if __name__ == '__main__':
    main()
