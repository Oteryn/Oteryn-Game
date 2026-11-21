"""Opt-in contacts from retained native NPC source metadata, never inferred event schedules."""
import json
import re


def candidates(directory, requested):
    if not isinstance(requested, list) or len(set(requested)) != len(requested):
        raise ValueError('native_contacts must be a distinct list')
    wanted, found = set(requested), {}
    if not wanted:
        return found
    for path in sorted(directory.glob('npcs-*.json')):
        for index, record in enumerate(json.loads(path.read_text())['records']):
            npc = record['declaration']
            key = npc['identity']['key']
            if key not in wanted:
                continue
            if key in found:
                raise ValueError(f'{key}: duplicate native contact definition')
            metadata = json.loads(next(f['value']['value'] for f in npc['fields']
                                       if f['field_path'].endswith('.bulk.source_metadata')))
            facts = metadata['wiki_facts']
            if facts['br_coordinate_frame'] != 'Tibia_reference_world_coordinates_not_Oteryn_placement':
                raise ValueError(f'{key}: unverified coordinate frame')
            source = next(s for s in metadata['sources'] if s['source'] == 'TibiaWiki BR')
            if not re.fullmatch(r'[0-9a-f]{64}', source['source_sha256']) or type(source['revision']) is not int or source['revision'] <= 0:
                raise ValueError(f'{key}: missing retained BR source proof')
            page = next(b for b in record['source_bindings'] if b['identity_namespace'] == 'mediawiki/page_id'
                        and b['source_revision'].startswith('tibiawiki-br-npc-'))
            points = facts['br_coordinates']
            if not 1 <= len(points) <= 16 or any(len(p) != 3 or any(type(c) is not int for c in p) for p in points):
                raise ValueError(f'{key}: invalid source contact coordinates')
            found[key] = {
                'identity': npc['identity'],
                'placements': [{'position': dict(zip(('x', 'y', 'z'), p)),
                                'direction': None, 'origin': 'native_wiki_br'} for p in points],
                'native_provenance': {
                    'origin': 'native_wiki_br', 'repository': 'www.tibiawiki.com.br',
                    'source_sha256': source['source_sha256'], 'revid': source['revision'],
                    'pageid': int(page['external_id']), 'url': source['url'],
                    'native_record': f'content/npcs/definitions/{path.name}#/records/{index}',
                    'coordinate_note': 'Retained Tibia reference; Oteryn source-informed contact, not retail placement equivalence',
                    'direction_note': 'No source direction; north is the project default',
                },
            }
    if set(found) != wanted:
        raise ValueError(f'missing native contacts: {sorted(wanted - set(found))}')
    return found
