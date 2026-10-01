#!/usr/bin/env python3
"""Deterministic, complete SOURCE quest bundle. Does not admit native or runtime data."""
import argparse
import hashlib
import json
from pathlib import Path

import jsonschema
from referencing import Registry, Resource
import bundle_semantics as semantics

LOCAL = Path(__file__).resolve().parent
SCHEMAS = {'quest_content.schema.json':'oteryn:schema/quest-content/v1',
           'interaction.schema.json':'oteryn:schema/interaction/v1',
           'quest_catalogue.schema.json':'oteryn:schema/quest-catalogue/v1',
           'quest_progress.schema.json':'oteryn:schema/quest-progress-source/v1',
           'quest_bundle.schema.json':'oteryn:schema/quest-source-bundle/v1'}
INPUTS = {'quests':'questlog/quests.json','progress':'questlog/progress.json',
          'interactions':'interactions/interactions.json','gates':'doors/gates.json',
          'claims':'chests/claims.json','wiki_catalogue':'catalogue/catalogue.json',
          'readiness':'readiness/readiness.json','questlog_manifest':'questlog/manifest.json',
          'interactions_manifest':'interactions/manifest.json','gates_manifest':'doors/manifest.json',
          'claims_manifest':'chests/manifest.json'}


def read(path):
    return json.loads(path.read_text())


def encoded(value):
    return json.dumps(value,indent=2,ensure_ascii=False,sort_keys=True)+'\n'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def schema_provenance(tool_root, schema_root):
    return [{'id':identity,'path':'tools/content-schema/quest-authoring/'+name,
             'sha256':digest((schema_root if name in ('quest_progress.schema.json','quest_bundle.schema.json') else tool_root)/name)}
            for name,identity in sorted(SCHEMAS.items())]


def tool_provenance():
    return [{'path':'tools/content-schema/quest-authoring/'+name,'sha256':digest(LOCAL/name)}
            for name in ('bundle_authoring.py','bundle_semantics.py')]


def offline_validator(tool_root, schema_root):
    registry, schemas = Registry(), {}
    for filename, identity in SCHEMAS.items():
        path=(schema_root if filename in ('quest_progress.schema.json','quest_bundle.schema.json') else tool_root)/filename
        schema=read(path)
        schema.setdefault('$id',identity)
        if schema['$id'] != identity:
            raise ValueError('unexpected schema identity: '+filename)
        jsonschema.Draft202012Validator.check_schema(schema)
        schemas[identity]=schema
        registry=registry.with_resource(identity,Resource.from_contents(schema))
    return jsonschema.Draft202012Validator(schemas[SCHEMAS['quest_bundle.schema.json']],registry=registry)


def schema_errors(bundle, validator):
    return [str(list(e.absolute_path))+': '+e.message for e in validator.iter_errors(bundle)]


def validate(bundle, validator, trusted_schemas=None):
    """Structural validation only; original input byte provenance is not verified here."""
    errors=schema_errors(bundle,validator)
    if errors:
        raise ValueError('\n'.join(errors[:20]))
    if trusted_schemas is not None and bundle['schema_provenance']!=trusted_schemas:
        raise ValueError('stale or forged local schema provenance')
    if bundle['tool_provenance']!=tool_provenance():
        raise ValueError('stale or forged generator provenance')
    for field in ('input_provenance','schema_provenance'):
        rows=bundle[field]; identities=[row.get('role',row.get('id')) for row in rows]
        if len(identities)!=len(set(identities)):
            raise ValueError('duplicate '+field)
    if {row['role'] for row in bundle['input_provenance']} != set(INPUTS):
        raise ValueError('incomplete exact input provenance')
    if {row['id'] for row in bundle['schema_provenance']} != set(SCHEMAS.values()):
        raise ValueError('incomplete offline schema provenance')
    pins={(r['repository'],r['revision']) for r in bundle['sources']}
    for track in bundle['progress']:
        for transition in track['transitions']:
            if any((o['repository'],o['revision']) not in pins for o in transition['source_occurrences']):
                raise ValueError('source occurrence has an undeclared repository pin')
    if [json.loads(row) for row in semantics.validate_relations(bundle)] != bundle['reference_gaps']:
        raise ValueError('missing or stale reference-gap inventory')
    quests={q['identity']['key'] for q in bundle['quests']}
    records={row['quest']:row for row in bundle['quest_gaps']}
    if records.keys()!=quests or len(records)!=len(bundle['quest_gaps']):
        raise ValueError('per-quest gaps do not cover the exact quest catalogue')
    for key,row in records.items():
        readiness=row['reported_source_readiness']
        if readiness and readiness['quest']!=key:
            raise ValueError('source readiness identity mismatch')
    expected={name:len(bundle[name]) for name in semantics.COLLECTIONS}
    expected.update(wiki_quests=len(bundle['wiki_catalogue']['quests']),
                    quests_with_reported_gaps=sum(bool(q['gaps']) for q in bundle['quest_gaps']),
                    reference_gaps=len(bundle['reference_gaps']),unassigned_gaps=len(bundle['unassigned_gaps']),
                    progress_source_transitions=sum(len(t['transitions']) for t in bundle['progress'] if not t.get('alias_of')),
                    source_write_occurrences=sum(len(e['source_occurrences']) for t in bundle['progress'] if not t.get('alias_of') for e in t['transitions']))
    if bundle['summary']!=expected:
        raise ValueError('stale bundle summary')
    derived=semantics.derive(bundle,bundle['gap_evidence'])
    for field in ('quest_gaps','unassigned_gaps'):
        if bundle[field]!=derived[field]:
            raise ValueError('missing or stale derived source gaps: '+field)
    conflicts=bundle['interaction_source_conflicts']
    if len({r['interaction'] for r in conflicts})!=len(conflicts):
        raise ValueError('duplicate source interaction conflict')
    for conflict in conflicts:
        sources=[a['source'] for a in conflict['alternatives']]
        if len(sources)!=len(set(sources)) or set(sources)!={r['source'] for r in conflict['source_witnesses']} or len(sources)<2:
            raise ValueError('incomplete or duplicate conflict alternatives/witnesses')
        if conflict['interaction'] not in {r['identity']['key'] for r in bundle['interactions']}:
            raise ValueError('source conflict lacks primary interaction')


