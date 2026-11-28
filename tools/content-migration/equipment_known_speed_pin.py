"""Regenerate exact equipment-speed assertions from qualified item sources."""

import hashlib

import qengine_reward_item_profiles as producer

IDS = frozenset({2833, 5805, 5806})
OVERLAP_IDS = frozenset({173, 646})
BEGIN = '    // BEGIN GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
END = '    // END GENERATED SOURCE-QUALIFIED ZERO SPEED\n'
LEGACY_BEGIN = '    // BEGIN GENERATED CUSTODY-RETAINED UNKNOWN SPEED\n'
LEGACY_END = '    // END GENERATED CUSTODY-RETAINED UNKNOWN SPEED\n'
FILTER_BEGIN = '        // BEGIN GENERATED KNOWN SPEED EXCLUSIONS\n'
FILTER_END = '        // END GENERATED KNOWN SPEED EXCLUSIONS\n'
ANCHOR = ('    let content = native(&input)?;\n'
          '    let mut unknown_speed = std::collections::BTreeSet::new();\n')
FILTER_ANCHOR = '        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))\n'
COLLECT = '        .collect()\n    );\n    Ok(())\n'


def _assertion(expected, *, unknown, merged=False):
    begin, end = (LEGACY_BEGIN, LEGACY_END) if unknown else (BEGIN, END)
    comment = ('    // Physical custody retains UNKNOWN speed for parchment and the two trophies.\n'
               if unknown else
               '    // Pinned Canary XML/client: parchment and the two arena trophy promotions.\n')
    if merged:
        comment = '    // Pinned source qualifies zero speed for these overlapping custody profiles.\n'
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
    if stop <= start or source[start:stop] not in expected:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    return source[:start] + replacement + source[stop:], True


def _filter_bodies(keys):
    literals = [f'"{key}"' for key in sorted(keys)]
    compact = '        .filter(|key| ![' + ', '.join(literals) + '].contains(&key.as_str()))\n'
    formatted = ('        .filter(|key| ![\n'
                 + ',\n'.join(f'            {key}' for key in literals)
                 + '\n        ]\n        .contains(&key.as_str()))\n')
    return compact, formatted


def refresh(source, packet, root=producer.ROOT):
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

    # Crystal 201189f items.xml:246–252,1137–1140 has neither a speed nor a slot
    # override for the parchment or rope. Crystal items.hpp:38,242 defaults to
    # zero. Verify the same fields in the pinned XML used by both admissions.
    raw = (root / producer.XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != producer.XML_SHA:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_SOURCE')
    entries = producer.source.xml_index(raw)
    for item in OVERLAP_IDS:
        if item not in entries or {'speed', 'slottype'} & producer.source.attrs(entries[item]).keys():
            raise ValueError('EQUIPMENT_KNOWN_SPEED_SOURCE')
    all_keys = expected | {f'oteryn:item.tibia.i{item}' for item in OVERLAP_IDS}
    assertion = _assertion(all_keys, unknown=False, merged=True)
    source, legacy = _replace_block(
        source, LEGACY_BEGIN, LEGACY_END,
        (_assertion(expected, unknown=True),), assertion)
    if not legacy:
        source, present = _replace_block(
            source, BEGIN, END, (_assertion(expected, unknown=False), assertion), assertion)
        if not present:
            source = source.replace(ANCHOR, assertion + ANCHOR)
    compact, formatted = _filter_bodies(all_keys)
    filter_bodies = (*_filter_bodies(expected), compact, formatted)
    source, _ = _replace_block(
        source, FILTER_BEGIN, FILTER_END,
        tuple(FILTER_BEGIN + body + FILTER_END for body in filter_bodies), '')
    if any(body in source for body in filter_bodies):
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    before, anchor, after = source.partition(FILTER_ANCHOR)
    if after.count(COLLECT) != 1:
        raise ValueError('EQUIPMENT_KNOWN_SPEED_ANCHOR')
    # Ordinary Crystal admissions are appended after the original filter site.
    return before + anchor + after.replace(
        COLLECT, FILTER_BEGIN + formatted + FILTER_END + COLLECT)
