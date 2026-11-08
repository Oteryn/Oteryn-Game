"""Retained source talisman facts; transformation semantics remain engine-owned."""
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

FACTS = {
    22008: (True, 'f5c9e177cb84a38fc0d73a65db2e654da4d3871a0f98de9d6d0e1387ec8c0435'),
    22009: (False, '218d0cf4f98124a9512574f81026938fa0ffbe3fdd80f19eeab1fb60d23a73a5'),
}


def apply_admissions(definitions):
    source = Path(__file__).resolve().parents[2] / 'imports/tibiawiki/facts/items-stats.json'
    observations = json.loads(source.read_bytes())['records']
    raw = (source.parents[2] / 'canary/items-xml/items.xml').read_bytes()
    if hashlib.sha256(raw).hexdigest() != '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2':
        raise ValueError('Tainted talisman XML changed')
    xml = ET.fromstring(raw)
    known = lambda value: {'state': 'KNOWN', 'value': value}
    for ident, (stackable, digest) in FACTS.items():
        key = f'oteryn:item.tibia.i{ident}'
        matches = [row for row in definitions if row['identity']['key'] == key]
        evidence = observations[key]
        if len(matches) != 1 or evidence['item_id'] != ident or len(evidence['observations']) != 1:
            raise ValueError('Tainted talisman identity changed')
        proof = evidence['observations'][0]
        if (proof['content_sha256'] != digest or proof['fields']['weight'] != '2.40'
                or proof['fields']['stackable'].lower() != ('yes' if stackable else 'no')):
            raise ValueError('Tainted talisman source changed')
        attrs = {a.get('key').lower() for a in xml.find(f"item[@id='{ident}']").findall('attribute')}
        if attrs & {'duration', 'decayto', 'stopduration', 'charges', 'type', 'field'}:
            raise ValueError('Tainted talisman lifetime changed')
        item = matches[0]
        stack = known({'stackable': known(stackable), 'stack_max': known(100)
                       if stackable else {'state': 'NOT_APPLICABLE'}})
        incomplete = known({'stackable': known(stackable), 'stack_max': {'state': 'UNKNOWN'}})
        classification = 'StackCapable' if stackable else 'NonStackable'
        if (item['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
                or item['semantics']['physical'] != known({'weight': known(240),
                    'movable': known(True), 'pickupable': known(True)})
                or item['semantics']['stack'] not in ({'state': 'UNKNOWN'}, stack, incomplete)
                or (item['materializable'], item['stack_class']) not in
                   ((False, 'Unknown'), (True, classification))):
            raise ValueError('Tainted talisman canonical facts changed')
        if item['semantics']['temporal'] not in ({'state': 'UNKNOWN'}, {'state': 'NOT_APPLICABLE'}):
            raise ValueError('Tainted talisman canonical lifetime changed')
        item_type = known({'item_type': {'state': 'NOT_APPLICABLE'}, 'capabilities': {'state': 'UNKNOWN'}})
        if item['semantics']['classification'] not in ({'state': 'UNKNOWN'}, item_type):
            raise ValueError('Tainted talisman classification changed')
        item['semantics']['classification'] = item_type
        item['semantics']['temporal'] = {'state': 'NOT_APPLICABLE'}
        item['semantics']['stack'] = stack
        item['materializable'] = True
        item['stack_class'] = classification
