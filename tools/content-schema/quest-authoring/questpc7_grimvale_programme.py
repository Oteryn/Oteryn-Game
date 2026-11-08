"""Selected personal Grimvale arcs over native quest, item and death owners."""
import hashlib
import json

QUEST='oteryn:quest.grimvale_quest'
TRACK='oteryn:quest-progress/questpc7/grimvale/programme'
FLOWERS='oteryn:quest-progress/questpc7/grimvale/flowers'
PASSAGES='oteryn:quest-progress/questpc7/grimvale/passages'
SITES='oteryn:quest-progress/questpc7/grimvale/sites'
OSKAYAAT='oteryn:quest-progress/questpc7/grimvale/oskayaat'
ACCESS='oteryn:quest-progress/questpc7/grimvale/investigation_access'
DATA={'flowers': [{'position': [33329, 31690, -7], 'appearance': 18464, 'arrival': [33328, 31690, -7], 'basis': 'CHOSEN_OTERYN_APPROXIMATION; personal botanical survey plot, no original full-moon plant claim'}, {'position': [33331, 31687, -7], 'appearance': 486, 'arrival': [33331, 31686, -7], 'basis': 'CHOSEN_OTERYN_APPROXIMATION; personal botanical survey plot, no original full-moon plant claim'}, {'position': [33335, 31685, -7], 'appearance': 18459, 'arrival': [33335, 31684, -7], 'basis': 'CHOSEN_OTERYN_APPROXIMATION; personal botanical survey plot, no original full-moon plant claim'}, {'position': [33338, 31682, -7], 'appearance': 18462, 'arrival': [33337, 31682, -7], 'basis': 'CHOSEN_OTERYN_APPROXIMATION; personal botanical survey plot, no original full-moon plant claim'}, {'position': [33340, 31680, -7], 'appearance': 486, 'arrival': [33341, 31680, -7], 'basis': 'CHOSEN_OTERYN_APPROXIMATION; personal botanical survey plot, no original full-moon plant claim'}], 'bosses': [{'name': 'werewolf', 'center': [33332, 31675, -7], 'arrival': [33332, 31678, -7], 'exit': [33331, 31678, -7], 'exit_appearance': 18459}, {'name': 'werebear', 'center': [33336, 31675, -7], 'arrival': [33336, 31679, -7], 'exit': [33335, 31679, -7], 'exit_appearance': 18464}, {'name': 'wereboar', 'center': [33340, 31675, -7], 'arrival': [33341, 31678, -7], 'exit': [33341, 31679, -7], 'exit_appearance': 486}, {'name': 'feroxa', 'center': [33390, 31540, -11], 'arrival': [33390, 31543, -11], 'exit': [33389, 31543, -11], 'exit_appearance': 104}, {'name': 'bloodback', 'center': [33184, 32016, -8], 'arrival': [33184, 32017, -8], 'exit': [33183, 32017, -8], 'exit_appearance': 1020}, {'name': 'darkfang', 'center': [33062, 31890, -9], 'arrival': [33062, 31893, -9], 'exit': [33061, 31893, -9], 'exit_appearance': 1020}, {'name': 'sharpclaw', 'center': [33120, 32002, -9], 'arrival': [33120, 32005, -9], 'exit': [33119, 32005, -9], 'exit_appearance': 1020}, {'name': 'black_vixen', 'center': [33450, 32034, -9], 'arrival': [33450, 32037, -9], 'exit': [33449, 32037, -9], 'exit_appearance': 22355}, {'name': 'werehyaena', 'center': [33211, 32359, -8], 'arrival': [33211, 32362, -8], 'exit': [33210, 32362, -8], 'exit_appearance': 231}], 'sites': [{'name': 'edron', 'position': [33166, 31977, -8], 'appearance': 1020, 'arrival': [33166, 31978, -8]}, {'name': 'cormaya', 'position': [33441, 32051, -9], 'appearance': 9707, 'arrival': [33441, 32052, -9]}, {'name': 'hyaena_site', 'position': [33211, 32359, -8], 'appearance': 19273, 'arrival': [33210, 32359, -8]}, {'name': 'oskayaat_mosaic', 'position': [33067, 32917, -7], 'appearance': 231, 'arrival': [33066, 32917, -7]}, {'name': 'oskayaat_moon', 'position': [32999, 32933, -8], 'appearance': 231, 'arrival': [32998, 32933, -8]}, {'name': 'oskayaat_ritual', 'position': [33002, 32933, -8], 'appearance': 38069, 'arrival': [33001, 32933, -8]}], 'rhargu': {'position': [33138, 32354, -5], 'appearance': 4559}, 'rhargu_exit': {'position': [33139, 32354, -5], 'appearance': 4408}, 'domizian': {'position': [33140, 32354, -5], 'appearance': 4559}, 'domizian_exit': {'position': [33141, 32354, -5], 'appearance': 4397}}

