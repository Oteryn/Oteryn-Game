"""Compare the first Canary sample with latest TibiaWiki BR structured facts.

Only evidence is emitted: no imported gameplay values are overwritten. No historical
date is selected. Each page is bound to the revision returned by the current-source API.
Narrative prose and artwork are excluded from the retained record.
"""
import argparse
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent
API = 'https://www.tibiawiki.com.br/api.php'
PAGES = {'rat': 'Rat', 'cyclops': 'Cyclops', 'orc_spearman': 'Orc Spearman', 'scorpion': 'Scorpion',
         'orc_shaman': 'Orc Shaman', 'necromancer': 'Necromancer', 'fire_elemental': 'Fire Elemental',
         'dragon': 'Dragon', 'ghost': 'Ghost', 'demodras': 'Demodras'}
DAMAGE = {'physicalDmgMod': 'physical', 'earthDmgMod': 'earth', 'fireDmgMod': 'fire',
          'deathDmgMod': 'death', 'energyDmgMod': 'energy', 'holyDmgMod': 'holy', 'iceDmgMod': 'ice'}
SCALARS = {'hp', 'exp', 'speed', 'defense', 'mitigation', 'summon', 'convince', 'charm',
           'pushable', 'pushobjects', 'illusionable', 'respawnblocked', 'ocorrencia', 'dificuldade'}


def infobox_fields(text):
    start = text.index('{{Infobox_Criatura')
    fields, part, braces, brackets = {}, [], 0, 0
    def save():
        value = ''.join(part)
        if '=' in value:
            key, value = value.split('=', 1)
            fields[key.strip()] = value.strip()
    for char in text[start:]:
        if char == '{': braces += 1
        elif char == '}': braces -= 1
        elif char == '[': brackets += 1
        elif char == ']': brackets -= 1
        if char == '|' and braces == 2 and brackets == 0:
            save(); part = []
        elif braces == 0:
            # The closing delimiter belongs to the template, not its last value.
            if part and part[-1] == '}': part.pop()
            save(); break
        else:
            part.append(char)
    return fields


def numeric(value):
    value = value.strip().removesuffix('%').strip()
    if not re.fullmatch(r'-?\d+(?:\.\d+)?', value): return None
    return Decimal(value)


def boolean(value):
    return {'sim': True, 'não': False, 'nao': False}.get(value.casefold())


def fact(value):
    number = numeric(value)
    if number is not None:
        return int(number) if number == number.to_integral_value() else str(number)
    truth = boolean(value)
    if truth is not None: return truth
    return value.strip()


def link_names(value):
    return sorted({match.split('|', 1)[0].strip() for match in re.findall(r'\[\[([^\]]+)\]\]', value)})


