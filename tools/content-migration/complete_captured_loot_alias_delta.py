"""Bounded, source-corroborated item-alias delta; frozen prior packets stay immutable."""
import argparse
from collections import Counter,defaultdict
from decimal import Decimal,InvalidOperation
import json
from pathlib import Path
import re
from complete_loot_quality_fields import read,sha,norm,item_index,source,verify_page,csv_rows
from complete_remaining_loot_fields import BASE_SHA,numeric_observation

CANDIDATES={
'wereboar tusk':'Wereboar Tusks','werewolf fang':'Werewolf Fangs','rod (bony sea devil)':'Rod (Creature Product)',
'hand (brachiodemon)':'Hand','secret instruction (gryphon mask)':'Secret Instruction','secret instruction (mirror mask)':'Secret Instruction',
'secret instruction (silver mask)':'Secret Instruction','crown (cloak of terror)':'Crown (Plant)',
'darklight core (item)':'Darklight Core (Object)','darklight matter (item)':'Darklight Matter (Object)',
'cookbook (book world)':'Cookbook (Creature Product)',"ferumbras' staff (club)":"Ferumbras' Staff (Blunt)",
'fish tail (trash)':'Fish Tail','spirit container (fighting spirit)':'Spirit Container','part of a rune (primeira)':'Part of a Rune (One)',
'part of a rune (segunda)':'Part of a Rune (Two)','part of a rune (terceira)':'Part of a Rune (Three)',
'part of a rune (quarta)':'Part of a Rune (Four)','part of a rune (quinta)':'Part of a Rune (Five)',
'part of a rune (sexta)':'Part of a Rune (Six)','the cube (item)':'The Cube','dead snake':'Dead Snake',
'candy floss (25 anos)':'Candy Floss (Large)','sabretooth (item)':'Sabretooth','grappling hook (rascacoon)':'Grappling Hook',
'parchment (grey)':'Parchment (Rewritable)','insectoid egg (food)':'Insectoid Egg','signet ring (valioso)':'Signet Ring',
'dark bell (sino)':'Dark Bell (Silver)','flamingo feather (asura)':'Flamingo Feather','watermelon tourmaline (fatia)':'Watermelon Tourmaline',
'icicle (the percht queen)':'Icicle (Percht)','jewel case (azul)':'Jewel Case'}

def exact_fields(page):
    """Parse each physical field line; empty values must not swallow the next key."""
    result=dict(page)
    result['fields']={m[1]:m[2].strip() for m in re.finditer(r'^\|\s*([A-Za-z0-9_]+)\s*=([^\n]*)',page.get('content',''),re.M)}
    return result

def number(value):
    try:return Decimal(re.sub(r'[^\d.]','',str(value)))
    except InvalidOperation:return None

def actual_titles(page):
    if not page:return set()
    return {norm(m[1]) for m in re.finditer(r'^\|\s*([^\n|]+?),\s*times:',page['content'],re.M)}

def corroborated(br,en,actor,stats,title):
    """No bare-name shortcut: require a real actor/statistics link and identity traits."""
    bf,ef=br.get('fields',{}),en.get('fields',{})
    weight=number(bf.get('weight'));other=number(ef.get('weight'))
    if weight is None or other is None or weight!=other:return False,'WEIGHT_IDENTITY_NOT_CORROBORATED'
    drops=' '.join(str(bf.get(f,'')) for f in ('droppedby','droppedRaidby','droppedEventby'))
    br_drops={norm(m[1]) for m in re.finditer(r'\[\[([^]|]+)(?:\|[^]]*)?\]\]',drops)}
    if norm(actor) not in br_drops:return False,'BR_ACTOR_DROP_APPLICABILITY_NOT_EXPLICIT'
    if norm(title) not in actual_titles(stats):return False,'NO_ACTUAL_ACTOR_STATISTICS_ITEM_LINK'
    # A disambiguation page listing many IDs cannot prove which quest document drops.
    return True,'SAME_ACTOR_EXPLICIT_DROPS_AND_CAPTURED_STATISTICS_LINK_AND_EQUAL_WEIGHT'

