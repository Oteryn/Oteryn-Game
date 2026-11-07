"""Offline pinned-source initialization capture; never invokes NPC callbacks.

The maintained linker consumes checked-in capture facts. This optional authoring
command uses the existing Lua C API helper to reproduce those facts, not a game
runtime, caller grant, quest completion, or world reachability proof.
"""
from pathlib import Path
import argparse
import functools
import hashlib
import importlib.util
import json
import re
import subprocess

SHA='201189f871d23597b82caf88fd611454a2ddefb8'
NAMES={'a_majestic_warwolf','ajax','bozo','bron','dedoras','emperor_kruzak','eruaran',
       'hugo','jack_springer','king_tibianus','queen_eloise','vescu'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path)
    parser.add_argument('--crystal-git',type=Path,required=True)
    parser.add_argument('--host',type=Path,default=Path(__file__).with_name('npc_source_capture_host.lua'))
    parser.add_argument('--output',type=Path)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    root=args.root or Path(__file__).resolve().parents[3]
    output=args.output or root/'tools/content-schema/cosmetics-authoring/source-default-greetings.json'
    spec=importlib.util.spec_from_file_location('npc_capture_capi',root/'tools/content-migration/cake_source_lua.py')
    capi=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(capi)
    @functools.lru_cache(maxsize=32)
    def donor(path):
        return subprocess.check_output(['git','--git-dir='+str(args.crystal_git),'show',SHA+':'+path]).decode()
    source=json.loads((root/'tools/content-schema/cosmetics-authoring/npc-source-transitions.json').read_bytes())
    nodes=[n for n in source['npcs'] if n['npc'].removeprefix('oteryn:npc.') in NAMES]
    if len(nodes)!=12:raise ValueError('exact registered/default source closure')
    base=donor('data/npclib/npc_system/npc_handler.lua')
    match=re.search(r'\[MESSAGE_GREET\]\s*=\s*("(?:[^"\\]|\\.)*")',base)
    if match is None:raise ValueError('shared pinned greeting initializer')
    default=json.loads(match[1])
    rows=[]
    for node in sorted(nodes,key=lambda n:n['npc']):
        code=donor(node['source']['path'])
        if hashlib.sha256(code.encode()).hexdigest()!=node['source']['sha256']:
            raise ValueError('pinned NPC source hash')
        state=capi.new();capi.opens(state)
        try:
            capi.execute(state,args.host.read_text())
            capi.execute(state,donor('data-global/lib/core/storages.lua'))
            capi.execute(state,'\n'.join(name+'='+number for name,number in re.findall(r'\b(CONST_ME_[A-Z0-9_]+)\s*=\s*(\d+)',donor('src/utils/utils_definitions.hpp'))))
            capi.execute(state,'__TOPLEVEL_MESSAGE_SETTERS={};__TOPLEVEL_DEFAULT_GREETING='+json.dumps(default)+'''
local h=NpcHandler:new(KeywordHandler:new());local previous=h.setMessage
h.setMessage=function(self,kind,text)
 if kind==MESSAGE_GREET then
  __TOPLEVEL_DEFAULT_GREETING=text
  table.insert(__TOPLEVEL_MESSAGE_SETTERS,{line=debug.getinfo(2,'l').currentline,text=text})
 end
 return previous(self,kind,text)
end
''')
            capi.execute(state,code)
            capi.get(state,b'__TOPLEVEL_DEFAULT_GREETING');greeting=capi.value(state,-1)
            capi.settop(state,0);capi.get(state,b'__TOPLEVEL_MESSAGE_SETTERS');setters=capi.value(state,-1)
            rows.append(dict(npc=node['npc'],source_path=node['source']['path'],source_sha256=node['source']['sha256'],default_greeting=greeting,top_level_setters=setters,
                             qualification='Actual pinned Lua executed through source initialization only; no Greet callback or message callback dispatched. Conditional/nested setters excluded naturally.'))
        finally:capi.close(state)
    document=dict(schema='OTERYN_SOURCE_TOPLEVEL_GREET_CAPTURE/v1',source_revision=SHA,
                  shared_default=dict(path='data/npclib/npc_system/npc_handler.lua',line=81,sha256=hashlib.sha256(base.encode()).hexdigest(),value=default),npcs=rows)
    raw=(json.dumps(document,indent=2,sort_keys=True)+'\n').encode()
    if args.check:
        if output.read_bytes()!=raw:raise ValueError('source greeting capture drift')
    else:output.write_bytes(raw)
    print('Pinned source initialization capture:',len(rows),'NPCs; no callbacks executed')

if __name__=='__main__':main()
