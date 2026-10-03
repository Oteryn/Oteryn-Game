"""Bounded source-backed authoring completions; no runtime or owner-acceptance claim.

The converter must represent every returned field before allowing a completion
to participate in plain combat extraction. Full-file source hashes fail closed
when a pinned implementation changes. No Lua script is rewritten or executed.
"""
import argparse
import copy
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

REVISIONS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
             'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
PARALYZE_BODY = ('if not combat:execute(creature, var) then return false end '
                 'creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN) return true')
SPECS = {
    ('instant', 'heal friend', 'canary'): {
        'file': 'data/scripts/spells/healing/heal_friend.lua',
        'sha256': 'eac2cbfaa994562b35b2f6d7448d8354270bf1f440d607d6198261549eb6008e',
        'body': 'creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) return combat:execute(creature, variant)',
        'kind': 'caster_presentation', 'caster_effect_asset_binding': 'canary.appearance:effect/magic_blue',
        'caster_effect_timing': 'before_combat', 'drop_params': [],
        'contract': 'OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md D.5',
    },
}
for _source in REVISIONS:
    SPECS[('rune', 'paralyze rune', _source)] = {
        'file': 'data/scripts/runes/paralyze_rune.lua',
        'sha256': '40b12d7797b1cc0448f1b8020543a977486db671c0be501408de0b15f0bd6781',
        'body': PARALYZE_BODY, 'kind': 'caster_presentation',
        'caster_effect_asset_binding': 'canary.appearance:effect/magic_green',
        'caster_effect_timing': 'after_success', 'drop_params': ['COMBAT_PARAM_TYPE'],
        'zero_damage_health_path': True,
        'contract': ('OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md D.5; this hash-pinned rune declares '
                     'COMBAT_UNDEFINEDDAMAGE without a direct-health formula/value callback: keep its paralysis '
                     'condition and visuals, without inventing a damage Effect. Ability.zero_damage_health_path '
                     'requires zero-magnitude health validation/block/change-health before condition/dispel '
                     '(combat.cpp:1496,878,912-914); runtime must reject this field until that path is implemented. '
                     'UNDEFINEDDAMAGE is not equivalent to COMBAT_NONE globally.'),
    }
for _source, _digest in [('canary', 'e43d1f1f8a37ef951b9c5c8ecddab7f3b2752fd4092784dc0f5214150abde4c5'),
                         ('crystal', 'ae79840bd482d5448be27de21a37c08848d827e053037e2d6b537e3bb46de08e')]:
    SPECS[('instant', 'inflict wound', _source)] = {
        'file': 'data/scripts/spells/attack/inflict_wound.lua', 'sha256': _digest,
        'body': 'return combat:execute(creature, var)', 'kind': 'target_selection',
        'target_selection': 'caster_or_top_creature',
        'drop_params': ['COMBAT_PARAM_TARGETCASTERORTOPMOST'],
        'contract': 'OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md D.6.2; combat.cpp:1737-1748',
    }


def cast_body(text):
    match = re.search(r'function\s+[\w.:]*onCastSpell\s*\([^)]*\)(.*?)\nend\b', text, re.S)
    return re.sub(r'\s+', ' ', match.group(1)).strip() if match else None


def identify_completion(name, spell_type, source, text):
    """Return a fresh completion only for one exact pinned implementation."""
    spec = SPECS.get((spell_type, str(name).lower(), source))
    if not spec or hashlib.sha256(text.encode()).hexdigest() != spec['sha256'] or cast_body(text) != spec['body']:
        return None
    return {**copy.deepcopy(spec), 'source': source, 'revision': REVISIONS[source],
            'name': str(name).lower(), 'spell_type': spell_type, 'authority': 'OtsHypothesisOnly'}


def accepted_cast_bodies(root, source):
    """Exact-body extraction allowlist. Caller must apply all completion fields."""
    result = {}
    for (spell_type, name, src), spec in SPECS.items():
        if source != src:
            continue
        path = Path(root) / spec['file']
        if path.is_file() and identify_completion(name, spell_type, source, path.read_text(encoding='utf-8')):
            result.setdefault(name, set()).add(spec['body'])
    return result


