"""RUNE-CONTENT-1: audit the 36 rune carriers of the lowered spell catalogue before any rune is admitted.

Evidence tooling only (RUNE-USE-0 §3, §9 and rows `RUNEUSE0-RL-07` and `RUNEUSE0-RL-08`). For each rune
carrier in `content/abilities/definitions/player-spells.json` the audit checks:

- the rune item ID against the wikis (S3) and the Canary and Crystal registrars;
- the rune users (vocations) against the wikis (S3), or, where no wiki states them, the agreeing
  Canary and Crystal registrars;
- that both the Canary and the Crystal registrar records exist;
- the cooldown group, every targeting flag the converter maps (`TARGETING_FLAGS`) and far use against
  the Canary and Crystal registrars, and the range against the wikis and the registrars; where the
  references conflict, our value must be the one the catalogue manifest records as the winning source
  (S3/S11/S13/S14), several recorded sources that now diverge follow the converter's S4 rule (Canary
  decides between the registrars), and a recorded native resolution (S27) requires a value that
  differs from the references;
- the target restriction (`allowed_targets`) against the accepted onCastSpell guard of
  `guard-behaviours.json` (S27); a rune without a guard has none;
- the parameter mode, the positional cast (`optionalTarget`) and Aim at Target as the converter
  derives them (S20); any other targeting field fails as unaudited;
- the dated current rune-use facts of `rune-use-source-resolutions.json`, which replace the field
  they name;
- `rune.charges` against the conjuring spell's `count` where both exist (§3);
- the rune item's stack facts in the native Item profiles (stackable, stack of 100, §3);
- the authored area (hit cells `x` and `C` only): at most 37 tiles, so the victim cap 2 x tiles stays at or below 74 (RL-08),
  and at most 12 tiles for a rune that creates fields (RL-07).

The rune joins and the aliases are those of `verify_spells.py`. A carrier that fails any check stays
blocked. A carrier that passes is still subject to the §9 needs of its group; passing is not runtime
admission.

Usage:
    python audit_rune_carriers.py            # rewrite samples/rune-carrier-audit.json
    python audit_rune_carriers.py --check    # fail if the committed audit is stale
"""
import argparse
import json
import sys
from pathlib import Path

import verify_spells as vs
import wiki_spells as ws

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
CATALOGUE = REPO / 'content/abilities/definitions/player-spells.json'
ITEM_PROFILES = REPO / 'content/items/definitions/spell-native-profiles.json'
CENSUS = HERE / 'samples/spell-census-canary-99902524-crystal-ff7ede5.json'
RESOLUTIONS = HERE / 'rune-use-source-resolutions.json'
GUARDS = HERE / 'guard-behaviours.json'
OUT = HERE / 'samples/rune-carrier-audit.json'
DECISION = 'docs/architecture/reviews/OTERYN_GAME_RUNE_USE0_USING_RUNES_DECISION_2026-09-30.md'

AREA_TILES_MAX = 37  # RUNEUSE0-RL-08: victim cap 2 x tiles, absolute ceiling 74
FIELD_TILES_MAX = 12  # RUNEUSE0-RL-07: fields created by one cast
STACK_MAX = 100
REGISTRARS = ('canary', 'crystal')
# RUNE-USE-0 §9, by the catalogue's rune names.
GROUPS = {
    'direct': ('RUNE-CAST-1', (
        'avalanche rune', 'explosion rune', 'fireball rune', 'great fireball rune', 'heavy magic missile rune',
        'holy missile rune', 'icicle rune', 'light magic missile rune', 'lightest magic missile rune',
        'lightest missile rune', 'light stone shower rune', 'stalagmite rune', 'stone shower rune',
        'sudden death rune', 'thunderstorm rune', 'intense healing rune', 'ultimate healing rune',
        'antidote rune')),
    'condition': ('RUNE-CAST-1; COND-1', ('soulfire rune',)),
    'field': ('FIELD-1; FIELD-WIRE-1', (
        'fire field rune', 'energy field rune', 'poison field rune', 'fire wall rune', 'energy wall rune',
        'poison wall rune', 'firebomb rune', 'energybomb rune', 'poison bomb rune')),
    'blocking_object': ('a later FND-04 gameplay-admission decision (RUNEUSE0-C7, R5)',
                        ('magic wall rune', 'wild growth rune')),
    'native_behaviour': ("each key's own child (S27)", (
        'animate dead rune', 'chameleon rune', 'convince creature rune', 'desintegrate rune',
        'destroy field rune', 'paralyze rune')),
}
REQUIRED = ('item_id', 'vocations', 'group')
# Registrar-backed targeting flags the converter maps (`convert_spells.py` targeting), by registrar call.
TARGETING_FLAGS = (('aggressive', 'isAggressive'), ('self_target', 'isSelfTarget'), ('needs_target', 'needTarget'),
                   ('needs_direction', 'needDirection'), ('target_or_direction', 'needCasterTargetOrDirection'),
                   ('block_walls', 'blockWalls'), ('allow_on_self', 'allowOnSelf'), ('check_floor', 'checkFloor'))