def build(baseline,items,out):
    baseline,items,out=map(Path,(baseline,items,out))
    if out.resolve()==baseline.resolve() or baseline.resolve() in out.resolve().parents:raise ValueError('cannot modify baseline')
    if sha(baseline/'population-index.json')!=BASE_SHA:raise ValueError('wrong baseline')
    priorfile=Path('/workspace/monster-field-next-20261002/loot-final-v5/field-patches.json');prior=read(priorfile)
    brpacket=read('/workspace/monster-field-fill-20261002/wiki/item-pages.json.gz')
    brpacket['pages']=[exact_fields(p) for p in brpacket['pages']]
    britems=item_index(brpacket)
    enpacket=read(items)
    enpacket['pages']=[exact_fields(p) for p in enpacket['pages']]
    enitems=item_index(enpacket)
    actorrows={r['monster']:r for r in csv_rows('/workspace/monster-field-audit-20261002/loot/fresh-br-itemsets.csv.gz')}
    actorpages={p['content_sha256']:p for p in read('/workspace/monster-field-audit-20261002/wiki/pages.json.gz')['pages']}
    enstats={norm(p['page_title'].split(':',1)[-1]):p for p in read('/workspace/monster-field-audit-20261002/wiki/en-pages.json.gz')['pages']}
    admitted={r['source_item_id'] for r in read('/workspace/monster-round7-output/native-item-map-rust.json')['records']}
    done,remaining,flags=[],[],defaultdict(set)
    for row in prior['unresolved']:
        n,t=row['monster'],row['wiki_item']
        if t=='zushuka (mortal)' and n=='zushuka':
            loot=json.loads(actorrows[n]['loot_fields_raw']).get('loot','')
            linked=britems.get(norm(t))
            if 'Disponível apenas na sua fase' in loot and linked and 'hp' in linked.get('fields',{}):
                done.append(dict(row,status='BOSS_PHASE_APPLICABILITY_LINK_NOT_A_LOOT_ITEM',proof={'actor_loot_source':source(actorpages[actorrows[n]['wiki_sha256']],'loot'),'linked_creature_source':source(linked,'hp'),'raw_loot_value':loot}))
                flags[n].add('WIKI_LOOT_PHASE_RESTRICTED_APPLICABILITY')
                continue
        if row['status']!='ITEM_ID_NOT_PROVEN' or t not in CANDIDATES:
            remaining.append(row);continue
        actor=read(baseline/'bundles'/n/'monster.json')
        br=britems.get(norm(t));en=enitems.get(norm(CANDIDATES[t]));stats=enstats.get(norm(actor['creature']['display_name']))
        if not br or not en:
            remaining.append(dict(row,alias_sweep_status='EXACT_CAPTURED_ITEM_PAGE_MISSING'));continue
        verify_page(br);verify_page(en)
        ok,status=corroborated(br,en,actor['creature']['display_name'],stats,CANDIDATES[t])
        if not ok:
            remaining.append(dict(row,alias_sweep_status=status));continue
        declared={int(v) for v in re.findall(r'\d+',en.get('fields',{}).get('itemid',''))}
        ids=declared & admitted
        if len(declared)!=1:
            remaining.append(dict(row,alias_sweep_status='CAPTURED_ITEM_PAGE_CONTAINS_MULTIPLE_VARIANT_IDS',declared_item_ids=sorted(declared)));continue
        existing={int(e['item']['key'].split('/')[-1]) for e in actor.get('loot',{}).get('entries',[])}
        for p in prior['patches']:
            if p['monster']!=n:continue
            if p['pointer']=='/loot/entries/-':existing.add(int(p['value']['item']['key'].split('/')[-1]))
            if p['pointer']=='/loot':existing.update(int(e['item']['key'].split('/')[-1]) for e in p['value']['entries'])
        selected=ids & existing
        if not selected:
            remaining.append(dict(row,alias_sweep_status='CONFIRMED_ITEM_NOT_PRESENT_REQUIRES_SEPARATE_LOOT_FILL',declared_item_ids=sorted(declared)));continue
        dropfield=next(f for f in ('droppedby','droppedRaidby','droppedEventby') if any(norm(m[1])==norm(actor['creature']['display_name']) for m in re.finditer(r'\[\[([^]|]+)(?:\|[^]]*)?\]\]',str(br['fields'].get(f,'')))))
        statproof=source(stats,'Loot2','https://tibia.fandom.com/api.php')
        statproof['source_field']='Loot2.'+CANDIDATES[t]+'.item_name'
        statproof['source_line']=next(i for i,line in enumerate(stats['content'].splitlines(),1) if re.match(r'^\|\s*'+re.escape(CANDIDATES[t])+r',\s*times:',line))
        proof=dict(item_identity_source=source(en,'itemid','https://tibia.fandom.com/api.php'),br_identity_source=source(br,dropfield),
                   numeric_statistics_source=statproof,identity_qualification='DERIVED_CROSS_WIKI_CORROBORATED',
                   correlation=status,matched_weight=str(number(br['fields']['weight'])),existing_item_ids=sorted(selected))
        done.append(dict(row,status='SOURCE_CORROBORATED_ALIAS_ALREADY_PRESENT',canonical_wiki_item=en['page_title'],proof=proof))
        flags[n].add('WIKI_LOOT_ITEM_ALIAS_RESOLVED')
    outstanding={r['monster'] for r in remaining}
    idx={r['monster']:r for r in read(baseline/'population-index.json')['monsters']}
    resolvedflags={n:sorted({'WIKI_LOOT_ITEM_PRESENCE_UNVERIFIED','WIKI_LOOT_ITEM_REFERENCE_AMBIGUOUS'} & set(idx[n].get('completion_flags',[]))) for n in flags if n not in outstanding}
    resolvedflags={n:v for n,v in resolvedflags.items() if v}
    counts=dict(examined_unknown_item_comparisons=sum(r['status']=='ITEM_ID_NOT_PROVEN' for r in prior['unresolved']),resolved_aliases=sum(r['status']=='SOURCE_CORROBORATED_ALIAS_ALREADY_PRESENT' for r in done),resolved_comparisons=len(done),phase_context_links=sum(r['status']=='BOSS_PHASE_APPLICABILITY_LINK_NOT_A_LOOT_ITEM' for r in done),
                actors=len(flags),remaining_comparisons=len(remaining),remaining_status_counts=dict(Counter(r['status'] for r in remaining)),retired_unresolved_flag_actors=len(resolvedflags),data_patches=0)
    packet=dict(schema='OTERYN_MONSTER_FIELD_PATCH_PACKET/v1',lane='captured-loot-alias-delta',baseline_index_sha256=BASE_SHA,
                previous_loot_packet_sha256=sha(priorfile),item_capture_sha256=sha(items),patches=[],actor_flags={n:sorted(v) for n,v in flags.items()},
                resolved_actor_flags=resolvedflags,resolved_comparisons=done,unresolved=remaining,counts=counts,runtime_qualification=False)
    out.mkdir(parents=True,exist_ok=False);(out/'field-patches.json').write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n');(out/'summary.json').write_text(json.dumps(counts,indent=2)+'\n')
    return packet

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    for field in ('baseline','items','out'):p.add_argument('--'+field,type=Path,required=True)
    a=p.parse_args();print(json.dumps(build(a.baseline,a.items,a.out)['counts']))