def pos(p): return {'x':p[0],'y':p[1],'floor':p[2]}
def counter(name): return 'oteryn:quest-progress/questpc7/grimvale/'+name

def state(key,lo,hi,value,preserve=False):
    row={'track':key,'minimum':lo,'maximum':hi,'set':value}
    if preserve:row['preserve']=True
    return row

def build(root):
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('collect',5),('kill',10),('use',1),('kill',1),('explore',2),('kill',4),('talk',1),('use',1),('kill',20),('talk',1),('explore',2),('use',1),('complete',1)]:raise ValueError('QUESTPC7_GRIMVALE_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    def sources(p):
        region=f'content/world/placements/region-z{-p[2]:02}-x{p[0]//256}-y{p[1]//256}.b3'
        return [canonical,region+':1:sha256:'+hashlib.sha256((root/region).read_bytes()).hexdigest(),
            'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/grimvale/actions_closed_silvered_trap.lua:1-19:sha256:cba8b2e3c3843fbb2f9226682faaf36c26c209fa8f95c312fe89042466c08e94']
    def cb(name,trigger,p,appearance,storage,message,teleport=None):
        return {'key':'oteryn:quest-callback/questpc7-grimvale-'+name,'quest':QUEST,'trigger':trigger,
            'selector':{'position':pos(p),'appearance':appearance,'unique':None,'action':None},
            'lever':None,'reversible':False,'passages':[],'teleport':None if teleport is None else pos(teleport),'storage':storage,
            'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
            'item_on_target':None,'return_on_failure':False,'failure_message':None,
            'requirements':{'min_level':0,'premium':True,'completed_quests':[]},
            'success_message':message,'sources':sources(p)}
    actors={r['name']:r for r in DATA['bosses']};sites={r['name']:r for r in DATA['sites']};plots=DATA['flowers']
    # The five personal botanical plots do not alter the global full-moon event.
    entry=plots[0]['arrival'];entry_appearance=18463
    # The prepared entry cell is an admitted native ground, distinct from the first plot.
    worlds=[cb('entry','step_in',entry,entry_appearance,[state(TRACK,-1,-1,0),state(FLOWERS,-1,-1,0)],'Investigate five nightshade plots on Grimvale.')]
    for i,plot in enumerate(plots):
        storage=[state(TRACK,0,0,1 if i==4 else 0,i!=4),state(FLOWERS,i,i,i+1)]
        if i==4:storage.append(state(counter('afflicted'),-1,-1,0))
        target=plots[i+1]['arrival'] if i<4 else actors['werewolf']['arrival']
        row=cb('nightshade-'+str(i),'use',plot['position'],plot['appearance'],storage,'You gather a purple nightshade blossom. Follow the investigation trail.',target)
        row['item_reward']={'item':'oteryn:item.tibia.i22082','count':1,'contents':[]};row['activation']={'track':FLOWERS,'minimum':i,'maximum':i};worlds.append(row)
    wolf=actors['werewolf'];feroxa=actors['feroxa']
    row=cb('prepare-trap','use',wolf['exit'],wolf['exit_appearance'],[state(TRACK,1,1,2),state(counter('afflicted'),10,10,10,True)],'You prepare the silvered trap for Feroxa.',feroxa['arrival'])
    row['requirements']['owned_items']=[{'item':'oteryn:item.tibia.i22082','count':5}]
    row['item_reward']={'item':'oteryn:item.tibia.i22074','count':1,'contents':[]};worlds.append(row)
    row=cb('deploy-trap','use',feroxa['exit'],feroxa['exit_appearance'],[state(TRACK,2,2,3),state(counter('feroxa'),-1,-1,0)],'The silvered trap is deployed. Confront Feroxa.')
    row['item_on_target']={'item':'oteryn:item.tibia.i22074','count':1,'replacement':None,'target_replacement':None};row['activation']={'track':TRACK,'minimum':2,'maximum':2};worlds.append(row)
    worlds.append(cb('feroxa-passage','step_in',feroxa['exit'],feroxa['exit_appearance'],[state(TRACK,3,3,4),state(counter('feroxa'),1,1,1,True),state(SITES,-1,-1,0)],'Feroxa is defeated. Trace the curse to Edron.',sites['edron']['arrival']))
    for i,name in enumerate(['edron','cormaya']):
        site=sites[name];storage=[state(TRACK,4,4,5 if i else 4,not i),state(SITES,i,i,i+1)]
        if i:storage += [state(PASSAGES,-1,-1,0),state(counter('bloodback'),-1,-1,0)]
        worlds.append(cb('survey-'+name,'step_in',site['position'],site['appearance'],storage,'You investigate the curse in '+name.title()+'.',actors['bloodback']['arrival'] if i else sites['cormaya']['arrival']))
    names=['bloodback','darkfang','sharpclaw','black_vixen']
    for i,name in enumerate(names):
        actor=actors[name];storage=[state(TRACK,5,5,6 if i==3 else 5,i!=3),state(PASSAGES,i,i,i+1),state(counter(name),1,1,1,True)]
        if i<3:storage.append(state(counter(names[i+1]),-1,-1,0))
        worlds.append(cb('leader-passage-'+str(i),'step_in',actor['exit'],actor['exit_appearance'],storage,'The were-beast leader is defeated.',DATA['rhargu']['position'] if i==3 else actors[names[i+1]]['arrival']))
    for name,phase,target in [('rhargu',7,sites['hyaena_site']['arrival']),('domizian',10,sites['oskayaat_mosaic']['arrival'])]:
        site=DATA[name+'_exit'];worlds.append(cb(name+'-passage','step_in',site['position'],site['appearance'],[state(TRACK,phase,phase,phase,True),state(counter(name+'_passage'),-1,-1,1)],'The investigation continues.',target))
    site=sites['hyaena_site']
    worlds.append(cb('hyaena-evil-site','use',site['position'],site['appearance'],[state(TRACK,7,7,8),state(counter('werehyaena'),-1,-1,0)],'You destroy the influence at the cursed site. Defeat twenty Werehyaenas.',actors['werehyaena']['arrival']))
    hyaena=actors['werehyaena'];worlds.append(cb('hyaena-passage','step_in',hyaena['exit'],hyaena['exit_appearance'],[state(TRACK,8,8,9),state(counter('werehyaena'),20,20,20,True)],"The lairs are cleared. Request Domizian's pardon.",DATA['domizian']['position']))
    for i,name in enumerate(['oskayaat_mosaic','oskayaat_moon']):
        site=sites[name];storage=[state(TRACK,10,10,11 if i else 10,not i),state(OSKAYAAT,i,i,i+1)]
        worlds.append(cb(name,'step_in',site['position'],site['appearance'],storage,"You investigate the island's ancient symbols.",sites['oskayaat_ritual']['arrival'] if i else sites['oskayaat_moon']['arrival']))
    site=sites['oskayaat_ritual'];row=cb('ritual','use',site['position'],site['appearance'],[state(TRACK,11,11,12),state(OSKAYAAT,2,2,2,True),state(ACCESS,-1,-1,1)],'The ancient ritual stills the local curse. You earn the Foxtail Amulet, Over the Moon and Drama in Darama.')
    row.update(completes=True,item_reward={'item':'oteryn:item.tibia.i27565','count':1,'contents':[]},achievements=['oteryn:achievement/over_the_moon','oteryn:achievement/drama_in_darama'],activation={'track':TRACK,'minimum':11,'maximum':11});worlds.append(row)
    for name,target in [('oskayaat_mosaic',sites['oskayaat_moon']['arrival']),('oskayaat_ritual',sites['oskayaat_mosaic']['arrival'])]:
        site=sites[name] if name!='oskayaat_ritual' else {'position':(33001, 32933, -8),'appearance':38069}
        row=cb('earned-access-'+name,'use',site['position'],site['appearance'],[state(ACCESS,1,1,1,True)],'Your completed investigation opens the island route.',target);worlds.append(row)

    objectives=[]
    for name in ['werewolf','werebear','wereboar','feroxa']+names+['werehyaena']:
        actor=actors[name];center=actor['center'];phase=1 if name in ['werewolf','werebear','wereboar'] else 3 if name=='feroxa' else 8 if name=='werehyaena' else 5
        stage=PASSAGES if name in names else TRACK;stage_value=names.index(name) if name in names else phase
        objectives.append({'key':'oteryn:quest-death/questpc7-grimvale-'+name,'quest':QUEST,'creature':'oteryn:creature.'+name,'stage':stage,'stage_value':stage_value,'counter':counter('afflicted' if phase==1 else name),'required':10 if phase==1 else 20 if name=='werehyaena' else 1,'minimum':pos((center[0]-8,center[1]-8,center[2])),'maximum':pos((center[0]+8,center[1]+8,center[2])),'guardian':{'position':pos(center),'respawn_ms':60000},'sources':sources(center)})
    callbacks=[]
    for name,before,after,word,reply in [('rhargu',6,7,'feud','Investigate the cursed site in the Hyaena Lairs.'),('domizian',9,10,'pardon',"You have earned the pardon. Investigate Oskayaat's symbols.")]:
        storage=[state(TRACK,before,before,after)]
        if name=='domizian':storage.append(state(OSKAYAAT,-1,-1,0))
        callbacks.append({'key':'oteryn:quest-reward/questpc7-grimvale-'+name,'quest':QUEST,'npc':'oteryn:npc.'+name,'keywords':[word],'topic':0,'next_topic':0,'storage':storage,'take':[],'give':None,'outfit':None,'achievement':None,'reply':[reply],'requirements':{'min_level':0,'premium':True,'completed_quests':[]},'sources':sources(DATA[name]['position'])})
    return worlds,objectives,callbacks
