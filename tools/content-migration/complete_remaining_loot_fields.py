"""Resolve remaining loot presence without guessing ItemIDs or mutating inputs."""
from collections import Counter, defaultdict
from pathlib import Path
import argparse
import hashlib
import json
import re
import sys
import shutil
from decimal import Decimal
from fractions import Fraction
from complete_loot_quality_fields import half_even
from complete_loot_quality_fields import read, sha, csv_rows, norm, source, item_index, item_table, BUCKETS

BASE_SHA = '1b87ba6bee339bbd0e485cecaf7d8862607a8bf473f8f5f44221142b79afa1c5'

def parsed_observations(fields):
    for field, text in fields.items():
        if field not in BUCKETS and field != 'loot':
            continue
        for match in re.finditer(r'\[\[([^\]|#]+)(?:\|[^\]]+)?\]\]', text):
            after = text[match.end():].split(',', 1)[0]
            before = text[:match.start()].split(',')[-1]
            conditional = bool(re.search(r'primeira vez|first time|apenas|somente|quest|miss[aã]o', after, re.I))
            count = re.search(r'(\d+)\s*[-–]\s*(\d+)\s*$', before)
            single = re.search(r'(\d+)\s*$', before)
            quantity = (max(1, int(count[1])), int(count[2])) if count else (int(single[1]), int(single[1])) if single else (1, 1)
            yield norm(match[1]), field, quantity, conditional, bool(count or single)

def stable_variant(ids, attrs):
    """Only an explicit equip/de-equip or decay relationship selects a variant."""
    if len(ids) < 2:
        return next(iter(ids)) if ids else None
    bases = [i for i in ids if 'transformequipto' in attrs.get(i, {})
             and int(attrs[i]['transformequipto']) in ids
             and int(attrs[int(attrs[i]['transformequipto'])].get('transformdeequipto', -1)) == i]
    if len(bases) == 1:
        return bases[0]
    stable = [i for i in ids if not attrs.get(i, {}).get('duration')
              and all(i == j or int(attrs.get(j, {}).get('decayto', -1)) == i for j in ids)]
    return stable[0] if len(stable) == 1 else None

def game_version(text):
    match = re.search(r'(\d+\.\d+)',str(text))
    return Decimal(match[1]) if match else None

def numeric_observation(page, title, implemented):
    candidates = []
    if not page:
        return None, 'NO_CAPTURED_LOOT_STATISTICS_PAGE'
    for block in re.finditer(r'\{\{Loot2\b(.*?)\}\}',page['content'],re.S|re.I):
        text = block[1]
        version = re.search(r'^\|\s*version\s*=([^\n]+)',text,re.M|re.I)
        kills = re.search(r'^\|\s*kills\s*=\s*(\d+)',text,re.M|re.I)
        if not kills or not version or int(kills[1])<=0:continue
        v = game_version(version[1]); introduction = game_version(implemented)
        if v is None or introduction is not None and v<introduction:continue
        for line in text.splitlines():
            match = re.match(r'^\|\s*(.*?),\s*times:\s*(\d+)(.*)',line)
            if not match or norm(re.sub(r'\[\[([^]|]+)(?:\|[^]]*)?\]\]',r'\1',match[1]))!=norm(title):continue
            times=int(match[2]); sample=int(kills[1])
            if times<10 or times>sample:continue
            amount=re.search(r'amount:\s*(\d+)(?:\s*[-–]\s*(\d+))?',match[3])
            quantity=(max(1,int(amount[1])),int(amount[2] or amount[1])) if amount else None
            evidence=source(page,'Loot2', 'https://tibia.fandom.com/api.php')
            evidence.update(source_field='Loot2.'+match[1]+'.times/kills',source_line=page['content'].count('\n',0,block.start())+text.splitlines().index(line)+1,
                            version=version[1].strip(),times=times,kills=sample,global_parity=False,qualification='COMMUNITY_STATISTICAL_ESTIMATE')
            candidates.append((v,sample,dict(ppm=half_even(times*1000000,sample),quantity=quantity,source=evidence)))
    if not candidates:return None,'NO_COMPATIBLE_LOOT2_ITEM_OBSERVATION_WITH_AT_LEAST_10_DROPS'
    return max(candidates,key=lambda r:(r[0],r[1]))[2], 'COMPATIBLE_FRESH_LOOT2_COMMUNITY_ESTIMATE'