# Every targeting field `convert_spells.Spell.targeting` writes; each has a check above.
AUDITED_TARGETING = ({field for field, _ in TARGETING_FLAGS} |
                     {'range_tiles', 'parameter', 'aim_at_target', 'cast_at_position', 'allowed_targets'})
# The registrar defaults of an omitted call (`convert_spells.DEFAULTS`).
REGISTRAR_DEFAULTS = {'isAggressive': True, 'isSelfTarget': False, 'needTarget': False, 'needDirection': False,
                      'needCasterTargetOrDirection': False, 'blockWalls': True, 'allowOnSelf': True,
                      'checkFloor': True, 'allowFarUse': False}

def group_of(name):
    for group, (needs, names) in GROUPS.items():
        if name in names:
            return group, needs
    return None, None


def area_tiles(ability):
    """Tiles of one ability's area: the largest direction; no area is the target tile only."""
    matrix = (ability.get('area') or {}).get('matrix')
    if not matrix:
        return 1
    # Only hit cells count, as in the runtime geometry: `x` and the hit centre `C`, never the non-hit centre `c`.
    return max(sum(ch in 'xC' for row in rows for ch in row) for rows in matrix.values())


def registrars(census):
    found = {}
    for source in REGISTRARS:
        for entry in census[source]:
            if entry.get('spell_type') == 'rune':
                found.setdefault(vs.rune_name(entry['name']), {})[source] = entry['registrar']
    return found


def check(ours, found, required=False, recorded=None):
    """Compare our value with its references. On a reference conflict our value must be that of the source the
    catalogue manifest records as the winner of its S3/S11/S13/S14 resolution; matching any reference is not enough.
    Several recorded sources are agreement provenance, not a winner: when they now diverge, the converter's S4 rule
    applies (Canary decides a registrar-only conflict) and any other such conflict fails.
    A recorded native resolution (S27) requires our value to differ from the agreeing references; it is the only
    way such a value passes."""
    recorded = recorded or {}
    verdict, follows = vs.classify(ours, found)
    passed = verdict == 'agree' or (verdict == 'no_source' and not required)
    row = {'ours': ours, 'references': found, 'verdict': verdict}
    if verdict == 'sources_disagree':
        row['follows'] = follows
        winners = recorded.get('sources', set()) & set(found)
        row['recorded_winner'] = sorted(winners)
        if len(winners) > 1:
            passed = winners <= set(REGISTRARS) and 'canary' in follows
        else:
            passed = bool(set(follows) & winners)
    if recorded.get('native'):
        # A recorded native override constrains the value whatever the references say: it must differ from them.
        row['resolution'] = recorded['native']
        passed = verdict == 'ours_differs'
        if passed:
            row['verdict'] = 'native_resolution'
    row['pass'] = passed
    return row


def source_ref(source):
    """The reference key of one manifest source (`verify_spells.FACTS` keys and the registrar names)."""
    origin = source.get('api') or source.get('repository') or ''
    for marker, ref in (('tibia.fandom.com', 'fandom'), ('tibiawiki.com.br', 'br'), ('opentibiabr/canary', 'canary'),
                        ('zimbadev/crystalserver', 'crystal')):
        if marker in origin:
            return ref
    return None


