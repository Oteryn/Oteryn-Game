"""One source-qualified acquisition support NPC, outside the56-grant census."""
import json

def append_support(root,program,bindings):
    packet=json.loads((root/'tools/content-schema/cosmetics-authoring/gnomilly-source-support.json').read_bytes())
    if packet['schema']!='OTERYN_NPC_COSMETIC_TRANSITIONS/v1' or packet['revision']!=program['revision'] or len(packet['npcs'])!=1:
        raise ValueError('Gnomilly support capture identity')
    source=packet['npcs'][0]
    if source['npc']!='oteryn:npc.gnomilly' or source['source']!={'path':'data-global/npc/gnomilly.lua','sha256':'1682a9cddf858e75286e1bb4323ad64bd43f8ab8a0f3a01c9973da921583b0cf'} or len(source['transitions'])!=11:
        raise ValueError('Gnomilly support capture coverage')
    if any(n['npc']==source['npc'] for n in program['npcs']):raise ValueError('duplicate Gnomilly support')
    node={'npc':source['npc'],'source':source['source'],'greeting':source['greeting'],'farewell':source['farewell'],'owned_tracks':[],'transitions':[]}
    for original in source['transitions']:
        transition=dict(original);transition['source_key']='oteryn:npc-cosmetic/gnomilly/'+transition['id'];transition['grants']=[]
        node['transitions'].append(transition)
    program['npcs'].append(node)
    for symbol,tail,owner in [
        ('Spike_Upper_Kill_Main','spike_upper_kill_main','oteryn:quest.spike_task'),
        ('Spike_Upper_Kill_Daily','spike_upper_kill_daily','oteryn:quest.spike_task'),
        ('Spike_Fame_Points','spike_fame_points','oteryn:quest.spike_fame_acquisition')]:
        row={'storage':'Storage.Quest.U10_20.SpikeTaskQuest.'+symbol,'key':'oteryn:quest-progress/quest/u10_20/spike_task_quest/'+tail,'quest':owner}
        if row['storage'] in bindings and bindings[row['storage']]!=row:raise ValueError('Gnomilly existing owner drift')
        bindings[row['storage']]=row
