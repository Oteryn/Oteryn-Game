"""Check the SPELL-PRESENT-0 §3 presentation bindings without the source checkouts.

- client-ids.json binds every effect, missile and sound key used in content/ (the hit table
  included) and no other key; each effect and missile id is one appearances.dat defines.
- hit-table.json colours are PresentationColor members, and it covers every race and damage type.
- creature-races.json gives every Creature one known race.
- spell-cues.json and client-ids.json agree on every key both bind.
"""
import json
import re
import sys
from pathlib import Path

import build_presentation_bindings as bindings
from build_creature_races import OUT as CREATURE_RACES, creature_records

ROOT = bindings.ROOT
sys.path.insert(0, str(ROOT / 'tools' / 'content-census'))
import stage_proficiencies  # noqa: E402

APPEARANCES = 'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'
# appearances.dat top-level fields: 3 effects, 4 missiles; each record's field 1 is its id.
APPEARANCE_FIELDS = {'effect': 3, 'missile': 4}
# The source's "no effect" members: id 0, which no appearance record carries.
NONE_MEMBERS = {'CONST_ME_NONE', 'CONST_ANI_NONE'}
COLOR_ENUM = ROOT / 'crates' / 'protocol-oteryn' / 'src' / 'presentation.rs'
SPELL_CUES = bindings.BINDINGS / 'spell-cues.json'
DAMAGE_TYPES = set(bindings.DAMAGE.values()) | {'mana_drain', 'healing', 'untyped'}


def appearance_ids(data):
    ids = {kind: set() for kind in APPEARANCE_FIELDS}
    for field, _, value in stage_proficiencies.parse(data):
        for kind, number in APPEARANCE_FIELDS.items():
            if field == number:
                ids[kind].add(next(v for f, _, v in stage_proficiencies.parse(value) if f == 1))
    return ids


def colors(source):
    body = re.search(r'pub enum PresentationColor \{(.*?)\}', source, re.S).group(1)
    return {re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower() for name in re.findall(r'(\w+) =', body)}


def key_kind(key):
    # canary.sound:<name> and canary.sound:effect/<n> are sounds; appearance keys name their kind.
    namespace, name = key.split(':', 1)
    return 'sound' if namespace.endswith('sound') else name.split('/', 1)[0]


def check(client_ids, hits, races, cues, keys, ids, color_names, creatures):
    errors = []
    records = {}
    for record in client_ids['records']:
        if record['key'] in records:
            errors.append('duplicate client id record ' + record['key'])
        records[record['key']] = record
    dynamic = set(bindings.DYNAMIC)
    if client_ids['dynamic_keys'] != bindings.DYNAMIC:
        errors.append('dynamic keys are not exactly the runtime-resolved weapon missile')
    for key in sorted(keys - set(records) - dynamic):
        errors.append('content key without a client id ' + key)
    for key in sorted(set(records) - keys):
        errors.append('client id record no content uses ' + key)
    for key, record in sorted(records.items()):
        if record['kind'] != key_kind(key):
            errors.append('%s kind %s is not its key kind %s' % (key, record['kind'], key_kind(key)))
        elif record['client_id'] is None:
            if key not in bindings.UNDEFINED:
                errors.append('client id missing for ' + key)
        elif record['source_member'] in NONE_MEMBERS:
            if record['client_id'] != 0:
                errors.append('no-effect key %s is not id 0' % key)
        elif record['kind'] in ids and record['client_id'] not in ids[record['kind']]:
            errors.append('%s id %d is not in appearances.dat' % (key, record['client_id']))
    for row in hits['physical_by_race'] + hits['by_damage_type']:
        if row['color'] is not None and row['color'] not in color_names:
            errors.append('hit table colour outside PresentationColor ' + row['color'])
    if sorted(r['race'] for r in hits['physical_by_race']) != sorted(bindings.RACES):
        errors.append('hit table races are not exactly the source races')
    if {r['damage_type'] for r in hits['by_damage_type']} != DAMAGE_TYPES - {'physical'}:
        errors.append('hit table damage types are not exactly the Oteryn damage types')
    if hits['player_race'] not in bindings.RACES:
        errors.append('unknown player race ' + hits['player_race'])
    by_creature = {(r['creature']['key'], r['creature']['revision']): r['race'] for r in races['records']}
    if len(by_creature) != len(races['records']) or races['record_count'] != len(by_creature):
        errors.append('creature race records are not one per creature')
    if set(by_creature) != creatures:
        errors.append('creature races do not cover exactly the Creature family')
    errors += ['unknown race %s for %s' % (race, creature[0])
               for creature, race in sorted(by_creature.items()) if race not in bindings.RACES]
    for cue in cues['records']:
        record = records.get(cue['alias'])
        if record is None or record['client_id'] != cue['value']:
            errors.append('spell-cues.json and client-ids.json disagree on ' + cue['alias'])
    return errors


def main():
    client_ids = json.loads(bindings.CLIENT_IDS.read_bytes())
    hits = json.loads(bindings.HIT_TABLE.read_bytes())
    errors = check(client_ids, hits, json.loads(CREATURE_RACES.read_bytes()),
                   json.loads(SPELL_CUES.read_bytes()), bindings.content_keys(bindings.CONTENT, hits),
                   appearance_ids((ROOT / APPEARANCES).read_bytes()),
                   colors(COLOR_ENUM.read_text()),
                   {(r['definition']['identity']['key'], r['definition']['identity']['revision'])
                    for r in creature_records()})
    for error in errors:
        print('FAIL ' + error)
    if not errors:
        print('PASS presentation bindings records=%d creatures=%d'
              % (len(client_ids['records']), len(json.loads(CREATURE_RACES.read_bytes())['records'])))
    return 1 if errors else 0


if __name__ == '__main__':
    raise SystemExit(main())
