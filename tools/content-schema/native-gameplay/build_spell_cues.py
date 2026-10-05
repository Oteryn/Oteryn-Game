"""Resolve source spell cue aliases from pinned enum declarations, not visual membership."""
import argparse, hashlib, json, re, subprocess
from pathlib import Path
PIN = '99902524e052f37574194466c2949c576e4ab269'
FILES = ['src/utils/utils_definitions.hpp', 'src/creatures/creatures_definitions.hpp',
         'src/lua/functions/core/game/lua_enums.cpp']

def without_comments(text):
    return re.sub(r'//[^\n]*|/\*.*?\*/', '', text, flags=re.S)

def enum_values(text, name):
    matches = list(re.finditer(r'\benum\s+' + re.escape(name) +
                              r'\s*:\s*uint(8|16)_t\s*\{([^}]*)\}\s*;',
                              without_comments(text), re.S))
    if len(matches) != 1:
        raise ValueError('missing or ambiguous source enum ' + name)
    values, current = {}, -1
    limit = (1 << int(matches[0][1])) - 1
    for entry in matches[0][2].split(','):
        if not entry.strip():
            continue
        member = re.fullmatch(r'\s*([A-Z][A-Z_0-9]*)(?:\s*=\s*(0x[0-9A-Fa-f]+|[0-9]+|[A-Z][A-Z_0-9]*))?\s*', entry)
        if member is None or member[1] in values:
            raise ValueError('unsupported or duplicate source enum member')
        explicit = member[2]
        if explicit is None:
            current += 1
        elif explicit in values:
            current = values[explicit]
        elif re.fullmatch(r'0x[0-9A-Fa-f]+|[0-9]+', explicit):
            current = int(explicit, 16 if explicit.startswith('0x') else 10)
        else:
            raise ValueError('unresolved source enum member')
        if not 0 <= current <= limit:
            raise ValueError('source enum value outside declared width')
        values[member[1]] = current
    return values

def qualified_cue_enums(blobs):
    tables = {
        'effect': enum_values(blobs[0].decode(), 'MagicEffectClasses'),
        'projectile': enum_values(blobs[0].decode(), 'ShootType_t'),
        'sound': enum_values(blobs[1].decode(), 'SoundEffect_t'),
    }
    lua = without_comments(blobs[2].decode())
    magic_loop = re.search(r'for\s*\(\s*auto\s+value\s*:\s*magic_enum::enum_values<MagicEffectClasses>\(\)\s*\)\s*\{\s*registerMagicEnum\(L,\s*value\);\s*\}', lua)
    if magic_loop is None:
        raise ValueError('source magic effect Lua registration unavailable')
    registered = {
        'effect': set(tables['effect']),
        'projectile': set(re.findall(r'\bregisterEnum\(L,\s*(CONST_ANI_[A-Z_0-9]+)\s*\)', lua)),
        'sound': set(re.findall(r'\bregisterEnumNamespace\(L,\s*soundNamespace,\s*SoundEffect_t::([A-Z_0-9]+)\s*\)', lua)),
    }
    if not re.search(r'soundNamespace\s*=\s*"SOUND_EFFECT_TYPE_"\s*;', lua):
        raise ValueError('source sound Lua namespace unavailable')
    return tables, registered

def build(source, catalog):
    revision = subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()
    if revision != PIN: raise ValueError('unqualified source revision')
    blobs = [subprocess.check_output(['git','-C',str(source),'show',PIN+':'+p]) for p in FILES]
    values, registered = qualified_cue_enums(blobs)
    aliases = set()
    def walk(v):
        if isinstance(v,dict):
            for z in v.values(): walk(z)
        elif isinstance(v,list):
            for z in v: walk(z)
        elif isinstance(v,str) and re.match(r'(canary\.)?(sound:|appearance:(effect|missile)/)',v): aliases.add(v)
    walk(json.loads(catalog.read_bytes()))
    records = []
    for alias in sorted(aliases):
        short = alias.removeprefix('canary.')
        if short == 'appearance:missile/weapontype':
            # Runtime weapon policy must resolve this dynamic symbol; no guessed projectile.
            continue
        if short.startswith('appearance:effect/'):
            kind, enum = 'effect', 'CONST_ME_' + short.split('/')[1].upper()
        elif short.startswith('appearance:missile/'):
            kind, enum = 'projectile', 'CONST_ANI_' + short.split('/')[1].upper()
        else:
            kind, enum = 'sound', short.split(':')[1].upper()
        if enum not in values[kind] or enum not in registered[kind]:
            raise ValueError('unknown or unregistered source cue '+alias)
        records.append({'alias':alias,'kind':kind,'source_enum':enum,'value':values[kind][enum]})
    return {'schema':'OTERYN_SOURCE_SPELL_CUES/v1','source_revision':PIN,'source_files':[{'path':p,'sha256':hashlib.sha256(b).hexdigest()} for p,b in zip(FILES,blobs)],'records':records,'dynamic_aliases':['canary.appearance:missile/weapontype']}
if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--canary',type=Path,required=True);p.add_argument('--catalog',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    a.out.write_text(json.dumps(build(a.canary,a.catalog),sort_keys=True,indent=2)+'\n')
