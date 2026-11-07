#!/usr/bin/env python3
"""Complete supported presentation fields from pinned donors without asset admission."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import complete_monster_audio_fields as audio


def names(text):
    """Lua string delimiters preserve apostrophes inside double-quoted names."""
    result = []
    for match in re.finditer(r'(\w+):name\(\s*(["\'])(.*?)\2\s*\)', text):
        variable, _, name = match.groups()
        if re.search(r'\b' + re.escape(variable) + r'\s*=\s*(?:Spell|RuneSpell)\(', text):
            result.append((variable, name))
    return result


def registry(donors):
    result = {}
    for repository, (folder, pin) in audio.PINS.items():
        root = donors / folder
        paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', pin], cwd=root, text=True).splitlines()
        for relative in paths:
            if '/scripts/' not in relative or not relative.endswith('.lua') or not (root / relative).is_file():
                continue
            for variable, name in names((root / relative).read_text()):
                result.setdefault((repository, audio.canary_batch.slug(name)), []).append((variable, relative))
    return result


def parent_key(key):
    return re.sub(r'/(?:component-\d+|variant-\d+|poff|teleport)$', '', key)


def source_for_ability(ability, repository, manifest, table):
    key = ability['identity']['key']
    base = parent_key(key)
    if '/spell/' in base:
        options = table.get((repository, base.split('/spell/', 1)[1]), [])
        # Native donor lookup explicitly prefers RuneSpell over same-name InstantSpell.
        rune_options = [r for r in options if '/scripts/runes/' in r[1]]
        if len(rune_options) == 1:
            return rune_options[0][1], base != key
        if len(options) == 1:
            return options[0][1], base != key
    # The authored familiar alias is backed by a specific manifest-bound spell file.
    if key == 'canary:ability/spell/summon_monk_familiar':
        files = {r.get('source_file') for r in manifest['entries']
                 if r.get('source_file', '').endswith('/spells/familiar/monk_familiar.lua')}
        if len(files) == 1:
            return files.pop(), False
    return None


def source_audio(text, ids):
    """No implicit donor defaults apply to registered custom spells."""
    result = {}
    for field, method in [('cast_cue', 'castSound'), ('impact_cue', 'impactSound')]:
        calls = re.findall(r':' + method + r'\(([^)]*)\)', text)
        if not calls:
            continue
        if len(set(calls)) != 1 or calls[0].strip() not in ids:
            return None
        value = ids[calls[0].strip()]
        if value:
            result[field] = value
    if re.search(r'(?:send\w*SoundEffect|COMBAT_PARAM_[A-Z_]*SOUND)', text):
        return None
    return result


def native_audio_row(ability, dependencies):
    refs = ability.get('effects',[])
    effects = [e for e in dependencies['effects'] if any(e['identity']['key']==r['key'] for r in refs) and e.get('operation')=='damage']
    if len(effects)!=1 or effects[0].get('damage_type') not in ('energy','earth'):
        return None
    row={'name':'combat','type':'@COMBAT_'+effects[0]['damage_type'].upper()+'DAMAGE'}
    area=ability.get('area',{})
    if area.get('shape')=='circle': row['radius']=area['radius_tiles']
    if area.get('shape')=='beam':
        row['length']=area['length_tiles']; row['spread']=area.get('spread_tiles',0)
    return row


def proof(repository, relative, donors, status):
    folder, pin = audio.PINS[repository]
    path = audio.verify_pinned_file(donors / folder, pin, relative)
    return {'repository': repository, 'pin': pin, 'file': relative,
            'sha256': audio.sha(path), 'lines': [], 'status': status}


def patch(monster, file, pointer, old, new, source, reason, present=True):
    return {'monster': monster, 'file': file, 'pointer': pointer,
            'expected_present': present, 'expected_value': old if present else None,
            'value': new, 'source': source, 'reason': reason}


def build(baseline, donors, output):
    baseline, donors, output = baseline.resolve(), donors.resolve(), output.resolve()
    if output == baseline or baseline in output.parents or audio.ROOT == output or audio.ROOT in output.parents:
        raise ValueError('Output must be outside immutable authoring and baseline')
    index_path = baseline / 'population-index.json'
    index = json.loads(index_path.read_text())
    table = registry(donors)
    defaults = {}
    for repo, (folder, pin) in audio.PINS.items():
        root = donors / folder
        loader = audio.verify_pinned_file(root, pin, 'data/scripts/lib/register_monster_type.lua')
        header = audio.verify_pinned_file(root, pin, 'src/creatures/creatures_definitions.hpp')
        defaults[repo] = audio.SoundDefaults(loader, header)
    patches, receipts, remaining, flags = [], [], [], {}
    counts = Counter()
    source_cache = {}
    for entry in index['monsters']:
        name = entry['monster']; directory = baseline / 'bundles' / name
        monster, deps, catalog, manifest = [json.loads((directory / f).read_text()) for f in
                                           ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')]
        repository, source_path, relative = audio.choose_donor(entry, manifest, donors)
        donor_proof = proof(repository, relative, donors, 'SOURCE_PRESENTATION_CONFIGURATION')
        if source_path not in source_cache:
            errors = []; _, value, _ = audio.canary_batch.load_monster(source_path, errors)
            source_cache[source_path] = value, errors
        value, errors = source_cache[source_path]
        actor_flags = set()
        # Explicit sound table absent AND pinned native defaults prove these donor defaults.
        sounds = value.get('sounds')
        if sounds or re.search(r'\bmonster\.sounds\s*=', source_path.read_text()):
            remaining.append({'monster': name, 'field': 'periodic/death audio', 'reason': 'EXPLICIT_DONOR_SOUND_TABLE_REQUIRES_CONVERSION'})
        else:
            header_rel = 'src/creatures/monsters/monsters.hpp'
            hp = proof(repository, header_rel, donors, 'DONOR_NATIVE_AUDIO_DEFAULTS')
            header = (donors / audio.PINS[repository][0] / header_rel).read_text()
            hp['lines'] = [i+1 for i,line in enumerate(header.splitlines()) if any(s in line for s in ('soundChance = 0', 'soundSpeedTicks = 0', 'deathSound = SoundEffect_t::SILENCE', 'std::vector<SoundEffect_t> soundVector'))]
            assert len(hp['lines']) == 4, 'Missing concrete native default proof'
            receipts.append({'monster': name, 'fields': ['behavior.periodic_audio', 'presentation.audio.death'],
                             'status': 'DONOR_NOT_CONFIGURED_PERIODIC_ZERO_DEATH_SILENCE',
                             'source': donor_proof, 'native_defaults': hp, 'global_absence_asserted': False})
            counts['periodic_death_positive_default_proofs'] += 1
        cues, events = {}, set()
        literal_rows = {}
        for section in ('attacks', 'defenses'):
            source_rows = audio.source_rows(value.get(section))
            for row_index, row in enumerate(source_rows):
                literal_rows[f'canary:ability/{name}/{section[:-1]}-{row_index+1}'] = row
                for mapped in manifest['entries']:
                    if mapped.get('source_field') != f'{section}[{row_index+1}]' or not mapped.get('destination','').startswith('/monster/behavior/'+section+'/'): continue
                    try:
                        native = monster['behavior'][section][int(mapped['destination'].split('/')[4])]['ability']['key']
                        literal_rows[native] = row
                    except (KeyError, IndexError, ValueError): pass
                for candidate in deps['abilities']:
                    tail = candidate['identity']['key'].split('/field-fill/')[-1].split('/field-fill-')[-1]
                    index_match = re.fullmatch(section+r'-(\d+)(?:-stage-\d+)?',tail)
                    if (index_match and int(index_match[1]) == row_index+1) or tail == audio.canary_batch.slug(row.get('name','')):
                        literal_rows[candidate['identity']['key']] = row
        for i, ability in enumerate(deps['abilities']):
            if ability.get('audio'):
                continue
            key = ability['identity']['key']
            resolved = source_for_ability(ability, repository, manifest, table)
            if not resolved:
                row = literal_rows.get(key)
                estimated = False
                if not row and name == 'dark_merudri':
                    row = native_audio_row(ability, deps)
                    estimated = row is not None
                if row and row.get('name') not in audio.STANDARD:
                    candidates = table.get((repository, audio.canary_batch.slug(row.get('name',''))),[])
                    if len(candidates) == 1: resolved = (candidates[0][1], '/stage-' in key or '-stage-' in key)
                if not resolved and row and row.get('name') in audio.STANDARD:
                    choices = defaults[repository].choices(row, value.get('flags',{}).get('targetDistance',1))
                    source = dict(donor_proof, source_field=row.get('name'), sound_rule=defaults[repository].proof)
                    if estimated:
                        source['status']='OWNER_ACCEPTED_NON_GLOBAL_NATIVE_PROFILE_AUDIO_ESTIMATE'
                        source['native_inputs']=row
                        source['native_ability']=key
                        source['global_parity']=False
                        actor_flags.add('OWNER_ACCEPTED_NON_GLOBAL_NATIVE_PROFILE_AUDIO_ESTIMATE')
                        counts['native_profile_sound_estimates'] += 1
                    selected = {}
                    for field, options in choices.items():
                        if options:
                            number = options[0]
                            if len(options)>1:
                                number, policy = audio.deterministic_selection(monster['creature']['identity']['key'],audio.PINS[repository][1],options)
                                source.setdefault('deterministic_selection',{})[field]=policy
                                actor_flags.add('OWNER_ACCEPTED_NON_GLOBAL_SOURCE_SOUND_SELECTION')
                            selected[field]=f'{audio.PINS[repository][0]}.sound:effect/{number}'
                    if selected:
                        correction=audio.routing_correction({'explicit_impact':'impactCast' in row},selected,defaults[repository].proof)
                        if correction:
                            source['routing_correction']=correction
                            actor_flags.add('SOURCE_NON_GLOBAL_AUDIO_ROUTING_CORRECTION')
                        patches.append(patch(name,'dependencies.json',f'/abilities/{i}/audio',ability.get('audio'),selected,source,'EXACT_LITERAL_SOURCE_AUDIO_FOR_NEW_NATIVE_ABILITY','audio' in ability))
                        for field,cue in selected.items(): cues[cue]=source;events.add((field.replace('_cue',''),cue))
                        counts['literal_source_ability_audio_fills'] += 1
                    else:
                        receipts.append({'monster':name,'ability':key,'status':'EXACT_LITERAL_SOURCE_AUDIO_DEFAULT_SILENCE','source':source,'global_absence_asserted':False})
                    continue
                if not resolved:
                    remaining.append({'monster':name,'ability':key,'reason':'NATIVE_ABILITY_HAS_NO_EXACT_SOURCE_AUDIO_BINDING','architecture_issue':162})
                    continue
            relative_spell, child = resolved
            source = proof(repository, relative_spell, donors, 'REGISTERED_SOURCE_AUDIO_CONFIGURATION')
            text = (donors / audio.PINS[repository][0] / relative_spell).read_text()
            values = source_audio(text, defaults[repository].ids)
            source['lines'] = [n+1 for n,line in enumerate(text.splitlines()) if ':name(' in line or ':castSound(' in line or ':impactSound(' in line]
            if values is None and key == 'canary:ability/spell/summon_monk_familiar' and repository == 'zimbadev/crystalserver':
                fallback_repo = 'opentibiabr/canary'
                fallback_source = proof(fallback_repo, relative_spell, donors, 'NON_GLOBAL_DONOR_UNDEFINED_SOUND_ENUM_CORRECTION')
                fallback_text = (donors / 'canary' / relative_spell).read_text()
                if 'local spellId = 282' in text and 'local spellId = 282' in fallback_text:
                    values = source_audio(fallback_text, defaults[fallback_repo].ids)
                    fallback_source['corrected_donor_source'] = source
                    fallback_source['lines'] = [n+1 for n,line in enumerate(fallback_text.splitlines()) if ':castSound(' in line or 'spellId = 282' in line]
                    source = fallback_source
                    actor_flags.add('NON_GLOBAL_DONOR_UNDEFINED_SOUND_ENUM_CORRECTION')
                    counts['undefined_donor_enum_source_corrections'] += 1
            if values is None:
                remaining.append({'monster': name, 'ability': key, 'reason': 'DYNAMIC_SOURCE_AUDIO_NOT_STATICALLY_EXPRESSIBLE', 'source': source})
                continue
            if not values:
                receipts.append({'monster': name, 'ability': key, 'status': 'EXACT_REGISTERED_SOURCE_NO_AUDIO_CONFIGURATION', 'source': source, 'global_absence_asserted': False})
                counts['registered_positive_no_audio_proofs'] += 1
                continue
            if child:
                receipts.append({'monster': name, 'ability': key, 'status': 'DERIVED_COMPONENT_INHERITS_SOURCE_PARENT_AUDIO', 'source': source})
                continue
            new = {field: f"{audio.PINS[source['repository']][0]}.sound:effect/{number}" for field,number in values.items()}
            patches.append(patch(name, 'dependencies.json', f'/abilities/{i}/audio', ability.get('audio'), new, source,
                                 'EXACT_MANIFEST_OR_REGISTERED_SOURCE_SOUND_IDS', 'audio' in ability))
            for field,cue in new.items():
                cues[cue] = source; events.add((field.replace('_cue',''), cue))
            counts['ability_audio_fills'] += 1
        # Donor flat attack/defense visual fields map to the referenced native Effect only.
        effects = {e['identity']['key']: (i,e) for i,e in enumerate(deps['effects'])}
        abilities = {a['identity']['key']: a for a in deps['abilities']}
        for section in ('attacks', 'defenses'):
            rows = audio.source_rows(value.get(section)) or audio.static_sound_rows(source_path.read_text(), section)
            for n,row in enumerate(rows):
                schedule = monster['behavior'].get(section, [])
                mappings = [r for r in manifest['entries'] if r.get('source_field') == f'{section}[{n+1}]'
                            and r.get('source_file') == relative and r.get('destination','').startswith('/monster/behavior/'+section+'/')]
                keys = {f'canary:ability/{name}/{section[:-1]}-{n+1}'}
                for mapping in mappings:
                    try: keys.add(schedule[int(mapping['destination'].split('/')[4])]['ability']['key'])
                    except (KeyError, ValueError, IndexError): pass
                for key in keys:
                    refs = abilities.get(key,{}).get('effects', [])
                    if len(refs) != 1 or refs[0]['key'] not in effects: continue
                    effect_index,effect = effects[refs[0]['key']]
                    old = effect.get('presentation',{}); new = dict(old)
                    for sf,native,prefix,kind in [('effect','impact_asset_binding','CONST_ME_','effect'),('shootEffect','projectile_asset_binding','CONST_ANI_','missile')]:
                        symbol = row.get(sf)
                        if isinstance(symbol,str) and symbol.startswith('@'+prefix) and symbol not in ('@CONST_ME_NONE','@CONST_ANI_NONE'):
                            binding = 'canary.appearance:'+kind+'/'+symbol[len(prefix)+1:].lower()
                            if native not in old: new[native] = binding
                            elif old[native] != binding:
                                remaining.append({'monster':name,'effect':effect['identity']['key'],'field':native,'reason':'EXISTING_BINDING_DIFFERS_FROM_EXACT_DONOR','kind':'SOURCE_CONFLICT_NOT_MISSING_FIELD','blocks_authoring':False,'existing':old[native],'source_value':binding,'source':donor_proof})
                            if native in old and old[native] != binding:
                                actor_flags.add('PREEXISTING_VISUAL_BINDING_SOURCE_DIFFERENCE_RETAINED')
                            counts['source_visual_fields_checked'] += 1
                    if new != old:
                        patches.append(patch(name,'dependencies.json',f'/effects/{effect_index}/presentation',effect.get('presentation'),new,donor_proof,'EXACT_STATIC_DONOR_VISUAL_BINDINGS', 'presentation' in effect))
                        for binding in new.values(): cues.setdefault(binding,donor_proof)
                        counts['effect_visual_fills'] += 1
        outfit = value.get('outfit', {})
        expected_attachments = []
        addons = outfit.get('lookAddons', 0)
        look = outfit.get('lookType', 0)
        appearance = monster['presentation']['appearance']
        same_source_appearance = appearance.get('asset_binding') == f'canary.appearance:outfit/{look}'
        if type(addons) is int and type(look) is int and look > 0 and same_source_appearance:
            for bit in (1, 2):
                if addons & bit:
                    expected_attachments.append({'slot':'addon','asset_binding':f'canary.appearance:outfit/{look}/addon-{bit}'})
        mount = outfit.get('lookMount', 0)
        if type(mount) is int and mount > 0 and same_source_appearance:
            expected_attachments.append({'slot':'mount','asset_binding':f'canary.appearance:outfit/{mount}'})
        old_attachments = monster['presentation']['appearance']['attachment_bindings']
        attachment_additions = [b for b in expected_attachments if b not in old_attachments]
        counts['source_attachment_bindings_checked'] += len(expected_attachments)
        if attachment_additions:
            patches.append(patch(name,'monster.json','/presentation/appearance/attachment_bindings',old_attachments,old_attachments+attachment_additions,donor_proof,'EXACT_SOURCE_OUTFIT_ADDON_OR_MOUNT_BINDING'))
            for binding in attachment_additions: cues[binding['asset_binding']]=donor_proof
            counts['attachment_bindings_filled'] += len(attachment_additions)
        if cues:
            old = catalog.get('assets',[]); new = old + sorted(set(cues)-set(old))
            if new != old: patches.append(patch(name,'catalog.json','/assets',old,new,donor_proof,'SOURCE_ASSET_DECLARATIONS_NOT_ADMITTED'))
            if new != old: patches[-1]['cue_sources'] = cues
            old = monster['presentation']['audio']['event_bindings']; existing={(x['event'],x['cue_id']) for x in old}
            new = old + [{'event':event,'cue_id':cue,'asset_binding':cue} for event,cue in sorted(events-existing)]
            if new != old: patches.append(patch(name,'monster.json','/presentation/audio/event_bindings',old,new,donor_proof,'SOURCE_AUDIO_EVENT_BINDINGS_RUNTIME_UNVERIFIED'))
            if new != old: patches[-1]['cue_sources'] = cues
            actor_flags.add('SOURCE_PRESENTATION_ASSET_DECLARATIONS_UNADMITTED')
            if events:
                actor_flags.update(['SOURCE_AUDIO_CUE_DECLARED_ASSET_UNADMITTED','AUDIO_RUNTIME_UNVERIFIED'])
        # Source attachment options are conditional, no unsourced accessories are invented.
        outfit = value.get('outfit', {})
        receipts.append({'monster':name,'fields':['appearance.attachments','appearance.visual_effect_bindings'],
                         'status':'SOURCE_OUTFIT_OPTIONAL_FIELDS_INSPECTED','source':donor_proof,
                         'lookAddons':outfit.get('lookAddons',0),'lookMount':outfit.get('lookMount',0),
                         'same_source_appearance':same_source_appearance,
                         'template_addons_not_transferred_to_different_native_appearance':not same_source_appearance,
                         'visual_effect_not_inferred_from_attack_effects':True})
        flags[name] = sorted(actor_flags)
    unique_patches = {}
    for item in patches:
        address = (item['monster'], item['file'], item['pointer'])
        if address in unique_patches and unique_patches[address]['value'] != item['value']:
            raise ValueError('Conflicting presentation patches: ' + repr(address))
        unique_patches.setdefault(address, item)
    patches = list(unique_patches.values())
    counts.update(actors=len(index['monsters']),total_field_patches=len(patches),remaining=len(remaining))
    packet={'schema':'OTERYN_MONSTER_FIELD_PATCH/v1','lane':'presentation-optional','baseline_index_sha256':audio.sha(index_path),
            'patches':patches,'actor_flags':flags,'conditional_field_proofs':receipts,'remaining':remaining,
            'counts':dict(counts),'runtime_qualified':False,'audio_assets_admitted':False}
    output.mkdir(parents=True,exist_ok=True)
    (output/'field-patches.json').write_text(json.dumps(packet,indent=2,sort_keys=True)+'\n')
    print(json.dumps({'counts':dict(counts),'sha256':audio.sha(output/'field-patches.json')}))
    return packet


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('baseline','donors','output'): parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args(); build(args.baseline,args.donors,args.output)
