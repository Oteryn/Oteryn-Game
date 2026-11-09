"""Resolve an opted-in native NPC's exact BR page binding, without name-only matching."""
import json
import re
from pathlib import Path
from urllib.parse import quote

CAPTURE = Path(__file__).resolve().parents[3] / 'imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json'


def resolve(record, capture=CAPTURE):
    npc = record['declaration']
    identity = npc['identity']
    bindings = [b for b in record.get('source_bindings', [])
                if b.get('identity_namespace') == 'mediawiki/page_id'
                and b.get('source_revision', '').startswith('tibiawiki-br-npc-')
                and b.get('source_key') == 'oteryn:source.tibiawiki']
    if len(bindings) != 1:
        raise ValueError(f"{identity['key']}: missing or ambiguous retained BR binding")
    binding = bindings[0]
    if binding.get('target') != dict(family='NPC', **identity):
        raise ValueError(f"{identity['key']}: foreign retained BR binding")
    pageid = binding.get('external_id')
    if not isinstance(pageid, str) or not re.fullmatch('[1-9][0-9]*', pageid):
        raise ValueError('invalid retained BR page identity')
    pages = [p for p in json.loads(capture.read_text())['pages'] if p.get('pageid') == int(pageid)]
    if len(pages) != 1:
        raise ValueError('missing or ambiguous retained BR page')
    page = pages[0]
    slug = re.sub('[^a-z0-9]+', '_', page.get('name', '').casefold()).strip('_')
    if identity['key'] != 'oteryn:npc.' + slug or page.get('title') != page.get('name') or page.get('role') != 'npc' or page.get('removed'):
        raise ValueError('retained BR page does not identify this active NPC')
    if not re.fullmatch('[0-9a-f]{64}', page.get('sha256', '')) or type(page.get('revid')) is not int or page['revid'] <= 0:
        raise ValueError('missing retained BR revision/hash')
    points = page.get('positions')
    if not isinstance(points, list) or not 1 <= len(points) <= 16 or any(
            not isinstance(p, list) or len(p) != 3 or any(type(v) is not int for v in p)
            or not (0 <= p[0] <= 65535 and 0 <= p[1] <= 65535 and 0 <= p[2] <= 15) for p in points):
        raise ValueError('invalid retained BR contact coordinates')
    return {'wiki_facts': {'br_coordinate_frame': 'Tibia_reference_world_coordinates_not_Oteryn_placement',
                           'br_coordinates': points},
            'sources': [{'source': 'TibiaWiki BR', 'source_sha256': page['sha256'],
                         'revision': page['revid'],
                         'url': 'https://www.tibiawiki.com.br/wiki/' + quote(page['title'].replace(' ', '_'))
                                + '?oldid=' + str(page['revid'])}]}
