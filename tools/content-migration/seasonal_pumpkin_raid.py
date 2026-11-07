"""Finite current-Tibia Pumpkin raid activation on the existing calendar owner."""
from pathlib import Path
import json,hashlib,re,html
PUMPKIN='oteryn:raid.northern_darama_raids.the_mutated_pumpkin'
PACKET='tools/content-schema/native-gameplay/seasonal-pumpkin-raid.json'
def admit_mutated_pumpkin(record:dict,observed:dict)->None:
 if record['key']!=PUMPKIN:return
 root=Path(__file__).resolve().parents[2]
 packet=json.loads((root/PACKET).read_text());source=packet['source'];raw=source['body'].encode()
 if hashlib.sha256(raw).hexdigest()!=source['sha256'] or source['sha256']!='c5fbb7ddf71779e795806b92038a209770324cb06e6dd610b5335708765ed742':raise ValueError('PUMPKIN_CURRENT_SOURCE_SHA')
 doc=json.loads(raw);text=' '.join(html.unescape(re.sub('<[^>]+>',' ',doc['html'])).split())
 required=['It spawns every 4 hours, beginning a few seconds after Server Open','between the Server Saves of October 31 and November 3','Oh noes! It\'s a mutated pumpkin!']
 if any(r not in text for r in required):raise ValueError('PUMPKIN_CURRENT_SOURCE_SCHEDULE')
 coords=[(33115,32398,7),(33095,32381,7),(33092,32418,7)]
 if any(f'data-coord="{x},{y},{z}"'not in doc['html']for x,y,z in coords):raise ValueError('PUMPKIN_EXACT_SOURCE_POSITIONS')
 record['schedule']={'kind':'calendar','interval_ms':14_400_000,'window':{'start_month':10,'start_day':31,'end_month':11,'end_day':2}}
 record['end']={'kind':'until_reset','despawn_after_ms':None}
 record['sources']=[{'source':'wiki:29889:grimorio-acc9994','selector':'The Mutated Pumpkin / Notes and Location','retained_path':PACKET,'retained_key':PUMPKIN}]
 record['source_resolution']={'decision':'Current retainedWiki: firsteligiblepost-opencheck thenfourhours; Oct31 throughNov3save; existingserver-savecalendar mapping fromPR2781','source_url':source['url'],'source_sha256':source['sha256'],'coordinates':'Threealternativeexactsourcepoints; existingdeterministicvariantselector, no newarea or replacementcoordinate','end':'ExistingUntilReset leavesactorsandfinishesafterwaveACK, noinventedbosstimeout'}
 variants=[]
 for index,(x,y,z)in enumerate(coords):
  at={'x':x,'y':y,'floor':z};anchor=f'raid-anchor:{x}_{y}_{z}-{x}_{y}_{z}'
  variants.append({'key':f'wiki:29889:grimorio-acc9994/location-{index+1}','announcements':[{'delay_ms':0,'text':"Oh noes! It's a mutated pumpkin!",'kind':'advance'}],
   'waves':[{'delay_ms':0,'entries':[{'creature':'oteryn:creature.the_mutated_pumpkin','source_name':'The Mutated Pumpkin','amount':{'min':1,'max':1},'area_anchor':anchor,'area':{'from':at,'to':dict(at)},'boss':True,'spawn_mode':'single','sources':record['sources']}]}],'unresolved':[]})
 record['variants']=variants;record['inactive_reasons']=[]