def apply_completion(spec, dependencies):
    """Attach represented semantics; reject ambiguous graphs instead of losing them."""
    abilities = dependencies['abilities']
    if len(abilities) != 1 or 'variants' in abilities[0] or not abilities[0].get('effects'):
        raise ValueError('completion requires one ordinary Ability with Effects')
    if spec['kind'] == 'target_selection':
        if 'target_selection' in abilities[0]:
            raise ValueError('target selection was already declared')
        abilities[0]['target_selection'] = spec['target_selection']
        return
    if spec['kind'] != 'caster_presentation':
        raise ValueError('unknown completion kind')
    first = abilities[0]['effects'][0]
    effects = [effect for effect in dependencies['effects']
               if effect['identity']['key'] == first['key'] and effect['identity']['revision'] == first['revision']]
    if len(effects) != 1:
        raise ValueError('first referenced Effect must have one local payload')
    presentation = effects[0].setdefault('presentation', {})
    if 'caster_effect_asset_binding' in presentation or 'caster_effect_timing' in presentation:
        raise ValueError('caster effect was already declared')
    if 'zero_damage_health_path' in spec:
        if spec['zero_damage_health_path'] is not True or 'zero_damage_health_path' in abilities[0]:
            raise ValueError('zero-damage health path requires one explicit true declaration')
        abilities[0]['zero_damage_health_path'] = True
    presentation.update({key: spec[key] for key in ('caster_effect_asset_binding', 'caster_effect_timing')})


SECTION = {'delayed_or_repeated': 'D.1', 'target_position': 'D.2', 'target_default': 'D.3',
           'equipment_dependent': 'D.4', 'extra_presentation_only': 'D.5', 'other': 'D.6',
           'wheel_of_destiny': 'A.1', 'monk_harmony_virtue': 'A.2', 'world_query': 'B.1',
           'house': 'B.2', 'player_parameter': 'B.3', 'item_grant': 'B.4', 'familiar': 'C.1',
           'summons_share_condition': 'C.2', 'party': 'C.3', 'stance': 'C.4', 'conditional_self_state': 'C.5'}


def audit(readiness, census, removed):
    records = {(kind, row['name'].lower(), source): row for source in ('canary', 'crystal')
               for row in census[source] for kind in [row['spell_type']]}
    result = []
    for row in readiness['spells']:
        if row['status'] != 'blocked':
            continue
        identity = (row['spell_type'], row['name'])
        sources = {source: records[(row['spell_type'], row['name'], source)] for source in row['sources']}
        patterns = sorted({pattern for record in sources.values() for pattern in record['cast'].get('patterns', [])})
        if identity in removed:
            lane = 'removed_do_not_complete'
        elif any((row['spell_type'], row['name'], source) in SPECS for source in row['sources']):
            lane = 'bounded_completion_hook'
        elif any('formula' in blocker for blocker in row['blockers']):
            lane = 'formula_extraction_or_input_contract'
        elif row['name'] in ('balanced brawl', 'challenge', 'chivalrous challenge', 'divine dazzle'):
            lane = 'monster_ai_override_contract_and_runtime'
        elif row['spell_type'] == 'rune' and row['name'] in ('magic wall rune', 'wild growth rune'):
            lane = 'create_item_lifetime_world_variant_and_refusal_contract'
        elif row['name'] == 'mass spirit mend':
            lane = 'callback_target_routing_caster_formula_and_area_gap'
        else:
            lane = 'documented_native_repertoire_not_implemented'
        result.append({'name': row['name'], 'spell_type': row['spell_type'], 'repair_lane': lane,
                       'blockers': row['blockers'], 'patterns': patterns,
                       'candidate_sections': sorted({SECTION[p] for p in patterns if p in SECTION}),
                       'source_evidence': {source: {'file': record['file'], 'blob': record['blob'], 'cast': record['cast'],
                           'revision': REVISIONS[source]} for source, record in sources.items()},
                       'qualification': 'Candidate document sections describe shape/evidence; do not assert an accepted core/runtime contract.'})
    return {'schema': 'OTERYN_BLOCKED_SPELL_REPAIR_LANES/v1', 'summary': dict(Counter(r['repair_lane'] for r in result)),
            'spells': result, 'note': 'No readiness was changed by this audit.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--readiness', type=Path, required=True)
    parser.add_argument('--census', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    changes = json.loads((Path(__file__).parent / 'official-changes.json').read_text())['changes']
    removed = {(change['value'] if change['value'] != 'yes' else 'instant', change['spell'])
               for change in changes if change['field'] == 'removed'}
    report = audit(json.loads(args.readiness.read_text()), json.loads(args.census.read_text()), removed)
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / 'blocked-repair-lanes.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    lines = ['# Blocked spell repair lanes', '', f"Distribution: {report['summary']}", '',
             'Every live blocked spell is listed below; removed records remain distinct. Candidate shape is evidence, not runtime acceptance.', '']
    lines += [f"- {row['spell_type']} **{row['name']}**: `{row['repair_lane']}`; sections {', '.join(row['candidate_sections']) or 'D.6'}; "
              + '; '.join(row['blockers']) for row in report['spells']]
    (args.out / 'BLOCKERS.md').write_text('\n'.join(lines) + '\n')
    print(json.dumps(report['summary']))


if __name__ == '__main__':
    main()