def compare(monster, wiki, names):
    c, b = monster['creature'], monster['behavior']
    rows = []
    def row(field, source, observed, comparable=True, note=None):
        status = 'UNKNOWN' if observed is None or source is None else ('NOT_COMPARABLE' if not comparable else
                 ('MATCH' if source == observed else 'CONFLICT'))
        rows.append({'field': field, 'canary': source, 'wiki': observed, 'classification': status,
                     **({'note': note} if note else {})})
    # TibiaWiki BR's `defense` parameter is rendered as Armadura (armor), not engine defense.
    for field, key in [('max_health', 'hp'), ('experience', 'exp'), ('armor', 'defense')]:
        row(field, c['stats'][field], numeric(wiki.get(key, '')))
    mitigation = c['stats'].get('mitigation_percent')
    row('mitigation_percent', Decimal(mitigation['numerator']) / mitigation['denominator'] if mitigation else None,
        numeric(wiki.get('mitigation', '')), comparable=mitigation is not None)
    # Wiki speed is not silently equated to the engine's raw speed unit.
    row('speed', c['stats']['speed'], numeric(wiki.get('speed', '')), False,
        'Different source speed representations; no Global/runtime unit equivalence is qualified here.')
    for field, key, source in [('pushable','pushable',b['movement']['pushable']),
            ('push_items','pushobjects',b['movement']['push_items']),
            ('illusionable','illusionable',c['flags']['illusionable']),
            ('blocked_by_nearby_players','respawnblocked',c['spawn_eligibility']['blocked_by_nearby_players'])]:
        row(field, source, boolean(wiki.get(key, '')))
    for field, key in [('summonable','summon'), ('convinceable','convince')]:
        cost = numeric(wiki.get(key, ''))
        observed = (cost > 0) if cost is not None else boolean(wiki.get(key, ''))
        row(field, c['summoning'][field], observed)
        if c['summoning'][field] and cost is not None:
            row(field+'_mana_cost', c['summoning'].get('mana_cost'), cost)
    if c.get('bestiary'):
        row('charm_points', c['bestiary']['charm_points'], numeric(wiki.get('charm', '')))
        difficulty = {'inofensivo':'harmless','trivial':'trivial','fácil':'easy','medio':'medium','médio':'medium',
                      'difícil':'hard','desafiador':'challenging'}.get(wiki.get('dificuldade','').casefold())
        occurrence = {'comum':'common','incomum':'uncommon','raro':'rare','muito raro':'very_rare'}.get(wiki.get('ocorrencia','').casefold())
        row('bestiary_difficulty', c['bestiary']['difficulty'], difficulty)
        row('bestiary_occurrence', c['bestiary']['occurrence'], occurrence)
    reductions = {v['damage_type']: Decimal(v['reduction_percent']['numerator']) / v['reduction_percent']['denominator']
                  for v in c['resistances']}
    for key, damage in DAMAGE.items():
        mod = numeric(wiki.get(key, ''))
        row('damage_received_percent/'+damage, 100-reductions.get(damage, 0), mod,
            note='Wiki damage modifier is received damage; Canary element percent is reduction. Compare 100 - reduction.')
    loot_keys = ['loot'] + [f'loot{suffix}' for suffix in ('comum','incomum','semiraro','raro','muitoraro')]
    wiki_items = sorted({name for key in loot_keys for name in link_names(wiki.get(key,''))})
    source_items = sorted({names.get(int(e['item']['key'].rsplit('/',1)[1]), e['item']['key'])
                           for e in monster.get('loot',{}).get('entries',[])})
    left, right = {s.casefold() for s in source_items}, {s.casefold() for s in wiki_items}
    row('base_loot_item_names', source_items, wiki_items, False,
        'Wiki names/frequency bands and Canary exact probabilities have different precision. '
        'Name set differences are evidence candidates, not automatic gameplay edits. Raid/event loot is excluded.')
    rows[-1]['only_canary'] = [s for s in source_items if s.casefold() not in right]
    rows[-1]['only_wiki'] = [s for s in wiki_items if s.casefold() not in left]
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', required=True, type=Path, help='Local raw API response, not committed')
    parser.add_argument('--refresh', action='store_true')
    parser.add_argument('--out', type=Path, default=ROOT/'samples'/'canary-47dfd51f'/'wiki-current-comparison.json')
    args = parser.parse_args()
    if args.refresh:
        params={'action':'query','prop':'revisions','titles':'|'.join(PAGES.values()),'rvprop':'ids|timestamp|content',
                'rvslots':'main','redirects':'1','format':'json','formatversion':'2','curtimestamp':'1','maxage':'0','smaxage':'0'}
        request=urllib.request.Request(API+'?'+urllib.parse.urlencode(params),
                  headers={'User-Agent':'OterynEvidenceCollector/0.1 (+https://github.com/Oteryn/Oteryn-Game)'})
        with urllib.request.urlopen(request,timeout=40) as response: data=response.read(4*1024*1024+1)
        if len(data)>4*1024*1024: raise ValueError('API response exceeds 4 MiB')
        cache={'retrieved_at':datetime.now(timezone.utc).isoformat(),'request_mode':'latest',
               'api_response':json.loads(data)}
        args.cache.parent.mkdir(parents=True,exist_ok=True)
        args.cache.write_text(json.dumps(cache,ensure_ascii=False,indent=2),encoding='utf-8')
    cache=json.loads(args.cache.read_text(encoding='utf-8'))
    if cache.get('request_mode') != 'latest':
        raise ValueError('Historical or unqualified cache is not a current-source observation; use --refresh')
    data=cache['api_response']
    if data.get('error'): raise ValueError(data['error'])
    pages={p['title']:p for p in data['query']['pages']}
    sources=json.loads((ROOT/'samples'/'canary-47dfd51f'/'sources.json').read_text(encoding='utf-8'))
    names={}
    # Item names are independently recorded by the first batch's manifest resolution.
    for folder in PAGES:
        manifest=json.loads((ROOT/'samples'/'canary-47dfd51f'/folder/'manifest.json').read_text(encoding='utf-8'))
        for entry in manifest['entries']:
            match=re.search(r'Item (\d+) "([^"]+)"',entry.get('resolution',''))
            if match: names[int(match[1])]=match[2]
    records=[]
    for slug,title in PAGES.items():
        page=pages[title]
        if page.get('missing'): raise ValueError('Wiki page missing: '+title)
        revision=page['revisions'][0]; content=revision['slots']['main']['content']
        fields=infobox_fields(content)
        retained={key:fact(value) for key,value in fields.items() if key in SCALARS or key in DAMAGE}
        abilities={key:{'references':link_names(value),
                       'numeric_ranges':[[int(a),int(b)] for a,b in re.findall(r'\b(\d+)\s*[-–]\s*(\d+)\b',value)]}
                   for key,value in fields.items() if (key=='abilities' or key.startswith('hab_')) and value.strip()}
        monster=json.loads((ROOT/'samples'/'canary-47dfd51f'/slug/'monster.json').read_text(encoding='utf-8'))
        rows=compare(monster,fields,names)
        records.append({'monster':slug,'page_title':title,'page_id':page['pageid'],'revision_id':revision['revid'],
            'revision_timestamp':revision['timestamp'],'url':'https://www.tibiawiki.com.br/index.php?'+
              urllib.parse.urlencode({'title':title,'oldid':revision['revid']}),
            'wikitext_sha256':hashlib.sha256(content.encode()).hexdigest(),'structured_facts':retained,
            'ability_observations':abilities,'ability_comparison':'UNKNOWN: wiki ranges and names do not qualify source formulas, intervals or cast geometry.',
            'comparisons':rows})
    counts={key:sum(row['classification']==key for record in records for row in record['comparisons'])
            for key in ('MATCH','CONFLICT','UNKNOWN','NOT_COMPARABLE')}
    result={'schema':'OTERYN_MONSTER_CURRENT_WIKI_COMPARISON/v1','source_role':'STRUCTURED_REFERENCE_DATA',
            'request_mode':'latest','historical_target_date':None,'retrieved_at':cache['retrieved_at'],
            'api_server_timestamp':data.get('curtimestamp'),'api':API,'canary_revision':sources['revision'],
            'runtime_qualified':False,'global_parity_proven':False,'summary':counts,'records':records}
    args.out.write_text(json.dumps(result,ensure_ascii=False,indent=2,default=str)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps({'pages':len(records),**counts}))


if __name__=='__main__': main()
