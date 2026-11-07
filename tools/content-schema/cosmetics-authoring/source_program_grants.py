from pathlib import Path
import hashlib,json

def admit(root,document,outfits,mappings,mounts):
 path=Path(root)/'content/cosmetics/npc-acquisitions.json';raw=path.read_bytes();programme=json.loads(raw)
 catalogue=json.loads((Path(root)/'tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json').read_bytes())
 qualified={g['key'] for n in programme['npcs'] for t in n['transitions'] for g in t['grants']}
 if qualified!={r['grant_key'] for r in catalogue['records']}:raise ValueError('source programme/grant catalogue differs')
 affected={g['outfit_key'] for n in programme['npcs'] for t in n['transitions'] for g in t['grants']}
 citation={'reference':'content/cosmetics/npc-acquisitions.json','revision':programme['revision'],'sha256':hashlib.sha256(raw).hexdigest()}
 for row in outfits:
  if row['key'] in affected:
   row['unlock_sources']=[c for c in row['unlock_sources'] if c['reference']!=citation['reference']]+[citation]
 for mapping in catalogue['records']:
  previous=mappings.get(mapping['grant_key'])
  if previous is not None and previous!=mapping:raise ValueError('source programme changes a previous grant identity')
  mappings[mapping['grant_key']]=mapping

 rental=json.loads((Path(root)/'tools/content-schema/cosmetics-authoring/horse-rental-cosmetics.json').read_bytes())
 for row in rental['mounts']:
  previous=mounts.get(row['key'])
  if previous is not None and previous!=row:raise ValueError('rental metadata identity conflict')
  mounts[row['key']]=row
 for mapping in rental['grant_mappings']:
  previous=mappings.get(mapping['grant_key'])
  if previous is not None and previous!=mapping:raise ValueError('rental temporary projection mapping conflict')
  mappings[mapping['grant_key']]=mapping

 permanent=json.loads((Path(root)/'tools/content-schema/cosmetics-authoring/npc-permanent-mount-cosmetics.json').read_bytes())
 if permanent['schema']!='OTERYN_NPC_SOURCE_PERMANENT_MOUNT_METADATA/v1':raise ValueError('permanent source mount metadata schema')
 expected={'oteryn:content.mount.stampor':(11,378),'oteryn:content.mount.noxious_ripptor':(202,1577)}
 if {row['key'] for row in permanent['mounts']}!=set(expected):raise ValueError('permanent source mount metadata coverage')
 for row in permanent['mounts']:
  identifier,look=expected[row['key']]
  if row['look']['id']!=look or not any(m['id']==identifier and m['look']==look and m['key']==row['key'] for m in programme['mounts']):raise ValueError('permanent source mount identity differs from programme')
  previous=mounts.get(row['key'])
  if previous is not None and previous!=row:raise ValueError('permanent source mount metadata identity conflict: '+row['key'])
  mounts[row['key']]=row
 expected_keys={'oteryn:appearance-grant/npc/chondur/stampor/mount':'oteryn:content.mount.stampor','oteryn:appearance-grant/npc/gnomadness/noxious_ripptor/mount':'oteryn:content.mount.noxious_ripptor'}
 if {row['grant_key'] for row in permanent['grant_mappings']}!=set(expected_keys):raise ValueError('permanent source mount grant coverage')
 for mapping in permanent['grant_mappings']:
  if mapping['target']['kind']!='Mount' or mapping['target']['mount_key']!=expected_keys[mapping['grant_key']] or mapping['compatible_provenance']!=['crystal-201189f-npc-cosmetics-v1']:raise ValueError('permanent source mount grant identity')
  previous=mappings.get(mapping['grant_key'])
  if previous is not None and previous!=mapping:raise ValueError('permanent source mount grant conflict: '+mapping['grant_key'])
  mappings[mapping['grant_key']]=mapping

 # The reciprocal two-Character owner is deliberately outside the ordinary
 # programme: this metadata grants no entitlement or marriage progress.
 wedding=json.loads((Path(root)/'tools/content-schema/cosmetics-authoring/npc-wedding-cosmetics.json').read_bytes())
 expected={'grant_key':'oteryn:appearance-grant/npc/lynda/newly_wed/base','target':{'kind':'Outfit','outfit_key':'oteryn:content.outfit.newly_wed_outfits'},'compatible_provenance':['crystal-201189f-npc-cosmetics-v1']}
 if wedding['schema']!='OTERYN_NPC_WEDDING_COSMETICS/v1' or wedding['grant_mapping']!=expected or wedding['looks']!={'male':328,'female':329}:raise ValueError('Wedding source metadata identity')
 citation=wedding['source']
 if citation!={'reference':'https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/lynda.lua#L166-L210','revision':'201189f871d23597b82caf88fd611454a2ddefb8','sha256':'c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9'}:raise ValueError('Wedding source citation')
 outfit=next((row for row in outfits if row['key']==expected['target']['outfit_key']),None)
 if outfit is None or outfit['male']['id']!=328 or outfit['female']['id']!=329 or outfit['addon_count']!=0:raise ValueError('Wedding existing outfit pair differs from pinned source')
 old=[source for source in outfit['unlock_sources'] if source['reference']==citation['reference']]
 if old and old!=[citation]:raise ValueError('Wedding existing source citation conflict')
 if not old:outfit['unlock_sources'].append(citation)
 previous=mappings.get(expected['grant_key'])
 if previous is not None and previous!=expected:raise ValueError('Wedding grant mapping conflict')
 mappings[expected['grant_key']]=expected
