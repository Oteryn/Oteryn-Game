"""Bounded ordinary native guardians for explicitly chosen Oteryn quests.

These are authored counterparts, never additions to donor XML or held bosses.
The existing world compiler still checks native creature admission and geometry.
"""
import json
import re

SCHEMA = 'OTERYN_AUTHORED_QUEST_SPAWNS/v1'
FRAME = 'global-target-2026-09-27'


def parse(raw, known, reserved, occupied):
    if len(raw) > 256 * 1024:
        raise ValueError('AUTHORED_SPAWN_DOCUMENT_BOUND')
    document = json.loads(raw)
    if (not isinstance(document, dict) or set(document) != {'schema', 'family', 'coordinate_frame', 'records'}
            or document['schema'] != SCHEMA or document['family'] != 'Spawn.Source'
            or document['coordinate_frame'] != FRAME
            or not isinstance(document['records'], list) or len(document['records']) > 64):
        raise ValueError('AUTHORED_SPAWN_DOCUMENT')
    keys, cells = set(), set(occupied)

    def position(value):
        return (isinstance(value, dict) and set(value) == {'x', 'y', 'floor'}
                and all(type(value[key]) is int and 0 <= value[key] <= 65535 for key in ('x', 'y'))
                and type(value['floor']) is int and 0 <= value['floor'] <= 15)

    total = 0
    for record in document['records']:
        if not isinstance(record, dict) or set(record) != {'declaration'}:
            raise ValueError('AUTHORED_SPAWN_RECORD')
        declaration = record['declaration']
        if not isinstance(declaration, dict) or set(declaration) != {'identity', 'centre', 'points', 'provenance'}:
            raise ValueError('AUTHORED_SPAWN_DECLARATION')
        identity, centre = declaration['identity'], declaration['centre']
        if (not isinstance(identity, dict) or set(identity) != {'key', 'revision'}
                or not isinstance(identity['key'], str) or len(identity['key']) > 256
                or not re.fullmatch(r'oteryn:spawn\.authored\.[a-z0-9_][a-z0-9_.-]*', identity['key'])
                or identity['revision'] != 'definition-r1' or identity['key'] in keys
                or not position(centre)):
            raise ValueError('AUTHORED_SPAWN_IDENTITY')
        keys.add(identity['key'])
        provenance = declaration['provenance']
        if (not isinstance(provenance, dict) or set(provenance) != {'kind', 'quest', 'sources', 'rule'}
                or provenance['kind'] != 'oteryn-authored-chosen'
                or not isinstance(provenance['quest'], str)
                or not provenance['quest'].startswith('oteryn:quest.') or len(provenance['quest']) > 256
                or not isinstance(provenance['rule'], str) or not 1 <= len(provenance['rule']) <= 1024
                or not isinstance(provenance['sources'], list) or not 1 <= len(provenance['sources']) <= 8
                or any(not isinstance(s, str) or not 1 <= len(s) <= 1024 for s in provenance['sources'])):
            raise ValueError('AUTHORED_SPAWN_PROVENANCE')
        points = declaration['points']
        if not isinstance(points, list) or not 1 <= len(points) <= 8:
            raise ValueError('AUTHORED_SPAWN_POINTS')
        total += len(points)
        if total > 64:
            raise ValueError('AUTHORED_SPAWN_POINT_BOUND')
        for point in points:
            if not isinstance(point, dict) or set(point) != {'cell', 'creature', 'direction', 'respawn_ms'}:
                raise ValueError('AUTHORED_SPAWN_POINT')
            cell = point['cell']
            if (not position(cell) or cell['floor'] != centre['floor']
                    or abs(cell['x'] - centre['x']) > 64 or abs(cell['y'] - centre['y']) > 64
                    or not isinstance(point['creature'], str) or point['creature'] not in known
                    or point['creature'] in reserved or point['direction'] not in ('north', 'east', 'south', 'west')
                    or type(point['respawn_ms']) is not int or not 1_000 <= point['respawn_ms'] <= 86_400_000):
                raise ValueError('AUTHORED_SPAWN_NATIVE_POINT')
            key = (cell['x'], cell['y'], cell['floor'])
            if key in cells:
                raise ValueError('AUTHORED_SPAWN_OCCUPIED_POINT')
            cells.add(key)
    return document['records']
