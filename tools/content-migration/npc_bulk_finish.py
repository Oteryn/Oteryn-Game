"""Finish basic provisional authoring with source speech and explicit Oteryn choices.

Source unknowns survive. This closes name/role/basic-dialogue/appearance choices,
not runtime placement, services, quests, ambient scheduling or canonical fidelity.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from npc_admission_stage import presentation_profile
from npc_bulk_enrich import ROOT, indexed, update, values
from npc_bulk_stage import canonical

EVIDENCE = ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r23'
ROLE_TRANSLATIONS = {
    'ochroniarz': 'guard', 'sklepikarz': 'shopkeeper', 'szaman': 'shaman',
    'więzień': 'prisoner', 'podróżnik': 'traveller', 'strażnik': 'guard',
    'górnik': 'miner', 'właściciel tawerny': 'tavern keeper', 'sztygar': 'mine foreman',
    'kapitan': 'captain', 'World Task': 'world task contact', 'Demon mother': 'demon mother',
    'pies': 'companion', 'profesor': 'professor', 'rycerz': 'knight',
    'członkini Towarzystwa Jagód': 'berry society member', 'członek Towarzystwa Jagód': 'berry society member',
    'wróżbita': 'fortune teller', 'naukowiec': 'scientist', 'nadzorca': 'overseer',
    'król': 'king', 'przewodnik': 'guide', 'stolarz': 'carpenter', 'gawędziarz': 'storyteller',
    'podróżnik w czasie': 'time traveller', 'Wiedźma': 'witch', 'książę': 'prince',
    'dostawca': 'supplier', 'królowa': 'queen', 'uczeń': 'apprentice', 'rybak': 'fisherman',
}


def build(baseline, facts):
    declarations = indexed(baseline['records'], lambda r: r['identity']['key'])
    profiles = indexed(baseline['authoring_profiles'], lambda r: r['target']['key'])
    previous = json.loads((ROOT / 'docs/agents/evidence/OTV2-20261002-npc-enrichment-r22/native-enrichment.json').read_text())
    actors = {r['after']['identity']['key'] for r in previous['repairs'] if r['after']['kind'] == 'NPC'}
    speech = indexed(facts['dialogue']['records'], lambda r: r['key'])
    roles = indexed(facts['profession']['records'], lambda r: r['key'])
    looks = indexed(facts['appearances'], lambda r: r['key'])
    expected_looks = {key for key in actors if json.loads(values(declarations[key])['quality'])['presentation'] == 'placeholder'}
    expected_roles = {key for key in actors if json.loads(values(declarations[key])['quality'])['profession'] == 'todo'}
    if len(actors) != 133 or set(speech) != actors or set(looks) != expected_looks or set(roles) != expected_roles:
        raise ValueError('finish closed actor/placeholder/unknown-role scope drifted')
    repairs, profile_repairs, progress = [], [], []
    counts = {'actors': 133, 'basic_components': 532, 'source_component_upgrades': 0,
              'original_component_replies': 0, 'retained_components': 0, 'source_inferred_new_roles': 0,
              'project_default_new_roles': 0, 'source_informed_approximate_appearances': 0,
              'neutral_project_appearances': 0}
    for key in sorted(actors):
        before = declarations[key]
        v = values(before)
        if before['kind'] != 'NPC' or before['services'] or v['status'] != 'provisional':
            raise ValueError('finish requires disabled provisional actor')
        row = copy.deepcopy(before)
        quality = json.loads(v['quality'])
        role = roles.get(key)
        if role:
            selection = role['profession_selection']
            if role['name'] != v['name'] or role['runtime_enabled'] or role['services_enabled'] or not selection['label'] or not role['evidence']:
                raise ValueError('finish role attribution/runtime/evidence drifted')
            if selection['classification'] not in {'INFERRED_APPROXIMATE', 'OTERYN_PROJECT_DEFAULT'} or selection['canonical_wiki_profession_claim']:
                raise ValueError('finish role invents canonical source fact')
            quality['profession'] = 'donor' if selection['classification'] == 'INFERRED_APPROXIMATE' else 'defaulted'
            quality['profession.wiki'] = 'todo'
            update(row, {'profession_selection': role})
            counts['source_inferred_new_roles' if selection['classification'] == 'INFERRED_APPROXIMATE' else 'project_default_new_roles'] += 1
        selected_role = json.loads(values(row)['profession_selection'])['profession_selection']['label'] if 'profession_selection' in values(row) else ROLE_TRANSLATIONS.get(v['wiki_profession'])
        if not selected_role:
            raise ValueError('finish lacks explicit role choice')
        if key in looks:
            look = looks[key]
            if look['name'] != v['name'] or look['actor_exact_match'] or look['target_native_appearance_verified'] or look['native_runtime_loaded'] or look['movement_changed'] or look['services_changed']:
                raise ValueError('finish appearance claims actor equivalence/runtime')
            template = look['source_template']
            if template and (look['outfit'] != template['outfit'] or not template['source']['sha256'] or not template['outfit_quote']):
                raise ValueError('finish appearance is not the cited literal template')
            if look['choice_classification'] not in {'OTERYN_SOURCE_INFORMED_APPROXIMATE_APPEARANCE', 'OTERYN_EXPLICIT_NEUTRAL_PROJECT_APPEARANCE_DEFAULT'}:
                raise ValueError('finish appearance classification drifted')
            informed = look['choice_classification'] == 'OTERYN_SOURCE_INFORMED_APPROXIMATE_APPEARANCE'
            neutral = {'look_type': 128, 'head': 0, 'body': 0, 'legs': 0, 'feet': 0, 'addons': 0, 'mount': None}
            if bool(template) != informed or (not template and look['outfit'] != neutral):
                raise ValueError('finish neutral choice or template classification drifted')
            quality['presentation'] = 'defaulted'
            for part in ['look_type', 'head', 'body', 'legs', 'feet', 'addons', 'mount']:
                quality['presentation.' + part] = 'defaulted'
            old_profile = profiles[before['presentation']['key']]
            next_profile = copy.deepcopy(old_profile)
            next_profile['data']['profile'] = presentation_profile(look['outfit'])
            profile_repairs.append({'before': old_profile, 'after': next_profile})
            update(row, {'appearance_selection': look})
            counts['source_informed_approximate_appearances' if template else 'neutral_project_appearances'] += 1
        d = speech[key]
        if d['name'] != v['name'] or d['runtime_enabled'] or not d['basic_components_closed']:
            raise ValueError('finish dialogue attribution/runtime drifted')
        if set(d['selected_parts']) & set(d['retained_parts']) or set(d['selected_parts']) | set(d['retained_parts']) != {'greet', 'farewell', 'name', 'job'}:
            raise ValueError('finish basic component partition drifted')
        dialogue = declarations[before['dialogue']['key']]
        next_dialogue = copy.deepcopy(dialogue)
        dq = json.loads(values(dialogue)['quality'])
        selected = []
        for part in ['greet', 'farewell', 'name', 'job']:
            old_reply = dialogue[part] if part in {'greet', 'farewell'} else next(n['reply'] for n in dialogue['keywords'] if n['key'] == part)
            if part in d['retained_parts']:
                retained = d['retained_parts'][part]
                if retained['reply'] != old_reply or not retained['preserve_existing']:
                    raise ValueError('finish retained speech drifted')
                retained_quality = retained['quality']
                if retained_quality == 'ORIGINAL_OTERYN_REPLY':
                    retained_quality = 'defaulted'
                if retained_quality not in {'verified', 'donor', 'defaulted'}:
                    raise ValueError('finish retained quality is not an authoring field flag')
                dq['dialogue.' + part] = retained_quality
                counts['retained_components'] += 1
                continue
            proposal = copy.deepcopy(d['selected_parts'][part])
            if proposal['runtime_enabled'] or not 1 <= len(proposal['reply']) <= 2 or not all(isinstance(s, str) and s.strip() for s in proposal['reply']):
                raise ValueError('finish invalid static reply')
            if proposal['source_quote']:
                if proposal['quality'] != 'SOURCE_QUOTED_OTERYN_STATIC_SELECTION' or not 0 <= proposal['source_index'] < len(d['sources']):
                    raise ValueError('finish invalid source quotation')
                if [e['original_exact_text'] for e in proposal['quote_evidence']] != proposal['reply'] or any(e['speaker_identity'] != v['name'] for e in proposal['quote_evidence']):
                    raise ValueError('finish source quote rewritten or misattributed')
                dq['dialogue.' + part] = 'donor'
                counts['source_component_upgrades'] += 1
            else:
                if proposal['quality'] != 'ORIGINAL_OTERYN_REPLY' or proposal['authoring_basis']['source_quote'] is not False:
                    raise ValueError('finish original text falsely attributed')
                if part == 'job':
                    proposal['reply'] = ['My role here is ' + selected_role + '.']
                    proposal['authoring_basis']['selected_project_role'] = selected_role
                    proposal['authoring_basis']['role_translation_policy'] = 'Original Oteryn prose; documented Polish roles translated or source-linked project role used; not transcript.'
                dq['dialogue.' + part] = 'defaulted'
                counts['original_component_replies'] += 1
            if part in {'greet', 'farewell'}:
                next_dialogue[part] = proposal['reply']
            else:
                next(n for n in next_dialogue['keywords'] if n['key'] == part)['reply'] = proposal['reply']
            selected.append({'part': part, **proposal})
        # Retain source story and all previous documentary speech selections.
        dq['dialogue'] = 'donor' if any(value == 'donor' for field, value in dq.items() if field.startswith('dialogue.')) else 'defaulted'
        update(next_dialogue, {'quality': dq, 'fallback_selection': {'sources': d['sources'], 'selected': selected,
                            'retained_parts': d['retained_parts'], 'source_quote_policy': 'Original Oteryn fallback explicitly distinguished; source matcher/state semantics unproven; no actions.'}})
        repairs.append({'before': dialogue, 'after': next_dialogue})
        quality.update(dq)
        # Existing voice evidence survives. Quiet idle policy is an explicit project choice,
        # not a claim that real actors are silent, and no ambient scheduler is enabled.
        quality['voices'] = 'defaulted'
        unknown = [field for field, value in quality.items() if value in {'todo', 'placeholder'}]
        completion = {'status': 'BASIC_AUTHORING_READY_APPROXIMATE', 'name_role_basic_dialogue_appearance_choices_closed': True,
                      'source_unknown_fields_preserved': unknown, 'canonical_tibia_fidelity_claim': False,
                      'native_runtime_loaded': False, 'interaction_smoke_passed': False,
                      'deferred_gameplay': ['native placement', 'NPC consumer and interaction', 'quest effects', 'trade/travel services', 'ambient voice scheduling'],
                      'idle_voice_choice': {'classification': 'OTERYN_PROJECT_DEFAULT', 'policy': 'quiet_until_runtime_scheduler_and_source_selection_are_admitted', 'real_actor_silence_claim': False}}
        update(row, {'quality': quality, 'completion': completion})
        repairs.append({'before': before, 'after': row})
        progress.append({'key': key, 'name': v['name'], 'state': completion['status'], 'field_quality': quality,
                         'native_runtime_loaded': False, 'source_unknown_fields': unknown, 'deferred_gameplay': completion['deferred_gameplay']})
    counts['source_selected_dialogues'] = sum(r['field_quality']['dialogue'] == 'donor' for r in progress)
    counts['original_only_dialogues'] = 133 - counts['source_selected_dialogues']
    counts['unselected_roles'] = counts['appearance_placeholders'] = counts['basic_dialogue_placeholders'] = 0
    packet = {'schema': 'OTERYN_NPC_BULK_ENRICHMENT/v1', 'from_project_revision': previous['project_revision'],
              'project_revision': 'g4-npc-provisional-enrichment-r23', 'repairs': repairs, 'profile_repairs': profile_repairs}
    return packet, progress, counts


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--predecessor', type=Path, required=True)
    args = parser.parse_args()
    raw = (EVIDENCE / 'source-facts.json').read_bytes()
    if hashlib.sha256(raw).hexdigest() != json.loads((EVIDENCE / 'source-custody.json').read_text())['portable_facts_sha256']:
        raise ValueError('finish source custody drifted')
    packet, rows, counts = build(json.loads((args.predecessor / 'definitions/declarations.json').read_text()), json.loads(raw))
    (EVIDENCE / 'native-enrichment.json').write_bytes(canonical(packet))
    (EVIDENCE / 'progress.json').write_text(json.dumps({'records': rows, 'counts': counts, 'runtime_loaded': 0}, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({**counts, 'packet_sha256': hashlib.sha256(canonical(packet)).hexdigest()}))