def recorded(entry, destination):
    """What the catalogue manifest records for one destination: the sources it was mapped from, and the
    resolution text of a native-behaviour override."""
    sources, native = set(), None
    manifest = entry.get('manifest', {})
    for row in manifest.get('entries', []):
        if row.get('destination') != destination:
            continue
        if row.get('status') == 'mapped':
            ref = source_ref(manifest['sources'][row['source_index']])
            if ref:
                sources.add(ref)
        elif row.get('status') == 'resolved_native_behavior':
            native = row.get('resolution')
    return {'sources': sources, 'native': native}


def wiki_values(pages, field):
    found = {}
    for ref, page in pages.items():
        value = vs.reference_value(field, page['fields'].get(field), page['fields'])
        if value is not None:
            found[ref] = value
    return found


def registrar_values(registrar, key, default=None):
    found = {}
    for source, fields in registrar.items():
        value = fields.get(key, default)
        if value is not None:
            found[source] = value
    return found


def registrar_vocations(registrar):
    """Canary/Crystal `vocation` entries ('elder druid;true') as base vocations, for a wiki-silent field."""
    found = {}
    for source, fields in registrar.items():
        entries = fields.get('vocation')
        if entries:
            names = {entry.split(';')[0].strip().lower() for entry in entries}
            found[source] = sorted({ws.BASE_VOCATION.get(name, name) for name in names})
    return found


def registrar_blocking(registrar, index):
    """`isBlocking` is (solid, creature); a single flag is solid only."""
    found = {}
    for source, fields in registrar.items():
        value = fields.get('isBlocking', False)
        found[source] = bool(value[index]) if isinstance(value, list) else bool(value) and index == 0
    return found


def stack_facts(profile):
    if profile is None:
        return {'pass': False, 'reason': 'no native Item profile'}
    stack = profile['semantics'].get('stack', {})
    value = stack.get('value', {}) if stack.get('state') == 'KNOWN' else {}
    stackable = value.get('stackable', {}).get('value')
    stack_max = value.get('stack_max', {}).get('value')
    stack_class = profile['admission'].get('stack_class')
    return {'stackable': stackable, 'stack_max': stack_max, 'stack_class': stack_class,
            'pass': stackable is True and stack_max == STACK_MAX and stack_class == 'StackCapable'}


