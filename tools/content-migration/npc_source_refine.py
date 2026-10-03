"""Closed R24 source upgrades; input records must be byte-custody reviewed first."""
from pathlib import Path
import copy, hashlib, json, re

ROOT = Path(__file__).resolve().parents[2]
from npc_bulk_enrich import indexed, values, update
from npc_bulk_stage import canonical
from npc_admission_stage import presentation_profile

FROM = 'g4-npc-provisional-enrichment-r23'
TO = 'g4-npc-provisional-enrichment-r24'
PARTS = {'look_type', 'head', 'body', 'legs', 'feet', 'addons', 'mount'}
def source(s):
    if not s.get('url') or not re.fullmatch('[0-9a-f]{64}', s.get('sha256', '')):
        raise ValueError('missing source URL/full-body custody SHA')

def speech(quote, actor):
    allowed = {'source', 'source_quote', 'quote_evidence', 'reply', 'classification',
               'normalization', 'matching_policy', 'source_fidelity_canonical_claim'}
    if set(quote) - allowed or quote.get('source_quote') is not True:
        raise ValueError('foreign speech/action keys or non-source quote')
    source(quote['source'])
    reply, evidence = quote.get('reply'), quote.get('quote_evidence')
    if not isinstance(reply, list) or not 1 <= len(reply) <= 2 or any(
            not isinstance(s, str) or not s.strip() or len(s) > 10240 for s in reply):
        raise ValueError('static quote must contain one or two nonempty text parts')
    if not isinstance(evidence, list) or len(evidence) != len(reply):
        raise ValueError('quote evidence must cover each literal part')
    normalized = quote.get('classification') == 'APPROXIMATE_PLACEHOLDER_NORMALIZATION'
    if normalized:
        if quote.get('normalization') != {'from': 'Jogador', 'to': '|PLAYERNAME|'}:
            raise ValueError('only explicit Jogador placeholder normalization allowed')
    elif quote.get('normalization'):
        raise ValueError('unclassified speech normalization')
    if quote.get('source_fidelity_canonical_claim'):
        raise ValueError('static mapping is not canonical source-fidelity proof')
    for text, proof in zip(reply, evidence):
        if set(proof) - {'original_exact_text', 'speaker_identity', 'start_byte',
                         'end_byte_exclusive', 'raw_fragment_sha256', 'source_line'}:
            raise ValueError('foreign quote evidence/action keys')
        if proof.get('speaker_identity') != actor:
            raise ValueError('quote speaker mismatch')
        raw = proof.get('original_exact_text')
        if not isinstance(raw, str) or not raw.strip():
            raise ValueError('missing exact original quote')
        expected = raw.replace('Jogador', '|PLAYERNAME|') if normalized else raw
        if text != expected or (normalized and 'Jogador' not in raw):
            raise ValueError('rewritten quote differs from permitted exact source text')
        start, end = proof.get('start_byte'), proof.get('end_byte_exclusive')
        if type(start) is not int or type(end) is not int or not 0 <= start < end:
            raise ValueError('invalid source quote byte bounds')
        if not re.fullmatch('[0-9a-f]{64}', proof.get('raw_fragment_sha256', '')):
            raise ValueError('missing quote fragment custody SHA')
        if quote['source'].get('bytes') is not None and end > quote['source']['bytes']:
            raise ValueError('source quote exceeds full body bytes')

