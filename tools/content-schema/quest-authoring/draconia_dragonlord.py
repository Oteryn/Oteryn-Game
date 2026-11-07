"""Generate Dragonlord source rules from the hash-verified offline donor corpus."""
from pathlib import Path
import argparse,hashlib,json,tarfile
ROOT=Path(__file__).resolve().parents[3]
NPC='oteryn:npc.an_old_dragonlord';QUEST='oteryn:quest.draconia_quest';TRACK='oteryn:quest-progress/dragonfetish'
REV='201189f871d23597b82caf88fd611454a2ddefb8';PATH='data-global/npc/an_old_dragon_lord.lua';SHA='85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052'
archive=ROOT/'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz'
assert hashlib.sha256(archive.read_bytes()).hexdigest()=='f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277'
with tarfile.open(archive) as corpus:
 raw=corpus.extractfile('blobs/1edc28f6c6bbbb9e5ea2ef74b836151a8a21ead4').read()
assert hashlib.sha256(raw).hexdigest()==SHA
assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()=='1edc28f6c6bbbb9e5ea2ef74b836151a8a21ead4'
assert all(part in raw for part in [b'removeItem(3723, 1)',b'addItem(3206, 1)',b'getStorageValue(Storage.Dragonfetish) == 1',b'setStorageValue(Storage.Dragonfetish, 1)'])
literal=json.loads((ROOT/'content/dialogues/quests-2/classic-donor-dialogues.json').read_bytes())
npc=next(n for n in literal['npcs'] if n['npc']==NPC);assert npc['source']['sha256']==SHA
replies={r['line']:r['reply'] for r in npc['branches']}
ref=lambda line:f'crystal:{REV}:{PATH}:{line}'
end={'kind':'end_conversation','delay_ms':0}
rules=[]
def rule(name,guards,line,effects,keywords=['hi','hello']):
 rules.append(dict(key=f'oteryn:quest-npc/draconia_quest/an_old_dragonlord/{name}',quest=QUEST,npc=NPC,trigger='greet',keywords=keywords,any_message=not keywords,guards=guards,reply=replies[line],effects=effects+[end],sources=[ref(line)]))
flag=lambda op:dict(kind='storage',track=TRACK,op=op,value=1)
item=lambda op:dict(kind='item',item='oteryn:item.tibia.i3723',count=1,subtype=None,op=op)
rule('already',[flag('eq')],62,[])
rule('missing',[flag('ne'),item('lt')],67,[])
rule('exchange',[flag('ne'),item('ge')],73,[dict(kind='take_item',item='oteryn:item.tibia.i3723',count=1,subtype=None,optional=False),dict(kind='set',track=TRACK,value=1),dict(kind='give_item',item='oteryn:item.tibia.i3206',count=1,subtype=None)])
rule('other-greeting',[],40,[],[])
doc=dict(schema=1,program_key='oteryn:quest-npc-program.draconia',quest=QUEST,tracks=[dict(key=TRACK,quest=QUEST,initial=-1,min=-1,max=9223372036854775807,sources=[ref('61-73')])],rules=rules,source_files=[dict(path=PATH,sha256=SHA,revision=REV)])
base=json.loads((ROOT/'content/dialogues/quests-2/npc-greetings.json').read_bytes())
def source(line):return dict(repository='zimbadev/crystalserver',revision=REV,path=PATH,line=line,sha256=SHA)
default=next(n for n in base['npcs'] if all(n[k]['source']['path']=='data/npclib/npc_system/npc_handler.lua' for k in ['farewell','walkaway']))
greetings=dict(schema=1,source_revision=REV,placeholder_source=base['placeholder_source'],npcs=[dict(npc=NPC,greet=dict(text=replies[67][0],source=source(67)),farewell=default['farewell'],walkaway=default['walkaway'],variants={})])
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
for relative,value in [('content/dialogues/quest-rules-supplemental/draconia.json',doc),('content/dialogues/quests-2/draconia-npc-greetings.json',greetings)]:
 target=ROOT/relative
 text=json.dumps(value,indent=2)+'\n'
 if args.check:
  assert target.read_text()==text,relative
 else:
  target.parent.mkdir(parents=True,exist_ok=True)
  target.write_text(text)
print('Draconia: four source-bound greeting branches verified')