def audit_carrier(entry, references, registrar, resolutions, conjures, profiles, guard):
    spell = entry['bundle']['spell']
    name = spell['name'].lower()
    item_key = spell['rune']['item']['key']
    item_id = vs.rune_item_id(spell)
    pages, issues = references.rune_matches(name, item_id)
    ours = vs.our_rune_values(spell)
    targeting = spell.get('targeting', {})
    rune = spell['rune']
    base = '/spell/spell/'
    flag = lambda key: registrar_values(registrar, key, REGISTRAR_DEFAULTS[key]) if registrar else {}
    checks = {
        'item_id': check(item_id, {**wiki_values(pages, 'itemid'), **registrar_values(registrar, 'runeId')},
                         required=True, recorded=recorded(entry, base + 'rune/item')),
        'vocations': check(ours['vocrequired'],
                           wiki_values(pages, 'vocrequired') or registrar_vocations(registrar), required=True,
                           recorded=recorded(entry, base + 'requirements/vocations')),
        'group': check(vs.group_key(spell['groups'][0]['group']) if spell.get('groups') else None,
                       registrar_values(registrar, 'group'), required=True,
                       recorded=recorded(entry, base + 'groups/0/group')),
        'range_tiles': check(targeting.get('range_tiles'),
                             {**wiki_values(pages, 'spellrange'), **registrar_values(registrar, 'range')},
                             recorded=recorded(entry, base + 'targeting/range_tiles')),
        'blocking_solid': check(rune.get('blocking', {}).get('solid'), registrar_blocking(registrar, 0),
                                recorded=recorded(entry, base + 'rune/blocking/solid')),
        'blocking_creature': check(rune.get('blocking', {}).get('creature'), registrar_blocking(registrar, 1),
                                   recorded=recorded(entry, base + 'rune/blocking/creature')),
        'allow_far_use': check(rune.get('allow_far_use'), flag('allowFarUse'),
                               recorded=recorded(entry, base + 'rune/allow_far_use')),
    }
    for field, key in TARGETING_FLAGS:
        if field in guard:
            continue  # the S27 guard replaces the registrar value (`convert_spells.Spell.targeting`)
        checks[field] = check(targeting.get(field), flag(key), recorded=recorded(entry, base + 'targeting/' + field))
    # S20: the positional cast (registrar `optionalTarget`) and Aim at Target (a wiki `aimattarget` of a directional
    # spell) are written only when set (`convert_spells.Spell.targeting`), so absent means false.
    checks['cast_at_position'] = check(targeting.get('cast_at_position', False),
                                       {source: bool(value) for source, value in
                                        registrar_values(registrar, 'optionalTarget', False).items()},
                                       recorded=recorded(entry, base + 'targeting/cast_at_position'))
    aim = any(page['fields'].get('aimattarget') == 'yes' for page in pages.values()) and targeting.get('needs_direction')
    checks['aim_at_target'] = {'ours': targeting.get('aim_at_target', False), 'expected': bool(aim),
                               'pass': targeting.get('aim_at_target', False) == bool(aim)}
    # The parameter mode as `convert_spells.Spell.targeting` derives it from either registrar's parameter flags.
    expected = ('player_name' if any(r.get('hasPlayerNameParam') for r in registrar.values()) else
                'text' if any(r.get('hasParams') for r in registrar.values()) else 'none')
    checks['parameter'] = {'ours': targeting.get('parameter'), 'registrars': expected,
                           'pass': targeting.get('parameter') == expected}
    # S27: the accepted onCastSpell guard of a rune fixes its target restriction; without one there is none.
    expected = {'allowed_targets': None, **guard}
    checks['target_guard'] = {'ours': {field: targeting.get(field) for field in expected}, 'guard': expected,
                              'pass': all(targeting.get(field) == value for field, value in expected.items())}
    # A targeting field this audit does not know is unaudited, so it fails until the audit covers it.
    unknown = sorted(set(targeting) - AUDITED_TARGETING)
    checks['targeting_fields'] = {'unaudited': unknown, 'pass': not unknown}
    # Every registrar-backed check compares both engines; a carrier with one registrar record stays blocked.
    checks['registrars'] = {'present': sorted(registrar), 'required': list(REGISTRARS),
                            'pass': all(source in registrar for source in REGISTRARS)}
    if issues:
        checks['item_id']['pass'] = False
        checks['item_id']['reference_match_issues'] = issues
    resolution = resolutions.get(name) or resolutions.get(vs.rune_name(name))
    if resolution and resolution.get('carrier') == 'rune':
        field = {'vocations': 'vocations', 'primary_group': 'group'}[resolution['field']]
        current = resolution['value']
        if field == 'vocations':
            ours_value = sorted(spell['requirements'].get('vocations', []))
            current = sorted(current)
        else:
            ours_value = checks['group']['ours']
        checks[field] = {'ours': ours_value, 'current': current, 'verdict': 'current_resolution',
                         'source': resolution['source']['url'], 'captured': resolution['source']['captured'],
                         'pass': ours_value == current}
    conjure = conjures.get(item_key)
    checks['charges'] = ({'ours': rune.get('charges'), 'conjure_counts': conjure,
                          'pass': all(count == rune.get('charges') for count in conjure.values())} if conjure else
                         {'ours': rune.get('charges'), 'verdict': 'no_conjuring_spell', 'pass': True})
    checks['stack'] = stack_facts(profiles.get(item_key))
    abilities = entry.get('dependencies', {}).get('abilities', [])
    tiles = sum(area_tiles(ability) for ability in abilities) if abilities else 1
    creates_fields = any(effect.get('operation') == 'create_item'
                         for effect in entry.get('dependencies', {}).get('effects', []))
    checks['area'] = {'tiles': tiles, 'victim_cap': 2 * tiles, 'creates_fields': creates_fields,
                      'pass': tiles <= AREA_TILES_MAX and (not creates_fields or tiles <= FIELD_TILES_MAX)}
    group, needs = group_of(name)
    failed = sorted(field for field, row in checks.items() if not row['pass'])
    return {'name': name, 'item': item_key, 'section_9_group': group, 'section_9_needs': needs,
            'checks': checks, 'failed': failed, 'verdict': 'checks_pass' if not failed and group else 'checks_fail'}