def donor_observation(baseline, name, number, by_name):
    manifest=read(baseline/'bundles'/name/'manifest.json')
    checked=[]
    for entry in manifest['entries']:
        relative=entry.get('source_file','')
        origin=manifest['sources'][entry['source_index']]
        repository=origin.get('repository')
        if not relative.endswith('.lua') or repository not in ('opentibiabr/canary','zimbadev/crystalserver'):continue
        checkout=Path('/workspace/monster-reference-sources')/('canary' if repository=='opentibiabr/canary' else 'crystal')
        file=checkout/relative
        if not file.exists() or relative in checked:continue
        checked.append(relative)
        content=file.read_text()
        block=re.search(r'monster\.loot\s*=\s*\{(.*?)\n\}',content,re.S)
        if not block:continue
        for line in block[1].splitlines():
            chance=re.search(r'\bchance\s*=\s*(\d+)',line)
            itemid=re.search(r'\bid\s*=\s*(\d+)',line)
            itemname=re.search(r'\bname\s*=\s*["\']([^"\']+)["\']',line)
            if not chance or not 0<int(chance[1])<=100000:continue
            ids={int(itemid[1])} if itemid else by_name.get(norm(itemname[1]),set()) if itemname else set()
            if ids!={number}:continue
            maximum=re.search(r'\bmaxCount\s*=\s*(\d+)',line)
            proof=dict(repository=repository,revision=origin['revision'],url='https://github.com/'+repository+'/blob/'+origin['revision']+'/'+relative,
                       sha=sha(file),source_file=relative,source_line=content[:block.start()].count('\n')+block[0].splitlines().index(line)+1,
                       source_field='monster.loot.chance',chance=int(chance[1]),denominator=100000,global_parity=False)
            return dict(ppm=int(chance[1])*10,quantity=(1,int(maximum[1])) if maximum else (1,1),source=proof),checked
    return None,checked

