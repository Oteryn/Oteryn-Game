"""Qualify selected perk values against the independent official-source audit.

Semantic authoring may tune numbers; claiming this source-qualified snapshot may
not. Raw planner observations stay immutable when newer official news supersedes
them. This verifies data, not combat execution or Global backend behavior.
"""
import hashlib
import re
from wheel_authoring import ROOT, VOCATIONS, read


def normalized(text):
    return re.sub(r'\s+', ' ', text).strip()


def validate_official_perks(candidate):
    def require(condition, code):
        if not condition:
            raise ValueError(code)

    binding = read(ROOT / 'samples/reference-selection.json')['official_perk_audit']
    require(binding['file'] == 'samples/official-perk-reference.json', 'OFFICIAL_PERK_PATH')
    path = ROOT / binding['file']
    require(hashlib.sha256(path.read_bytes()).hexdigest() == binding['sha256'], 'OFFICIAL_PERK_DIGEST')
    reference = read(path)
    require(reference['schema'] == 'OTERYN_WHEEL_OFFICIAL_PERK_REFERENCE/v1' and
            reference['runtime_admitted'] is False and
            reference['live_global_parity_confirmed'] is False, 'OFFICIAL_PERK_ENVELOPE')
    require(set(reference['vocations']) == set(VOCATIONS), 'OFFICIAL_VOCATION_COVERAGE')
    for vocation in VOCATIONS:
        expected = reference['vocations'][vocation]
        data = candidate['vocations'][vocation]
        require(len(expected['slots']) == len(data['slots']) == 36, 'OFFICIAL_SLOT_COVERAGE')
        require([s['state_slot'] for s in expected['slots']] == list(range(1, 37)),
                'OFFICIAL_SLOT_IDENTITIES')
        seen_augments, seen_unique = set(), set()
        for slot, fact in zip(data['slots'], expected['slots'], strict=True):
            top = candidate['topology'][slot['state_slot'] - 1]
            conviction = slot['conviction']
            require(slot['state_slot'] == fact['state_slot'] and
                    top['source']['tibiapal_tile'] == fact['tile'] and
                    top['capacity'] == fact['capacity'] and
                    conviction['source_info_id'] == fact['conviction_id'] and
                    slot['dedication_icon']['source_index'] == fact['dedication_id'],
                    'OFFICIAL_SLOT_BINDING')
            dedication = [{k: e[k] for k in ('stat', 'value_per_point')}
                          for e in slot['dedication']]
            require(dedication == fact['dedication'], 'OFFICIAL_DEDICATION_VALUE')
            require(conviction['native_value'] == fact['native_value'], 'OFFICIAL_CONVICTION_VALUE')
            key = conviction['key']
            if conviction['augment_stages']:
                seen_augments.add(key)
                require(key in expected['augments'], 'OFFICIAL_AUGMENT_COVERAGE')
                for stage in conviction['augment_stages']:
                    audit = expected['augments'][key][str(stage['stage'])]
                    require(normalized(stage['reference_text']) == normalized(audit['reference_text']),
                            'OFFICIAL_AUGMENT_DESCRIPTION')
                    actual = {e['kind']: e for e in stage['numeric_effects']}
                    require(len(actual) == len(stage['numeric_effects']), 'OFFICIAL_DUPLICATE_EFFECT')
                    for effect in audit['numeric_effects']:
                        require(actual.get(effect['kind']) == effect, 'OFFICIAL_AUGMENT_VALUE')
                    if key == 'augmented_flurry_of_blows' and stage['stage'] == 1:
                        require('range_increase' not in actual and stage['area_reference'] is not None,
                                'OFFICIAL_FLURRY_AREA')
            if conviction['unique_parameters'] is not None:
                seen_unique.add(key)
                require(key in expected['unique'], 'OFFICIAL_UNIQUE_COVERAGE')
                audit = expected['unique'][key]
                require(normalized(conviction['reference_description']) ==
                        normalized(audit['reference_description']), 'OFFICIAL_UNIQUE_DESCRIPTION')
                actual = {e['kind']: e['value'] for e in conviction['unique_parameters']['numeric_effects']}
                require(actual == audit['numeric_effects'], 'OFFICIAL_UNIQUE_VALUE')
        require(seen_augments == set(expected['augments']) and
                seen_unique == set(expected['unique']), 'OFFICIAL_CONVICTION_COVERAGE')
        require({r['key'] for r in data['revelations']} == set(expected['revelations']),
                'OFFICIAL_REVELATION_COVERAGE')
        for revelation in data['revelations']:
            audit = expected['revelations'][revelation['key']]
            require(normalized(revelation['stage_zero_description']) ==
                    normalized(audit['reference_descriptions']['0']), 'OFFICIAL_REVELATION_DESCRIPTION')
            for stage in revelation['stages']:
                number = stage['stage']
                require(normalized(stage['reference_description']) ==
                        normalized(audit['reference_descriptions'][str(number)]),
                        'OFFICIAL_REVELATION_DESCRIPTION')
                actual = {e['kind']: e['value'] for e in stage['numeric_effects']}
                for kind, values in audit['numeric_effects'].items():
                    require(actual.get(kind) == values[number - 1], 'OFFICIAL_REVELATION_VALUE')


if __name__ == '__main__':
    validate_official_perks(read(ROOT / 'samples/wheel-candidate.json'))
    print('PASS: official-source perk values and descriptions across five vocations / 180 slots.')
