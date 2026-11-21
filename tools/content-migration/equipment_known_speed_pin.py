"""Qualify the closed three source speeds while preserving generated guard integrity."""

import qengine_reward_item_profiles as producer

IDS = frozenset({2833, 5805, 5806})
BEGIN = '    // BEGIN GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
END = '    // END GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
LEGACY_BEGIN = '    // BEGIN GENERATED CUSTODY-RETAINED UNKNOWN SPEED\n'
LEGACY_END = '    // END GENERATED CUSTODY-RETAINED UNKNOWN SPEED\n'
FILTER_BEGIN = '        // BEGIN GENERATED KNOWN SPEED EXCLUSIONS\n'
FILTER_END = '        // END GENERATED KNOWN SPEED EXCLUSIONS\n'
ANCHOR = ('    let content = native(&input)?;\n'
          '    let mut unknown_speed = std::collections::BTreeSet::new();\n')
FILTER_ANCHOR = '        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))\n'


def _assertion(expected, *, unknown):
    begin, end = (LEGACY_BEGIN, LEGACY_END) if unknown else (BEGIN, END)
    comment = ('    // Physical custody retains UNKNOWN speed for parchment and the two trophies.\n'
               if unknown else
               '    // Pinned Canary XML/client: parchment and the two arena trophy promotions.\n')
    missing = ('missing custody-owned unknown-speed Item' if unknown else
               'missing source-qualified zero-speed Item')
    speed = 'None' if unknown else 'Some(0)'
    return (begin + comment
        + '    for key in [\n'
        + ''.join(f'        "{key}",\n' for key in sorted(expected))
        + '    ] {\n'
        + '        let record = profiles\n'
        + '            .records\n'
        + '            .iter()\n'
        + '            .find(|record| record.production_definition.production_key == key)\n'
        + f'            .ok_or("{missing}")?;\n'
        + f'        assert_eq!(record.attributes.speed_bonus, {speed}, "{{key}}");\n'
        + '    }\n' + end)


def _replace_block(source, begin, end, expected, replacement):
    if source.count(begin) != source.count(end) or source.count(begin) > 1:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    if begin not in source:
        return source, False
    start, stop = source.index(begin), source.index(end) + len(end)
    if stop <= start or source[start:stop] != expected:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    return source[:start] + replacement + source[stop:], True


def refresh(source, packet):
    expected = {f'oteryn:item.tibia.i{i}' for i in IDS}
    records = packet['records']
    sources = [
        {'path': producer.XML, 'revision': '47dfd51f45280a59a1d3e50ba7edd573d7234446',
         'sha256': producer.XML_SHA},
        {'path': producer.CLIENT, 'sha256': producer.CLIENT_SHA},
    ]
    # The dedicated QENGINE source owner qualifies only these three speeds.
    # Require its pinned, composed integer zero before excluding any UNKNOWN leaf.
    if (packet.get('sources') != sources or len(records) != len(IDS)
            or {r['production_definition']['production_key'] for r in records} != expected
            or any(type(r['attributes'].get('speed_bonus')) is not int
                   or r['attributes']['speed_bonus'] != 0 for r in records)):
        raise ValueError('EQUIPMENT_KNOWN_SPEED_SOURCE')
    if (source.count(ANCHOR) != 1 or source.count(FILTER_ANCHOR) != 1
            or source.count(BEGIN) != source.count(END) or source.count(BEGIN) > 1
            or (LEGACY_BEGIN in source and BEGIN in source)):
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    assertion = _assertion(expected, unknown=False)
    source, legacy = _replace_block(
        source, LEGACY_BEGIN, LEGACY_END,
        _assertion(expected, unknown=True), assertion)
    if not legacy:
        source, present = _replace_block(source, BEGIN, END, assertion, assertion)
        if not present:
            source = source.replace(ANCHOR, assertion + ANCHOR)
    keys = ', '.join(f'"{key}"' for key in sorted(expected))
    compact_body = f'        .filter(|key| ![{keys}].contains(&key.as_str()))\n'
    filter_body = ('        .filter(|key| ![\n'
                   + ',\n'.join(f'            "{key}"' for key in sorted(expected))
                   + '\n        ]\n        .contains(&key.as_str()))\n')
    compact = FILTER_BEGIN + compact_body + FILTER_END
    formatted = FILTER_BEGIN + filter_body + FILTER_END
    # Accept only the exact prior compact block or its rustfmt rendering.
    source, _ = _replace_block(
        source, FILTER_BEGIN, FILTER_END,
        compact if compact in source else formatted, '')
    if compact_body in source or filter_body in source:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    return source.replace(FILTER_ANCHOR, FILTER_ANCHOR + formatted)