def build(baseline, records):
    if baseline.get('project_revision', FROM) != FROM:
        raise ValueError('source-upgrade predecessor revision drift')
    declarations = indexed(baseline['records'], lambda r: r['identity']['key'])
    profiles = indexed(baseline['authoring_profiles'], lambda r: r['target']['key'])
    parent = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r23/native-enrichment.json'
    actors = {r['after']['identity']['key'] for r in json.loads(parent.read_text())['repairs']
              if r['after']['kind'] == 'NPC'}
    upgrades = indexed(records, lambda r: r['key'])
    if len(actors) != 133 or set(upgrades) - actors:
        raise ValueError('closed source-upgrade actor inventory drift')
    repairs, profile_repairs = [], []
    for key in sorted(actors):
        before = declarations[key]; row = copy.deepcopy(before)
        v = values(before); quality = json.loads(v['quality']); selected = upgrades.get(key)
        if before['kind'] != 'NPC' or before['services'] or v['status'] != 'provisional':
            raise ValueError('source-upgrade predecessor NPC scope')
        if selected:
            if selected['name'] != v['name']:
                raise ValueError('source-upgrade actor identity drift')
            documentary = json.loads(v.get('source_metadata', '{}'))
            documentary['r24_source_upgrades'] = selected
            update(row, {'source_metadata': documentary})
            if 'profession' in selected:
                p = selected['profession']; source(p['source'])
                if not p['label'].strip() or p['classification'] != 'SOURCE_SELECTED':
                    raise ValueError('profession is not reviewed source selection')
                update(row, {'profession_selection': p})
                quality['profession'] = 'donor'
                quality['profession.wiki.latest'] = 'verified' if p['source']['kind'] == 'wiki' else 'donor'
                # Old wiki_profession and source_unknown history remain unchanged.
            if 'appearance' in selected:
                a = selected['appearance']; source(a['source'])
                if quality['presentation'] != 'defaulted':
                    raise ValueError('source upgrade cannot overwrite existing donor profile')
                components = a['selected_outfit_fields']
                if not components or set(components) - PARTS:
                    raise ValueError('foreign appearance component')
                for part, value in components.items():
                    if part == 'mount' and value is None: continue
                    if type(value) is not int or value < 0:
                        raise ValueError('nonliteral numeric appearance component')
                if a['classification'] == 'ACTOR_EXACT_PUBLIC_DONOR':
                    if a['source']['literal_name'].casefold() != selected['name'].casefold():
                        raise ValueError('exact public donor actor mismatch')
                elif a['classification'] == 'WIKI_BRIDGE_FIELD_ONLY':
                    if set(components) != {'look_type'} or not a.get('bridge_evidence'):
                        raise ValueError('wiki bridge cannot promote unproven palette')
                else: raise ValueError('unsupported source appearance classification')
                previous = json.loads(v['appearance_selection'])
                outfit = dict(previous['outfit']); outfit.update(components)
                for part in components: quality['presentation.' + part] = 'donor'
                quality['presentation'] = 'donor'
                target = before['presentation']['key']; old = profiles[target]
                new = copy.deepcopy(old); new['data']['profile'] = presentation_profile(outfit)
                profile_repairs.append({'before': old, 'after': new})
                update(row, {'appearance_selection': {
                    'previous_project_choice': previous, 'outfit': outfit, **a,
                    'actor_exact_match': a['classification'] == 'ACTOR_EXACT_PUBLIC_DONOR',
                    'canonical_tibia_fidelity_claim': False,
                    'unproven_components_preserved_as_project_defaults': sorted(PARTS-set(components))}})
            if selected.get('dialogue') or ('profession' in selected and quality.get('dialogue.job') == 'defaulted'):
                old = declarations[before['dialogue']['key']]; new = copy.deepcopy(old)
                dq = json.loads(values(old)['quality'])
                role_alignment = None
                if 'profession' in selected and dq.get('dialogue.job') == 'defaulted':
                    role_reply = ['My role here is ' + selected['profession']['label'] + '.']
                    next(n for n in new['keywords'] if n['key'] == 'job')['reply'] = role_reply
                    role_alignment = {'reply': role_reply, 'source_quote': False,
                                      'classification': 'ORIGINAL_OTERYN_ROLE_ALIGNMENT'}
                for component, quote in selected.get('dialogue', {}).items():
                    if component not in {'greet','farewell','name','job','story'}:
                        raise ValueError('foreign dialogue component')
                    speech(quote, selected['name'])
                    if component in {'greet','farewell'}: new[component] = quote['reply']
                    else:
                        node = next((n for n in new['keywords'] if n['key'] == component), None)
                        if node is None:
                            if component != 'story' or not 1 <= len(quote['reply']) <= 2:
                                raise ValueError('foreign/new dialogue matcher')
                            node = {'key':'story','triggers':['story'],'reply':[]}; new['keywords'].append(node)
                        node['reply'] = quote['reply']
                    quality['dialogue.'+component] = dq['dialogue.'+component] = 'donor'
                if selected.get('dialogue'):
                    quality['dialogue.reference'] = dq['dialogue.reference'] = 'verified'
                quality['dialogue'] = dq['dialogue'] = 'donor' if any(
                    val == 'donor' for field, val in dq.items() if field.startswith('dialogue.')) else 'defaulted'
                update(new, {'quality': dq, 'dialogue_source': {
                    'previous_source_selection': json.loads(values(old).get('dialogue_source','{}')),
                    'r24_selected': selected.get('dialogue', {}),
                    'r24_original_role_alignment': role_alignment,
                    'matcher_policy':'OTERYN_STATIC_MAPPING_APPROXIMATE_NO_ACTIONS'}})
                repairs.append({'before': old, 'after': new})
        update(row, {'quality': quality})
        repairs.append({'before': before, 'after': row})
    return {'schema':'OTERYN_NPC_BULK_ENRICHMENT/v1', 'from_project_revision':FROM,
            'project_revision':TO, 'repairs':repairs, 'profile_repairs':profile_repairs}

if __name__ == '__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--baseline', type=Path, required=True)
    p.add_argument('--upgrades', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args=p.parse_args(); packet=build(json.loads(args.baseline.read_text()), json.loads(args.upgrades.read_text()))
    args.output.write_bytes(canonical(packet)); print(hashlib.sha256(args.output.read_bytes()).hexdigest())
