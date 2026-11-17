"""Materialize source acquisition intents against the admitted outfit identities.

The immutable capture remains untouched. Numeric pair errors in the random
outfit script are resolved by its explicit family label against outfits.xml;
addonless client looks produce a base outfit, preserving the source family draw.
"""
import copy
import re


def extend_catalogue(outfits, source_rows, looks, addon_patterns, source):
    """Admit missing explicit source families without aliasing an existing pair."""
    def norm(value):
        return re.sub(r'[^a-z0-9]', '', value.lower())
    names = {norm(r['key'].rsplit('.', 1)[1]) for r in outfits}
    pairs = {tuple(sorted([r['female']['id'], r['male']['id']])) for r in outfits}
    for row in source_rows.values():
        female, male, name = row[1], row[2], row[3]
        if tuple(sorted([female, male])) in pairs or norm(name) in names:
            continue
        if male not in looks or female not in looks:
            raise ValueError('unadmitted source acquisition appearance')
        key = 'oteryn:outfit.' + name.lower().replace(' ', '_').replace('-', '_')
        outfits.append({'key': key, 'revision': 'crystal-9f5a72c-r1',
            'female': {'id': female, 'source_record_sha256': looks[female]},
            'male': {'id': male, 'source_record_sha256': looks[male]},
            'addon_count': min(2, addon_patterns[female], addon_patterns[male]),
            'starter_female': False, 'starter_male': False,
            'starter_addons_female': 0, 'starter_addons_male': 0,
            'premium': True, 'unlock_sources': [source], 'source': source})
        names.add(norm(name)); pairs.add(tuple(sorted([female, male])))
    outfits.sort(key=lambda r: r['key'])


def build(source_packet, outfits, outfit_source, source_packet_sha256):
    packet = copy.deepcopy(source_packet)
    norm = lambda value: re.sub(r'[^a-z0-9]', '', value.lower())
    by_name = {norm(r['key'].rsplit('.', 1)[1]): r for r in outfits}
    by_pair = {tuple(sorted([r['female']['id'], r['male']['id']])): r for r in outfits}
    grants = {r['key']: r for r in packet['grants']}
    decisions = []
    for rule in packet['rules']:
        if rule['mode'] != 'random_outfit':
            continue
        for choice in rule['choices']:
            match = re.fullmatch(r'You received the (.+) outfit with addon ([12])!', choice['feedback'])
            if match is None:
                raise ValueError('unrecognized source outfit intent')
            name, requested = match.group(1), int(match.group(2))
            original = grants[choice['grants'][-1]]
            row = by_pair.get(tuple(original['look_types'])) or by_name[norm(name)]
            pair = sorted([row['female']['id'], row['male']['id']])
            bit = requested if requested <= row['addon_count'] else 0
            old = list(choice['grants'])
            native = []
            for addon in sorted({0, bit}):
                grant_key = f'oteryn:item-grant.outfit.{pair[0]}-{pair[1]}.' + (f'addon{addon}' if addon else 'base')
                template = copy.deepcopy(grants[old[-1]])
                template.update(key=grant_key, kind='addon' if addon else 'outfit', look_types=pair, addon=addon)
                grants.setdefault(grant_key, template)
                native.append(grant_key)
            choice['grants'] = native
            choice['owned'] = native[-1]
            if bit == 0:
                choice['feedback'] = f'You received the {name} outfit!'
                choice['owned_feedback'] = 'You already have this outfit.'
            if old != native:
                decisions.append({'source_grants': old, 'native_grants': native,
                                  'family': name, 'source': outfit_source,
                                  'reason': 'explicit source family and admitted sex pair; only renderable addons'})
    used = set()
    for rule in packet['rules']:
        for choice in rule['choices']:
            for field in ('grants', 'required', 'complete_grants'):
                used.update(choice.get(field, []))
            used.add(choice['owned'])
    packet['grants'] = [grants[key] for key in sorted(used)]
    packet['native_admission'] = {
        'decision': 'owner:QENGINE-2:S06:source-intent-and-admitted-appearance:2026-11-07',
        'source_packet_sha256': source_packet_sha256,
        'outfit_source': outfit_source, 'corrections': decisions,
    }
    return packet
