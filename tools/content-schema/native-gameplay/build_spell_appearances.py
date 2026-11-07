#!/usr/bin/env python3
"""Author native spell Outfit definitions from verified pinned source blobs; no Lua is run."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
PIN = '99902524e052f37574194466c2949c576e4ab269'
HEADER = 'src/creatures/creatures_definitions.hpp'
DEFAULT_SHA = 'dd403b4c2291b66bd5df49d1a59b45667698d8a916ecfb8fa87801688e1ca622'
FIELDS = ['lookAddons','lookBody','lookFeet','lookHead','lookLegs','lookMount','lookType','lookTypeEx']

def git(root, *args):
    return subprocess.check_output(['git','-C',str(root),*args])

CRYSTAL_PIN = 'ff7ede593c69d4c658b382c97443e8155926924a'
CRYSTAL_FORMAL = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'
STAG_PATH = 'data-global/monster/winter_update_2025/stag.lua'
CRYSTAL_DEFAULT_SHA = '8fbc70bf2416b6e76f17cce3dc1b075f8a5451f0f85b522b0a4ae34cf2cccf94'

def pinned(root, path, revision=PIN):
    raw = git(root, 'show', f'{revision}:{path}')
    blob = git(root, 'rev-parse', f'{revision}:{path}').decode().strip()
    actual = hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest()
    if blob != actual:
        raise ValueError(f'git blob substitution: {path}')
    return raw, blob, hashlib.sha256(raw).hexdigest()

def outfit(raw, avatar=False):
    text = re.sub(r'--[^\n]*', '', raw.decode('utf-8'))
    marker = r'condition:setOutfit\s*\(\s*\{' if avatar else r'monster\.outfit\s*=\s*\{'
    starts = list(re.finditer(marker,text))
    if len(starts) != 1:
        raise ValueError('outfit source is not one static literal')
    end = text.find('}',starts[0].end())
    if end < 0:
        raise ValueError('unterminated outfit literal')
    body = text[starts[0].end():end]
    values = {}
    for field in body.split(','):
        if not field.strip():
            continue
        match = re.fullmatch(r'\s*(\w+)\s*=\s*(\d+)\s*',field)
        if not match or match[1] not in FIELDS or match[1] in values:
            raise ValueError('unsupported/dynamic/duplicate outfit field')
        values[match[1]] = int(match[2])
    return values

def record(root,path,creature=None,avatar=False,crystal_stag=False):
    revision = CRYSTAL_PIN if crystal_stag else PIN
    raw,blob,sha = pinned(root,path,revision)
    values = outfit(raw,avatar)
    look = values.get('lookType',0)
    if look <= 0 or look > 65535 or values.get('lookTypeEx',0) or values.get('lookMount',0):
        raise ValueError(f'non-Outfit/dynamic/mounted appearance requires another explicit mapping: {path}')
    colours = [values.get(k,0) for k in ['lookHead','lookBody','lookLegs','lookFeet']]
    if max(colours+[values.get('lookAddons',0)]) > 255:
        raise ValueError('Outfit_t byte overflow')
    result = {'outfit_key':f'oteryn:outfit.source.{"crystal" if crystal_stag else "canary"}.look{look}',
              'outfit_revision':'source-outfit-r1','look_type':look,'colours':colours,
              'addons':values.get('lookAddons',0),'mount_key':None,'creature':creature,
              'source':{'revision':revision,'path':path,'git_blob':blob,'sha256':sha,
                        'explicit_fields':values,'defaulted_fields':sorted(set(FIELDS)-set(values))}}
    if crystal_stag:
        if creature != {'family':'Creature','key':'oteryn:creature.stag','revision':'definition-r1'} or path != STAG_PATH or look != 1913 or sha != 'e3ec6434cabac687b502bb69d0a5abd3f40a11fc24ffa92c04d3cef45e777aeb':
            raise ValueError('Crystal Stag exact source identity/look differs')
        formal_raw, formal_blob, _ = pinned(root,path,CRYSTAL_FORMAL)
        if formal_raw != raw or formal_blob != blob:
            raise ValueError('Crystal Stag formal/current Lua differs')
        default_raw,default_blob,default_sha = pinned(root,HEADER,revision)
        if default_sha != CRYSTAL_DEFAULT_SHA:
            raise ValueError('Crystal Outfit defaults changed')
        block = default_raw.decode().split('struct Outfit_t {',1)[1].split('};',1)[0]
        if any(not re.search(rf'\b{field}\s*=\s*0\s*;',block) for field in FIELDS):
            raise ValueError('Crystal Outfit source default is unqualified')
        result['source']['default_source'] = {'revision':revision,'path':HEADER,'git_blob':default_blob,'sha256':default_sha}
    result['qualification_sha256'] = hashlib.sha256(json.dumps(result,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    return result

def build(root,profiles,source_paths=None,crystal_source=None):
    if git(root,'rev-parse','HEAD').decode().strip() != PIN:
        raise ValueError('source checkout revision mismatch')
    raw,_,sha = pinned(root,HEADER)
    if sha != DEFAULT_SHA:
        raise ValueError('Outfit_t default closure changed')
    block = raw.decode().split('struct Outfit_t {',1)[1].split('};',1)[0]
    for field in FIELDS:
        if not re.search(rf'\b{field}\s*=\s*0\s*;',block):
            raise ValueError(f'unqualified source default: {field}')
    paths = git(root,'ls-tree','-r','--name-only',PIN,'data-otservbr-global/monster').decode().splitlines()
    rows = []
    for target in profiles['records']:
        data = target['profile']['data']['profile']
        details = data['details']
        if not details['flags']['illusionable']:
            continue  # No fixed illusion is authorized for a non-illusionable creature.
        key = target['profile']['target']['key']
        if key == 'oteryn:creature.stag':
            crystal_source = crystal_source or Path('/workspace/spell-sources/crystal')
            if git(crystal_source,'rev-parse','HEAD').decode().strip() != CRYSTAL_PIN:
                raise ValueError('Crystal source checkout revision differs')
            rows.append(record(crystal_source,STAG_PATH,target['profile']['target'],crystal_stag=True))
            continue
        source_path = (source_paths or {}).get(key)
        if source_path:
            matches = [source_path] if source_path in paths else []
        else:
            slug = key.rsplit('/',1)[-1]
            matches = [p for p in paths if p.endswith('/'+slug+'.lua')]
        if len(matches) != 1:
            raise ValueError(f'nonunique/missing source creature: {details["display_name"]}')
        raw,_,_ = pinned(root,matches[0])
        name = re.search(r'Game\.createMonsterType\("([^"\n]+)"\)',raw.decode())
        if not name or name[1].casefold() != details['display_name'].casefold():
            raise ValueError('source creature identity mismatch')
        rows.append(record(root,matches[0],target['profile']['target']))
    for name in ['balance','light','nature','steel','storm']:
        rows.append(record(root,f'data/scripts/spells/support/avatar_of_{name}.lua',avatar=True))
    rows.sort(key=lambda r:(r['outfit_key'],r['source']['path']))
    return {'schema':'OTERYN_NATIVE_SPELL_APPEARANCES/v1','default_source_revision':PIN,
            'default_source_path':HEADER,'default_source_sha256':DEFAULT_SHA,'records':rows}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source',type=Path,required=True)
    parser.add_argument('--creatures',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args = parser.parse_args()
    document = build(args.source,json.loads(args.creatures.read_text()))
    args.out.write_text(json.dumps(document,indent=2,ensure_ascii=False)+'\n')
    print(f'Authored {len(document["records"])} source-qualified spell appearance selections')
if __name__ == '__main__':
    main()
