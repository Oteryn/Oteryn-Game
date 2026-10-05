"""Finite chosen-recipe minima from already cached wiki; original Source remains unchanged."""
import copy
import hashlib
import json

PATH = 'tools/content-schema/quest-authoring/samples/recipe-followup/requirements.json'
SHA256 = '32c2548a79f3df3d0d088df5f8fa0def7ff692027f8e6c8ea45e17fbaad74eb2'
SPEC_PATH = 'tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json'
ALLOWED = {'oteryn:quest.battle_mage_outfits_quest': 250,
           'oteryn:quest.falconer_outfits_quest': 100,
           'oteryn:quest.makeshift_warrior_outfits_quest': 100}


def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def apply_packet(rows, packet, spec_bytes):
    if (packet['schema'] != 'OTERYN_CHOSEN_REQUIREMENT_FOLLOWUP/v1'
            or packet['runtime_enabled'] is not False or packet['source_holds_preserved'] is not True
            or hashlib.sha256(spec_bytes).hexdigest() != packet['source_spec_sha256']):
        raise ValueError('requirement followup qualification differs')
    specs = json.loads(spec_bytes)['entries']
    fixes = packet['changes']
    if len(fixes) != len(ALLOWED) or {f['canonical_key'] for f in fixes} != set(ALLOWED):
        raise ValueError('requirement followup finite scope differs')
    out = copy.deepcopy(rows)
    for fix in fixes:
        key = fix['canonical_key']
        selected = [r for r in out if r['definition']['identity']['key'] == key]
        if len(selected) != 1:
            raise ValueError('requirement followup owner differs')
        definition = selected[0]['definition']
        payload = definition['oteryn_recipe']['payload']
        recipe = payload['recipe']
        if digest(recipe) != fix['baseline_recipe_sha256']:
            raise ValueError('requirement followup recipe fence differs')
        if (fix['old_value'], fix['new_value'], fix['path']) != (0, ALLOWED[key], '/requirements/min_level'):
            raise ValueError('requirement followup change differs')
        entries = [e for e in specs if e['wiki_title'] == recipe['wiki_title']]
        if len(entries) != 1 or digest(entries[0]) != fix['source_entry_sha256']:
            raise ValueError('requirement followup source entry differs')
        entry = entries[0]
        selected_specs = [s for s in entry['source_specification'] if s['provider'] == 'tibiawiki_br' and s['relation'] == 'crosscheck']
        refs = [s for s in entry['source_refs'] if s['provider'] == 'tibiawiki_br' and s['relation'] == 'crosscheck']
        if len(selected_specs) != 1 or len(refs) != 1 or refs[0] != fix['source_ref']:
            raise ValueError('requirement followup source revision differs')
        semantic = selected_specs[0]['semantic_enrichment']
        level = semantic['level']
        if (level['kind'], level['plain_integer'], level['source_value']) != ('plain_integer', ALLOWED[key], str(ALLOWED[key])):
            raise ValueError('requirement followup level witness differs')
        if semantic['source_revision_sha256'] != refs[0]['content_sha256']:
            raise ValueError('requirement followup source body witness differs')
        if recipe['requirements']['min_level'] != 0 or definition['requirements'].get('min_level') is not None:
            raise ValueError('requirement followup original unknown differs')
        if definition['requirements_from_wiki'].get('lvl'):
            raise ValueError('requirement followup original level was stated')
        if refs[0] not in recipe['source_refs']:
            raise ValueError('requirement followup recipe revision link missing')
        recipe['requirements']['min_level'] = ALLOWED[key]
        recipe['source_notes'].append(fix['source_note'])
    return out


def apply(root, rows):
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('requirement followup packet differs')
    result = apply_packet(rows, json.loads(raw), (root / SPEC_PATH).read_bytes())
    return result, {'path': PATH, 'sha256': SHA256}
