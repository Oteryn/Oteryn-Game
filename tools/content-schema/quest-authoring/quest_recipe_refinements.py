"""Apply the approved chosen-recipe corrections without rewriting SOURCE inputs."""
import copy
import hashlib
import json
from pathlib import Path

DIRECTORY = 'tools/content-schema/quest-authoring/samples/refinements242/'
PACKET = DIRECTORY + 'refinements.json'
APPROVED_SHA256 = 'b69dbe6d580221b089d03e8952329c54a7e117abc02f11d476f56f18818254cf'


def digest(value):
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
    return hashlib.sha256(raw.encode()).hexdigest()


def protected(row):
    value = copy.deepcopy(row)
    recipe = value['recipe']
    recipe.pop('summary')
    recipe.pop('source_notes')
    recipe['requirements'].pop('prerequisites')
    for stage in recipe['stages']:
        for field in ('objective', 'kind', 'targets'):
            stage.pop(field)
    return value


def location(recipe, path):
    parts = path.split('/')[1:]
    if not path.startswith('/') or not parts or any(not x for x in parts):
        raise ValueError('invalid refinement path')
    allowed = path in ('/summary', '/stages', '/requirements/prerequisites')
    allowed |= len(parts) == 2 and parts[0] == 'source_notes' and parts[1].isdigit()
    allowed |= (len(parts) == 3 and parts[0] == 'stages' and parts[1].isdigit()
                and parts[2] in ('objective', 'kind', 'targets'))
    if not allowed:
        raise ValueError('refinement changes a protected field')
    value = recipe
    for field in parts[:-1]:
        value = value[int(field)] if isinstance(value, list) else value[field]
    field = int(parts[-1]) if isinstance(value, list) else parts[-1]
    return value, field


def apply_packet(payload, packet, original_sha256):
    """Return new rows; the caller supplies the original file's byte digest."""
    from quest_completion_authoring import validate_journey
    if (packet['schema'] != 'OTERYN_QUEST_RECIPE_REFINEMENTS/v1'
            or packet['runtime_enabled'] is not False
            or packet['source_holds_preserved'] is not True
            or payload['runtime_enabled'] is not False
            or original_sha256 != packet['original_recipe_packet_sha256']):
        raise ValueError('refinement input or Native admission differs')
    rows = copy.deepcopy(payload['records'])
    indexed = {r['identity']['key']: r for r in rows}
    original = {r['identity']['key']: r for r in payload['records']}
    if (len(rows) != 242 or len(indexed) != 242
            or set(indexed) != set(packet['original_record_digests'])
            or set(indexed) != set(packet['refined_record_digests'])):
        raise ValueError('refinement selection differs')
    for key, row in indexed.items():
        if digest(row) != packet['original_record_digests'][key]:
            raise ValueError('original refinement record differs: ' + key)
    changed, paths = set(), set()
    for change in packet['changes']:
        key, path = change['canonical_key'], change['path']
        if key not in indexed or (key, path) in paths:
            raise ValueError('unknown or duplicate refinement target')
        paths.add((key, path))
        parent, field = location(indexed[key]['recipe'], path)
        if parent[field] != change['old_value'] or change['old_value'] == change['new_value']:
            raise ValueError('refinement old-value fence differs')
        parent[field] = copy.deepcopy(change['new_value'])
        changed.add(key)
    if len(paths) != 21 or len(changed) != 13:
        raise ValueError('refinement must contain exactly 21 changes in 13 recipes')
    for key, row in indexed.items():
        if protected(row) != protected(original[key]):
            raise ValueError('refinement changes identity, journey counts or provenance')
        validate_journey(row['recipe'])
        if digest(row) != packet['refined_record_digests'][key]:
            raise ValueError('refined record digest differs: ' + key)
    return rows


def apply_refinements(root, payload):
    """Optional portable overlay plus provenance, preserving the original packet."""
    path = Path(root) / PACKET
    if not path.exists():
        return copy.deepcopy(payload['records']), None
    raw = path.read_bytes()
    sha = hashlib.sha256(raw).hexdigest()
    if sha != APPROVED_SHA256:
        raise ValueError('unapproved refinement packet')
    packet = json.loads(raw)
    original_path = Path(root) / 'tools/content-schema/quest-authoring/samples/completion242/recipes.json'
    original_raw = original_path.read_bytes()
    if json.loads(original_raw) != payload:
        raise ValueError('refinement payload is not the immutable original')
    rows = apply_packet(payload, packet, hashlib.sha256(original_raw).hexdigest())
    return rows, {'path': PACKET, 'sha256': sha,
                  'original_recipe_packet_sha256': packet['original_recipe_packet_sha256'],
                  'reviewed_input_sha256': packet['reviewed_input']['sha256'],
                  'qualification_sha256': packet['reviewed_input']['qualification_sha256']}
