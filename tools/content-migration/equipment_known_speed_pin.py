"""Qualify three former UNKNOWN leaves from the pinned source decoder, not runtime output."""
import re

import qengine_reward_item_profiles as producer

IDS = frozenset({2833, 5805, 5806})
BEGIN = '    // BEGIN GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
END = '    // END GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
FILTER_BEGIN = '        // BEGIN GENERATED KNOWN SPEED EXCLUSIONS\n'
FILTER_END = '        // END GENERATED KNOWN SPEED EXCLUSIONS\n'


def refresh(source, packet):
    expected = {f'oteryn:item.tibia.i{i}' for i in IDS}
    records = packet['records']
    sources = [
        {'path': producer.XML, 'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
         'sha256': producer.XML_SHA},
        {'path': producer.CLIENT, 'sha256': producer.CLIENT_SHA},
    ]
    if (packet.get('sources') != sources or len(records) != len(IDS)
            or {r['production_definition']['production_key'] for r in records} != expected
            or any(type(r['attributes'].get('speed_bonus')) is not int
                   or r['attributes']['speed_bonus'] != 0 for r in records)):
        raise ValueError('EQUIPMENT_KNOWN_SPEED_SOURCE')
    keys = ', '.join(f'"{key}"' for key in sorted(expected))
    assertion = (BEGIN
        + '    // Pinned Canary XML/client: parchment and the two arena trophy promotions.\n'
        + '    for key in [\n'
        + ''.join(f'        "{key}",\n' for key in sorted(expected))
        + '    ] {\n'
        + '        let record = profiles\n'
        + '            .records\n'
        + '            .iter()\n'
        + '            .find(|record| record.production_definition.production_key == key)\n'
        + '            .ok_or("missing source-qualified zero-speed Item")?;\n'
        + '        assert_eq!(record.attributes.speed_bonus, Some(0), "{key}");\n'
        + '    }\n' + END)
    exclusion = (FILTER_BEGIN
        + f'        .filter(|key| ![{keys}].contains(&key.as_str()))\n' + FILTER_END)
    for begin, end, block, anchor in (
            (BEGIN, END, assertion, '    let content = native(&input)?;\n'
             '    let mut unknown_speed = std::collections::BTreeSet::new();\n'),
            (FILTER_BEGIN, FILTER_END, exclusion,
             '        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))\n')):
        if source.count(begin) != source.count(end) or source.count(begin) > 1:
            raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
        if begin in source:
            source = re.sub(re.escape(begin) + r'.*?' + re.escape(end),
                            lambda _: block, source, flags=re.S)
        else:
            if source.count(anchor) != 1:
                raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
            source = source.replace(anchor, block + anchor if begin == BEGIN else anchor + block)
    return source
