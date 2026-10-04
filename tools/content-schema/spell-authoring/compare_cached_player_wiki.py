"""R40 cached-only wiki header comparisons. Never replace source facts or admit execution."""
import argparse
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import re
import tempfile
from urllib.parse import quote
from jsonschema import Draft202012Validator

ROOT=Path(__file__).resolve().parents[3]
FIELDS=('name','words','carrier','vocations','level','mana','soul','premium','cooldown_ms','group_cooldown_ms','secondary_group_cooldown_ms','range')
STATUSES=('AGREE','DIFFERS','WIKI_DISAGREEMENT','SOURCE_FIELD_ABSENT','WIKI_FIELD_MISSING','NO_MATCHING_PAGE','UNCOMPARABLE')
FAMILIES={'druid':'druid','elder_druid':'druid','sorcerer':'sorcerer','master_sorcerer':'sorcerer','knight':'knight','elite_knight':'knight','paladin':'paladin','royal_paladin':'paladin','monk':'monk','exalted_monk':'monk','none':'none'}
WIKI_KEYS={'name':'name','words':'words','level':'levelrequired','mana':'mana','soul':'soul','premium':'premium','cooldown_ms':'cooldown','group_cooldown_ms':'cooldowngroup','secondary_group_cooldown_ms':'cooldowngroup2','range':'spellrange'}
PINS={'canary-main-current':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal-summer-current':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
def sha(data):return hashlib.sha256(data).hexdigest()
def write(path,value):path.write_text(json.dumps(value,sort_keys=True,indent=2,ensure_ascii=False)+'\n')
def key(value):return re.sub(r'\s+',' ',value.strip()).casefold()
def closed(properties):return {'type':'object','additionalProperties':False,'required':list(properties),'properties':properties}
def nullable(value):return {'anyOf':[value,{'type':'null'}]}
def value(kind, data):return {'kind':kind,'value':data}

def schema():
    text={'type':'string'};digest={'type':'string','pattern':'^[0-9a-f]{64}$'};boolean={'type':'boolean'}
    v={'oneOf':[closed({'kind':{'const':'text'},'value':text}),closed({'kind':{'const':'integer'},'value':{'type':'integer','minimum':0}}),closed({'kind':{'const':'boolean'},'value':boolean}),closed({'kind':{'const':'string_set'},'value':{'type':'array','items':text,'uniqueItems':True}}),closed({'kind':{'const':'group_cooldown'},'value':closed({'group':text,'milliseconds':{'type':'integer','minimum':0}})})]}
    fact=closed({'field':{'enum':list(FIELDS)},'raw':nullable(text),'normalized':nullable(v),'unit':{'enum':['text','milliseconds','integer','boolean','base_vocation_families','engine_carrier','group_milliseconds','unqualified_range']},'reason':text})
    page=closed({'schema':{'const':'OTERYN_CACHED_WIKI_HEADER_FACT/v1'},'page_key':digest,'reference':{'enum':['br','fandom','tibiopedia']},'title':text,'url':text,'url_basis':{'enum':['recorded_url','derived_recorded_revision_permalink']},'page_content_sha256':digest,'page_revision_timestamp':nullable(text),'fetched_timestamp':nullable(text),'capture_cut':text,'original_read_method':{'enum':['normal_http','historical_capture_method_not_recorded']},'remote_browser_used':nullable(boolean),'current_read_method':{'const':'local_cache'},'network_fetched_this_run':{'const':False},'cached_input_path':text,'cached_input_sha256':digest,'context':{'enum':['spell_cast','rune_creation','rune_use']},'template':text,'recorded_identity_names':{'type':'array','items':text,'uniqueItems':True},'reference_item_id':nullable({'type':'integer','minimum':1}),'raw_wheel_marker':nullable(text),'level_context_evidence_page_keys':{'type':'array','items':digest,'uniqueItems':True},'facts':{'type':'array','prefixItems':[{'allOf':[fact,{'properties':{'field':{'const':name}}}]} for name in FIELDS],'items':False,'minItems':len(FIELDS),'maxItems':len(FIELDS)}})
    evidence=closed({'page_key':digest,'value':nullable(v),'status':{'enum':list(STATUSES)},'reason':text})
    field=closed({'field':{'enum':list(FIELDS)},'source_value':nullable(v),'source_field_present':boolean,'status':{'enum':list(STATUSES)},'reason':text,'evidence':{'type':'array','items':evidence}})
    comparison=closed({'schema':{'const':'OTERYN_SOURCE_WIKI_HEADER_COMPARISON/v1'},'registration_key':text,'source_revision':{'enum':list(PINS.values())},'source_sha256':digest,'source_header_sha256':digest,'name':text,'carrier':{'enum':['instant','rune','@spell_instant','@spell_rune']},'context':{'enum':['spell_cast','rune_creation','rune_use','disabled_symbolic']},'candidate_status':{'enum':['CANDIDATE_SCHEMA_VALID','BLOCKED']},'reference_coverage':{'type':'array','items':closed({'reference':{'enum':['br','fandom','tibiopedia']},'matching_page_count':{'type':'integer','minimum':0}}),'minItems':3,'maxItems':3},'candidate_overlays':{'type':'array','items':{'enum':['r29','r30','r34']}},'source_rune_item_id':nullable({'type':'integer','minimum':1}),'matching_evidence':{'type':'array','items':closed({'page_key':digest,'basis':{'enum':['direct_recorded_name','exact_instant_words','source_rune_item_id','linked_recorded_wiki_identity']},'identity_anchor_page_keys':{'type':'array','items':digest,'uniqueItems':True}})},'matched_page_keys':{'type':'array','items':digest,'uniqueItems':True},'fields':{'type':'array','prefixItems':[{'allOf':[field,{'properties':{'field':{'const':name}}}]} for name in FIELDS],'items':False,'minItems':len(FIELDS),'maxItems':len(FIELDS)},'runtime_activation':{'const':False},'source_values_overwritten':{'const':False},'source_execution_gaps_remain':{'const':True}})
    supplemental=closed({'schema':{'const':'OTERYN_CACHED_WIKI_SUPPLEMENTAL_HEADER_FACT/v1'},'registration_key':text,'field':{'enum':list(FIELDS)},'source_field_present':{'const':False},'evidence':{'type':'array','items':evidence,'minItems':1},'admission':{'const':'unselected_external_observation'},'source_values_overwritten':{'const':False},'runtime_activation':{'const':False},'source_execution_gaps_remain':{'const':True}})
    return {'$schema':'https://json-schema.org/draft/2020-12/schema','$defs':{'value':v,'page':page,'comparison':comparison,'supplemental':supplemental},'oneOf':[page,comparison,supplemental]}

def integer(raw, milliseconds=False):
    if not re.fullmatch(r'\d+(?:\.\d+)?',raw.strip()):return None
    from decimal import Decimal
    number=Decimal(raw.strip())*(1000 if milliseconds else 1)
    return int(number) if number==int(number) else None

def vocation(raw):
    raw=re.sub(r'\[\[([^]|]+)\|([^]]+)\]\]',lambda m:m[2],raw)
    raw=re.sub(r'\[\[([^]]+)\]\]',lambda m:m[1],raw).casefold().strip().rstrip('.')
    if raw=='jogadores sem vocação':return ['none'],None
    if any(p in raw for p in ['elder','master','elite','royal','exalted']):return None,'promoted-only wiki restriction is not a generic vocation family set'
    tokens=re.split(r'\s*(?:,|\band\b|\be\b)\s*',raw)
    names={'druid':'druid','druids':'druid','sorcerer':'sorcerer','sorcerers':'sorcerer','sorcereres':'sorcerer','knight':'knight','knights':'knight','paladin':'paladin','paladins':'paladin','monk':'monk','monks':'monk'}
    if any(t and t not in names for t in tokens):return None,'unrecognized vocation grammar retained raw'
    return sorted({names[t] for t in tokens if t}),None

def normalize(field, raw, context):
    unit='text';reason=''
    if raw is None:return None,unit,'cached infobox does not state this field'
    if field=='range':return None,'unqualified_range','Infobox range is not qualified as engine selector range rather than attack footprint'
    if field in ['name','words']:return value('text',key(raw)),unit,reason
    if field=='carrier':return value('text','rune' if context=='rune_use' else 'instant'),'engine_carrier','Rune creation infobox describes an instant conjuration, not rune use' if context=='rune_creation' else ''
    if field=='vocations':
        vals,reason=vocation(raw);return value('string_set',vals) if vals is not None else None,'base_vocation_families',reason or 'Compare base family membership only; promotion access/display flags are not inferred'
    if field=='premium':
        normal=raw.strip().casefold();return value('boolean',normal=='yes') if normal in ['yes','no'] else None,'boolean','' if normal in ['yes','no'] else 'Unrecognized Premium value'
    unit='milliseconds' if field in ['cooldown_ms','group_cooldown_ms','secondary_group_cooldown_ms'] else 'integer'
    parsed=integer(raw,unit=='milliseconds')
    return value('integer',parsed) if parsed is not None else None,unit,'' if parsed is not None else 'Not a bare unambiguous numeric value in documented units'

def context(page):
    if page.get('template')=='Infobox Object':return 'rune_use'
    return 'rune_creation' if page.get('fields',{}).get('type','').casefold()=='rune' or page.get('from_rune_page') else 'spell_cast'

def source_context(raw,header):
    if header.get('carrier',header.get('source_carrier_symbol','')).startswith('@'):return 'disabled_symbolic'
    if header['carrier']=='rune':return 'rune_use'
    return 'rune_creation' if raw['cast'].get('conjure',{}).get('reagent_item_id')==3147 else 'spell_cast'

def source_values(header):
    req=header.get('requirements',{});cost=header.get('costs',{})
    result={f:None for f in FIELDS}
    for f in ['name','words','carrier']:
        if f in header:result[f]=value('text',key(header[f]))
    for f, container, name in [('level',req,'level'),('mana',cost,'mana'),('soul',cost,'soul'),('premium',req,'premium'),('cooldown_ms',header,'cooldown_ms'),('range',header.get('targeting',{}),'range_tiles')]:
        if name in container:result[f]=value('boolean' if f=='premium' else 'integer',container[name])
    if 'vocations' in req:
        vocs=req['vocations']
        if all(v in FAMILIES for v in vocs) and all(FAMILIES[v] in vocs for v in vocs):result['vocations']=value('string_set',sorted({FAMILIES[v] for v in vocs}))
    groups=header.get('groups',[])
    if groups and 'cooldown_ms' in groups[0]:result['group_cooldown_ms']=value('integer',groups[0]['cooldown_ms'])
    if len(groups)>1 and 'cooldown_ms' in groups[1]:result['secondary_group_cooldown_ms']=value('group_cooldown',{'group':groups[1]['group'],'milliseconds':groups[1]['cooldown_ms']})
    return result

def load_pages(root, proofs):
    pages=[]
    sources=[('br',root/'tools/content-schema/spell-authoring/samples/wiki-spell-facts-br-2026-09-27.json'),('fandom',root/'tools/content-schema/spell-authoring/samples/wiki-spell-facts-fandom-2026-09-27.json'),('tibiopedia',root/'docs/reference/spells/r22-audit/tibiopedia/tibiopedia-spell-facts-2026-10-02.json')]
    capture=root/'docs/reference/spells/r22-audit/tibiopedia/capture-manifest.json';meta=json.loads(capture.read_text());proofs[capture.relative_to(root).as_posix()]=sha(capture.read_bytes())
    declared={p['url']:p['sha256'] for p in meta['pages']}
    for reference,path in sources:
        raw=path.read_bytes();data=json.loads(raw);relative=path.relative_to(root).as_posix();proofs[relative]=sha(raw)
        for i,page in enumerate(data['pages']):
            if page.get('template') not in ['Infobox Spell','Infobox Object']:continue
            if reference=='tibiopedia' and declared.get(page['url'])!=page['content_sha256']:raise ValueError('Tibiopedia page/capture digest mismatch')
            pages.append(page_fact(page,reference,relative,sha(raw),data['target_cut'],meta['fetched'] if reference=='tibiopedia' else None,i))
    direct=root/'docs/reference/spells/r22-audit/tibiopedia/unmatched-direct-lookup.json';raw=direct.read_bytes();proofs[direct.relative_to(root).as_posix()]=sha(raw)
    for i,lookup in enumerate(json.loads(raw)['lookups']):
        if lookup.get('status')!=200:continue
        for j,page in enumerate(lookup.get('facts',[])):
            p=dict(page,content_sha256=lookup['sha256'])
            pages.append(page_fact(p,'tibiopedia',direct.relative_to(root).as_posix(),sha(raw),'2026-10-02',lookup['requested_at'],f'{i}/{j}'))
    marked=[p for p in pages if p['raw_wheel_marker'] and p['raw_wheel_marker'].casefold() not in ['no','none','false']]
    for page in pages:
        vals={f['field']:f['normalized']['value'] for f in page['facts'] if f['field'] in ['name','words'] and f['normalized']}
        relevant=[]
        for other in marked:
            if other['context']!=page['context']:continue
            ov={f['field']:f['normalized']['value'] for f in other['facts'] if f['field'] in ['name','words'] and f['normalized']}
            if key(other['title'])==key(page['title']) or any(vals.get(f) and vals.get(f)==ov.get(f) for f in ['name','words']):relevant.append(other['page_key'])
        if relevant:
            page['level_context_evidence_page_keys']=relevant
            level=next(f for f in page['facts'] if f['field']=='level')
            if level['raw'] is not None:level['normalized']=None;level['reason']='Cached same-subject infobox marks Wheel involvement; unlock level versus registration minimum is unqualified. Context page keys retained.'
    return pages

def page_fact(page,reference,path,input_sha,cut,fetched,index):
    ctx=context(page);facts=[]
    for field in FIELDS:
        wiki_key='vocrequired' if field=='vocations' and ctx=='rune_use' else 'voc' if field=='vocations' else WIKI_KEYS.get(field)
        raw=ctx if field=='carrier' else page.get('fields',{}).get(wiki_key)
        normalized,unit,reason=normalize(field,raw,ctx)
        if field=='secondary_group_cooldown_ms':
            group=page.get('fields',{}).get('secondarygroup')
            if normalized is not None and group and re.fullmatch('[A-Za-z ]+',group):
                normalized=value('group_cooldown',{'group':key(group),'milliseconds':normalized['value']});unit='group_milliseconds'
            elif raw is not None: normalized=None;unit='group_milliseconds';reason='Secondary cooldown lacks an unambiguous owning group label'
        # Object.words commonly quotes rune-conjuration incantation; never use it as rune activation words.
        if ctx=='rune_use' and field in ['words','mana','soul','premium','group_cooldown_ms','secondary_group_cooldown_ms'] and raw is not None:
            normalized=None;reason='Rune-use object facts cannot establish rune-creation costs/words or group cooldown'
        facts.append({'field':field,'raw':str(raw) if raw is not None else None,'normalized':normalized,'unit':unit,'reason':reason})
    if 'url' in page:url=page['url'];basis='recorded_url'
    else:
        domain='www.tibiawiki.com.br' if reference=='br' else 'tibia.fandom.com'
        url=f'https://{domain}/index.php?title={quote(page["title"].replace(" ","_"),safe="")}&oldid={page["revision_id"]}';basis='derived_recorded_revision_permalink'
    page_key=sha(json.dumps([reference,path,index,page['content_sha256'],page.get('template')],sort_keys=True).encode())
    return {'schema':'OTERYN_CACHED_WIKI_HEADER_FACT/v1','page_key':page_key,'reference':reference,'title':page['title'],'url':url,'url_basis':basis,'page_content_sha256':page['content_sha256'],'page_revision_timestamp':page.get('timestamp'),'fetched_timestamp':fetched,'capture_cut':cut,'original_read_method':'normal_http' if reference=='tibiopedia' else 'historical_capture_method_not_recorded','remote_browser_used':False if reference=='tibiopedia' else None,'current_read_method':'local_cache','network_fetched_this_run':False,'cached_input_path':path,'cached_input_sha256':input_sha,'context':ctx,'template':page['template'],'recorded_identity_names':sorted({key(v) for v in [page['title'],page.get('fields',{}).get('name'),page.get('fields',{}).get('actualname')] if v}),'reference_item_id':integer(str(page.get('fields',{}).get('itemid',''))),'raw_wheel_marker':page.get('fields',{}).get('wheelspell'),'level_context_evidence_page_keys':[],'facts':facts}

def match_pages_and_proofs(pages,header,ctx):
    if ctx=='disabled_symbolic':return [],[]
    result=[];proofs=[];name=key(header.get('name',''));words=key(header.get('words',''))
    item_id=header.get('reference_rune_item_id')
    anchors=[p for p in pages if ctx=='rune_use' and p['context']==ctx and item_id is not None and p['reference_item_id']==item_id]
    aliases=set(n for p in anchors for n in p['recorded_identity_names'])
    for page in pages:
        if page['context']!=ctx:continue
        fields={f['field']:f for f in page['facts']};page_words=fields['words']['normalized']
        if name in page['recorded_identity_names']:basis='direct_recorded_name';evidence=[]
        elif ctx!='rune_use' and words and page_words and page_words['value']==words:basis='exact_instant_words';evidence=[]
        elif page in anchors:basis='source_rune_item_id';evidence=[page['page_key']]
        elif set(page['recorded_identity_names']) & aliases:basis='linked_recorded_wiki_identity';evidence=[p['page_key'] for p in anchors if set(p['recorded_identity_names']) & set(page['recorded_identity_names'])]
        else:continue
        result.append(page);proofs.append({'page_key':page['page_key'],'basis':basis,'identity_anchor_page_keys':evidence})
    return result,proofs

def matching_pages(pages,header,ctx):return match_pages_and_proofs(pages,header,ctx)[0]

def compare_field(field, source_value, pages, source_present=None):
    source_present = source_value is not None if source_present is None else source_present
    evidence=[]
    for page in pages:
        fact=next(f for f in page['facts'] if f['field']==field);normal=fact['normalized']
        if fact['raw'] is None:status='WIKI_FIELD_MISSING'
        elif normal is None:status='UNCOMPARABLE'
        elif source_value is None:status='UNCOMPARABLE' if source_present else 'SOURCE_FIELD_ABSENT'
        else:status='AGREE' if source_value==normal else 'DIFFERS'
        evidence.append({'page_key':page['page_key'],'value':normal,'status':status,'reason':fact['reason']})
    normalized=[e['value'] for e in evidence if e['value'] is not None]
    if not pages:status='NO_MATCHING_PAGE';reason='No context-compatible cached page matches source name or exact instant incantation'
    elif not normalized:status='UNCOMPARABLE' if any(e['status']=='UNCOMPARABLE' for e in evidence) else 'WIKI_FIELD_MISSING';reason='No unambiguous cached field in matching pages'
    elif source_value is None:status='UNCOMPARABLE' if source_present else 'SOURCE_FIELD_ABSENT';reason='Explicit source field cannot be unambiguously normalized' if source_present else 'No explicit registrar field; supplemental cached Wiki value retained separately, no engine default inferred'
    elif any(v!=normalized[0] for v in normalized[1:]):status='WIKI_DISAGREEMENT';reason='Matching cached page contexts state different normalized values; no majority override'
    else:status='AGREE' if normalized[0]==source_value else 'DIFFERS';reason='Unambiguous explicit source header compared with cached Wiki fact'
    return {'field':field,'source_value':source_value,'source_field_present':source_present,'status':status,'reason':reason,'evidence':evidence}

def gzip_rows(path,rows):
    payload=b''.join((json.dumps(row,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode() for row in rows)
    with path.open('wb') as f,gzip.GzipFile(filename='',fileobj=f,mtime=0,mode='wb') as stream:stream.write(payload)
    return {'records':len(rows),'payload_sha256':sha(payload),'gzip_sha256':sha(path.read_bytes())}

def generate(root,out):
    proofs={};pages=load_pages(root,proofs);validator=Draft202012Validator(schema())
    for page in pages:validator.validate(page)
    r28=root/'imports/spells/r28/player-source-bundles';manifest=json.loads((r28/'package-manifest.json').read_text())['files']
    callbacks=r28/'source-callback-facts.jsonl.gz'
    if manifest.get(callbacks.name)!=sha(callbacks.read_bytes()):raise ValueError('sealed source callback digest mismatch')
    proofs[callbacks.relative_to(root).as_posix()]=sha(callbacks.read_bytes())
    overlays={};candidate_count=0
    for rev in ['r28','r29','r30','r34']:
        p=root/f'imports/spells/{rev}/import-manifest.json';raw=p.read_bytes();imported=json.loads(raw);proofs[p.relative_to(root).as_posix()]=sha(raw)
        if rev!='r28':
            for reg in imported['source_keys']:overlays.setdefault(reg,[]).append(rev)
    rows=[];missing=[];supplements=[];counts=Counter();candidate_keys=set()
    records=[json.loads(line) for line in gzip.decompress(callbacks.read_bytes()).splitlines()]
    if len(records)!=483 or len({r['registration_key'] for r in records})!=483:raise ValueError('exact483 current source identities required')
    for record in sorted(records,key=lambda r:r['registration_key']):
        reg=record['registration_key'];snapshot=reg.split('/')[0]
        if record['source_revision']!=PINS[snapshot]:raise ValueError('wrong source donor pin')
        folder=r28/snapshot/sha(reg.encode())[:16];h=folder/'source-header.json';receipt_path=folder/'receipt.json'
        for path in [h,receipt_path]:
            if manifest.get(path.relative_to(r28).as_posix())!=sha(path.read_bytes()):raise ValueError('sealed source header/receipt digest mismatch')
        header=json.loads(h.read_text())['spell'];receipt=json.loads(receipt_path.read_text());ctx=source_context(record['source_callback_facts'],header)
        status='CANDIDATE_SCHEMA_VALID' if reg in overlays else receipt['status']
        if status=='CANDIDATE_SCHEMA_VALID':candidate_keys.add(reg)
        matched,match_proofs=match_pages_and_proofs(pages,header,ctx);sv=source_values(header)
        fields=[compare_field(field,sv[field],matched, True if field=='vocations' and 'vocations' in header.get('requirements',{}) else None) for field in FIELDS]
        row={'schema':'OTERYN_SOURCE_WIKI_HEADER_COMPARISON/v1','registration_key':reg,'source_revision':record['source_revision'],'source_sha256':record['source_sha256'],'source_header_sha256':sha(h.read_bytes()),'name':header['name'],'carrier':header.get('carrier',header.get('source_carrier_symbol')),'context':ctx,'candidate_status':status,'reference_coverage':[{'reference':ref,'matching_page_count':sum(p['reference']==ref for p in matched)} for ref in ['br','fandom','tibiopedia']],'candidate_overlays':overlays.get(reg,[]),'source_rune_item_id':header.get('reference_rune_item_id'),'matching_evidence':match_proofs,'matched_page_keys':[p['page_key'] for p in matched],'fields':fields,'runtime_activation':False,'source_values_overwritten':False,'source_execution_gaps_remain':True}
        validator.validate(row);rows.append(row)
        for field in fields:
            counts[field['status']]+=1
            if field['status']=='SOURCE_FIELD_ABSENT':
                extra={'schema':'OTERYN_CACHED_WIKI_SUPPLEMENTAL_HEADER_FACT/v1','registration_key':reg,'field':field['field'],'source_field_present':False,'evidence':[e for e in field['evidence'] if e['value'] is not None],'admission':'unselected_external_observation','source_values_overwritten':False,'runtime_activation':False,'source_execution_gaps_remain':True}
                validator.validate(extra);supplements.append(extra)
        for reference in ['br','fandom','tibiopedia']:
            selected_pages=[p for p in matched if p['reference']==reference]
            for field in fields:
                result=compare_field(field['field'],field['source_value'],selected_pages,field['source_field_present'])
                if result['status'] not in ['AGREE','SOURCE_FIELD_ABSENT']:
                    missing.append({'registration_key':reg,'name':header['name'],'context':ctx,'reference':reference,'field':field['field'],'status':result['status'],'reason':result['reason'],'cached_page_urls':sorted({p['url'] for p in selected_pages})})
    if len(candidate_keys)!=308:raise ValueError('source candidate population changed: '+str(len(candidate_keys)))
    write(out/'cached-player-wiki.schema.json',schema())
    a=gzip_rows(out/'cached-wiki-page-facts.jsonl.gz',pages);b=gzip_rows(out/'source-wiki-header-comparisons.jsonl.gz',rows);supp=gzip_rows(out/'supplemental-header-facts.jsonl.gz',supplements)
    gap_schema={'$schema':'https://json-schema.org/draft/2020-12/schema', **closed({'schema':{'const':'OTERYN_CACHED_WIKI_RESEARCH_GAPS/v1'}, 'records':{'type':'array','items':closed({'registration_key':{'type':'string'},'name':{'type':'string'},'context':{'enum':['spell_cast','rune_creation','rune_use','disabled_symbolic']},'reference':{'enum':['br','fandom','tibiopedia']},'field':{'enum':list(FIELDS)},'status':{'enum':list(STATUSES)},'reason':{'type':'string'},'cached_page_urls':{'type':'array','items':{'type':'string'},'uniqueItems':True}})},'network_fetched_this_run':{'const':False},'source_values_overwritten':{'const':False}})}
    write(out/'targeted-research-gaps.schema.json',gap_schema)
    gaps={'schema':'OTERYN_CACHED_WIKI_RESEARCH_GAPS/v1','records':missing,'network_fetched_this_run':False,'source_values_overwritten':False}
    Draft202012Validator(gap_schema).validate(gaps);write(out/'targeted-research-gaps.json',gaps)
    summary={'schema':'OTERYN_CACHED_SOURCE_WIKI_COVERAGE/v1','source_records':483,'candidate_records_unchanged':308,'wiki_page_fact_records':len(pages),'field_comparisons':len(rows)*len(FIELDS),'status_counts':dict(sorted(counts.items())),'page_records_by_reference':dict(Counter(p['reference'] for p in pages)),'source_records_with_context_compatible_page':sum(bool(r['matched_page_keys']) for r in rows),'source_records_without_context_compatible_page':sum(not r['matched_page_keys'] for r in rows),'supplemental_source_absent_fields':counts['SOURCE_FIELD_ABSENT'],'input_proofs':dict(sorted(proofs.items())),'outputs':{'pages':a,'comparisons':b,'supplemental_facts':supp},'producer_sha256':sha(Path(__file__).read_bytes()),'network_fetched_this_run':False,'runtime_activation':False,'source_values_overwritten':False,'source_execution_gaps_remain':True,'limitations':['Wiki capture cut/revision time is distinct from fetch timestamp; unknown original BR/Fandom method remains unknown.','Base vocation family comparison does not establish promoted-access/display equivalence.','Range facts retained raw but cannot establish engine selector distance.','No Wiki fact repairs execution/formula/condition mechanics or changes source/header/candidate/native authority.']}
    write(out/'coverage.json',summary)
    write(out/'package-manifest.json',{'schema':'OTERYN_CACHED_WIKI_PACKAGE/v1','files':{p.name:sha(p.read_bytes()) for p in sorted(out.iterdir()) if p.is_file()}})
    return summary

def run(root,out):
    if out.exists():raise ValueError('output must be new; existing packages immutable')
    out.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.r40-cached-wiki-',dir=out.parent) as tmp:
        stage=Path(tmp)/'packet';stage.mkdir();result=generate(root,stage);stage.rename(out)
    return result
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=ROOT);parser.add_argument('--out',type=Path,required=True);args=parser.parse_args();print(json.dumps(run(args.root,args.out),sort_keys=True))
