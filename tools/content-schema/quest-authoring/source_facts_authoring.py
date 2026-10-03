"""Portable replay of bounded SOURCE facts; recorded witnesses do not recheck raw bodies."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest()

def projection(entry):
    row={'wiki_title':entry['wiki_title'],'parent_entry_sha256':digest(entry),'source_refs':copy.deepcopy(entry['source_refs']),
         'objective_entity_references':[],'source_heading_references':[],'source_directive_references':[],
         'source_requirement_expressions':[],'source_reward_references':[],'source_reward_groups':[],
         'source_reward_quantitative_facts':[],'source_fields':copy.deepcopy(entry.get('source_fields_with_provenance',entry.get('source_fieldsets',[]))),
         'known_source_ids':[],'unresolved':copy.deepcopy(entry['unresolved']),'source_binding_status':'UNKNOWN',
         'full_walkthrough_complete':False,'definition_complete':False,'runtime_readiness':'UNKNOWN'}
    fields={'prerequisite_expressions':'source_requirement_expressions','reward_entity_references':'source_reward_references',
            'reward_groups':'source_reward_groups','reward_quantitative_facts':'source_reward_quantitative_facts'}
    for origin,semantic in [('source_specification','semantic_enrichment'),('source_fact_supplements','semantic_enrichment_delta')]:
        for source in entry.get(origin,[]):
            context={k:source[k] for k in ('provider','relation')}
            for field,target in fields.items():
                row[target]+=[{**context,'origin':origin,'value':copy.deepcopy(value)} for value in source[semantic].get(field,[])]
            row['source_heading_references'] += [{**context,'value':copy.deepcopy(value)} for value in source.get('heading_witnesses',[])]
    return row

def schema(wiki_schema):
    def obj(fields):return {'type':'object','properties':fields,'required':list(fields),'additionalProperties':False}
    text={'type':'string','minLength':1};integer={'type':'integer','minimum':1};hashed={'type':'string','pattern':'^[0-9a-f]{64}$'}
    context={'provider':{'enum':['tibia_fandom','tibiawiki_br']},'relation':{'enum':['base','spoiler','crosscheck']}}
    def array(items):return {'type':'array','items':items}
    def ref(name):return {'$ref':'#/$defs/legacy_'+name}
    def wrapped(value,origin=False):return obj({**context,'value':value})
    def semantic(legacy,field):
        fresh=copy.deepcopy(wiki_schema['$defs']['fresh_semanticEnrichment']['properties'][field]['items'])
        return {'oneOf':[obj({**context,'origin':{'const':'source_specification'},'value':{'anyOf':[ref(legacy),fresh]}}),obj({**context,'origin':{'const':'source_fact_supplements'},'value':ref(legacy)})]}
    witness={**context,'pageid':integer,'revid':integer,'content_sha256':hashed,'line':integer,'line_sha256':hashed,
             'entity_references':array(text),'execution_order':{'const':'UNKNOWN'},'binding_status':{'const':'UNKNOWN'},'classification':{'const':'LEXICAL_REFERENCE_ONLY'}}
    fields={'wiki_title':text,'parent_entry_sha256':hashed,'source_refs':array(ref('source_ref')),
        'objective_entity_references':array(obj(witness)),'source_directive_references':array(obj({**witness,'action_keywords':array(text)})),
        'source_heading_references':array(wrapped(ref('heading'),False)),
        'source_requirement_expressions':array(semantic('prerequisite','prerequisite_expressions')),'source_reward_references':array(semantic('entity','reward_entity_references')),
        'source_reward_groups':array(semantic('reward_group','reward_groups')),'source_reward_quantitative_facts':array(semantic('quantitative_reward','reward_quantitative_facts')),
        'source_fields':{'type':'array'},'known_source_ids':{'type':'array','maxItems':0},'unresolved':{'type':'array'},
        'source_binding_status':{'const':'UNKNOWN'},'full_walkthrough_complete':{'const':False},'definition_complete':{'const':False},'runtime_readiness':{'const':'UNKNOWN'}}
    defs=copy.deepcopy(wiki_schema['$defs'])
    return {'$schema':'https://json-schema.org/draft/2020-12/schema','$defs':defs,**obj({
        'schema':{'const':'OTERYN_UNBOUND_QUEST_SOURCE_FACTS/v1'},'entries':array(obj(fields)),
        'proof_mode':{'const':'RECORDED_LOCAL_PINNED_WITNESSES'},'raw_body_rechecked':{'const':False},
        'definition_complete':{'const':False},'runtime_promotion':{'const':False}})}

def build(specs,selected,authored,receipt,author_schema):
    from jsonschema import Draft202012Validator
    Draft202012Validator(author_schema).validate(authored)
    if receipt['proof_mode']!='RECORDED_LOCAL_PINNED_WITNESSES' or any(receipt[k]!=digest(v) for k,v in
        [('parent_specifications_sha256',specs),('selection_sha256',selected),('authored_sha256',authored),('schema_sha256',author_schema)]):
        raise ValueError('SOURCE input/selection/schema/recorded witness anchor differs')
    entries={e['wiki_title']:e for e in specs['entries']};rows=authored['entries'];titles=[r['wiki_title'] for r in rows]
    if len(entries)!=373 or len(set(titles))!=len(titles) or sorted(titles)!=sorted(selected):raise ValueError('Missing, duplicate or extra selected title')
    for row in rows:
        expected=projection(entries[row['wiki_title']])
        for key in expected:
            if key not in ('objective_entity_references','source_directive_references') and row[key]!=expected[key]:raise ValueError('Historical projection changed: '+key)
        refs={(r['provider'],r['relation'],r['pageid'],r['revid'],r['content_sha256']) for r in row['source_refs']}
        for proof in row['objective_entity_references']+row['source_directive_references']:
            if tuple(proof[k] for k in ('provider','relation','pageid','revid','content_sha256')) not in refs:raise ValueError('Witness is not a selected pinned source')
    if sorted(digest(r) for r in rows)!=receipt['record_digests']:raise ValueError('Recorded authored row membership differs')
    return copy.deepcopy(authored)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--specifications',type=Path,required=True);parser.add_argument('--samples',type=Path,required=True);parser.add_argument('--check',action='store_true')
    args=parser.parse_args();read=lambda name:json.loads((args.samples/name).read_text())
    specs=json.loads(args.specifications.read_text());receipt=read('receipt.json')
    if hashlib.sha256(args.specifications.read_bytes()).hexdigest()!=receipt['parent_specifications_file_sha256']:raise ValueError('Frozen specification bytes changed')
    result=build(specs,read('selection.json'),read('authored-facts.json'),receipt,read('source-facts.schema.json'))
    content=json.dumps(result,ensure_ascii=False,indent=2)+'\n';path=args.samples/'source-facts.json'
    if args.check:
        if not path.is_file() or path.read_text()!=content:raise ValueError('SOURCE facts projection stale')
    else:path.write_text(content)
    print(json.dumps({'titles':len(result['entries']),'raw_body_rechecked':False,'runtime_promotion':False}))
if __name__=='__main__':main()
