"""Generate Curos's source-qualified Uneasy Alliance acceptance and literal greeting."""
import argparse,hashlib,json,re,tarfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
REV='201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE='data-global/npc/curos.lua'
SHA='6feee8de2475788bca144210fd47cbf857b5dce3a16215ff2647e971bcd3a42e'
BLOB='4fdffd8e90b8de6d2f17ce5c38c45020397c48c9'
NPC='oteryn:npc.curos';QUEST='oteryn:quest.an_uneasy_alliance_quest'
PREFIX='oteryn:quest-progress/quest/u8_54/'
DOOR=PREFIX+'an_uneasy_alliance/quest_door';LINE=PREFIX+'an_uneasy_alliance/questline'
with tarfile.open(ROOT/'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
 raw=corpus.extractfile('blobs/'+BLOB).read()
assert hashlib.sha256(raw).hexdigest()==SHA
assert hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==BLOB
# Public 201189 source independently verified byte-identical to this retained9f5a72c blob.
text=raw.decode();lines=text.splitlines()
line=lambda needle:next(i+1 for i,s in enumerate(lines) if needle in s)
ref=lambda n:f'crystal:{REV}:{SOURCE}:{n}'
marker='elseif player:getStorageValue(TheNewFrontier.Mission06) >= 5 and player:getStorageValue(AnUneasyAlliance.Questline) < 1'
start=text.index(marker);end=text.index('\n\t\telseif ',start+len(marker));chunk=text[start:end]
assert 'setStorageValue(AnUneasyAlliance.Questline, 1)' in chunk
assert 'setStorageValue(AnUneasyAlliance.QuestDoor, 0)' in chunk
say=re.search(r'npcHandler:say\((.*?)\s*,\s*npc,\s*creature\)',chunk,re.S)
assert say
reply=[json.loads(s) for s in re.findall(r'"(?:\\.|[^"\\])*"',say[1])]
assert len(reply)==8 and any('seek out this rebel commander' in s for s in reply)
def storage(track,op,value):return dict(kind='storage',track=track,op=op,value=value)
rule=dict(key='oteryn:quest-npc/an_uneasy_alliance_quest/curos/start',quest=QUEST,npc=NPC,trigger='say',keywords=['mission'],guards=[storage(PREFIX+'the_new_frontier/questline','ne',16),storage(PREFIX+'the_new_frontier/questline','ne',18),storage(PREFIX+'the_new_frontier/mission06','ge',5),storage(LINE,'lt',1)],reply=reply,effects=[dict(kind='set',track=LINE,value=1),dict(kind='set',track=DOOR,value=0)],sources=[ref(line(marker)),ref(line('setStorageValue(AnUneasyAlliance.Questline, 1)')),ref(line('setStorageValue(AnUneasyAlliance.QuestDoor, 0)'))])
program=dict(schema=1,program_key='oteryn:quest-npc-program.uneasy-curos',quest=QUEST,tracks=[dict(key=DOOR,quest=QUEST,initial=0,min=0,max=1,sources=[ref(line('setStorageValue(AnUneasyAlliance.QuestDoor, 0)'))]),dict(key=LINE,quest=QUEST,initial=0,min=0,max=5,sources=[ref(line('setStorageValue(AnUneasyAlliance.Questline, 1)'))])],rules=[rule],source_files=[dict(path=SOURCE,sha256=SHA,revision=REV)])
base=json.loads((ROOT/'content/dialogues/quests-2/npc-greetings.json').read_text())
default=next(n for n in base['npcs'] if all(n[k]['source']['path']=='data/npclib/npc_system/npc_handler.lua' for k in ['farewell','walkaway']))
greet='Be greeted blank skin.';assert f'MESSAGE_GREET, "{greet}"' in text
greetings=dict(schema=1,source_revision=REV,placeholder_source=base['placeholder_source'],npcs=[dict(npc=NPC,greet=dict(text=greet,source=dict(repository='zimbadev/crystalserver',revision=REV,path=SOURCE,line=line('MESSAGE_GREET'),sha256=SHA)),farewell=default['farewell'],walkaway=default['walkaway'],variants={})])
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
for relative,value in [('content/dialogues/quest-rules-supplemental/uneasy-curos.json',program),('content/dialogues/quests-2/uneasy-curos-npc-greetings.json',greetings)]:
 target=ROOT/relative;render=json.dumps(value,indent=2)+'\n'
 if args.check:assert target.read_text()==render,relative
 else:target.parent.mkdir(parents=True,exist_ok=True);target.write_text(render)
print('Curos: source-bound acceptance, two atomic flags, exact greeting; no later stages enabled yet')
