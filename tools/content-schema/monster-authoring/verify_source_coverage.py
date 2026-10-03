"""Account for every inventoried standard registrar/spell path in the formal schema."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from build_formal_schema import main
from verify_monster_source_values import PINS
ROOT=Path(__file__).resolve().parent

def dest(owner,*path):return '/$defs/'+owner+'/properties/'+'/properties/'.join(path)
base={
 'name':dest('creature','display_name'),'description':dest('creature','inspection','description'),
 'variant':dest('presentation','variant_label'),'experience':dest('stats','experience'),
 'Bestiary':dest('creature','bestiary'),'bosstiary':dest('creature','bosstiary'),
 'skull':dest('presentation','status_marker'),'outfit':dest('presentation','appearance'),
 'maxHealth':dest('stats','max_health'),'health':dest('stats','initial_health'),
 'race':dest('creature','death_residue'),'manaCost':dest('summoning','mana_cost'),
 'speed':dest('stats','speed'),'corpse':dest('creature','corpse_item'),
 'faction':dest('behavior','faction_and_preferences','faction'),
 'targetPreferPlayer':dest('behavior','faction_and_preferences','prefer_player'),
 'targetPreferMaster':dest('behavior','faction_and_preferences','prefer_master'),
 'enemyFactions':dest('behavior','faction_and_preferences','enemy_factions'),
 'flags':dest('creature','flags'),'light':dest('presentation','light'),
 'changeTarget':dest('behavior','targeting','change_target'),
 'strategiesTarget':dest('behavior','targeting','strategy_weights'),
 'respawnType':dest('creature','spawn_eligibility'),'sounds':dest('presentation','audio'),
 'voices':dest('behavior','voices'),'summon':dest('behavior','summons'),
 'events':dest('behavior','event_bindings'),'loot':dest('loot','entries'),
 'elements':dest('creature','resistances'),'reflects':dest('creature','damage_reflection'),
 'heals':dest('creature','healing_from_damage'),'immunities':dest('creature','immunities'),
 'attacks':dest('behavior','attacks'),'defenses':dest('behavior','defenses')}
flags={
 'attackable':dest('creature','flags','attackable'),'illusionable':dest('creature','flags','illusionable'),
 'healthHidden':dest('creature','flags','health_hidden'),'convinceable':dest('summoning','convinceable'),
 'summonable':dest('summoning','summonable'),'familiar':dest('summoning','is_familiar'),
 'isPreyable':dest('creature','system_eligibility','prey'),'isPreyExclusive':dest('creature','system_eligibility','exclusive_prey'),
 'isForgeCreature':dest('creature','system_eligibility','forge'),'rewardBoss':dest('creature','system_eligibility','reward_boss'),
 'isBlockable':dest('creature','spawn_eligibility','blocked_by_nearby_players'),
 'critChance':dest('stats','critical_chance_percent'),'hostile':dest('behavior','targeting','hostile'),
 'canTarget':dest('behavior','targeting','can_target'),'targetDistance':dest('behavior','targeting','target_distance_tiles'),
 'runHealth':dest('behavior','targeting','flee_health'),'staticAttackChance':dest('behavior','targeting','static_attack_chance_percent'),
 'canWalk':dest('behavior','movement','can_walk'),'pushable':dest('behavior','movement','pushable'),
 'canPushItems':dest('behavior','movement','push_items'),'canPushCreatures':dest('behavior','movement','push_creatures'),
 'canWalkOnEnergy':dest('behavior','movement','field_permissions','energy'),
 'canWalkOnFire':dest('behavior','movement','field_permissions','fire'),'canWalkOnPoison':dest('behavior','movement','field_permissions','poison')}
specific={
 'mask.Bestiary.class':dest('bestiary','class'),'mask.Bestiary.race':dest('bestiary','taxonomy'),
 'mask.Bestiary.toKill':dest('bestiary','kill_thresholds'),'mask.Bestiary.FirstUnlock':dest('bestiary','kill_thresholds'),
 'mask.Bestiary.SecondUnlock':dest('bestiary','kill_thresholds'),'mask.Bestiary.CharmsPoints':dest('bestiary','charm_points'),
 'mask.Bestiary.Stars':dest('bestiary','stars'),'mask.Bestiary.Locations':dest('bestiary','locations'),'mask.Bestiary.Occurrence':dest('bestiary','occurrence'),
 'mask.bosstiary.bossRace':dest('bosstiary','category'),
 'mask.defenses.armor':dest('stats','armor'),'mask.defenses.defense':dest('stats','defense'),
 'mask.defenses.mitigation':dest('stats','mitigation_percent'),
 'mask.light.level':dest('presentation','light','level'),'mask.light.color':dest('presentation','light','color_binding'),
 'mask.changeTarget.chance':dest('behavior','targeting','change_target','chance_percent'),
 'mask.changeTarget.interval':dest('behavior','targeting','change_target','interval_ms'),
 'mask.respawnType.period':dest('creature','spawn_eligibility','period'),
 'mask.respawnType.underground':dest('creature','spawn_eligibility','ignore_period_underground'),
 'mask.sounds.chance':dest('behavior','periodic_audio','chance_percent'),'mask.sounds.ticks':dest('behavior','periodic_audio','interval_ms'),
 'mask.sounds.ids':dest('behavior','periodic_audio','cue_ids'),'mask.sounds.death':dest('presentation','audio','event_bindings'),
 'mask.voices.chance':dest('voices','chance_percent'),'mask.voices.interval':dest('voices','interval_ms'),
 'mask.summon.maxSummons':dest('behavior','summons','max_summons'),'mask.summon.summons':dest('behavior','summons','entries')}
for key in ('nearest','health','damage','random'):specific['mask.strategiesTarget.'+key]=dest('behavior','targeting','strategy_weights',key)
spell={
 'attack':dest('formula','melee','attack'),'skill':dest('formula','melee','skill'),
 'chance':dest('schedule','chance_percent'),'interval':dest('schedule','interval_ms'),'name':dest('ability','kind'),
 'type':dest('effect','damage_type'),'range':dest('ability','range_tiles'),'target':dest('ability','needs_target'),
 'duration':dest('effect','duration_ms'),'speedChange':dest('condition','speed_formula'),
 'length':dest('area','length_tiles'),'spread':dest('area','spread_tiles'),'radius':dest('area','radius_tiles'),
 'minDamage':dest('formula','magnitude','minimum'),'maxDamage':dest('formula','magnitude','maximum'),
 'startDamage':dest('damageOverTime','initial_tick','amount'),
 'outfitMonster':dest('effect','appearance_transform','creature'),'outfitItem':dest('effect','appearance_transform','item'),
 'effect':dest('effect','presentation','impact_asset_binding'),
 'shootEffect':dest('effect','presentation','projectile_asset_binding'),'shooteffect':dest('effect','presentation','projectile_asset_binding'),
 'soundCast':dest('ability','audio','cast_cue'),'impactCast':dest('ability','audio','impact_cue'),
 'condition':dest('effect','condition'),'condition.type':dest('condition','type'),
 'condition.duration':dest('effect','duration_ms'),'condition.interval':dest('damageOverTime','tick_interval_ms'),
 'condition.totalDamage':dest('damageOverTime','total_damage_range')}

def resolve(p):
    value=main
    for key in p.strip('/').split('/'):
        if '$ref' in value:value=main['$defs'][value['$ref'].split('/')[-1]]
        value=value[key]
    return value

def verify_census(census):
    result={'scope':'Every inventoried standard mask and incomingLua path has a formal destination or explicit non-field disposition. Not arbitrary Lua/runtime parity.',
        'sources':{},'unclassified':[],'invalid_destinations':[]}
    for name,source in census['sources'].items():
        rows=[]
        for group in ('mask_paths_with_lines','spell_paths_with_lines'):
            for path,lines in source[group].items():
                status='mapped';target=None;note='Requires explicit source normalization and dependency resolution.'
                if path in ('mask.raceId','mask.bosstiary.bossRaceId'):
                    status='metadata_only';note='Foreign identifier belongs in provenance, never native identity.'
                elif path in ('mask.flags.respawnType','mask.flags.respawntype'):
                    status='deprecated_warning';note='Inspected registrar warns and does not apply this field.'
                elif path=='incomingLua.script':
                    status='requires_native_behavior_resolution';target=dest('behavior','event_bindings')
                    note='Original script remains blocked until its required behavior is resolved; no Lua execution in this schema.'
                elif path.startswith('incomingLua.'):
                    target=spell.get(path.removeprefix('incomingLua.'))
                elif path.startswith('mask.flags.'):
                    target=flags.get(path.split('.')[2])
                else:target=specific.get(path,base.get(path.split('.')[1]))
                if status=='mapped' and target is None:result['unclassified'].append(name+':'+path)
                if target:
                    try:resolve(target)
                    except (KeyError,TypeError):result['invalid_destinations'].append(name+':'+path+' -> '+target)
                if path=='mask.skull' and name=='canary':note+=' Registrar/API registration discrepancy must be resolved in source profile.'
                if path=='mask.manaCost':note+=' One shared source mana_cost applies when summonable or convinceable; wiki conflicts remain separate.'
                if path=='incomingLua.chance':note+=' Inspected melee branch ignores the declared chance; never apply it without branch normalization.'
                if path=='incomingLua.speedChange':note+=' Source speed branch clamps below -1000; multiplier=1+value/1000 and formula variables=(multiplier/2,40,multiplier,40). Map the whole speed formula, not a direct percent.'
                if path in ('incomingLua.minDamage','incomingLua.maxDamage'):note+=' Named-condition branch stores a nominal total damage range in damage_over_time.total_damage_range, not per-tick damage; native decreasing schedule parity remains gated.'
                if path=='incomingLua.startDamage':note+=' Zero selects initial_tick.mode=automatic; positive selects fixed+amount. Never store zero as a fixed tick.'
                if path=='incomingLua.condition.totalDamage':note+=' Fixed nominal budget becomes minimum=maximum; no fabricated duration_ms.'
                if path in ('incomingLua.length','incomingLua.radius'):note+=' Positive radius replaces length area while length-derived needs_direction remains true. Cast center precedence is target, facing-adjacent position, caster.'
                if path=='mask.respawnType.underground':note+=' Bypasses period restriction underground when true; false does not forbid underground spawns.'
                if path=='mask.race':note+=' Source race maps to death residue/fluid behavior, not taxonomy.'
                rows.append({'source_path':path,'lines':lines,'status':status,'schema_destination':target,'note':note})
        result['sources'][name]={'repository':source['repository'],'revision':source['revision'],
            'mask_paths':len(source['mask_paths_with_lines']),'spell_paths':len(source['spell_paths_with_lines']),'rows':rows}
    result['accounted_paths']=sum(len(s['rows']) for s in result['sources'].values())
    result['passed']=not(result['unclassified'] or result['invalid_destinations'])
    return result


def fresh_inventory(census, roots):
    """Re-read exact pinned registrar and C++ API; static path proof only."""
    fresh = {'sources': {}}
    evidence = {}
    for name, original in census['sources'].items():
        root = roots[name]
        revision = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
        if revision != PINS[name]:
            raise ValueError(name + ': checkout does not match bound census revision')
        subprocess.run(['git', '-C', str(root), 'diff', '--quiet', 'HEAD', '--'], check=True)
        source = {k: original[k] for k in ('repository', 'revision', 'register_path')}
        source['revision'] = revision
        path = root / source['register_path']; text = path.read_text(encoding='utf-8')
        for variable, group in (('mask', 'mask_paths_with_lines'), ('incomingLua', 'spell_paths_with_lines')):
            rows = {}
            for match in re.finditer(r'\b' + variable + r'(?:\.[A-Za-z_]\w*)+', text):
                rows.setdefault(match.group(), []).append(text[:match.start()].count('\n')+1)
            source[group] = {key: sorted(set(lines)) for key, lines in sorted(rows.items())}
        api = {'MonsterType': set(), 'MonsterSpell': set()}
        api_files = []
        for pattern in ('monster_type_functions.cpp', 'monster_spell_functions.cpp'):
            for file in sorted((root / 'src').rglob(pattern)):
                body = file.read_text(encoding='utf-8')
                for cls, method in re.findall(r'registerMethod\(\s*(?:L\s*,\s*)?"(MonsterType|MonsterSpell)"\s*,\s*"(\w+)"', body):
                    api[cls].add(method)
                api_files.append({'file': str(file.relative_to(root)), 'sha256': hashlib.sha256(file.read_bytes()).hexdigest()})
        lua_declarations = {m.group(1): text[:m.start()].count('\n')+1 for m in
            re.finditer(r'(?:function\s+MonsterType[:.]|\bMonsterType\.)(\w+)\s*(?:\(|=\s*function)', text)}
        calls = []
        for match in re.finditer(r'\b(mtype|spell)\s*:\s*(\w+)\s*\(', text):
            cls = 'MonsterType' if match.group(1) == 'mtype' else 'MonsterSpell'
            calls.append({'class': cls, 'method': match.group(2), 'line': text[:match.start()].count('\n')+1,
                          'registered': match.group(2) in api[cls] or (cls == 'MonsterType' and match.group(2) in lua_declarations)})
        old = {p for g in ('mask_paths_with_lines', 'spell_paths_with_lines') for p in original[g]}
        observed = {p for g in ('mask_paths_with_lines', 'spell_paths_with_lines') for p in source[g]}
        evidence[name] = {'committed_census_revision': original['revision'], 'source_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                          'new_paths': sorted(observed-old), 'removed_paths': sorted(old-observed),
                          'registered_api': {k: sorted(v) for k,v in api.items()}, 'api_files': api_files,
                          'registrar_calls': calls, 'registrar_lua_methods': lua_declarations}
        fresh['sources'][name] = source
    return fresh, evidence


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path)
    parser.add_argument('--crystal', type=Path)
    parser.add_argument('--out', type=Path, default=ROOT/'formal-source-coverage.json')
    args = parser.parse_args()
    census = json.loads((ROOT/'field-census.json').read_text(encoding='utf-8'))
    evidence = None
    if args.canary or args.crystal:
        if not (args.canary and args.crystal):
            parser.error('fresh coverage requires both pinned source checkouts')
        census, evidence = fresh_inventory(census, {'canary': args.canary, 'crystal': args.crystal})
    result = verify_census(census)
    result['evidence_kind'] = 'FRESH_PINNED_REGISTRAR' if evidence else 'COMMITTED_CENSUS_ONLY'
    result['schema_complete'] = False
    if evidence is not None:
        result['fresh_source_evidence'] = evidence
        result['passed'] = result['passed'] and all(not s['new_paths'] and not s['removed_paths'] and
            s['api_files'] and all(c['registered'] for c in s['registrar_calls']) for s in evidence.values())
    args.out.write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({k: v for k,v in result.items() if k not in ('sources', 'fresh_source_evidence')}, ensure_ascii=False))
    raise SystemExit(not result['passed'])