def build(tool_root, samples_root, schema_root):
    docs={role:read(samples_root/path) for role,path in INPUTS.items()}
    data={name:docs[name][name] for name in semantics.COLLECTIONS}
    data['wiki_catalogue']=docs['wiki_catalogue']
    data['interaction_source_conflicts']=[{'interaction':e['destination'],'classification':'CONFLICT',
                                           'source_witnesses':e['sources'],'alternatives':e['conflict_alternatives']}
                                          for e in docs['interactions_manifest']['entries'] if e.get('conflict_alternatives')]
    manifests={name:docs[role] for name,role in [('questlog','questlog_manifest'),('interactions','interactions_manifest'),('gates','gates_manifest'),('claims','claims_manifest')]}
    pins=set()
    for manifest in manifests.values():
        pins.update((row['repository'],row['revision']) for row in manifest.get('sources',[]) if row.get('kind')=='git')
    bundle={'schema':'OTERYN_QUEST_SOURCE_BUNDLE/v1','classification':'OTS_HYPOTHESIS_ONLY',
            'scope':'source_inventory_only; no native or runtime authority',
            'canonical_admission':'NOT_ASSESSED','runtime_readiness':'UNKNOWN',
            'sources':[{'repository':repo,'revision':pin} for repo,pin in sorted(pins)],
            'input_provenance':[{'role':role,'path':'tools/content-schema/quest-authoring/samples/'+path,
                                 'sha256':digest(samples_root/path)} for role,path in sorted(INPUTS.items())],
            'schema_provenance':schema_provenance(tool_root,schema_root),
            'tool_provenance':tool_provenance(),**data}
    checks=manifests['questlog'].get('source_checks',{})
    titles={q['display_name']:q['identity']['key'] for q in data['quests']}
    evidence={'reported_readiness':docs['readiness']['quests'],
              'coverage_holds':[{'quest':h['quest'],'record':h['quest']+'/'+h.get('npc_source','|'.join(h.get('source_paths',[]))), 'reason':h['coverage_gap']}
                                for category in ('npc_only_quests','script_coverage_holds') for h in checks.get(category,[])],
              'deferred_owners':[{'record':key,'quest':h.get('owner_evidence',{}).get('quest') or titles.get(h.get('owner_evidence',{}).get('wiki_quest')),'reason':h['reason']}
                                 for key,h in sorted(checks.get('deferred_track_owners',{}).items())]}
    bundle['gap_evidence']=evidence
    bundle.update(semantics.derive(data,evidence))
    validate(bundle,offline_validator(tool_root,schema_root),schema_provenance(tool_root,schema_root))
    return bundle


def validate_source_backed(bundle, tool_root, samples_root, schema_root):
    """Require the entire packet to equal deterministic regeneration from exact local inputs."""
    expected=build(tool_root,samples_root,schema_root)
    if bundle!=expected:
        raise ValueError('bundle differs from exact local source-backed regeneration')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tool-root',type=Path,default=LOCAL)
    parser.add_argument('--samples-root',type=Path)
    parser.add_argument('--schema-root',type=Path,default=LOCAL)
    parser.add_argument('--out',type=Path)
    parser.add_argument('--check',action='store_true')
    parser.add_argument('--validate',type=Path,help='Structural validation only unless --source-backed is supplied')
    parser.add_argument('--source-backed',action='store_true',help='With --validate, verify full equality against exact local sample inputs')
    args=parser.parse_args()
    if args.source_backed and not args.validate:
        parser.error('--source-backed requires --validate')
    if args.validate and (args.check or args.out):
        parser.error('--validate cannot be combined with --check or --out')
    try:
        validator=offline_validator(args.tool_root,args.schema_root)
        if args.validate:
            value=read(args.validate);validate(value,validator,schema_provenance(args.tool_root,args.schema_root))
            mode='STRUCTURAL_ONLY';provenance='NOT_VERIFIED'
            if args.source_backed:
                validate_source_backed(value,args.tool_root,args.samples_root or LOCAL/'samples',args.schema_root)
                mode='SOURCE_BACKED';provenance='VERIFIED_AGAINST_LOCAL_INPUTS'
        else:
            mode='SOURCE_BACKED';provenance='VERIFIED_AGAINST_LOCAL_INPUTS'
            value=build(args.tool_root,args.samples_root or args.tool_root/'samples',args.schema_root)
            output=args.out or args.tool_root/'samples/source_migration/bundle.json'
            content=encoded(value)
            if args.check:
                if not output.is_file() or output.read_text()!=content:
                    raise ValueError('stale or missing complete source bundle; regenerate before checking')
            else:
                output.parent.mkdir(parents=True,exist_ok=True);output.write_text(content)
        print(json.dumps({'valid':True,'validation_mode':mode,'input_provenance_verification':provenance,'scope':value['scope'],'summary':value['summary']}))
    except (ValueError,KeyError,OSError) as error:
        print(json.dumps({'valid':False,'error':str(error)}));return 1
    return 0


if __name__=='__main__':
    raise SystemExit(main())
