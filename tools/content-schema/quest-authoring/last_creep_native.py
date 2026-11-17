"""M23 chosen personal Last Creep arena; source identities do not claim Global geometry."""
from pathlib import Path
import argparse
import hashlib
import json
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = 'content/quests/last-creep-native.json'
CAPTURE = 'docs/reference/content-prep/m23-last-creep/source-identities.json'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
BASIS = 'CHOSEN_OTERYN_APPROXIMATION'
QUEST = 'oteryn:quest.authored.last_creep_standing'
FORMS = [('lost-soul',12052),('pharaoh',12047),('carniphila',12053),('quara-hydromancer',12048),
         ('blightwalker',12056),('mutated-rat',12049),('zombie',12050),('nightstalker',12051)]
NAMES = {231:'sand',12052:'lost soul dummy',12047:'pharaoh dummy',12053:'carniphila dummy',
         12048:'hydromancer dummy',12056:'blightwalker dummy',12049:'mutated rat dummy',
         12050:'zombie dummy',12051:'nightstalker dummy',12251:'shapechanger',409:'white marble floor',1949:'magic forcefield'}

def position(x,y,floor=-15): return {'x':x,'y':y,'floor':floor}
def key(name): return 'oteryn:quest-callback/last-creep-native-'+name

def capture_xml(source):
    raw = Path(source).read_bytes()
    xml = ET.fromstring(raw)
    selected = {}
    for row in xml:
        if row.get('id','').isdigit() and int(row.get('id')) in NAMES:
            item = int(row.get('id'))
            if row.get('name') != NAMES[item]: raise ValueError('LAST_CREEP_XML_IDENTITY_DRIFT')
            selected[str(item)] = {'name':row.get('name'),
                'attributes':{a.get('key'):a.get('value') for a in row.findall('attribute')}}
    if set(map(int,selected)) != set(NAMES): raise ValueError('LAST_CREEP_XML_IDENTITY_MISSING')
    return {'url':f'https://raw.githubusercontent.com/zimbadev/crystalserver/{PIN}/data/items/items.xml',
            'revision':PIN,'sha256':hashlib.sha256(raw).hexdigest(),
            'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),
            'records':selected}

def build(source=None,root=ROOT):
    proof = capture_xml(source) if source else json.loads((root/CAPTURE).read_bytes())
    if proof['revision'] != PIN or {int(identifier):row['name'] for identifier,row in proof['records'].items()} != NAMES:
        raise ValueError('LAST_CREEP_CAPTURE_IDENTITY_DRIFT')
    forms = [{'slug':slug,'condition':'quest.last_creep.'+slug.replace('-','_'),
              'appearance':item,'order':index+1,
              'previous':'oteryn:quest-progress/authored/last_creep_standing/s'+str(index+1),
              'stage':'oteryn:quest-progress/authored/last_creep_standing/s'+str(index+2),
              'portal':position(20483+index*3,20483),
              'marker':position(20483+index*3,20482),
              'callback':key(slug)} for index,(slug,item) in enumerate(FORMS)]
    return {'schema':1,'basis':BASIS,'quest':QUEST,
            'source_equivalence':False,'visual_binding':'CHOSEN_OTERYN_ITEM_EXACT_DUMMY_IDS',
            'source_display_encoding':'UNPROVEN_NOT_ASSUMED',
            'item':12251,'ground':409,'portal_appearance':1949,
            'central':position(20494,20492),'safe':position(20485,20506),
            'return_portal':position(20485,20504),'exit_portal':position(20508,20493),
            'start':position(33261,32365,-7),
            'outside':position(33264,32366,-7),
            'start_appearance':12047,'forms':forms,
            'sources':[f'crystal:{PIN}:data/items/items.xml:literal-identities;NOT_DONOR_GEOMETRY',
                       'wiki:50863@1050074:CHOSEN_FIXED_ORDER_PERSONAL_TRIAL;NOT_GLOBAL_DISPLAY_ENCODING'],
            'source_xml':proof}

def item_disguises(document):
    return [{'key':form['condition'],'item':'oteryn:item.tibia.i12251',
             'appearance':form['appearance'],'track':form['stage'],'required':0,
             'requires':[{'track':form['previous'],'minimum':1,'maximum':1}],
             'activation':document['central'],'restore':None,
             'use_text':'You take the '+form['slug'].replace('-',' ')+' disguise. Find the matching portal.',
             'restore_text':'','use_effect':3,'speech_type':36,
             'sources':document['sources']} for form in document['forms']]

def native_item_ids(root=ROOT):
    document=build(root=root)
    return {document['item'],*(form['appearance']for form in document['forms'])}

def map_item_ids(root=ROOT):
    return {form['appearance']for form in build(root=root)['forms']}

def encoded(value): return (json.dumps(value,sort_keys=True,separators=(',',':'))+'\n').encode()

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--items-xml',type=Path)
    parser.add_argument('--root',type=Path,default=ROOT);parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    if args.items_xml:
        raw=encoded(capture_xml(args.items_xml));target=args.root/CAPTURE
        if args.check:
            if target.read_bytes()!=raw:raise ValueError('LAST_CREEP_SOURCE_CAPTURE_STALE')
        else:target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
    raw=encoded(build(root=args.root));target=args.root/OUTPUT
    if args.check:
        if target.read_bytes()!=raw:raise ValueError('LAST_CREEP_DESCRIPTOR_STALE')
    else:target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw)
