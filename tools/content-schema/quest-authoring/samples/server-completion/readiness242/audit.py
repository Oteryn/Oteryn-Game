"""Separate preserved Source readiness, CHOSEN data and execution qualifications."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

NATIVE = {'quest_native_lowering_missing', 'claim_native_lowering_missing'}
SOURCE = {'reported_source_gap', 'requirement_unknown', 'source_kind_log_flag_conflict', 'claim_source_data_missing', 'source_reference_missing', 'gate_source_gap'}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()


def profile_readiness(definition):
    """No Source mutation or runtime inference; require all selected profile fences."""
    recipe = definition.get('oteryn_recipe') or {}
    chosen = (recipe.get('profile') == 'chosen_source_completion_v1'
              and recipe.get('chosen_data_complete') is True
              and recipe.get('runtime_enabled') is False
              and recipe.get('readiness') == 'waiting_native_bindings')
    return dict(original_source_readiness=definition['readiness'], chosen_data_complete=chosen,
                chosen_data_readiness='waiting_native_bindings' if chosen else 'waiting_data',
                source_holds_preserved=True, runtime_enabled=False, native_admission=False)


def classify_issue(issue, claims, lowering):
    code = issue['code']
    result = dict(issue=issue)
    if code in NATIVE:
        result.update(category='execution_binding', actual_data_hole=False)
        if code == 'quest_native_lowering_missing':
            result.update(artifact_present=lowering is not None,
                          derived_status='TRACK_ARTIFACT_PRESENT_EXECUTION_UNPROVEN' if lowering else 'NO_NATIVE_TRACK_ARTIFACT',
                          recommendation='Bind and qualify lowering/trigger/NPC execution; preserve Source core flag')
        else:
            claim = claims.get(issue.get('source_key'))
            result.update(artifact_present=claim is not None, derived_status='CLAIM_IMPLEMENTATION_UNPROVEN',
                          current_claim_readiness=claim.get('readiness') if claim else None,
                          recommendation='Use existing RewardClaim; qualify runtime variant and repeat denial')
    elif code == 'claim_item_semantics_missing':
        claim = claims.get(issue.get('source_key'))
        holds = claim.get('data_holds', []) if claim else []
        live = claim is not None and (claim.get('readiness') == 'waiting_item_semantics' or any(h['category'] == 'item' for h in holds))
        result.update(category='item_definition', actual_data_hole=live,
                      derived_status='LIVE_ITEM_SEMANTICS_HOLD' if live else 'STALE_OR_UNPROVEN_ITEM_HOLD',
                      current_claim_readiness=claim.get('readiness') if claim else None, holds=holds,
                      recommendation='Complete exact referenced Item semantics then regenerate' if live else 'Recheck exact source claim before changing flag')
    elif code in SOURCE:
        result.update(category='preserved_source', actual_data_hole=False,
                      derived_status='SOURCE_HOLD_NOT_CHOSEN_DATA_HOLE',
                      recommendation='Preserve Source disagreement/unknown; selected behavior is separate CHOSEN data')
    else:
        result.update(category='unclassified_data_hole', actual_data_hole=True,
                      derived_status='INVESTIGATE_EXACT_DEFINITION', recommendation='Do not classify unknown code as complete')
    return result


def audit(root):
    root = Path(root)
    inputs = {}
    def read(relative):
        raw = (root / relative).read_bytes()
        inputs[relative] = hashlib.sha256(raw).hexdigest()
        return json.loads(raw)
    idx = read('content/quests/definitions/index.json')
    definitions = [row['definition'] for p in idx['shards'] for row in read(p)['records']]
    cidx = read('content/interactions/reward_claims/index.json')
    claims = {row['definition']['provenance']['pilot_key']:row['definition'] for p in cidx['shards'] for row in read(p)['records']}
    lower = read('content/quests/missions/quest-state.json')
    lowers = {q['quest']:q for q in lower['quests']}
    prefix = 'tools/content-schema/quest-authoring/samples/donor-source/refinements/'
    conditions = read(prefix + 'conditions.json')
    specs = read(prefix + 'guard-specs.json')
    records = []
    for d in definitions:
        if d['readiness'] != 'waiting_data':
            continue
        key = d['identity']['key']
        issues = [classify_issue(i, claims, lowers.get(key)) for i in d['missing_data']]
        refs = d.get('oteryn_recipe', {}).get('donor_source_data', {}).get('refs', [])
        attached = [r for r in conditions['records'] if key in r['quest_keys']]
        operational = [r for r in specs['records'] if key in r['quest_keys']]
        tracks = lowers.get(key, {}).get('tracks', [])
        native = None if key not in lowers else dict(tracks=len(tracks), transitions=len(lowers[key]['transitions']),
            completion=lowers[key].get('completion'),
            effect_kinds=dict(Counter(e['effect']['kind'] for t in lowers[key]['transitions'] for e in t['effects'])), inexact_conditions=sum(not e.get('from_exact', True) for t in lowers[key]['transitions'] for e in t['effects']))
        holes = [i for i in issues if i['actual_data_hole']]
        holds = [i for i in issues if i['category'] == 'preserved_source']
        bindings = [i for i in issues if i['category'] == 'execution_binding']
        records.append(dict(quest_key=key, kind=d['kind'], definition_sha256=digest(d),
                            readiness_profiles=profile_readiness(d), original_missing_data=d['missing_data'],
                            gap_resolution=issues, remaining_item_or_unknown_holds=len(holes),
                            preserved_source_holds=len(holds), execution_binding_holds=len(bindings),
                            exclusively_execution_flags=bool(bindings) and len(bindings) == len(issues),
                            imported_donor_ref_counts=dict(Counter(r['category'] for r in refs)),
                            donor_guard_projection_counts=dict(Counter(r['status'] for r in attached)),
                            donor_guard_operational_counts=dict(Counter(r['status'] for r in operational)),
                            native_track_artifact=native,
                            next_work='ITEM_SEMANTICS_THEN_BINDINGS' if holes else 'CHOSEN_NATIVE_BINDINGS_AND_SMOKE',
                            source_completion_claimed=False, runtime_enabled=False))
    counts = dict(total_definitions=len(definitions), definition_readiness=dict(Counter(d['readiness'] for d in definitions)),
                  audited=len(records), chosen_data_complete=sum(r['readiness_profiles']['chosen_data_complete'] for r in records),
                  chosen_recipe_data_holes=sum(not r['readiness_profiles']['chosen_data_complete'] for r in records),
                  item_or_unclassified_hold_quests=sum(bool(r['remaining_item_or_unknown_holds']) for r in records),
                  item_or_unclassified_hold_refs=sum(r['remaining_item_or_unknown_holds'] for r in records),
                  preserved_source_hold_quests=sum(bool(r['preserved_source_holds']) for r in records),
                  execution_binding_quests=sum(bool(r['execution_binding_holds']) for r in records),
                  exclusively_execution_flag_quests=sum(r['exclusively_execution_flags'] for r in records),
                  native_track_artifact_quests=sum(r['native_track_artifact'] is not None for r in records),
                  donor_supplement_quests=sum(bool(r['imported_donor_ref_counts']) for r in records),
                  operational_guard_spec_quests=sum(bool(r['donor_guard_operational_counts']) for r in records),
                  source_code_counts=dict(Counter(i['code'] for r in records for i in r['original_missing_data'])),
                  operational_guard_global_status=dict(Counter(r['status'] for r in specs['records'])))
    return dict(schema='OTERYN_QUEST_READINESS_PROFILE_AUDIT/v1', counts=counts,
                scope='Exact current local files; counts overlap. Historical Source data holes retained; not live-main audit.',
                input_sha256=inputs, records=records, source_core_mutated=False, native_admission=False, runtime_enabled=False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    result = audit(args.root)
    Path(args.out).write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(result['counts'], indent=2))