def build(baseline, out):
    baseline, out = Path(baseline), Path(out)
    if out.resolve() == baseline.resolve() or baseline.resolve() in out.resolve().parents:
        raise ValueError('output may not modify population')
    if sha(baseline/'population-index.json') != BASE_SHA:
        raise ValueError('baseline differs from guarded population')
    prior = read('/workspace/monster-field-fill-20261002/loot/final-qualified/field-patches.json')
    names, by_name, attrs = item_table(Path('/workspace/monster-reference-sources/canary'))
    mapfile = Path('/workspace/monster-round7-output/native-item-map-rust.json')
    admitted = {r['source_item_id'] for r in read(mapfile)['records']}
    items = item_index(read('/workspace/monster-field-fill-20261002/wiki/item-pages.json.gz'))
    items.update(item_index(read('/workspace/monster-field-fill-20261002/wiki/en-item-pages.json.gz')))
    xmlfile = Path('/workspace/monster-reference-sources/canary/data/items/items.xml')
    xmltext = xmlfile.read_text()
    xmlsha = sha(xmlfile)
    xmlproof = {}
    for line, text in enumerate(xmltext.splitlines(), 1):
        match = re.search(r'<item\b[^>]*\bid="(\d+)"', text)
        if match:
            number = int(match[1])
            xmlproof[number] = dict(source_file='data/items/items.xml',source_line=line,sha=xmlsha,
                                   url='https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml',attributes=attrs.get(number,{}))
    ordinary_new_loot = {'heoni','memory_of_a_wolf','shiversleep','the_keeper','phosphorus'}
    pages = {p['content_sha256']:p for p in read('/workspace/monster-field-audit-20261002/wiki/pages.json.gz')['pages']}
    rows = {r['monster']:r for r in csv_rows('/workspace/monster-field-audit-20261002/loot/fresh-br-itemsets.csv.gz')}
    observations = {n:{t:(f,q,c,explicit) for t,f,q,c,explicit in parsed_observations(json.loads(r['loot_fields_raw']))} for n,r in rows.items()}
    music = {}
    for p in read('/workspace/monster-field-next-20261002/references/music-sheet-qualified-evidence.json'):
        music[norm(p['title'])] = p
    capture = read('/workspace/monster-field-next-20261002/references/music-sheet-first-fourth-pages.json.gz')
    for p in json.loads(capture['text'])['query']['pages']:
        revision = p['revisions'][0]
        content = revision['slots']['main']['content']
        music[norm(p['title'])] = dict(page_title=p['title'],page_id=p['pageid'],revision_id=revision['revid'],content=content,content_sha256=hashlib.sha256(content.encode()).hexdigest(),url='https://tibia.fandom.com/wiki/'+p['title'].replace(' ','_'),fields={'itemid':re.search(r'^\|\s*itemid\s*=\s*([^\n]*)',content,re.M)[1]},method=capture['method'])
    # The ordinal translation is corroborated by the actual verse's flavortext and donor XML.
    translations = {'music sheet (first)':'music sheet (first verse)', 'music sheet (second)':'music sheet (second verse)',
                    'music sheet (third)':'music sheet (third verse)', 'music sheet (fourth)':'music sheet (fourth verse)'}
    ledger = Path('/workspace/monster-field-fill-20261002/loot/final-qualified/loot-rarity-calibration.json')
    calibration = read(ledger)
    enstats = {norm(p['page_title'].split(':',1)[-1]):p for p in read('/workspace/monster-field-audit-20261002/wiki/en-pages.json.gz')['pages']}
    rate_counts=Counter()
    patches, resolved, remaining, flags = [], [], [], defaultdict(set)
    monsters, catalogs = {}, {}
    def add(n,file,pointer,value,evidence,reason):
        patches.append(dict(monster=n,file=file,pointer=pointer,expected_present=False,expected_value=None,value=value,source=json.loads(json.dumps(evidence)),reason=reason))
    for r in prior['unresolved']:
        n,t = r['monster'],r['wiki_item']
        if n not in monsters:
            monsters[n] = read(baseline/'bundles'/n/'monster.json')
            catalogs[n] = read(baseline/'bundles'/n/'catalog.json')
        m = monsters[n]
        existing = {int(e['item']['key'].split('/')[-1]) for e in m.get('loot',{}).get('entries',[])}
        ids = set(r.get('item_ids',[])) & admitted
        evidence = r.get('item_identity_source')
        declared = set()
        title = translations.get(norm(t), norm(t))
        ip = music.get(title) or items.get(title)
        if ip:
            text = ip.get('fields',{}).get('itemid','')
            if not text: text=' '.join(v['text'] for v in ip.get('raw_proof_lines',[]) if 'itemid' in v['text'])
            declared = {int(v) for v in re.findall(r'\d+',text)}
            if declared:
                ids = declared & admitted
                evidence = source(dict(ip,page_title=ip.get('page_title',ip.get('title'))), 'itemid','https://tibia.fandom.com/api.php')
        number = stable_variant(ids,attrs)
        wp = pages.get(rows[n]['wiki_sha256'])
        presence = source(wp, observations.get(n,{}).get(norm(t),('loot',))[0]) if wp else {'url':rows[n]['wiki_url'],'sha':rows[n]['wiki_sha256']}
        if ids & existing:
            resolved.append(dict(r,status='ALIAS_ALREADY_PRESENT',item_ids=sorted(ids & existing),item_identity_source=evidence))
            flags[n].add('WIKI_LOOT_ITEM_ALIAS_RESOLVED')
            continue
        obs = observations.get(n,{}).get(norm(t))
        status = None
        if obs and obs[2]: status='CONDITIONAL_QUEST_OR_FIRST_TIME_DROP_NOT_ORDINARY'
        elif 'loot' not in m and n not in ordinary_new_loot: status='SOURCE_PHASE_SUMMON_OR_NO_ORDINARY_LOOT_CONTEXT_RETAINED'
        elif not ids: status='PROVEN_ITEM_ID_OUTSIDE_PROTECTED_REGISTRY' if declared else 'ITEM_ID_NOT_PROVEN'
        elif number is None: status='COSMETIC_CHARGE_OR_VARIANT_ITEM_ID_AMBIGUITY'
        elif obs is None: status='WIKI_LOOT_PRESENCE_PARSER_UNRESOLVED'
        elif obs[1][0] > obs[1][1]: status='INVALID_WIKI_QUANTITY_RANGE'
        wp = pages.get(rows[n]['wiki_sha256'])
        if status or wp is None:
            remaining.append(dict(r,status=status or 'ACTOR_PAGE_IDENTITY_UNRESOLVED',item_presence_source=presence,item_ids=sorted(ids),declared_wiki_item_ids=sorted(declared),item_identity_source=evidence,
                                  source_context='Creature ordinary Loot selection is absent; variant names do not alone establish independent loot eligibility.' if 'loot' not in m else None))
            continue
        field, quantity, conditional, explicit = obs
        bucket = field if field in BUCKETS else 'lootmuitoraro'
        statpage = enstats.get(norm(m['creature']['display_name']))
        observation, rate_status = numeric_observation(statpage,translations.get(norm(t),t),wp.get('fields',{}).get('implemented',''))
        donor, donor_checked = donor_observation(baseline,n,number,by_name)
        selected = observation or donor
        rate = selected['ppm'] if selected else calibration['buckets'][bucket]['estimated_ppm']
        rate_counts['fresh_loot2' if observation else 'donor' if donor else 'calibrated_fallback'] += 1
        if selected and selected['quantity'] and not explicit:
            quantity=selected['quantity']
            explicit=True
        if rate is None or calibration['buckets'][bucket]['samples'] < 20: raise ValueError('insufficient estimate calibration')
        proof = dict(kind='oteryn_balance_estimate',ledger_sha256=sha(ledger),source_file=ledger.name,source_line=1,
                     source_field='loot-rarity.'+bucket,qualification='OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE',evidence_classification='DERIVED',global_parity=False,
                     owner_acceptance='2026-10-02 owner authorizes practical flagged non-Global monster completion; coordinator accepts established rarity medians.',
                     item_presence_source=source(wp,field),item_identity_source=evidence,
                     numeric_policy='Preserve existing reference rates. Missing documented rarity uses conservative existing very-rare median as Oteryn fallback, not a Wiki rarity claim.' if field=='loot' else 'Documented Wiki rarity; Oteryn calibrated median, not Global drop rate.',
                     variant_policy='Explicit donor equip/de-equip or decay relationship chooses the stable unequipped item.' if len(ids)>1 else 'Exact admitted item identity.',
                     variant_evidence=[xmlproof.get(i) for i in sorted(ids)] if len(ids)>1 else [],
                     item_identity_donor_source=xmlproof.get(number),rate_observation_audit=rate_status,
                     donor_rate_audit={'checked_files':donor_checked,'status':'EXACT_DONOR_CHANCE_FOUND' if donor else 'NO_EXACT_ITEM_ID_DONOR_CHANCE_FOUND'},
                     numeric_statistics_page=source(statpage,'Loot2','https://tibia.fandom.com/api.php') if statpage else None)
        if donor and not observation:
            proof.update(donor['source'])
            proof.pop('kind',None)
            proof['numeric_policy']='Exact current donor item chance; donor-authored reference, no Global parity asserted.'
        if observation:
            proof.update(observation['source'])
            proof['numeric_policy']='Highest compatible game-version Loot2 observation with at least 10 drops; community sample estimate, not Global parity.'
        if 'loot' not in m:
            identity = dict(key='canary:loot/'+n,revision='canary-47dfd51f')
            lootref = dict(family='Loot',**identity)
            table = dict(identity=identity,algorithm='IndependentBernoulli',entries=[])
            add(n,'monster.json','/loot',table.copy(),proof,'Independent exact Wiki actor loot; author standard Loot table using existing contract.')
            add(n,'monster.json','/creature/loot',lootref,proof,'Select the new standard Loot table for this independent actor.')
            if lootref not in catalogs[n]['definitions']:
                add(n,'catalog.json','/definitions/-',lootref,proof,'Declare canonical Loot identity; no Item identity allocation.')
                catalogs[n]['definitions'].append(lootref)
            m['loot'] = dict(table,entries=[])
            flags[n].add('SOURCE_EMPTY_LOOT_COMPLETED_FROM_WIKI')
        ref = dict(family='Item',key='canary:item/'+str(number),revision='canary-47dfd51f')
        entry = dict(item=ref,min_count=quantity[0],max_count=quantity[1],probability_percent=rate/10000,skip_later_same_item_after_success=False)
        add(n,'monster.json','/loot/entries/-',entry,proof,'Source-proven admitted item presence; flagged practical missing-rate estimate; existing loot rates unchanged.')
        m['loot']['entries'].append(entry)
        if ref not in catalogs[n]['definitions']:
            add(n,'catalog.json','/definitions/-',ref,proof,'Declare an already-admitted Item reference; allocate no ID.')
            catalogs[n]['definitions'].append(ref)
        flags[n].update(('WIKI_LOOT_ESTIMATE','OWNER_ACCEPTED_NON_GLOBAL_LOOT_ESTIMATE','GAMEPLAY_UNVERIFIED'))
        if not selected: flags[n].add('LOOT_RARITY_MEDIAN_BALANCE')
        if field=='loot' and not selected: flags[n].add('RARITY_UNDOCUMENTED_NON_GLOBAL_BALANCE')
        if not explicit: flags[n].add('LOOT_QUANTITY_SINGLE_UNIT_ESTIMATE')
        resolved.append(dict(r,status='SOURCE_PROVEN_ITEM_ADDED_FLAGGED_BALANCE',resolved_item_id=number))
    newtables = {p['monster']:p for p in patches if p['pointer']=='/loot'}
    filtered = []
    for patch in patches:
        if patch['pointer']=='/loot/entries/-' and patch['monster'] in newtables:
            tablepatch = newtables[patch['monster']]
            tablepatch['value']['entries'].append(patch['value'])
            tablepatch['source'].setdefault('entry_sources',[]).append(dict(entry_index=len(tablepatch['value']['entries'])-1,source=patch['source'].copy()))
        else:
            filtered.append(patch)
    patches = filtered
    outstanding = {r['monster'] for r in remaining}
    indexrows = {r['monster']:r for r in read(baseline/'population-index.json')['monsters']}
    retired = {n:['WIKI_LOOT_ITEM_REFERENCE_AMBIGUOUS'] for n in flags if n not in outstanding
               and 'WIKI_LOOT_ITEM_REFERENCE_AMBIGUOUS' in indexrows[n].get('completion_flags',[])}
    counts = dict(comparisons_examined=len(prior['unresolved']),resolved_comparisons=len(resolved),loot_entries_added=sum(r['status']=='SOURCE_PROVEN_ITEM_ADDED_FLAGGED_BALANCE' for r in resolved),
                  aliases_resolved=sum(r['status']=='ALIAS_ALREADY_PRESENT' for r in resolved),remaining_comparisons=len(remaining),remaining_status_counts=dict(Counter(r['status'] for r in remaining)),
                  affected_actors=len(flags),new_standard_loot_tables=sum(p['pointer']=='/loot' for p in patches),rate_sources=dict(rate_counts))
    out.mkdir(parents=True,exist_ok=False)
    shutil.copyfile(ledger,out/ledger.name)
    packet = dict(schema='OTERYN_MONSTER_FIELD_PATCH_PACKET/v1',lane='remaining-loot-fields',baseline_index_sha256=BASE_SHA,patches=patches,
                  actor_flags={n:sorted(v) for n,v in flags.items()},resolved_actor_flags=retired,resolved_comparisons=resolved,unresolved=remaining,counts=counts,
                  calibration_sha256=sha(ledger),item_map_sha256=sha(mapfile),runtime_qualification=False)
    (out/'field-patches.json').write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n')
    (out/'summary.json').write_text(json.dumps(counts,indent=2)+'\n')
    return packet

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--baseline',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    a=p.parse_args(); print(json.dumps(build(a.baseline,a.out)['counts']))