def rune_guard(guard):
    """The targeting fields of an accepted rune onCastSpell guard (`convert_spells.Spell.guard`), or {}."""
    return guard.get('targeting', {}) if guard.get('spell_type') == 'rune' else {}


def audit(catalogue, documents, census, resolutions, profiles, guards=None):
    references = vs.References(documents)
    regs = registrars(census)
    conjures = {}
    for entry in catalogue['bundles']:
        spell = entry['bundle']['spell']
        conjure = spell.get('execution', {}).get('conjure')
        if conjure:
            conjures.setdefault(conjure['result']['key'], {})[spell['name'].lower()] = conjure['count']
    by_item = {profile['authoring']['item']['key']: profile for profile in profiles['records']}
    carriers = [audit_carrier(entry, references, regs.get(vs.rune_name(entry['bundle']['spell']['name']), {}),
                              resolutions.get('spells', {}), conjures, by_item,
                              rune_guard((guards or {}).get(entry['bundle']['spell']['name'].lower(), {})))
                for entry in catalogue['bundles'] if entry['bundle']['spell']['carrier'] == 'rune']
    carriers.sort(key=lambda row: row['name'])
    names = {row['name'] for row in carriers}
    duplicates = sorted(name for name in names if sum(row['name'] == name for row in carriers) > 1)
    expected = {name for _, group in GROUPS.values() for name in group}
    summary = {'carriers': len(carriers),
               'inventory': {'expected': len(expected), 'missing': sorted(expected - names),
                             'unexpected': sorted(names - expected), 'duplicates': duplicates},
               'checks_pass': sum(1 for row in carriers if row['verdict'] == 'checks_pass'),
               'checks_fail': sorted(row['name'] for row in carriers if row['verdict'] == 'checks_fail'),
               'largest_area_tiles': max((row['checks']['area']['tiles'] for row in carriers), default=0),
               'largest_field_tiles': max((row['checks']['area']['tiles'] for row in carriers
                                           if row['checks']['area']['creates_fields']), default=0)}
    return summary, carriers


def document():
    load = lambda path: json.loads(path.read_text(encoding='utf-8'))
    documents = {name: load(path) for name, path in vs.FACTS.items()}
    summary, carriers = audit(load(CATALOGUE), documents, load(CENSUS), load(RESOLUTIONS), load(ITEM_PROFILES),
                              load(GUARDS)['spells'])
    inputs = [CATALOGUE, ITEM_PROFILES, CENSUS, RESOLUTIONS, GUARDS, *vs.FACTS.values()]
    return {'schema': 'OTERYN_RUNE_CARRIER_AUDIT/v1', 'decision': DECISION,
            'inputs': [str(path.relative_to(REPO)) for path in inputs],
            'limits': {'area_tiles_max': AREA_TILES_MAX, 'victim_cap_max': 2 * AREA_TILES_MAX,
                       'field_tiles_max': FIELD_TILES_MAX, 'stack_max': STACK_MAX},
            'note': ('A failed check keeps the carrier blocked. A pass is subject to the needs of its section 9 group; '
                     'it does not establish runtime admission or cast parity. Missing evidence is not agreement: '
                     'item_id, vocations and group need a source.'),
            'summary': summary, 'carriers': carriers}


def render(doc):
    return json.dumps(doc, indent=1, sort_keys=True, ensure_ascii=False) + '\n'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--check', action='store_true', help='fail if the committed audit is stale')
    args = parser.parse_args(argv)
    doc = document()
    inventory = doc['summary']['inventory']
    if inventory['missing'] or inventory['unexpected'] or inventory['duplicates']:
        print(f'rune carrier inventory differs from RUNE-USE-0 section 9: {inventory}', file=sys.stderr)
        return 1
    text = render(doc)
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding='utf-8') != text:
            print(f'{OUT.name} is stale; run python audit_rune_carriers.py', file=sys.stderr)
            return 1
        print(f'{OUT.name}: up to date')
        return 0
    OUT.write_text(text, encoding='utf-8')
    print(json.dumps(json.loads(text)['summary'], indent=1))
    return 0


if __name__ == '__main__':
    sys.exit(main())
